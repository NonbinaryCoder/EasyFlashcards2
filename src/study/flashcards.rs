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
    flashcards::{Set, Side},
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

        let cards = iter::repeat(Side::Term)
            .take(set.cards.len())
            .collect::<Vec<_>>();
        let card_count = self.card_count.unwrap_or(Vec2::splat(1));
        let mut term_size: Vec2<_> = terminal::size()
            .expect("unable to get terminal size")
            .into();
        let mut card_size = term_size / card_count;
        card_size.x = card_size.x.max(5);
        card_size.y = card_size.y.max(3);

        queue!(io::stdout(), EnterAlternateScreen).unwrap();
        set.cards[0].draw(Vec2::new(0, 0), card_size, Side::Term);
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
