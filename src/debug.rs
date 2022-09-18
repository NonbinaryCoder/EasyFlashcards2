use std::path::PathBuf;

use argh::FromArgs;

use crate::{flashcards::Set, load_set};

/// Debug a flashcard set
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "debug")]
pub struct Entry {
    /// the set to debug
    #[argh(positional)]
    set: PathBuf,
}

impl Entry {
    pub fn run(self) {
        let set = load_set!(&self.set);
        dbg!(set);
    }
}
