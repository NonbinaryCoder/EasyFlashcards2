use std::path::PathBuf;

use argh::FromArgs;

use crate::flashcards::Set;

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
        if let Some(set) = Set::load_from_file_path(&self.set) {
            dbg!(set);
        }
    }
}
