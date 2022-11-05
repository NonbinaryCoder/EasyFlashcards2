use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::Color,
};

use crate::{
    output::{self, word_wrap::WordWrap, TerminalSettings, TextAlign},
    vec2::{Rect, Vec2},
};

use super::{draw_outline, overwrite_text, OutlineType};

#[derive(Debug)]
pub struct TextInput {
    dims: Rect<u16>,
    outline_type: Option<OutlineType>,
    /// 0th buffer is the one that is currently visible
    buffers: (String, String),
}

impl TextInput {
    pub fn new(dims: Rect<u16>) -> Self {
        Self {
            dims,
            outline_type: None,
            buffers: (String::new(), String::new()),
        }
    }

    pub fn get_input(&mut self, term_settings: &mut TerminalSettings) -> &str {
        fn go_to_cursor(dims: Rect<u16>, buffer: &str, mut cursor_pos: usize) {
            let mut last_len = buffer.len();
            let mut wrap = WordWrap::new(buffer, (dims.size.x - 2) as usize);
            for y in 0.. {
                if let Some(line) = wrap.next() {
                    let len = wrap.remaining_text_len();
                    let diff = last_len - len;
                    if cursor_pos < diff || len == 0 {
                        queue!(
                            io::stdout(),
                            cursor::MoveTo(
                                dims.pos.x + 1 + (line[..cursor_pos].chars().count()) as u16,
                                dims.pos.y + 1 + y,
                            ),
                        )
                        .unwrap();
                        break;
                    }
                    last_len = len;
                    cursor_pos -= diff;
                } else {
                    queue!(
                        io::stdout(),
                        cursor::MoveTo(dims.pos.x + 1, dims.pos.y + 1 + y,),
                    )
                    .unwrap();
                    break;
                }
            }
        }
        term_settings.show_cursor();
        let mut cursor_pos = self.buffers.0.len();
        go_to_cursor(self.dims, &self.buffers.0, cursor_pos);
        io::stdout().flush().unwrap();
        loop {
            match event::read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers,
                    ..
                }) if modifiers.contains(KeyModifiers::CONTROL) => {
                    panic!("Exited with ctrl-c");
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    ..
                }) => {
                    break;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                }) => {
                    if cursor_pos > 0 {
                        let idx = output::floor_char_boundary(&self.buffers.0, cursor_pos - 1);
                        let c = self.buffers.0.remove(idx);
                        overwrite_text(
                            self.inner_size(),
                            Color::Reset,
                            &self.buffers.1,
                            TextAlign::TOP_LEFT,
                            &self.buffers.0,
                            TextAlign::TOP_LEFT,
                        );
                        self.buffers.1.remove(idx);
                        cursor_pos -= c.len_utf8();
                        go_to_cursor(self.dims, &self.buffers.0, cursor_pos);
                        io::stdout().flush().unwrap();
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    ..
                }) => {
                    if cursor_pos > 0 {
                        cursor_pos = output::floor_char_boundary(&self.buffers.0, cursor_pos - 1);
                        go_to_cursor(self.dims, &self.buffers.0, cursor_pos);
                        io::stdout().flush().unwrap();
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    ..
                }) => {
                    if let Some(pos) = output::ceil_char_boundary(&self.buffers.0, cursor_pos + 1) {
                        cursor_pos = pos;
                        go_to_cursor(self.dims, &self.buffers.0, cursor_pos);
                        io::stdout().flush().unwrap();
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    ..
                }) => {
                    self.buffers.0.insert(cursor_pos, c);
                    overwrite_text(
                        self.inner_size(),
                        Color::Reset,
                        &self.buffers.1,
                        TextAlign::TOP_LEFT,
                        &self.buffers.0,
                        TextAlign::TOP_LEFT,
                    );
                    self.buffers.1.insert(cursor_pos, c);
                    cursor_pos += c.len_utf8();
                    go_to_cursor(self.dims, &self.buffers.0, cursor_pos);
                    io::stdout().flush().unwrap();
                }
                _ => {}
            }
        }
        term_settings.hide_cursor();
        &self.buffers.0
    }

    pub fn set_outline(&mut self, outline: OutlineType) -> &mut Self {
        if self.outline_type != Some(outline) {
            draw_outline(self.dims, Color::Reset, outline);
        }
        self
    }

    pub fn hide(&mut self) {
        if self.outline_type.is_some() {
            draw_outline(self.dims, Color::White, OutlineType::ERASE);
        }
        if !self.buffers.0.is_empty() {
            overwrite_text(
                self.inner_size(),
                Color::Reset,
                &self.buffers.0,
                TextAlign::TOP_LEFT,
                "",
                TextAlign::TOP_LEFT,
            );
            self.buffers.0.clear();
            self.buffers.1.clear();
        }
    }

    fn inner_size(&self) -> Rect<u16> {
        self.dims.shrink_centered(Vec2::splat(1))
    }
}
