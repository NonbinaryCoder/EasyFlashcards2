use std::{
    io::{self, Write},
    path::PathBuf,
};

use argh::FromArgs;
use crossterm::{
    cursor, queue,
    style::{self, Color},
    terminal,
};

use crate::{
    flashcards::{Flashcard, Set, Side},
    load_set,
    output::{len_base10_u16, Repeat, TerminalSettings},
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
        let cards = CardList::from_set(&set);
        let term_size: Vec2<_> = terminal::size()
            .expect("unable to get terminal size")
            .into();
        let mut term_settings = TerminalSettings::new();
        term_settings.enter_alternate_screen().hide_cursor();

        cards.print_footer(term_size);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut String::new()).unwrap();

        drop(term_settings);
    }
}

#[derive(Debug)]
struct CardList<'a>(Vec<CardListItem<'a>>);

#[derive(Debug)]
struct CardListItem<'a> {
    card: &'a Flashcard,
    side: Side,
    studied_state: StudiedState,
}

impl<'a> CardList<'a> {
    fn from_set(set: &'a Set) -> Self {
        let count = [set.recall_t.is_used(), set.recall_d.is_used()]
            .into_iter()
            .filter(|b| *b)
            .count();
        let mut v = Vec::with_capacity(count * set.cards.len());
        if set.recall_t.is_used() {
            v.extend(set.cards.iter().map(|card| CardListItem {
                card,
                side: Side::Term,
                studied_state: StudiedState::None,
            }));
        }
        if set.recall_d.is_used() {
            v.extend(set.cards.iter().map(|card| CardListItem {
                card,
                side: Side::Definition,
                studied_state: StudiedState::None,
            }));
        }
        Self(v)
    }

    fn print_footer(&self, term_size: Vec2<u16>) {
        let mut counts = [0; COLORS.len()];
        for item in self.0.iter() {
            use StudiedState::*;
            counts[match item.studied_state {
                None => 0,
                MatchedOnce => 1,
                MatchedTwice => 2,
                SpelledOnce => 2,
                Learned => 3,
            }] += 1;
        }

        let sum = counts.iter().sum::<u16>() as f32;
        let fractions = counts.map(|c| c as f32 / sum);
        let mut widths = fractions.map(|f| (f * term_size.x as f32) as u16);
        widths[0] = term_size.x - widths[1..].iter().sum::<u16>();

        queue!(io::stdout(), cursor::MoveTo(0, term_size.y - 1)).unwrap();
        for ((count, width), color) in counts.into_iter().zip(widths).zip(COLORS).rev() {
            let len_base10_u16 = len_base10_u16(count);
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
        queue!(io::stdout(), style::SetForegroundColor(Color::Reset)).unwrap();
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum StudiedState {
    None,
    MatchedOnce,
    MatchedTwice,
    SpelledOnce,
    Learned,
}
