use std::{fmt::Display, io};

use crossterm::{cursor, execute, queue, style::Stylize, terminal};

pub mod text_box;
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
    exiting_properly: bool,
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

    pub fn clear(mut self) {
        self.exiting_properly = true;
    }
}

impl Drop for TerminalSettings {
    fn drop(&mut self) {
        if !self.exiting_properly {
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TextAlignH {
    Left,
    Center,
    Right,
}

impl TextAlignH {
    fn padding_for(self, s: &str, width: impl Into<usize>) -> u16 {
        match self {
            TextAlignH::Left => 0,
            TextAlignH::Center => ((width.into() - s.chars().count()) / 2) as u16,
            TextAlignH::Right => (width.into() - s.chars().count()) as u16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TextAlignV {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAlign {
    h: TextAlignH,
    v: TextAlignV,
}

impl TextAlign {
    pub fn center() -> Self {
        Self {
            h: TextAlignH::Center,
            v: TextAlignV::Center,
        }
    }

    pub fn new(h: TextAlignH, v: TextAlignV) -> TextAlign {
        Self { h, v }
    }
}
