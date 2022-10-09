use std::{
    io::{self, Write},
    path::PathBuf,
};

use argh::FromArgs;
use crossterm::{
    cursor,
    event::{self, Event},
    queue,
    style::{self, Color},
    terminal::{self, ClearType},
};
use rand::seq::SliceRandom;
use text_box::{BoxOutline, TextBox};

use crate::{
    flashcards::{Flashcard, FlashcardText, RecallSettings, Set, Side},
    load_set,
    output::{self, len_base10, text_box, MultiTextBox, Repeat, TerminalSettings},
    vec2::Vec2,
};

/// Learn a set
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "learn")]
pub struct Entry {
    /// the set to learn
    #[argh(positional)]
    set: PathBuf,
}

const COLORS: [Color; 4] = [
    Color::Black,
    Color::DarkRed,
    Color::DarkYellow,
    Color::DarkGreen,
];

impl Entry {
    pub fn run(self) {
        let set = load_set!(&self.set);
        if set.cards.is_empty() {
            output::write_fatal_error("Set must have at least 1 card to learn");
            return;
        }
        let mut cards = CardList::from_set(&set);
        let mut term_size: Vec2<_> = terminal::size()
            .expect("unable to get terminal size")
            .into();
        let mut term_settings = TerminalSettings::new();
        term_settings
            .enter_alternate_screen()
            .enable_raw_mode()
            .hide_cursor();
        let mut asker = Asker::new(term_size);

        while let Some(card) = cards.get_unstudied() {
            match card {
                AskerData::Matching {
                    question,
                    answers,
                    correct_answer,
                } => {
                    asker.draw_matching(question, answers);
                    cards.print_footer(term_size);
                    io::stdout().flush().unwrap();
                    loop {
                        match event::read().expect("Unable to read event") {
                            crate::esc!() => panic!("Exited app"),
                            Event::Resize(w, h) => {
                                queue!(io::stdout(), terminal::Clear(ClearType::All)).unwrap();
                                if w < 24 || h < 24 {
                                    continue;
                                }
                                term_size = Vec2::new(w, h);
                                asker.resize_to(term_size);
                                asker.draw_matching(question, answers);
                                cards.print_footer(term_size);
                                io::stdout().flush().unwrap();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        io::stdin().read_line(&mut String::new()).unwrap();
        drop(term_settings);
    }
}

#[derive(Debug)]
struct CardList<'a> {
    cards: Vec<CardListItem<'a>>,
    set: &'a Set,
}

#[derive(Debug)]
struct CardListItem<'a> {
    card: &'a Flashcard,
    side: Side,
    next_study_type: StudyType,
    footer_color: u8,
}

impl<'a> CardList<'a> {
    fn from_set(set: &'a Set) -> Self {
        let count = [set.recall_t.is_used(), set.recall_d.is_used()]
            .into_iter()
            .filter(|b| *b)
            .count();
        let mut v = Vec::with_capacity(count * set.cards.len());
        if set.recall_t.is_used() {
            let next_study_type = if set.recall_t.matching {
                StudyType::Matching(0)
            } else {
                StudyType::Text(0)
            };
            v.extend(set.cards.iter().map(|card| CardListItem {
                card,
                side: Side::Definition,
                next_study_type,
                footer_color: 0,
            }));
        }
        if set.recall_d.is_used() {
            let next_study_type = if set.recall_d.matching {
                StudyType::Matching(0)
            } else {
                StudyType::Text(0)
            };
            v.extend(set.cards.iter().map(|card| CardListItem {
                card,
                side: Side::Term,
                next_study_type,
                footer_color: 0,
            }));
        }
        Self { cards: v, set }
    }

    fn print_footer(&self, term_size: Vec2<u16>) {
        let mut counts = [0; COLORS.len()];
        for item in self.cards.iter() {
            counts[item.footer_color as usize] += 1;
        }

        let sum = counts.iter().sum::<u16>() as f32;
        let fractions = counts.map(|c| c as f32 / sum);
        let mut widths = fractions.map(|f| (f * term_size.x as f32) as u16);
        widths[0] = term_size.x - widths[1..].iter().sum::<u16>();

        queue!(io::stdout(), cursor::MoveTo(0, term_size.y - 1)).unwrap();
        for ((count, width), color) in counts.into_iter().zip(widths).zip(COLORS).rev() {
            let len_base10_u16 = len_base10(count);
            if count > 0 && len_base10_u16 <= width {
                let remaining_len = width - len_base10_u16;
                let before_len = remaining_len / 2;
                let after_len = remaining_len - before_len;
                queue!(
                    io::stdout(),
                    style::SetBackgroundColor(color),
                    style::Print(Repeat(' ', before_len)),
                    style::Print(count),
                    style::Print(Repeat(' ', after_len)),
                )
                .unwrap();
            } else {
                queue!(
                    io::stdout(),
                    style::SetBackgroundColor(color),
                    style::Print(Repeat(' ', width)),
                )
                .unwrap();
            }
        }
        queue!(io::stdout(), style::SetBackgroundColor(Color::Reset)).unwrap();
    }

    fn get_unstudied(&self) -> Option<AskerData> {
        let mut rng = rand::thread_rng();
        self.cards
            .choose(&mut rng)
            .map(|card| match card.next_study_type {
                StudyType::Matching(_) => {
                    let correct_answer = &card.card[!card.side];
                    let mut answers = [""; 4];
                    answers[0] = correct_answer.display();
                    for i in 1..4 {
                        for _ in 0..12 {
                            answers[i] =
                                self.set.cards.choose(&mut rng).unwrap()[!card.side].display();
                            if !answers[..i].contains(&answers[i]) {
                                break;
                            }
                        }
                    }
                    answers.shuffle(&mut rng);
                    AskerData::Matching {
                        question: card.card[card.side].display(),
                        answers,
                        correct_answer,
                    }
                }
                StudyType::Text(_) => todo!(),
            })
    }

    fn recall_settings(&self, side: Side) -> RecallSettings {
        match side {
            Side::Term => self.set.recall_t,
            Side::Definition => self.set.recall_d,
        }
    }
}

#[derive(Debug)]
struct Asker {
    question_box: TextBox,
    matching_answers_box: MultiTextBox,
}

impl Asker {
    fn new(term_size: Vec2<u16>) -> Self {
        let mut this = Self {
            question_box: TextBox::new(),
            matching_answers_box: MultiTextBox::new(),
        };
        this.question_box.outline(Some(BoxOutline::DOUBLE)).y(2);
        this.matching_answers_box
            .x(4)
            .box_count(Vec2::new(4, 1))
            .number(true);
        this.resize_to(term_size);
        this
    }

    fn resize_to(&mut self, term_size: Vec2<u16>) -> &mut Self {
        let inner_y = term_size.y - 7;
        let box_height = inner_y / 2;
        self.question_box
            .width(term_size.x / 3)
            .x(term_size.x / 3)
            .height(box_height);
        self.matching_answers_box
            .width(term_size.x - 8)
            .height(box_height)
            .y(term_size.y - 3 - box_height);
        self
    }

    pub fn draw_matching(&self, question: &str, answers: [&str; 4]) -> &Self {
        self.question_box.draw_outline_and_text(question);
        self.matching_answers_box.draw_outline().draw_text(answers);
        self
    }
}

#[derive(Debug)]
enum AskerData<'a> {
    /// Layout:
    /// 2 lines blank
    /// 1/2 question box
    /// 2+ lines blank
    /// 1/2 answers box
    /// 2 lines blank
    /// 1 line footer
    Matching {
        question: &'a str,
        answers: [&'a str; 4],
        correct_answer: &'a FlashcardText,
    },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum StudyType {
    Matching(u8),
    Text(u8),
}
