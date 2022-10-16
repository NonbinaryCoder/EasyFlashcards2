use std::{
    io::{self, Write},
    iter,
    path::PathBuf,
};

use argh::FromArgs;
use crossterm::{
    event::{self, Event},
    queue,
    terminal::{self, ClearType},
};

use crate::{
    flashcards::{Set, Side},
    load_set,
    output::{
        text_box::{OutlineType, TextBox},
        TerminalSettings,
    },
    vec2::{Rect, Vec2},
};

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "flashcards")]
/// Study with some classic flashcards!
pub struct Entry {
    /// the set to study
    #[argh(positional)]
    set: PathBuf,
    /// how many flashcards to put on each row and column, defaults to 1x1
    #[argh(positional, from_str_fn(parse_size))]
    card_count: Option<Vec2<u16>>,
}

impl Entry {
    pub fn run(self) {
        let set = load_set!(&self.set);
        let mut scroll_dst = 0u16;
        let mut selected_card = Vec2::ZERO;

        let card_count = self.card_count.unwrap_or_else(|| Vec2::splat(1));
        let cards = set.cards;
        let mut sides = vec![Side::Term; cards.len()];

        let mut term_settings = TerminalSettings::new();
        term_settings
            .enter_alternate_screen()
            .hide_cursor()
            .enable_raw_mode();

        let mut visible_cards = Vec::with_capacity(card_count.area() as usize);
        {
            let term_size: Vec2<_> = terminal::size()
                .expect("unable to get terminal size")
                .into();
            let card_size = term_size / card_count;
            let offset = (term_size - (card_size * card_count)) / Vec2::splat(2);
            let mut pos = Vec2::splat(0);

            visible_cards.push(TextBox::from_fn(
                Rect {
                    pos: offset,
                    size: card_size,
                },
                |updater| {
                    updater.set_color(Side::Term.color());
                    if let Some(card) = cards.get(0) {
                        updater
                            .set_outline(OutlineType::DOUBLE)
                            .set_text(card[Side::Term].display().clone());
                    }
                },
            ));

            let mut index = 0;
            visible_cards.extend(iter::from_fn(|| {
                let make_text_box = |pos, index: usize| {
                    TextBox::from_fn(
                        Rect {
                            pos: pos * card_size + offset,
                            size: card_size,
                        },
                        |updater| {
                            updater.set_color(Side::Term.color());
                            if let Some(card) = cards.get(index) {
                                updater
                                    .set_outline(OutlineType::HEAVY)
                                    .set_text(card[Side::Term].display().clone());
                            }
                        },
                    )
                };

                index += 1;
                pos.x += 1;
                if pos.x < card_count.x {
                    Some(make_text_box(pos, index))
                } else {
                    pos.x = 0;
                    pos.y += 1;
                    (pos.y < card_count.y).then(|| make_text_box(pos, index))
                }
            }))
        }
        io::stdout().flush().unwrap();

        loop {
            match event::read().expect("Unable to read event") {
                Event::Resize(x, y) => {
                    queue!(io::stdout(), terminal::Clear(ClearType::All)).unwrap();
                    let term_size = Vec2 { x, y };
                    let card_size = term_size / card_count;
                    let offset = (term_size - (card_size * card_count)) / Vec2::splat(2);
                    let mut pos = Vec2::splat(0);
                    for card in &mut visible_cards {
                        card.force_move_resize(Rect {
                            pos: pos * card_size + offset,
                            size: card_size,
                        });
                        pos.x += 1;
                        if pos.x >= card_count.x {
                            pos.x = 0;
                            pos.y += 1;
                        }
                    }
                    io::stdout().flush().unwrap();
                }
                crate::up!() => {
                    if selected_card.y > 0 {
                        visible_cards[selected_card.index_row_major(card_count.x as usize)].update(
                            |updater| {
                                updater.set_outline(OutlineType::HEAVY);
                            },
                        );
                        selected_card.y -= 1;
                        visible_cards[selected_card.index_row_major(card_count.x as usize)].update(
                            |updater| {
                                updater.set_outline(OutlineType::DOUBLE);
                            },
                        );
                    } else if scroll_dst > 0 {
                        scroll_dst -= 1;
                        for i in (card_count.x..card_count.area()).rev() {
                            let new_text = visible_cards[(i - card_count.x) as usize]
                                .get_text()
                                .clone();
                            visible_cards[i as usize].update(|updater| {
                                updater
                                    .set_color(
                                        sides[(i + scroll_dst * card_count.x) as usize].color(),
                                    )
                                    .add_outline(OutlineType::HEAVY)
                                    .set_text(new_text.unwrap());
                            });
                        }
                        for x in (0..card_count.x).rev() {
                            let index =
                                Vec2::new(x, scroll_dst).index_row_major(card_count.x as usize);
                            let side = sides[index];
                            let new_text = cards[index][side].display().clone();
                            visible_cards[x as usize].update(|updater| {
                                updater
                                    .set_color(side.color())
                                    .add_outline(OutlineType::HEAVY)
                                    .set_text(new_text);
                            })
                        }
                    }
                    io::stdout().flush().unwrap();
                }
                crate::down!() => {
                    if selected_card.y < card_count.y - 1 {
                        if (selected_card + Vec2::y(scroll_dst + 1))
                            .index_row_major(card_count.x as usize)
                            < cards.len()
                        {
                            visible_cards[selected_card.index_row_major(card_count.x as usize)]
                                .update(|updater| {
                                    updater.set_outline(OutlineType::HEAVY);
                                });
                            selected_card.y += 1;
                            visible_cards[selected_card.index_row_major(card_count.x as usize)]
                                .update(|updater| {
                                    updater.set_outline(OutlineType::DOUBLE);
                                });
                        }
                    } else {
                        let overflow_selected = selected_card + Vec2::Y + Vec2::y(scroll_dst);
                        let index = overflow_selected.index_row_major(card_count.x as usize);
                        if index < cards.len() {
                            scroll_dst += 1;
                            for i in 0..(card_count - Vec2::Y).area() {
                                let new_text = visible_cards[(i + card_count.x) as usize]
                                    .get_text()
                                    .clone();
                                visible_cards[i as usize].update(|updater| {
                                    if let Some(text) = new_text {
                                        updater
                                            .add_outline(OutlineType::HEAVY)
                                            .set_color(
                                                sides[(i + scroll_dst * card_count.x) as usize]
                                                    .color(),
                                            )
                                            .set_text(text);
                                    } else {
                                        updater.clear_outline().clear_text();
                                    }
                                })
                            }
                            for x in 0..card_count.x {
                                let index = Vec2::new(x, scroll_dst + card_count.y - 1)
                                    .index_row_major(card_count.x as usize);
                                if let Some(side) = sides.get(index) {
                                    let new_text = cards[index][*side].display().clone();
                                    visible_cards[Vec2::new(x, card_count.y - 1)
                                        .index_row_major(card_count.x as usize)]
                                    .update(|updater| {
                                        updater.set_color(side.color()).set_text(new_text);
                                    })
                                } else {
                                    visible_cards[Vec2::new(x, card_count.y - 1)
                                        .index_row_major(card_count.x as usize)]
                                    .update(|updater| {
                                        updater.clear_all();
                                    })
                                }
                            }
                        }
                    }
                    io::stdout().flush().unwrap();
                }
                crate::left!() => {
                    if selected_card.x > 0 {
                        visible_cards[selected_card.index_row_major(card_count.x as usize)].update(
                            |updater| {
                                updater.set_outline(OutlineType::HEAVY);
                            },
                        );
                        selected_card.x -= 1;
                        visible_cards[selected_card.index_row_major(card_count.x as usize)].update(
                            |updater| {
                                updater.set_outline(OutlineType::DOUBLE);
                            },
                        );
                        io::stdout().flush().unwrap();
                    }
                }
                crate::right!() => {
                    if selected_card.x < card_count.x - 1
                        && (selected_card + Vec2::new(1, scroll_dst))
                            .index_row_major(card_count.x as usize)
                            < cards.len()
                    {
                        visible_cards[selected_card.index_row_major(card_count.x as usize)].update(
                            |updater| {
                                updater.set_outline(OutlineType::HEAVY);
                            },
                        );
                        selected_card.x += 1;
                        visible_cards[selected_card.index_row_major(card_count.x as usize)].update(
                            |updater| {
                                updater.set_outline(OutlineType::DOUBLE);
                            },
                        );
                        io::stdout().flush().unwrap();
                    }
                }
                crate::click!() => {
                    let index = (selected_card + Vec2::y(scroll_dst))
                        .index_row_major(card_count.x as usize);
                    let side = &mut sides[index];
                    *side = !*side;
                    visible_cards[selected_card.index_row_major(card_count.x as usize)].update(
                        |updater| {
                            updater
                                .set_color(side.color())
                                .set_text(cards[index][*side].display().clone());
                        },
                    );
                    io::stdout().flush().unwrap();
                }
                Event::Key(_) => break,
                _ => {}
            }
        }

        term_settings.clear();
    }
}

fn parse_size(s: &str) -> Result<Vec2<u16>, String> {
    let (x, y) = s.split_once('x').ok_or("expects inputs like \"1x1\"")?;
    let x = x.parse::<u16>().map_err(|e| e.to_string())?;
    let y = y.parse::<u16>().map_err(|e| e.to_string())?;
    let v = Vec2 { x, y };
    match v.into_iter().any(|x| x < 1) {
        false => Ok(v),
        true => Err("Size must be at least 1x1".to_owned()),
    }
}
