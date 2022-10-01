use std::path::PathBuf;

use argh::FromArgs;
use crossterm::{
    event::{self, Event},
    terminal,
};

use crate::{
    flashcards::{Set, Side},
    load_set,
    output::TerminalSettings,
    vec2::Vec2,
};

mod grid;

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

        let card_count = self.card_count.unwrap_or_else(|| Vec2::splat(1));
        let cards = set.cards;
        let mut sides = vec![Side::Term; cards.len()];
        let term_size: Vec2<_> = terminal::size()
            .expect("unable to get terminal size")
            .into();

        let mut term_settings = TerminalSettings::new();
        term_settings
            .enter_alternate_screen()
            .hide_cursor()
            .enable_raw_mode();

        let mut grid = grid::FlashcardGrid::new(card_count);
        grid.fill_from_text(cards.iter().map(|card| card[Side::Term].display()))
            .size_to(term_size);

        loop {
            match event::read().expect("Unable to read event") {
                Event::Resize(x, y) => {
                    grid.size_to(Vec2::new(x, y));
                }
                crate::up!() => grid.update(|grid| {
                    if let Some(y) = grid.selected().y.checked_sub(1) {
                        grid.set_selected(Vec2::new(grid.selected().x, y));
                    } else if scroll_dst > 0 {
                        scroll_dst -= 1;
                        grid.fill_from_cards(
                            cards
                                .iter()
                                .zip(sides.iter())
                                .map(|(card, side)| (card[*side].display(), *side))
                                .skip((scroll_dst * grid.card_count().x) as usize),
                        );
                    }
                }),
                crate::down!() => grid.update(|grid| {
                    let new_selected = grid.selected() + Vec2::new(0, 1);
                    if (new_selected + Vec2::new(0, scroll_dst))
                        .index_row_major(grid.card_count().x as usize)
                        < cards.len()
                    {
                        if new_selected.y < grid.card_count().y {
                            grid.set_selected(new_selected);
                        } else {
                            scroll_dst += 1;
                            grid.fill_from_cards(
                                cards
                                    .iter()
                                    .zip(sides.iter())
                                    .map(|(card, side)| (card[*side].display(), *side))
                                    .skip((scroll_dst * grid.card_count().x) as usize),
                            );
                        }
                    }
                }),
                crate::left!() => grid.update(|grid| {
                    grid.selected_mut().x = grid.selected().x.saturating_sub(1);
                }),
                crate::right!() => grid.update(|grid| {
                    let new_selected = grid.selected() + Vec2::new(1, 0);
                    if (new_selected + Vec2::new(0, scroll_dst))
                        .index_row_major(grid.card_count().x as usize)
                        < cards.len()
                        && new_selected.x < grid.card_count().x
                    {
                        grid.set_selected(new_selected);
                    }
                }),
                crate::click!() => {
                    grid.update(|grid| {
                        let mut selected = grid.selected();
                        let width = grid.card_count().x as usize;
                        let card = (&mut grid[selected]).as_mut().unwrap();
                        let new_side = !card.1;
                        selected.y += scroll_dst;
                        let index = selected.index_row_major(width);
                        sides[index] = new_side;
                        *card = (cards[index][new_side].display(), new_side);
                    });
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
