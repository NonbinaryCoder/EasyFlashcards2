use std::num::NonZeroU16;

use argh::FromArgs;

use crate::output::word_wrap::WordWrap;

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "flashcards")]
/// Study with some classic flashcards!
pub struct Entry {
    /// the number of flashcards to show on the horizontal axis.  Defaults to 1
    #[argh(option, short = 'h')]
    horizontal: Option<NonZeroU16>,
    /// the number of flashcards to show on the vertical axis.  Defaults to `width`
    #[argh(option, short = 'v')]
    vertical: Option<NonZeroU16>,
}

impl Entry {
    pub fn run(self) {
        let (w, _) = self.size();
        for word in WordWrap::new(
            "Lots of text.  Lots!  So mutch!  Even words of aaaaaaaaaaaaaaaaaaaaaaaaaaaa?",
            w.into(),
        ) {
            println!("{}", word);
        }
    }

    fn size(&self) -> (u16, u16) {
        let w = self.horizontal.map(|w| w.get()).unwrap_or(1);
        let h = self.vertical.map(|h| h.get()).unwrap_or(w);
        (w, h)
    }
}
