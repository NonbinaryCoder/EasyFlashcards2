use std::{borrow::Cow, io};

use crossterm::{
    cursor, queue,
    style::{self, Color},
};

use crate::{
    output::{word_wrap::WordWrap, Repeat},
    vec2::{Rect, Vec2},
};

use super::{TextAlign, TextAlignH, TextAlignV};

#[derive(Debug)]
pub struct TextBox<S: AsRef<str>> {
    dims: Rect<u16>,
    outline_type: Option<OutlineType>,
    outline_color: Color,
    text: Option<S>,
    text_align: TextAlign,
    text_color: Color,
}

impl<S: AsRef<str>> TextBox<S> {
    pub fn new(dims: Rect<u16>) -> Self {
        Self {
            dims: Self::make_valid_dims(dims),
            outline_type: None,
            outline_color: Color::White,
            text: None,
            text_align: TextAlign::center(),
            text_color: Color::White,
        }
    }

    fn make_valid_dims(mut dims: Rect<u16>) -> Rect<u16> {
        dims.size.x = dims.size.x.max(5);
        dims.size.y = dims.size.y.max(3);
        dims
    }

    pub fn from_fn(dims: Rect<u16>, f: impl FnOnce(&mut TextBoxUpdater<S>)) -> TextBox<S> {
        let mut this = Self::new(dims);
        this.update(f);
        this
    }

    pub fn update(&mut self, f: impl FnOnce(&mut TextBoxUpdater<S>)) {
        let mut updater = TextBoxUpdater {
            new_text: None,
            new_text_align: self.text_align,
            redraw_text: false,
            redraw_outline: false,
            inner: self,
        };
        f(&mut updater);
        let TextBoxUpdater {
            inner: _,
            new_text,
            new_text_align,
            redraw_text,
            redraw_outline,
        } = updater;

        if redraw_outline {
            draw_outline(
                self.dims,
                self.outline_color,
                self.outline_type.unwrap_or(OutlineType::ERASE),
            );
        }

        if redraw_text {
            match (self.text.as_ref(), new_text) {
                (None, None) => unreachable!(),
                (None, Some(new_text)) => {
                    draw_text(
                        self.inner_size(),
                        self.text_color,
                        new_text.as_ref(),
                        self.text_align,
                    );
                    self.text = Some(new_text);
                }
                (Some(old_text), None) => {
                    overwrite_text(
                        self.inner_size(),
                        self.text_color,
                        old_text.as_ref(),
                        self.text_align,
                        "",
                        TextAlign::new(TextAlignH::Left, TextAlignV::Top),
                    );
                    self.text = None;
                }
                (Some(old_text), Some(new_text)) => {
                    overwrite_text(
                        self.inner_size(),
                        self.text_color,
                        old_text.as_ref(),
                        self.text_align,
                        new_text.as_ref(),
                        new_text_align,
                    );
                    self.text = Some(new_text);
                }
            }
            self.text_align = new_text_align;
        }
    }

    /// Moves and redraws this without erasing it's past position first
    pub fn force_move_resize(&mut self, new_dims: Rect<u16>) {
        self.dims = Self::make_valid_dims(new_dims);
        if let Some(outline_type) = self.outline_type {
            draw_outline(self.dims, self.outline_color, outline_type);
        }
        if let Some(text) = &self.text {
            draw_text(
                self.inner_size(),
                self.text_color,
                text.as_ref(),
                self.text_align,
            )
        }
    }

    fn inner_size(&self) -> Rect<u16> {
        self.dims.shrink_centered(Vec2::splat(1))
    }

    pub fn get_text(&self) -> &Option<S> {
        &self.text
    }
}

fn draw_outline(dims: Rect<u16>, color: Color, r#type: OutlineType) {
    queue!(
        io::stdout(),
        dims.pos.move_to(),
        style::SetForegroundColor(color),
        style::Print(r#type.tl),
        style::Print(Repeat(r#type.h, dims.size.x - 2)),
        style::Print(r#type.tr)
    )
    .unwrap();
    for _ in 0..(dims.size.y - 2) {
        queue!(
            io::stdout(),
            cursor::MoveDown(1),
            cursor::MoveToColumn(dims.pos.x),
            style::Print(r#type.v),
            cursor::MoveRight(dims.size.x - 2),
            style::Print(r#type.v),
        )
        .unwrap();
    }
    queue!(
        io::stdout(),
        cursor::MoveDown(1),
        cursor::MoveToColumn(dims.pos.x),
        style::Print(r#type.bl),
        style::Print(Repeat(r#type.h, dims.size.x - 2)),
        style::Print(r#type.br)
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

fn draw_text(dims: Rect<u16>, color: Color, text: &str, align: TextAlign) {
    let lines = get_lines_iter(dims.size, text, align.v);
    queue!(
        io::stdout(),
        dims.pos.move_to(),
        style::SetForegroundColor(color)
    )
    .unwrap();
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

fn overwrite_text(
    dims: Rect<u16>,
    color: Color,
    old_text: &str,
    old_align: TextAlign,
    new_text: &str,
    new_align: TextAlign,
) {
    let mut old_lines = get_lines_iter(dims.size, old_text, old_align.v);
    let mut new_lines = get_lines_iter(dims.size, new_text, new_align.v);
    queue!(
        io::stdout(),
        dims.pos.move_to(),
        style::SetForegroundColor(color)
    )
    .unwrap();
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
                style::Print(new_line),
            )
            .unwrap(),
            (Some(old_line), None) => queue!(
                io::stdout(),
                cursor::MoveTo(
                    dims.pos.x + old_align.h.padding_for(old_line, dims.size.x),
                    y
                ),
                style::Print(Repeat(' ', old_line.chars().count() as u16)),
            )
            .unwrap(),
            (Some(old_line), Some(new_line)) => {
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

#[derive(Debug)]
pub struct TextBoxUpdater<'a, S: AsRef<str>> {
    inner: &'a mut TextBox<S>,
    new_text: Option<S>,
    new_text_align: TextAlign,
    redraw_text: bool,
    redraw_outline: bool,
}

impl<'a, S: AsRef<str>> TextBoxUpdater<'a, S> {
    pub fn set_text(&mut self, text: S) -> &mut Self {
        match &self.inner.text {
            Some(old_text) => self.redraw_text |= old_text.as_ref() != text.as_ref(),
            None => self.redraw_text = true,
        }
        self.new_text = Some(text);
        self
    }

    pub fn clear_text(&mut self) -> &mut Self {
        self.new_text = None;
        self.redraw_text |= self.inner.text.is_some();
        self
    }

    pub fn set_text_color(&mut self, color: Color) -> &mut Self {
        self.redraw_text |= !set_and_compare(&mut self.inner.text_color, color);
        self
    }

    pub fn set_outline(&mut self, outline: OutlineType) -> &mut Self {
        self.redraw_outline |= !set_and_compare(&mut self.inner.outline_type, Some(outline));
        self
    }

    pub fn add_outline(&mut self, outline: OutlineType) -> &mut Self {
        if self.inner.outline_type.is_none() {
            self.set_outline(outline)
        } else {
            self
        }
    }

    pub fn clear_outline(&mut self) -> &mut Self {
        self.redraw_outline |= !set_and_compare(&mut self.inner.outline_type, None);
        self
    }

    pub fn set_outline_color(&mut self, color: Color) -> &mut Self {
        self.redraw_outline |= !set_and_compare(&mut self.inner.outline_color, color);
        self
    }

    pub fn set_color(&mut self, color: Color) -> &mut Self {
        self.set_text_color(color).set_outline_color(color)
    }

    pub fn clear_all(&mut self) -> &mut Self {
        self.clear_outline().clear_text()
    }
}

/// Sets `dst` to `new`, and returns true if they compare equal
fn set_and_compare<T: PartialEq>(dst: &mut T, new: T) -> bool {
    let flag = *dst == new;
    *dst = new;
    flag
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineType {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
}

#[allow(dead_code)]
impl OutlineType {
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

    pub const ERASE: Self = Self {
        tl: ' ',
        tr: ' ',
        bl: ' ',
        br: ' ',
        h: ' ',
        v: ' ',
    };
}
