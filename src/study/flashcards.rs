use std::{
    io::{self, Write},
    path::PathBuf,
};

use argh::FromArgs;
use crossterm::{
    event::{self, Event},
    queue,
    style::Attribute,
    terminal,
};

use crate::{
    flashcards::{Flashcard, Set, Side},
    load_set,
    output::{BoxOutline, TerminalSettings, TextBox},
    vec2::Vec2,
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
        let mut cards = set
            .cards
            .into_iter()
            .map(|card| (card, Side::Term))
            .collect::<Vec<_>>();

        let card_count = self.card_count.unwrap_or_else(|| Vec2::splat(1));
        let mut term_size: Vec2<_> = terminal::size()
            .expect("unable to get terminal size")
            .into();
        let mut too_small = false;
        let mut card_printer = TextBox::new();
        card_printer.outline(Some(BoxOutline::HEAVY));
        card_printer.text_align_h(crate::output::TextAlignH::Center);
        card_printer.text_align_v(crate::output::TextAlignV::Center);
        card_printer.size({
            let mut card_size = term_size / card_count;
            if card_size.x < 5 {
                card_size.x = 5;
                too_small = true;
            }
            if card_size.y < 3 {
                card_size.y = 3;
                too_small = true;
            }
            card_size
        });
        let mut offset = Vec2::splat(0);
        let mut selected = Vec2::splat(0);
        let mut start_x = 0;

        let draw_all_cards = |start_x,
                              selected,
                              offset,
                              count: Vec2<_>,
                              cards: &mut Vec<(Flashcard, Side)>,
                              card_printer: &mut TextBox| {
            card_printer
                .unset_attribute(Attribute::Bold)
                .outline(Some(BoxOutline::HEAVY));
            let mut pos = Vec2::splat(0);
            for (card, side) in &cards[(start_x * card_count.y) as usize..] {
                if pos == selected {
                    card_printer
                        .set_attribute(Attribute::Bold)
                        .outline(Some(BoxOutline::DOUBLE));
                }
                card_printer
                    .pos(pos * card_printer.size + offset)
                    .color(side.color())
                    .draw_outline_and_text(card[*side].first());
                if pos == selected {
                    card_printer
                        .unset_attribute(Attribute::Bold)
                        .outline(Some(BoxOutline::HEAVY));
                }

                pos.y += 1;
                if pos.y >= count.y {
                    pos.y = 0;
                    pos.x += 1;
                    if pos.x >= count.x {
                        break;
                    }
                }
            }
        };

        let mut term_settings = TerminalSettings::new();
        term_settings
            .enter_alternate_screen()
            .hide_cursor()
            .enable_raw_mode();
        if !too_small {
            offset = (term_size - (card_count * *card_printer.get_size())) / Vec2::splat(2);
            draw_all_cards(
                start_x,
                selected,
                offset,
                card_count,
                &mut cards,
                &mut card_printer,
            );
        }
        io::stdout().flush().unwrap();
        terminal::enable_raw_mode().unwrap();

        loop {
            match event::read().expect("Unable to read event") {
                Event::Resize(x, y) => {
                    term_size = Vec2::new(x, y);
                    too_small = false;
                    card_printer.size({
                        let mut card_size = term_size / card_count;
                        if card_size.x < 5 {
                            card_size.x = 5;
                            too_small = true;
                        }
                        if card_size.y < 3 {
                            card_size.y = 3;
                            too_small = true;
                        }
                        card_size
                    });
                    if !too_small {
                        offset =
                            (term_size - (card_count * *card_printer.get_size())) / Vec2::splat(2);
                        queue!(io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
                        draw_all_cards(
                            start_x,
                            selected,
                            offset,
                            card_count,
                            &mut cards,
                            &mut card_printer,
                        );
                        io::stdout().flush().unwrap();
                    }
                }
                crate::up!() => {
                    let new_y = selected.y.checked_sub(1);
                    if let Some(new_y) = new_y {
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::HEAVY))
                            .draw_outline();
                        selected.y = new_y;
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::DOUBLE))
                            .draw_outline();
                        io::stdout().flush().unwrap();
                    }
                }
                crate::down!() => {
                    let new_selection = Vec2::new(selected.x, selected.y + 1);
                    if new_selection.y < card_count.y
                        && new_selection.to_index_col_major(card_count.y) < cards.len()
                    {
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::HEAVY))
                            .draw_outline();
                        selected = new_selection;
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::DOUBLE))
                            .draw_outline();
                        io::stdout().flush().unwrap();
                    }
                }
                crate::left!() => {
                    let new_x = selected.x.checked_sub(1);
                    if let Some(new_x) = new_x {
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::HEAVY))
                            .draw_outline();
                        selected.x = new_x;
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::DOUBLE))
                            .draw_outline();
                        io::stdout().flush().unwrap();
                    }
                }
                crate::right!() => {
                    let new_selection = Vec2::new(selected.x + 1, selected.y);
                    let index = new_selection.to_index_col_major(card_count.y);
                    if index < cards.len() {
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::HEAVY))
                            .draw_outline();
                        selected = new_selection;
                        card_printer
                            .pos(selected * card_printer.size + offset)
                            .color(cards[selected.to_index_col_major(card_count.y)].1.color())
                            .outline(Some(BoxOutline::DOUBLE))
                            .draw_outline();
                        io::stdout().flush().unwrap();
                    }
                }
                crate::click!() => {
                    let (ref card, ref mut side) = cards[selected.to_index_col_major(card_count.y)];
                    *side = !*side;
                    card_printer
                        .pos(selected * card_printer.size + offset)
                        .color(side.color())
                        .outline(Some(BoxOutline::DOUBLE))
                        .draw_outline()
                        .overwrite_text(card[!*side].first(), card[*side].first());
                    io::stdout().flush().unwrap();
                }
                Event::Key(_) => break,
                _ => {}
            }
        }

        drop(term_settings);
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
