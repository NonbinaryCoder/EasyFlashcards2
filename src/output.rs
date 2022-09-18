use std::fmt::Display;

use crossterm::style::Stylize;

pub mod word_wrap;

pub fn write_fatal_error(text: &str) {
    println!("{}", text.dark_red());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Repeat(pub char, pub u16);

impl Display for Repeat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.1 {
            write!(f, "{}", self.0)?;
        }
        Ok(())
    }
}
