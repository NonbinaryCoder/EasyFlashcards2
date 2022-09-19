use std::{fmt::Display, io};

use crossterm::{cursor, execute, queue, style::Stylize, terminal};

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

#[derive(Debug, Default)]
pub struct TerminalSettings {
    alternate_screen: bool,
    cursor_hidden: bool,
    raw_mode: bool,
}

#[allow(dead_code)]
impl TerminalSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter_alternate_screen(&mut self) -> &mut Self {
        queue!(io::stdout(), terminal::EnterAlternateScreen).unwrap();
        self.alternate_screen = true;
        self
    }

    pub fn leave_alternate_screen(&mut self) -> &mut Self {
        queue!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
        self.alternate_screen = false;
        self
    }

    pub fn hide_cursor(&mut self) -> &mut Self {
        queue!(io::stdout(), cursor::Hide).unwrap();
        self.cursor_hidden = true;
        self
    }

    pub fn show_cursor(&mut self) -> &mut Self {
        queue!(io::stdout(), cursor::Show).unwrap();
        self.cursor_hidden = false;
        self
    }

    pub fn enable_raw_mode(&mut self) -> &mut Self {
        terminal::enable_raw_mode().unwrap();
        self.raw_mode = true;
        self
    }

    pub fn disable_raw_mode(&mut self) -> &mut Self {
        terminal::disable_raw_mode().unwrap();
        self.raw_mode = false;
        self
    }
}

impl Drop for TerminalSettings {
    fn drop(&mut self) {
        if self.alternate_screen {
            let _ = execute!(io::stdout(), terminal::LeaveAlternateScreen);
        }
        if self.cursor_hidden {
            let _ = execute!(io::stdout(), cursor::Show);
        }
        if self.raw_mode {
            let _ = terminal::disable_raw_mode();
        }
    }
}
