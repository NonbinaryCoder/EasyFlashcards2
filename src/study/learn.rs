use std::{
    array,
    io::{self, Write},
    path::PathBuf,
};

use argh::FromArgs;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    queue,
    terminal::{self, ClearType},
};
use rand::{seq::SliceRandom, Rng};

use crate::{
    flashcards::{Flashcard, RecallSettings, Set, Side},
    load_set,
    output::{
        self,
        text_box::{MultiOutlineType, MultiTextBox, OutlineType, TextBox},
        TerminalSettings,
    },
    vec2::{Rect, Vec2},
};

/// Learn a set
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "learn")]
pub struct Entry {
    /// the set to learn
    #[argh(positional)]
    set: PathBuf,
}

impl Entry {
    pub fn run(self) {
        let set = load_set!(&self.set);
        if set.cards.is_empty() {
            output::write_fatal_error("Set must have at least 1 card to learn");
            return;
        }
        let mut cards = CardList::from_set(&set);

        let mut term_settings = TerminalSettings::new();
        term_settings
            .enable_raw_mode()
            .enter_alternate_screen()
            .hide_cursor();

        let mut question_box;
        let mut matching_answers_boxes: MultiTextBox<_, 4>;
        {
            let term_size: Vec2<_> = terminal::size()
                .expect("unable to get terminal size")
                .into();
            let height_minus_padding = term_size.y.saturating_sub(7);
            let box_height = height_minus_padding / 2;

            question_box = TextBox::from_fn(
                Rect {
                    size: Vec2::new(term_size.x / 3, box_height),
                    pos: Vec2::new(term_size.x / 3, 2),
                },
                |updater| {
                    updater.set_outline(OutlineType::DOUBLE);
                },
            );

            matching_answers_boxes = MultiTextBox::new(Rect {
                size: Vec2::new(term_size.x.saturating_sub(8), box_height),
                pos: Vec2::new(4, term_size.y.saturating_sub(box_height).saturating_sub(3)),
            })
        }

        while cards.select_next() {
            let &CardListItem {
                card,
                side,
                next_study_type,
            } = cards.selected();
            match next_study_type {
                StudyType::Matching(studied_before) => {
                    question_box.update(|updater| {
                        updater.set_text(card[side].display().clone());
                    });
                    matching_answers_boxes.update(|updater| {
                        updater.set_outline(MultiOutlineType::DOUBLE_LIGHT);

                        let mut answers: [_; 4] = array::from_fn(|_| None);
                        answers[0] = Some(card[!side].display().clone());
                        for i in 1..4 {
                            for _ in 0..12 {
                                answers[i] = Some(
                                    set.cards.choose(&mut rand::thread_rng()).unwrap()[!side]
                                        .display()
                                        .clone(),
                                );
                                if !answers[..i].contains(&answers[i]) {
                                    break;
                                }
                            }
                        }
                        let mut answers = answers.map(Option::unwrap);
                        answers.shuffle(&mut rand::thread_rng());

                        for (index, text) in answers.into_iter().enumerate() {
                            updater.text(index).set(text);
                        }
                    });
                    io::stdout().flush().unwrap();
                    loop {
                        match event::read().unwrap() {
                            Event::Key(KeyEvent {
                                code: KeyCode::Char('c'),
                                modifiers,
                                ..
                            }) if modifiers.contains(KeyModifiers::CONTROL) => {
                                term_settings.clear();
                                panic!("Exited with ctrl-c");
                            }
                            Event::Resize(x, y) => {
                                let term_size: Vec2<_> = terminal::size()
                                    .expect("unable to get terminal size")
                                    .into();
                                let height_minus_padding = term_size.y.saturating_sub(7);
                                let box_height = height_minus_padding / 2;

                                queue!(io::stdout(), terminal::Clear(ClearType::All)).unwrap();

                                question_box.force_move_resize(Rect {
                                    size: Vec2::new(term_size.x / 3, box_height),
                                    pos: Vec2::new(term_size.x / 3, 2),
                                });

                                matching_answers_boxes.force_move_resize(Rect {
                                    size: Vec2::new(term_size.x.saturating_sub(8), box_height),
                                    pos: Vec2::new(
                                        4,
                                        term_size.y.saturating_sub(box_height).saturating_sub(3),
                                    ),
                                });

                                io::stdout().flush().unwrap();
                            }
                            _ => break,
                        }
                    }
                }
                _ => todo!(),
            }
        }

        term_settings.clear();
    }
}

struct CardListItem<'a> {
    card: &'a Flashcard,
    side: Side,
    next_study_type: StudyType,
}

struct CardList<'a> {
    cards: Vec<CardListItem<'a>>,
    set: &'a Set,
    selected_index: usize,
}

impl<'a> CardList<'a> {
    fn from_set(set: &'a Set) -> Self {
        let count = [set.recall_t.is_used(), set.recall_d.is_used()]
            .into_iter()
            .filter(|b| *b)
            .count();
        let mut cards = Vec::with_capacity(count * set.cards.len());

        let mut extend_cards = |recall_settings: &RecallSettings, side| {
            let next_study_type = match *recall_settings {
                RecallSettings {
                    matching: true,
                    text: true,
                } => StudyType::Matching(true),
                RecallSettings {
                    matching: true,
                    text: false,
                } => StudyType::Matching(false),
                RecallSettings {
                    matching: false,
                    text: true,
                } => StudyType::Text(false),
                RecallSettings {
                    matching: false,
                    text: false,
                } => return,
            };
            cards.extend(set.cards.iter().map(|card| CardListItem {
                card,
                side,
                next_study_type,
            }))
        };

        extend_cards(&set.recall_t, Side::Term);
        extend_cards(&set.recall_d, Side::Definition);

        Self {
            cards,
            set,
            selected_index: usize::MAX,
        }
    }

    fn select_next(&mut self) -> bool {
        if self.cards.is_empty() {
            false
        } else {
            let mut index = self.selected_index;
            let mut counter = 0;
            while index == self.selected_index && counter < 12 {
                index = rand::thread_rng().gen_range(0..self.cards.len());
            }
            self.selected_index = index;
            true
        }
    }

    fn selected(&self) -> &CardListItem {
        &self.cards[self.selected_index]
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum StudyType {
    Matching(bool),
    Text(bool),
}

impl StudyType {
    fn index(self) -> usize {
        match self {
            StudyType::Matching(false) => 0,
            StudyType::Matching(true) => 1,
            StudyType::Text(false) => 2,
            StudyType::Text(true) => 3,
        }
    }
}
