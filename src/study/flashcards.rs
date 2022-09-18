use std::{
    io::{self, Write},
    iter,
    path::PathBuf,
};

use argh::FromArgs;
use crossterm::{
    execute, queue,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{
    flashcards::{Flashcard, Set, Side},
    load_set,
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
        let cards = set
            .cards
            .into_iter()
            .map(|card| (card, Side::Definition))
            .collect::<Vec<_>>();

        let card_count = self.card_count.unwrap_or(Vec2::splat(1));
        let mut effective_count = card_count;
        let mut term_size: Vec2<_> = terminal::size()
            .expect("unable to get terminal size")
            .into();
        let mut card_size = term_size / card_count;
        if card_size.x < 5 {
            card_size.x = 5;
            effective_count.x = term_size.x / card_size.x;
        }
        if card_size.y < 3 {
            card_size.y = 3;
            effective_count.y = term_size.y / card_size.y;
        }
        let mut offset = (term_size - (effective_count * card_size)) / Vec2::splat(2);

        let draw_all_cards = |start_pos| {
            let mut pos = Vec2::splat(0);
            for (card, side) in &cards[start_pos..] {
                card.draw(pos * card_size + offset, card_size, *side);
                pos.y += 1;
                if pos.y >= effective_count.y {
                    pos.y = 0;
                    pos.x += 1;
                    if pos.x >= effective_count.x {
                        break;
                    }
                }
            }
        };

        queue!(io::stdout(), EnterAlternateScreen).unwrap();
        draw_all_cards(0);
        io::stdout().flush().unwrap();

        io::stdin().read_line(&mut String::new()).unwrap();
        execute!(io::stdout(), LeaveAlternateScreen).unwrap();
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
