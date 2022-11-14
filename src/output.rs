use core::fmt;
use std::{borrow::Cow, fmt::Display, io};

use crossterm::{
    cursor, execute, queue,
    style::{self, Color, Stylize},
    terminal,
};

use crate::{
    output::word_wrap::WordWrap,
    vec2::{Rect, Vec2},
};

use self::text_box::OutlineType;

pub mod text_box;
pub mod word_wrap;

pub fn write_fatal_error(text: &str) {
    println!("{}", text.dark_red());
}

pub fn floor_char_boundary(s: &str, mut pos: usize) -> usize {
    while !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

pub fn ceil_char_boundary(s: &str, mut pos: usize) -> Option<usize> {
    (pos <= s.len()).then(|| {
        while !s.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    })
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
        let _ = execute!(io::stdout(), style::SetForegroundColor(Color::Reset));
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
    pub const CENTER: Self = Self {
        h: TextAlignH::Center,
        v: TextAlignV::Center,
    };

    pub const TOP_LEFT: Self = Self {
        h: TextAlignH::Left,
        v: TextAlignV::Top,
    };
}

pub fn draw_outline(dims: Rect<u16>, color: Color, typ: OutlineType) {
    queue!(
        io::stdout(),
        dims.pos.move_to(),
        style::SetForegroundColor(color),
        style::Print(typ.tl),
        style::Print(Repeat(typ.h, dims.size.x - 2)),
        style::Print(typ.tr)
    )
    .unwrap();
    for _ in 0..(dims.size.y - 2) {
        queue!(
            io::stdout(),
            cursor::MoveDown(1),
            cursor::MoveToColumn(dims.pos.x),
            style::Print(typ.v),
            cursor::MoveRight(dims.size.x - 2),
            style::Print(typ.v),
        )
        .unwrap();
    }
    queue!(
        io::stdout(),
        cursor::MoveDown(1),
        cursor::MoveToColumn(dims.pos.x),
        style::Print(typ.bl),
        style::Print(Repeat(typ.h, dims.size.x - 2)),
        style::Print(typ.br)
    )
    .unwrap();
}

fn get_lines_iter(
    size: Vec2<u16>,
    text: &str,
    align: TextAlignV,
) -> impl Iterator<Item = Cow<str>> {
    enum LinesIter<'a> {
        Top(std::iter::Take<WordWrap<'a>>),
        Other(std::vec::IntoIter<Cow<'a, str>>, usize),
    }
    impl<'a> Iterator for LinesIter<'a> {
        type Item = Cow<'a, str>;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                LinesIter::Top(iter) => iter.next(),
                LinesIter::Other(iter, offset) => {
                    if *offset > 0 {
                        *offset -= 1;
                        Some(Cow::Borrowed(""))
                    } else {
                        iter.next()
                    }
                }
            }
        }
    }

    match align {
        TextAlignV::Top => {
            LinesIter::Top(WordWrap::new(text, size.x as usize).take(size.y as usize))
        }
        _ => {
            let lines = {
                let mut lines = WordWrap::new(text, size.x as usize);
                let mut vec = Vec::from_iter(lines.by_ref().take(size.y as usize));
                if lines.next().is_some() {
                    if let Some(line) = vec.last_mut() {
                        let line = line.to_mut();
                        let mut len = line.chars().count();
                        while len > (size.x - 3) as usize {
                            line.pop();
                            len -= 1;
                        }
                        line.push_str("...");
                    }
                }
                vec
            };
            let len = lines.len();
            LinesIter::Other(
                lines.into_iter(),
                match align {
                    TextAlignV::Top => unreachable!(),
                    TextAlignV::Center => (size.y as usize).saturating_sub(len) / 2,
                    TextAlignV::Bottom => (size.y as usize).saturating_sub(len),
                },
            )
        }
    }
}

pub fn draw_text(dims: Rect<u16>, color: Color, text: &str, align: TextAlign) {
    let lines = get_lines_iter(dims.size, text, align.v);
    queue!(io::stdout(), style::SetForegroundColor(color)).unwrap();
    for (index, line) in lines.enumerate() {
        let line = &line;
        if !line.is_empty() {
            queue!(
                io::stdout(),
                cursor::MoveTo(
                    dims.pos.x + align.h.padding_for(line, dims.size.x),
                    dims.pos.y + index as u16
                ),
                style::Print(line),
            )
            .unwrap();
        }
    }
}

pub fn overwrite_text(
    dims: Rect<u16>,
    color: Color,
    old_text: &str,
    old_align: TextAlign,
    new_text: &str,
    new_align: TextAlign,
    always_overwrite: bool,
) {
    let mut old_lines = get_lines_iter(dims.size, old_text, old_align.v);
    let mut new_lines = get_lines_iter(dims.size, new_text, new_align.v);
    queue!(io::stdout(), style::SetForegroundColor(color)).unwrap();
    for y in dims.pos.y..dims.pos.y + dims.size.y {
        match (
            &old_lines.next().filter(|s| !s.is_empty()),
            &new_lines.next().filter(|s| !s.is_empty()),
        ) {
            (None, None) => {}
            (None, Some(new_line)) => queue!(
                io::stdout(),
                cursor::MoveTo(
                    dims.pos.x + new_align.h.padding_for(new_line, dims.size.x),
                    y
                ),
                style::Print(new_line.trim_end()),
            )
            .unwrap(),
            (Some(old_line), None) => queue!(
                io::stdout(),
                cursor::MoveTo(
                    dims.pos.x + old_align.h.padding_for(old_line.trim_end(), dims.size.x),
                    y
                ),
                style::Print(Repeat(' ', old_line.chars().count() as u16)),
            )
            .unwrap(),
            (Some(old_line), Some(new_line)) => {
                let old_line = old_line.trim_end();
                let new_line = new_line.trim_end();
                if always_overwrite || old_line != new_line {
                    let old_pad = old_align.h.padding_for(old_line, dims.size.x);
                    let new_pad = new_align.h.padding_for(new_line, dims.size.x);
                    if new_pad > old_pad {
                        queue!(
                            io::stdout(),
                            cursor::MoveTo(dims.pos.x + old_pad, y),
                            style::Print(Repeat(' ', new_pad - old_pad))
                        )
                        .unwrap();
                    } else {
                        queue!(io::stdout(), cursor::MoveTo(dims.pos.x + new_pad, y)).unwrap();
                    }
                    queue!(io::stdout(), style::Print(new_line)).unwrap();

                    let old_len = old_pad + old_line.chars().count() as u16;
                    let new_len = new_pad + new_line.chars().count() as u16;
                    if old_len > new_len {
                        queue!(io::stdout(), style::Print(Repeat(' ', old_len - new_len))).unwrap();
                    }
                }
            }
        }
    }
}

/// (total, fract)
#[derive(Debug, Clone, Copy)]
pub struct Proportion<T: Copy + PartialEq + Into<f64>>(pub T, pub T);

impl<T: Copy + PartialEq + Into<f64>> fmt::Display for Proportion<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.0.into();
        let fract = self.1.into();
        let frac_str;
        let (frac_str, is_nonneg) = if total > f64::EPSILON {
            let frac = (fract / total) * 100.0;
            frac_str = format!("{frac:.0}%");
            (frac_str.as_str(), frac >= 0.0)
        } else {
            ("NaN", true)
        };
        f.pad_integral(is_nonneg, "", frac_str)
    }
}
