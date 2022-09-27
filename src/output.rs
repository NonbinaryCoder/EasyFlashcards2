use std::{borrow::Cow, fmt::Display, io};

use crossterm::{
    cursor, execute, queue,
    style::{self, Attribute, Attributes, Color, Stylize},
    terminal,
};

use crate::{output::word_wrap::WordWrap, vec2::Vec2};

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

#[derive(Debug, Clone)]
pub struct TextBox {
    pub pos: Vec2<u16>,
    pub size: Vec2<u16>,
    pub outline: Option<BoxOutline>,
    pub text_align_h: TextAlignH,
    pub text_align_v: TextAlignV,
    pub outline_color: Color,
    pub content_color: Color,
    pub attributes: Attributes,
}

#[allow(dead_code)]
impl TextBox {
    /// Draws a text box on screen.  Does not flush stdout
    ///
    /// # Panics
    ///
    /// Panics if size is not at least 5x3 (outlined) or at least 3x1 (no outline)
    pub fn draw_outline_and_text(&self, text: &str) -> &Self {
        // TODO: improve rendering?
        self.draw_outline().draw_text(text)
    }

    /// Draws just the outline of this, or does nothing if `self.outline` is `None`
    ///
    /// # Panics
    ///
    /// Panics if size is not at least 2x2 and this is outlined
    pub fn draw_outline(&self) -> &Self {
        if let Some(outline) = self.outline {
            assert!(self.size.x >= 2 && self.size.y >= 2);

            queue!(
                io::stdout(),
                self.pos.move_to(),
                style::SetForegroundColor(self.outline_color),
                style::SetAttributes(self.attributes),
                style::Print(outline.tl),
                style::Print(Repeat(outline.h, self.size.x - 2)),
                style::Print(outline.tr)
            )
            .unwrap();
            for _ in 0..(self.size.y - 2) {
                queue!(
                    io::stdout(),
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(self.pos.x),
                    style::Print(outline.v),
                    cursor::MoveRight(self.size.x - 2),
                    style::Print(outline.v),
                )
                .unwrap();
            }
            queue!(
                io::stdout(),
                cursor::MoveDown(1),
                cursor::MoveToColumn(self.pos.x),
                style::Print(outline.bl),
                style::Print(Repeat(outline.h, self.size.x - 2)),
                style::Print(outline.br)
            )
            .unwrap();
        }
        self
    }

    /// Draws just the text of this
    ///
    /// # Panics
    ///
    /// Panics if size is not at least 5x3 (outlined) or at least 3x1 (no outline)
    pub fn draw_text(&self, text: &str) -> &Self {
        let lines_iter = self.get_lines_iter(text);

        match self.text_align_h {
            TextAlignH::Left => self.draw_text_left_align(lines_iter),
            TextAlignH::Center => self.draw_text_center_align(lines_iter),
            TextAlignH::Right => self.draw_text_right_align(lines_iter),
        }
        self
    }

    fn get_lines_iter<'a>(&self, text: &'a str) -> impl Iterator<Item = Cow<'a, str>> {
        let inner_size = self.inner_size();

        enum LinesIter<'a> {
            Top(WordWrap<'a>),
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

        match self.text_align_v {
            TextAlignV::Top => LinesIter::Top(WordWrap::new(text, inner_size.x as usize)),
            _ => {
                let lines = {
                    let mut lines = WordWrap::new(text, inner_size.x as usize);
                    let mut vec = Vec::from_iter(lines.by_ref().take(inner_size.y as usize));
                    if lines.next().is_some() {
                        if let Some(line) = vec.last_mut() {
                            let line = line.to_mut();
                            let mut len = line.chars().count();
                            while len > (inner_size.x - 3) as usize {
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
                    match self.text_align_v {
                        TextAlignV::Top => unreachable!(),
                        TextAlignV::Center => (inner_size.y as usize).saturating_sub(len) / 2,
                        TextAlignV::Bottom => (inner_size.y as usize).saturating_sub(len),
                    },
                )
            }
        }
    }

    fn draw_text_left_align<'a>(&self, lines: impl Iterator<Item = Cow<'a, str>>) {
        let inner_size = self.inner_size();
        let corner_pos = if self.outline.is_some() {
            self.pos + Vec2::splat(1)
        } else {
            self.pos
        };

        queue!(
            io::stdout(),
            corner_pos.move_to(),
            style::SetForegroundColor(self.content_color),
            style::SetAttributes(self.attributes)
        )
        .unwrap();
        for line in lines.take(inner_size.y as usize) {
            queue!(
                io::stdout(),
                style::Print(line),
                cursor::MoveDown(1),
                cursor::MoveToColumn(corner_pos.x)
            )
            .unwrap();
        }
    }

    fn draw_text_center_align<'a>(&self, lines: impl Iterator<Item = Cow<'a, str>>) {
        let inner_size = self.inner_size();
        let corner_pos = if self.outline.is_some() {
            self.pos + Vec2::splat(1)
        } else {
            self.pos
        };

        for (index, line) in lines.enumerate().take(inner_size.y as usize) {
            if !line.is_empty() {
                queue!(
                    io::stdout(),
                    cursor::MoveTo(
                        corner_pos.x + ((inner_size.x - line.chars().count() as u16) / 2),
                        corner_pos.y + index as u16,
                    ),
                    style::Print(line),
                )
                .unwrap();
            }
        }
    }

    fn draw_text_right_align<'a>(&self, lines: impl Iterator<Item = Cow<'a, str>>) {
        let inner_size = self.inner_size();
        let corner_pos = {
            let outer_pos = self.pos.map_x(|x| x + self.size.x);
            if self.outline.is_some() {
                Vec2::new(outer_pos.x - 1, outer_pos.y + 1)
            } else {
                outer_pos
            }
        };

        for (index, line) in lines.enumerate().take(inner_size.y as usize) {
            if !line.is_empty() {
                queue!(
                    io::stdout(),
                    cursor::MoveTo(
                        corner_pos.x - line.chars().count() as u16,
                        corner_pos.y + index as u16
                    ),
                    style::Print(line),
                )
                .unwrap();
            }
        }
    }

    /// Draws just the text of this, replacing the previous text
    /// Behavior is unspecified when the text align used for `old_text` is
    /// different from the current text align of this
    ///
    /// # Panics
    ///
    /// Panics if size is not at least 5x3 (outlined) or at least 3x1 (no outline)
    pub fn overwrite_text(&self, old_text: &str, new_text: &str) -> &Self {
        let old_lines = self.get_lines_iter(old_text);
        let new_lines = self.get_lines_iter(new_text);

        match self.text_align_h {
            TextAlignH::Left => self.overwrite_text_left_align(old_lines, new_lines),
            TextAlignH::Center => self.overwrite_text_center_align(old_lines, new_lines),
            TextAlignH::Right => self.overwrite_text_right_align(old_lines, new_lines),
        }
        self
    }

    fn overwrite_text_left_align<'a>(
        &self,
        old_lines: impl Iterator<Item = Cow<'a, str>>,
        new_lines: impl Iterator<Item = Cow<'a, str>>,
    ) {
        let inner_size = self.inner_size();
        let corner_pos = if self.outline.is_some() {
            self.pos + Vec2::splat(1)
        } else {
            self.pos
        };

        let old_lines = old_lines.take(inner_size.y as usize);
        let mut new_lines = new_lines.take(inner_size.y as usize);

        queue!(
            io::stdout(),
            corner_pos.move_to(),
            style::SetForegroundColor(self.content_color),
            style::SetAttributes(self.attributes)
        )
        .unwrap();
        for old_line in old_lines {
            let old_line_len = old_line.chars().count();
            if let Some(new_line) = new_lines.next().filter(|l| !l.is_empty()) {
                let extra_len = old_line_len
                    .checked_sub(new_line.chars().count())
                    .unwrap_or_default();
                queue!(
                    io::stdout(),
                    style::Print(new_line),
                    style::Print(Repeat(' ', extra_len as u16)),
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(corner_pos.x)
                )
                .unwrap();
            } else {
                queue!(
                    io::stdout(),
                    style::Print(Repeat(' ', old_line_len as u16)),
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(corner_pos.x)
                )
                .unwrap();
            }
        }
        for line in new_lines {
            queue!(
                io::stdout(),
                style::Print(line),
                cursor::MoveDown(1),
                cursor::MoveToColumn(corner_pos.x)
            )
            .unwrap();
        }
    }

    fn overwrite_text_center_align<'a>(
        &self,
        old_lines: impl Iterator<Item = Cow<'a, str>>,
        new_lines: impl Iterator<Item = Cow<'a, str>>,
    ) {
        let inner_size = self.inner_size();
        let corner_pos = if self.outline.is_some() {
            self.pos + Vec2::splat(1)
        } else {
            self.pos
        };

        let old_lines = old_lines.take(inner_size.y as usize);
        let mut new_lines = new_lines.take(inner_size.y as usize);
        let mut index = 0;

        for old_line in old_lines {
            let old_line_len = old_line.chars().count();
            if let Some(new_line) = new_lines.next().filter(|l| !l.is_empty()) {
                let new_line_len = new_line.chars().count();
                if new_line_len >= old_line_len {
                    queue!(
                        io::stdout(),
                        cursor::MoveTo(
                            corner_pos.x + ((inner_size.x - new_line_len as u16) / 2),
                            corner_pos.y + index as u16,
                        ),
                        style::Print(new_line),
                    )
                    .unwrap();
                } else {
                    let old_line_start = (inner_size.x - old_line_len as u16) / 2;
                    let old_line_end = old_line_start + inner_size.x;
                    let new_line_start = (inner_size.x - new_line_len as u16) / 2;
                    let new_line_end = new_line_start + inner_size.x + 1;
                    queue!(
                        io::stdout(),
                        cursor::MoveTo(corner_pos.x + old_line_start, corner_pos.y + index),
                        style::Print(Repeat(' ', new_line_start - old_line_start)),
                        style::Print(new_line),
                        style::Print(Repeat(
                            ' ',
                            (new_line_end - old_line_end).min(
                                inner_size.x
                                    - (new_line_start - old_line_start)
                                    - new_line_len as u16
                            )
                        ))
                    )
                    .unwrap();
                }
            } else if !old_line.is_empty() {
                queue!(
                    io::stdout(),
                    cursor::MoveTo(
                        corner_pos.x + ((inner_size.x - old_line_len as u16) / 2),
                        corner_pos.y + index as u16,
                    ),
                    style::Print(Repeat(' ', old_line_len as u16)),
                )
                .unwrap();
            }
            index += 1;
        }

        for line in new_lines {
            if !line.is_empty() {
                queue!(
                    io::stdout(),
                    cursor::MoveTo(
                        corner_pos.x + ((inner_size.x - line.chars().count() as u16) / 2),
                        corner_pos.y + index as u16,
                    ),
                    style::Print(line),
                )
                .unwrap();
            }
            index += 1;
        }
    }

    fn overwrite_text_right_align<'a>(
        &self,
        old_lines: impl Iterator<Item = Cow<'a, str>>,
        new_lines: impl Iterator<Item = Cow<'a, str>>,
    ) {
        let inner_size = self.inner_size();
        let corner_pos = {
            let outer_pos = self.pos.map_x(|x| x + self.size.x);
            if self.outline.is_some() {
                Vec2::new(outer_pos.x - 1, outer_pos.y + 1)
            } else {
                outer_pos
            }
        };

        let old_lines = old_lines.take(inner_size.y as usize);
        let mut new_lines = new_lines.take(inner_size.y as usize);
        let mut index = 0;

        for old_line in old_lines {
            let old_line_len = old_line.chars().count();
            if let Some(new_line) = new_lines.next().filter(|l| !l.is_empty()) {
                let new_line_len = new_line.chars().count();
                if new_line_len >= old_line_len {
                    queue!(
                        io::stdout(),
                        cursor::MoveTo(corner_pos.x - new_line_len as u16, corner_pos.y + index),
                        style::Print(new_line),
                    )
                    .unwrap();
                } else {
                    queue!(
                        io::stdout(),
                        cursor::MoveTo(corner_pos.x - old_line_len as u16, corner_pos.y + index),
                        style::Print(Repeat(' ', (old_line_len - new_line_len) as u16)),
                        style::Print(new_line),
                    )
                    .unwrap();
                }
            } else {
                queue!(
                    io::stdout(),
                    cursor::MoveTo(corner_pos.x - old_line_len as u16, corner_pos.y + index),
                    style::Print(Repeat(' ', old_line_len as u16)),
                )
                .unwrap();
            }
            index += 1;
        }
        for line in new_lines {
            if !line.is_empty() {
                queue!(
                    io::stdout(),
                    cursor::MoveTo(
                        corner_pos.x - line.chars().count() as u16,
                        corner_pos.y + index as u16
                    ),
                    style::Print(line),
                )
                .unwrap();
            }
            index += 1;
        }
    }

    pub fn inner_size(&self) -> Vec2<u16> {
        if self.outline.is_some() {
            self.size - Vec2::splat(2)
        } else {
            self.size
        }
    }

    pub fn new() -> Self {
        Self {
            pos: Vec2::splat(0),
            size: Vec2::new(5, 3),
            outline: Some(BoxOutline::LIGHT),
            text_align_h: TextAlignH::Center,
            text_align_v: TextAlignV::Center,
            outline_color: Color::White,
            content_color: Color::White,
            attributes: Attributes::default(),
        }
    }

    builder_impl::field!(pub pos(pos: Vec2<u16>));
    builder_impl::field!(pub x(pos.x: u16));
    builder_impl::field!(pub y(pos.y: u16));

    builder_impl::field!(pub size(size: Vec2<u16>));
    builder_impl::field!(pub width(size.x: u16));
    builder_impl::field!(pub height(size.y: u16));

    builder_impl::field!(pub outline(outline: Option<BoxOutline>));

    builder_impl::field!(pub text_align_h(text_align_h: TextAlignH));
    builder_impl::field!(pub text_align_v(text_align_v: TextAlignV));

    builder_impl::field!(pub outline_color(outline_color: Color));
    builder_impl::field!(pub content_color(content_color: Color));
    pub fn color(&mut self, color: Color) -> &mut Self {
        self.outline_color = color;
        self.content_color = color;
        self
    }

    builder_impl::field!(pub attributes(attributes: Attributes));
    pub fn set_attribute(&mut self, attribute: Attribute) -> &mut Self {
        self.attributes.set(attribute);
        self
    }
    pub fn unset_attribute(&mut self, attribute: Attribute) -> &mut Self {
        self.attributes.unset(attribute);
        self
    }
    pub fn toggle_attribute(&mut self, attribute: Attribute) -> &mut Self {
        self.attributes.toggle(attribute);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoxOutline {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
}

#[allow(dead_code)]
impl BoxOutline {
    pub const LIGHT: Self = Self {
        tl: '┌',
        tr: '┐',
        bl: '└',
        br: '┘',
        h: '─',
        v: '│',
    };

    pub const HEAVY: Self = Self {
        tl: '┏',
        tr: '┓',
        bl: '┗',
        br: '┛',
        h: '━',
        v: '┃',
    };

    pub const DOUBLE: Self = Self {
        tl: '╔',
        tr: '╗',
        bl: '╚',
        br: '╝',
        h: '═',
        v: '║',
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TextAlignH {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TextAlignV {
    Top,
    Center,
    Bottom,
}
