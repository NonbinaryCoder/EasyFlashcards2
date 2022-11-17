use std::{
    fmt::Display,
    io::{self, Write},
    ops::{Add, ControlFlow},
    rc::Rc,
};

use crossterm::{
    event::{KeyCode, KeyEvent},
    execute, queue,
    style::{self, Color, Stylize},
    terminal::{self, ClearType},
};

use crate::{
    flashcards::{Set, Side},
    input::text::TextInput,
    output::{
        text_box::{MultiOutlineType, MultiTextBox, OutlineType, TextBox},
        Proportion, TerminalSettings,
    },
    study::learn::world::card_list::fail_count::FailCount,
    vec2::{Rect, Vec2},
};

use self::{
    card_list::{CardList, StudyType},
    footer::Footer,
};

mod card_list;
mod footer;

thread_local! {
    static CORRECT: Rc<str> = Rc::from("Correct!  Press any key to continue");
    static TEXT_FAILED: Rc<str> = Rc::from("Incorrect!  Shift-Tab: typo, I know this");
}

#[derive(Debug)]
pub struct World<'a> {
    question_box: TextBox<Rc<str>>,
    matching_answers_boxes: MultiTextBox<Rc<str>, 4>,
    text_input: TextInput,
    footer: Footer,
    card_list: CardList<'a>,
    term_settings: TerminalSettings,
    matches_made: [u32; 2],
    texts_entered: [u32; 2],
    typ: Option<WorldType>,
}

#[derive(Debug)]
enum WorldType {
    Matching {
        studying: card_list::Token,
        failed: bool,
    },
    Text {
        studying: card_list::Token,
    },
    TextFailed {
        studying: card_list::Token,
        hidden_question: Option<Rc<str>>,
    },
    WaitingForKeypress,
}

impl<'a> World<'a> {
    #[must_use]
    pub fn new(size: Vec2<u16>, set: &'a Set, term_settings: TerminalSettings) -> Self {
        let card_list = CardList::from_set(set);

        let height_minus_padding = size.y.saturating_sub(7);
        let box_height = height_minus_padding / 2;
        let bottom_box_y = size.y.saturating_sub(box_height).saturating_sub(3);

        let mut this = World {
            question_box: TextBox::from_fn(
                Rect {
                    size: Vec2::new(size.x / 3, box_height),
                    pos: Vec2::new(size.x / 3, 2),
                },
                |updater| {
                    updater.set_outline(OutlineType::DOUBLE);
                },
            ),
            matching_answers_boxes: MultiTextBox::new(Rect {
                size: Vec2::new(size.x.saturating_sub(8), box_height),
                pos: Vec2::new(4, bottom_box_y),
            }),
            text_input: TextInput::new(Rect {
                size: Vec2::new(size.x / 3, box_height),
                pos: Vec2::new(size.x / 3, bottom_box_y),
            }),
            footer: Footer::new(card_list.remaining_unstudied() as u32, size),
            card_list,
            term_settings,
            matches_made: [0, 0],
            texts_entered: [0, 0],
            typ: None,
        };

        this.study_next();

        this
    }

    pub fn study_next(&mut self) -> bool {
        let res = match self.card_list.next_unstudied(None) {
            Some(item) => {
                let (item, token) = item.tup();
                match item.next_study_type {
                    StudyType::Matching(_) => {
                        self.text_input.hide();

                        self.question_box.update(|updater| {
                            updater
                                .set_text(item.card[!item.side].display().clone())
                                .set_text_color(Color::White);
                        });
                        self.matching_answers_boxes.update(|updater| {
                            for (i, answer) in self
                                .card_list
                                .matching_answers_for(item)
                                .into_iter()
                                .enumerate()
                            {
                                updater.text(i).set(answer).set_color(Color::White);
                            }
                            updater.set_outline(MultiOutlineType::DOUBLE_LIGHT);
                        });

                        self.typ = Some(WorldType::Matching {
                            studying: token,
                            failed: false,
                        });
                    }
                    StudyType::Text(_) => {
                        self.matching_answers_boxes.hide();

                        self.question_box.update(|updater| {
                            updater
                                .set_text(item.card[!item.side].display().clone())
                                .set_text_color(Color::White);
                        });
                        self.text_input.update(|updater| {
                            updater
                                .set_outline(OutlineType::DOUBLE)
                                .clear_text()
                                .clear_correct_answer();
                        });
                        self.term_settings.show_cursor();
                        self.text_input.go_to_cursor();

                        self.typ = Some(WorldType::Text { studying: token });
                    }
                }
                true
            }
            None => {
                self.typ = None;
                false
            }
        };
        io::stdout().flush().unwrap();
        res
    }

    pub fn resize(&mut self, size: Vec2<u16>) {
        let height_minus_padding = size.y.saturating_sub(7);
        let box_height = height_minus_padding / 2;
        let bottom_box_y = size.y.saturating_sub(box_height).saturating_sub(3);

        queue!(io::stdout(), terminal::Clear(ClearType::All)).unwrap();

        self.question_box.force_move_resize(Rect {
            size: Vec2::new(size.x / 3, box_height),
            pos: Vec2::new(size.x / 3, 2),
        });

        self.matching_answers_boxes.force_move_resize(Rect {
            size: Vec2::new(size.x.saturating_sub(8), box_height),
            pos: Vec2::new(4, bottom_box_y),
        });

        self.text_input.force_move_resize(Rect {
            size: Vec2::new(size.x / 3, box_height),
            pos: Vec2::new(size.x / 3, bottom_box_y),
        });

        self.footer.resize(size);

        io::stdout().flush().unwrap();
    }

    pub fn key_pressed(&mut self, event: KeyEvent) -> ControlFlow<()> {
        let code = event.code;
        match self.typ {
            Some(WorldType::Matching {
                studying,
                ref mut failed,
            }) => {
                if let KeyCode::Char(key) = code {
                    if ('1'..='4').contains(&key) {
                        let index = key as usize - '1' as usize;
                        let card = &self.card_list[studying];
                        let text_color;

                        if card.card[card.side].contains(
                            self.matching_answers_boxes.text(index).unwrap(),
                            self.card_list.recall_settings(card.side),
                        ) {
                            text_color = Color::Green;
                            let failed = *failed;
                            self.typ = Some(WorldType::WaitingForKeypress);
                            if !failed {
                                self.matches_made[card.side as usize] += 1;
                                self.card_list.progress(studying, &mut self.footer);
                            }

                            self.question_box.update(|updater| {
                                updater
                                    .set_text(CORRECT.with(Clone::clone))
                                    .set_text_color(Color::Green);
                            });
                        } else {
                            text_color = Color::Red;
                            if !*failed {
                                *failed = true;
                                self.matches_made[card.side as usize] += 1;
                                self.card_list.fail(studying);
                                self.card_list.regress(studying, &mut self.footer);
                            }
                        };

                        self.matching_answers_boxes.update(|updater| {
                            updater.text(index).set_color(text_color);
                        });
                        io::stdout().flush().unwrap();
                    }
                }
                ControlFlow::Continue(())
            }
            Some(WorldType::Text { studying }) => {
                if let Some(answer) = self.text_input.read_input(code) {
                    let card = &self.card_list[studying];
                    if card.card[card.side]
                        .contains(answer, self.card_list.recall_settings(card.side))
                    {
                        self.texts_entered[card.side as usize] += 1;
                        self.card_list.progress(studying, &mut self.footer);
                        self.question_box.update(|updater| {
                            updater
                                .set_text(CORRECT.with(Clone::clone))
                                .set_text_color(Color::Green);
                        });
                        self.term_settings.hide_cursor();

                        self.typ = Some(WorldType::WaitingForKeypress);
                    } else {
                        self.texts_entered[card.side as usize] += 1;
                        let hidden_question = self.question_box.get_text().clone();
                        self.question_box.update(|updater| {
                            updater
                                .set_text(TEXT_FAILED.with(Clone::clone))
                                .set_text_color(Color::Red);
                        });
                        self.text_input
                            .correct_answer_is(card.card[card.side].display().clone());
                        self.card_list.fail(studying);
                        self.card_list.regress(studying, &mut self.footer);
                        self.text_input.go_to_cursor();

                        self.typ = Some(WorldType::TextFailed {
                            studying,
                            hidden_question,
                        });
                    }
                } else {
                    self.text_input.go_to_cursor();
                }
                io::stdout().flush().unwrap();
                ControlFlow::Continue(())
            }
            Some(WorldType::TextFailed {
                studying,
                ref mut hidden_question,
            }) => {
                if code == KeyCode::BackTab {
                    self.card_list.progress(studying, &mut self.footer);
                    self.card_list.progress(studying, &mut self.footer);
                    self.term_settings.hide_cursor();
                    match self.study_next() {
                        true => ControlFlow::Continue(()),
                        false => ControlFlow::Break(()),
                    }
                } else {
                    if let Some(question) = hidden_question.take() {
                        self.question_box.update(|updater| {
                            updater.set_text(question).set_text_color(Color::White);
                        })
                    }
                    self.text_input.read_input(code);
                    let card = &self.card_list[studying];
                    if card.card[card.side].contains(
                        self.text_input.get_text(),
                        self.card_list.recall_settings(card.side),
                    ) {
                        self.term_settings.hide_cursor();
                        match self.study_next() {
                            true => ControlFlow::Continue(()),
                            false => ControlFlow::Break(()),
                        }
                    } else {
                        self.text_input.go_to_cursor();
                        io::stdout().flush().unwrap();
                        ControlFlow::Continue(())
                    }
                }
            }
            Some(WorldType::WaitingForKeypress) => match self.study_next() {
                true => ControlFlow::Continue(()),
                false => ControlFlow::Break(()),
            },
            None => ControlFlow::Break(()),
        }
    }

    pub fn print_stats(self, error_code: Option<&str>) {
        let Self {
            mut card_list,
            term_settings,
            matches_made,
            texts_entered,
            ..
        } = self;
        term_settings.clear();

        if let Some(error_code) = error_code {
            println!("{}", error_code.red());
        }
        println!("{}", "Stats:".bold());
        println!("                |  total   |   term   |definition|");

        let mut match_fails = [0, 0];
        let mut text_fails = [0, 0];
        let fails = card_list.fails();
        for fail in &*fails {
            match_fails[fail.side as usize] += fail.match_fails.count() as u32;
            text_fails[fail.side as usize] += fail.text_fails.count() as u32;
        }

        #[derive(Debug)]
        struct StatsSection {
            made: StatsLine<u32>,
            fails: StatsLine<u32>,
        }

        #[derive(Debug)]
        struct StatsLine<T> {
            tag: &'static str,
            term: T,
            definition: T,
            total: T,
        }

        impl StatsSection {
            fn print(&self) {
                self.made.print();
                self.fails.print();
                let fail_prop = StatsLine {
                    tag: "",
                    term: Proportion(self.made.term, self.fails.term),
                    definition: Proportion(self.made.definition, self.fails.definition),
                    total: Proportion(self.made.total, self.fails.total),
                };
                fail_prop.print();
            }

            fn try_print(&self) {
                if self.is_used() {
                    self.print();
                }
            }

            fn is_used(&self) -> bool {
                self.made.total > 0
            }
        }

        impl<T: Display> StatsLine<T> {
            fn print(&self) {
                let StatsLine {
                    tag,
                    term,
                    definition,
                    total,
                } = self;
                println!("{tag:15} |{total:^10}|{term:^10}|{definition:^10}|");
            }
        }

        impl<T: Copy + Add<Output = T>> StatsLine<T> {
            fn new(tag: &'static str, data: [T; 2]) -> Self {
                let term = data[Side::Term as usize];
                let definition = data[Side::Definition as usize];
                Self {
                    tag,
                    term,
                    definition,
                    total: term + definition,
                }
            }

            fn add(tag: &'static str, a: &Self, b: &Self) -> Self {
                Self {
                    tag,
                    term: a.term + b.term,
                    definition: a.definition + b.definition,
                    total: a.total + b.total,
                }
            }
        }

        let matches = StatsSection {
            made: StatsLine::new("Matches made:", matches_made),
            fails: StatsLine::new("Match fails:", match_fails),
        };
        matches.try_print();

        let texts = StatsSection {
            made: StatsLine::new("Texts entered:", texts_entered),
            fails: StatsLine::new("Text fails:", text_fails),
        };
        texts.try_print();

        if matches.is_used() && texts.is_used() {
            let total = StatsSection {
                made: StatsLine::add("Total answered:", &matches.made, &texts.made),
                fails: StatsLine::add("Total fails:", &matches.fails, &texts.fails),
            };
            total.print();
        }

        const COLORS: [Color; 5] = [
            Color::Grey,
            Color::Yellow,
            Color::Red,
            Color::Magenta,
            Color::Blue,
        ];
        fn get_color(color: usize) -> Color {
            *COLORS.get(color - 1).unwrap_or(&Color::Cyan)
        }

        let mut fails: Vec<_> = fails
            .iter()
            .filter_map(|fail| {
                (fail.total_fails() > FailCount::ZERO)
                    .then(|| (fail.total_fails(), fail.card[!fail.side].display()))
            })
            .collect();
        fails.sort_unstable_by(|a, b| (a.0.cmp(&b.0)).then_with(|| a.1.cmp(b.1)));

        if matches.is_used() {
            println!("\n{}", "Fails:".bold());
            for (count, text) in fails {
                execute!(
                    io::stdout(),
                    style::SetForegroundColor(get_color(count.count() as usize)),
                    style::Print(format!("  {text} ({count})\n")),
                    style::SetForegroundColor(Color::Reset),
                )
                .unwrap();
            }
        }
    }
}
