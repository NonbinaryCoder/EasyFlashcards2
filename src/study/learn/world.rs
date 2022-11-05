use std::{
    io::{self, Write},
    ops::ControlFlow,
    rc::Rc,
};

use crossterm::{
    queue,
    style::Color,
    terminal::{self, ClearType},
};

use crate::{
    flashcards::Set,
    output::{
        text_box::{input::TextInput, MultiOutlineType, MultiTextBox, OutlineType, TextBox},
        TerminalSettings,
    },
    vec2::{Rect, Vec2},
};

use self::{
    card_list::{CardList, StudyType},
    footer::Footer,
};

mod card_list;
mod footer;

#[derive(Debug)]
pub struct World<'a> {
    question_box: TextBox<Rc<str>>,
    matching_answers_boxes: MultiTextBox<Rc<str>, 4>,
    text_input: TextInput,
    footer: Footer,
    card_list: CardList<'a>,
    term_settings: &'a mut TerminalSettings,
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
    WaitingForKeypress,
}

impl<'a> World<'a> {
    #[must_use]
    pub fn new(size: Vec2<u16>, set: &'a Set, term_settings: &'a mut TerminalSettings) -> Self {
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
            footer: Footer::new(card_list.len() as u32, size),
            card_list,
            term_settings,
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
                        self.question_box.update(|updater| {
                            updater.set_text(item.card[!item.side].display().clone());
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
                        self.text_input.hide();

                        self.typ = Some(WorldType::Matching {
                            studying: token,
                            failed: false,
                        });
                    }
                    StudyType::Text(_) => {
                        self.question_box.update(|updater| {
                            updater.set_text(item.card[!item.side].display().clone());
                        });
                        self.matching_answers_boxes.hide();
                        self.text_input.set_outline(OutlineType::DOUBLE);
                        io::stdout().flush().unwrap();

                        let answer = self.text_input.get_input(self.term_settings);
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

        queue!(io::stdout(), terminal::Clear(ClearType::All)).unwrap();

        self.question_box.force_move_resize(Rect {
            size: Vec2::new(size.x / 3, box_height),
            pos: Vec2::new(size.x / 3, 2),
        });

        self.matching_answers_boxes.force_move_resize(Rect {
            size: Vec2::new(size.x.saturating_sub(8), box_height),
            pos: Vec2::new(4, size.y.saturating_sub(box_height).saturating_sub(3)),
        });

        self.footer.resize(size);

        io::stdout().flush().unwrap();
    }

    pub fn key_pressed(&mut self, key: char) -> ControlFlow<()> {
        match self.typ {
            Some(WorldType::Matching {
                studying,
                ref mut failed,
            }) => {
                if ('1'..='4').contains(&key) {
                    let index = key as usize - '1' as usize;
                    let card = &self.card_list[studying];
                    let text_color = match card.card[card.side].contains(
                        self.matching_answers_boxes.text(index).unwrap(),
                        self.card_list.recall_settings(card.side),
                    ) {
                        true => {
                            let failed = *failed;
                            self.typ = Some(WorldType::WaitingForKeypress);
                            let old_color = card.footer_color;
                            let new_color = self.card_list.progress(studying);
                            if !failed {
                                self.footer.r#move(old_color, new_color);
                            }
                            Color::Green
                        }
                        false => {
                            if !*failed {
                                *failed = true;
                                let old_color = card.footer_color;
                                if let Some(new_color) = self.card_list.regress(studying) {
                                    self.footer.r#move(old_color, new_color);
                                }
                            }
                            Color::Red
                        }
                    };
                    self.matching_answers_boxes.update(|updater| {
                        updater.text(index).set_color(text_color);
                    });
                    io::stdout().flush().unwrap();
                }
                ControlFlow::Continue(())
            }
            Some(WorldType::Text { studying }) => {
                self.text_input.get_input(self.term_settings);
                ControlFlow::Break(())
            }
            Some(WorldType::WaitingForKeypress) => match self.study_next() {
                true => ControlFlow::Continue(()),
                false => ControlFlow::Break(()),
            },
            None => ControlFlow::Break(()),
        }
    }
}
