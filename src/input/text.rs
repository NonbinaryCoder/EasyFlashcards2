use std::{borrow::Cow, io, rc::Rc};

use crossterm::{
    cursor,
    event::KeyCode,
    queue,
    style::{self, Color},
};

use crate::{
    output::{self, text_box::OutlineType, word_wrap::WordWrap, Repeat, TextAlign},
    vec2::{Rect, Vec2},
};

#[derive(Debug)]
pub struct TextInput<S: AsRef<str> = Rc<str>> {
    dims: Rect<u16>,
    outline_type: Option<OutlineType>,
    outline_color: Color,
    text_buffers: (String, String),
    cursor_pos: usize,
    correct_answer: Option<S>,
}

impl<S: AsRef<str>> TextInput<S> {
    pub fn new(dims: Rect<u16>) -> Self {
        Self {
            dims: Self::make_valid_dims(dims),
            outline_type: None,
            outline_color: Color::Reset,
            text_buffers: Default::default(),
            cursor_pos: 0,
            correct_answer: None,
        }
    }

    fn make_valid_dims(mut dims: Rect<u16>) -> Rect<u16> {
        dims.size = dims.size.join(Vec2::new(5, 3), |a, b| a.max(b));
        dims
    }

    /// Reads a single key code of input
    /// Does not move the cursor, [`go_to_cursor`] should be called after this
    /// If the user pressed enter, returns the text in this
    pub fn read_input(&mut self, code: KeyCode) -> Option<&str> {
        fn redraw_text<S: AsRef<str>, O>(
            this: &mut TextInput<S>,
            mut f: impl FnMut(&mut String, usize) -> O,
        ) -> O {
            let ret = f(&mut this.text_buffers.0, this.cursor_pos);
            this.overwrite_text();
            f(&mut this.text_buffers.1, this.cursor_pos);
            ret
        }

        match code {
            KeyCode::Enter => Some(&self.text_buffers.0),
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos =
                        output::floor_char_boundary(&self.text_buffers.0, self.cursor_pos - 1);
                    redraw_text(self, String::remove);
                }
                None
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.text_buffers.0.len() {
                    redraw_text(self, String::remove);
                }
                None
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos =
                        output::floor_char_boundary(&self.text_buffers.0, self.cursor_pos - 1);
                }
                None
            }
            KeyCode::Right => {
                if let Some(cursor_pos) =
                    output::ceil_char_boundary(&self.text_buffers.0, self.cursor_pos + 1)
                {
                    self.cursor_pos = cursor_pos;
                }
                None
            }
            KeyCode::Char(c) => {
                redraw_text(self, |s, pos| s.insert(pos, c));
                self.cursor_pos += c.len_utf8();
                None
            }
            _ => None,
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text_buffers.0
    }

    fn overwrite_text(&self) {
        if let Some(correct_answer) = &self.correct_answer {
            let dims = self.inner_size();

            let mut buffers = (
                WordWrap::new(&self.text_buffers.0, dims.size.x as usize),
                WordWrap::new(&self.text_buffers.1, dims.size.x as usize),
            );
            let mut correct_answer = WordWrap::new(correct_answer.as_ref(), dims.size.x as usize);
            for y in dims.pos.y..dims.pos.y + dims.size.y {
                let buffers = (buffers.0.next(), buffers.1.next());
                let correct_answer = correct_answer.next();

                if buffers.0.is_none() && buffers.1.is_none() && correct_answer.is_none() {
                    break;
                }

                if buffers.0 != buffers.1 {
                    if buffers.0.is_some() || correct_answer.is_some() {
                        draw_diff_line(dims.pos.x, y, buffers.0.as_ref(), correct_answer.as_ref());
                    } else {
                        queue!(io::stdout(), cursor::MoveTo(dims.pos.x, y)).unwrap();
                    }
                    if let Some(buffer_1) = buffers.1 {
                        if let Some(len) = buffer_1.trim_end().chars().count().checked_sub(
                            buffers
                                .0
                                .unwrap_or(Cow::Borrowed(""))
                                .trim_end()
                                .chars()
                                .count(),
                        ) {
                            queue!(io::stdout(), style::Print(Repeat(' ', len as u16))).unwrap();
                        }
                    }
                }
            }
        } else {
            output::overwrite_text(
                self.inner_size(),
                Color::White,
                &self.text_buffers.1,
                TextAlign::TOP_LEFT,
                &self.text_buffers.0,
                TextAlign::TOP_LEFT,
                false,
            );
        }
    }

    fn redraw_text(&mut self) {
        if let Some(correct_answer) = &self.correct_answer {
            let dims = self.inner_size();

            let mut first = WordWrap::new(&self.text_buffers.0, dims.size.x as usize);
            let mut second = WordWrap::new(correct_answer.as_ref(), dims.size.x as usize);
            for y in dims.pos.y..dims.pos.y + dims.size.y {
                let first = first.next();
                let second = second.next();

                if first.is_none() && second.is_none() {
                    break;
                }

                draw_diff_line(dims.pos.x, y, first.as_ref(), second.as_ref())
            }
        } else {
            output::draw_text(
                self.inner_size(),
                Color::White,
                &self.text_buffers.0,
                TextAlign::TOP_LEFT,
            );
        }
    }

    fn erase_text(&mut self) {
        if let Some(correct_answer) = &self.correct_answer {
            let dims = self.inner_size();

            fn find_len(t: Option<Cow<str>>) -> u16 {
                t.map(|t| t.chars().count()).unwrap_or(0) as u16
            }

            let mut first = WordWrap::new(&self.text_buffers.0, dims.size.x as usize);
            let mut second = WordWrap::new(correct_answer.as_ref(), dims.size.x as usize);
            for y in dims.pos.y..dims.pos.y + dims.size.y {
                let first = first.next();
                let second = second.next();

                if first.is_none() && second.is_none() {
                    break;
                }

                queue!(
                    io::stdout(),
                    cursor::MoveTo(dims.pos.x, y),
                    style::Print(Repeat(' ', find_len(first).max(find_len(second))))
                )
                .unwrap();
            }
        } else {
            output::overwrite_text(
                self.inner_size(),
                Color::White,
                &self.text_buffers.1,
                TextAlign::TOP_LEFT,
                "",
                TextAlign::TOP_LEFT,
                true,
            )
        }
    }

    pub fn update(&mut self, f: impl FnOnce(&mut TextInputUpdater<S>)) {
        let mut updater = TextInputUpdater {
            inner: self,
            redraw_outline: false,
            new_correct_answer: None,
        };
        f(&mut updater);
        let TextInputUpdater {
            inner: _,
            redraw_outline,
            new_correct_answer,
        } = updater;

        if redraw_outline {
            output::draw_outline(
                self.dims,
                self.outline_color,
                self.outline_type.unwrap_or(OutlineType::ERASE),
            );
        }

        if let Some(new_correct_answer) = new_correct_answer {
            self.erase_text();
            self.correct_answer = new_correct_answer;
            self.text_buffers.1.clone_from(&self.text_buffers.0);
            self.redraw_text();
        } else if self.text_buffers.0 != self.text_buffers.1 {
            self.overwrite_text();
            self.text_buffers.1.clone_from(&self.text_buffers.0);
        }
    }

    fn inner_size(&self) -> Rect<u16> {
        self.dims.shrink_centered(Vec2::splat(1))
    }

    // Moves the terminal cursor to the position of the cursor in this
    pub fn go_to_cursor(&self) {
        let mut last_len = self.text_buffers.0.len();
        let mut wrap = WordWrap::new(&self.text_buffers.0, (self.dims.size.x - 2) as usize);
        let mut cursor_pos = self.cursor_pos;
        for y in 0.. {
            if let Some(line) = wrap.next() {
                let len = wrap.remaining_text_len();
                let diff = last_len - len;
                if cursor_pos < diff || len == 0 {
                    if cursor_pos < line.len() {
                        queue!(
                            io::stdout(),
                            cursor::MoveTo(
                                self.dims.pos.x + 1 + (line[..cursor_pos].chars().count()) as u16,
                                self.dims.pos.y + 1 + y,
                            ),
                        )
                        .unwrap();
                    } else {
                        queue!(
                            io::stdout(),
                            cursor::MoveTo(
                                self.dims.pos.x
                                    + 1
                                    + line.chars().count() as u16
                                    + (cursor_pos - line.len()) as u16,
                                self.dims.pos.y + 1 + y,
                            ),
                        )
                        .unwrap();
                    }
                    break;
                }
                last_len = len;
                cursor_pos -= diff;
            } else {
                queue!(
                    io::stdout(),
                    cursor::MoveTo(self.dims.pos.x + 1, self.dims.pos.y + 1 + y,),
                )
                .unwrap();
                break;
            }
        }
    }

    pub fn hide(&mut self) {
        self.update(|updater| {
            updater.clear_outline().clear_text().clear_correct_answer();
        });
    }

    pub fn correct_answer_is(&mut self, answer: S) {
        self.correct_answer = Some(answer);
        self.redraw_text();
    }

    /// Moves and redraws this without erasing it's past position first
    pub fn force_move_resize(&mut self, new_dims: Rect<u16>) {
        self.dims = Self::make_valid_dims(new_dims);
        if let Some(outline_type) = self.outline_type {
            output::draw_outline(self.dims, self.outline_color, outline_type);
        }
        self.redraw_text();
    }
}

fn draw_diff_line(x: u16, y: u16, first_line: Option<&Cow<str>>, second_line: Option<&Cow<str>>) {
    match (first_line, second_line) {
        (None, None) => unreachable!(),
        (None, Some(line)) => {
            queue!(
                io::stdout(),
                cursor::MoveTo(x, y),
                style::SetForegroundColor(Diff::SecondOnly.color()),
                style::Print(line),
            )
            .unwrap();
        }
        (Some(line), None) => {
            queue!(
                io::stdout(),
                cursor::MoveTo(x, y),
                style::SetForegroundColor(Diff::FirstOnly.color()),
                style::Print(line),
            )
            .unwrap();
        }
        (Some(first_line), Some(second_line)) => {
            queue!(io::stdout(), cursor::MoveTo(x, y)).unwrap();

            let mut last_diff: Option<Diff> = None;
            let mut print_next_diff = |diff, ch: char| {
                if last_diff == Some(diff) {
                    queue!(io::stdout(), style::Print(ch)).unwrap();
                } else {
                    last_diff = Some(diff);
                    queue!(
                        io::stdout(),
                        style::SetForegroundColor(diff.color()),
                        style::Print(ch),
                    )
                    .unwrap();
                }
            };

            let mut first = first_line.trim_end().chars();
            let mut second = second_line.trim_end().chars();

            loop {
                let first = first.next();
                let second = second.next();

                if first.is_none() && second.is_none() {
                    break;
                }

                match (
                    first.filter(|c| !c.is_whitespace()),
                    second.filter(|c| !c.is_whitespace()),
                ) {
                    (None, None) => {
                        queue!(io::stdout(), style::Print(' ')).unwrap();
                    }
                    (None, Some(s)) => print_next_diff(Diff::SecondOnly, s),
                    (Some(f), None) => print_next_diff(Diff::FirstOnly, f),
                    (Some(f), Some(s)) if f == s => print_next_diff(Diff::Same, f),
                    (Some(f), Some(_)) => print_next_diff(Diff::Different, f),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TextInputUpdater<'a, S: AsRef<str>> {
    inner: &'a mut TextInput<S>,
    redraw_outline: bool,
    new_correct_answer: Option<Option<S>>,
}

impl<'a, S: AsRef<str>> TextInputUpdater<'a, S> {
    pub fn set_outline(&mut self, outline: OutlineType) -> &mut Self {
        self.redraw_outline |= !set_and_compare(&mut self.inner.outline_type, Some(outline));
        self
    }

    pub fn clear_outline(&mut self) -> &mut Self {
        self.redraw_outline |= !set_and_compare(&mut self.inner.outline_type, None);
        self
    }

    pub fn clear_text(&mut self) -> &mut Self {
        self.inner.text_buffers.0.clear();
        self.inner.cursor_pos = 0;
        self
    }

    pub fn clear_correct_answer(&mut self) -> &mut Self {
        self.new_correct_answer = Some(None);
        self
    }
}

/// Sets `dst` to `new`, and returns true if they compare equal
fn set_and_compare<T: PartialEq>(dst: &mut T, new: T) -> bool {
    let flag = *dst == new;
    *dst = new;
    flag
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Diff {
    Same,
    Different,
    FirstOnly,
    SecondOnly,
}

impl Diff {
    pub const fn color(self) -> Color {
        match self {
            Diff::Same => Color::White,
            Diff::Different => Color::DarkRed,
            Diff::FirstOnly => Color::Rgb {
                r: 255,
                g: 105,
                b: 180,
            },
            Diff::SecondOnly => Color::Rgb {
                r: 128,
                g: 128,
                b: 128,
            },
        }
    }
}
