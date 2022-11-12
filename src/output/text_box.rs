use std::{array, io};

use crossterm::{
    cursor, queue,
    style::{self, Color},
};

use crate::{
    output::Repeat,
    vec2::{Rect, Vec2},
};

use super::TextAlign;

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
            text_align: TextAlign::CENTER,
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
            text_color_changed: false,
            new_text_align: self.text_align,
            redraw_text: false,
            redraw_outline: false,
            inner: self,
        };
        f(&mut updater);
        let TextBoxUpdater {
            inner: _,
            new_text,
            text_color_changed,
            new_text_align,
            redraw_text,
            redraw_outline,
        } = updater;

        if redraw_outline {
            super::draw_outline(
                self.dims,
                self.outline_color,
                self.outline_type.unwrap_or(OutlineType::ERASE),
            );
        }

        if redraw_text {
            match (self.text.as_ref(), new_text) {
                (None, None) => {}
                (None, Some(new_text)) => {
                    super::draw_text(
                        self.inner_size(),
                        self.text_color,
                        new_text.as_ref().map(AsRef::as_ref).unwrap_or_default(),
                        self.text_align,
                    );
                    self.text = new_text;
                }
                (Some(old_text), None) => {
                    super::overwrite_text(
                        self.inner_size(),
                        self.text_color,
                        old_text.as_ref(),
                        self.text_align,
                        "",
                        TextAlign::TOP_LEFT,
                        text_color_changed,
                    );
                    self.text = None;
                }
                (Some(old_text), Some(new_text)) => {
                    super::overwrite_text(
                        self.inner_size(),
                        self.text_color,
                        old_text.as_ref(),
                        self.text_align,
                        new_text.as_ref().map(AsRef::as_ref).unwrap_or_default(),
                        new_text_align,
                        text_color_changed,
                    );
                    self.text = new_text;
                }
            }
            self.text_align = new_text_align;
        }
    }

    /// Moves and redraws this without erasing it's past position first
    pub fn force_move_resize(&mut self, new_dims: Rect<u16>) {
        self.dims = Self::make_valid_dims(new_dims);
        if let Some(outline_type) = self.outline_type {
            super::draw_outline(self.dims, self.outline_color, outline_type);
        }
        if let Some(text) = &self.text {
            super::draw_text(
                self.inner_size(),
                self.text_color,
                text.as_ref(),
                self.text_align,
            )
        }
    }

    pub fn hide(&mut self) {
        if let Some(old_text) = self.text.as_ref() {
            super::overwrite_text(
                self.inner_size(),
                self.text_color,
                old_text.as_ref(),
                self.text_align,
                "",
                TextAlign::TOP_LEFT,
                true,
            );
        }
        if self.outline_type.is_some() {
            super::draw_outline(self.dims, self.outline_color, OutlineType::ERASE);
        }
    }

    fn inner_size(&self) -> Rect<u16> {
        self.dims.shrink_centered(Vec2::splat(1))
    }

    pub fn get_text(&self) -> &Option<S> {
        &self.text
    }
}

#[derive(Debug)]
pub struct TextBoxUpdater<'a, S: AsRef<str>> {
    inner: &'a mut TextBox<S>,
    new_text: Option<Option<S>>,
    text_color_changed: bool,
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
        self.new_text = Some(Some(text));
        self
    }

    pub fn clear_text(&mut self) -> &mut Self {
        self.new_text = Some(None);
        self.redraw_text |= self.inner.text.is_some();
        self
    }

    pub fn set_text_color(&mut self, color: Color) -> &mut Self {
        let changed = !set_and_compare(&mut self.inner.text_color, color);
        self.text_color_changed |= changed;
        self.redraw_text |= changed;
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
}

/// Sets `dst` to `new`, and returns true if they compare equal
fn set_and_compare<T: PartialEq>(dst: &mut T, new: T) -> bool {
    let flag = *dst == new;
    *dst = new;
    flag
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineType {
    pub tl: char,
    pub tr: char,
    pub bl: char,
    pub br: char,
    pub h: char,
    pub v: char,
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

#[derive(Debug)]
pub struct MultiTextBox<S: AsRef<str>, const ITEMS: usize> {
    /// The position of the top left corner border
    corner_pos: Vec2<u16>,
    /// The inner size of each text box
    item_size: Vec2<u16>,
    outline_type: Option<MultiOutlineType>,
    outline_color: Color,
    items: [MultiTextBoxItem<S>; ITEMS],
}

#[derive(Debug, Clone)]
struct MultiTextBoxItem<S: AsRef<str>> {
    text: Option<S>,
    align: TextAlign,
    color: Color,
}

impl<S: AsRef<str>, const ITEMS: usize> MultiTextBox<S, ITEMS> {
    pub fn new(dims: Rect<u16>) -> Self {
        let dims = Self::inner_dims_from_size(dims);
        Self {
            corner_pos: dims.pos,
            item_size: dims.size,
            outline_type: None,
            outline_color: Color::White,
            items: [(); ITEMS].map(|()| MultiTextBoxItem {
                text: None,
                align: TextAlign::CENTER,
                color: Color::White,
            }),
        }
    }

    fn inner_dims_from_size(dims: Rect<u16>) -> Rect<u16> {
        let line_sizes = Vec2::new(ITEMS as u16 + 1, 2);

        let size_without_borders = dims.size.join(line_sizes, u16::saturating_sub);
        let box_size = size_without_borders / Vec2::new(ITEMS as u16, 1);
        let box_size = box_size.join(Vec2::new(3, 1), Ord::max);

        let inaccuracy = dims.size.join(
            box_size * Vec2::new(ITEMS as u16, 1) + line_sizes,
            u16::saturating_sub,
        );
        let corner_pos = inaccuracy / Vec2::splat(2) + dims.pos;

        Rect {
            size: box_size,
            pos: corner_pos,
        }
    }

    pub fn update(&mut self, f: impl FnOnce(&mut MultiTextBoxUpdater<S, ITEMS>)) {
        let mut updater = MultiTextBoxUpdater {
            items: array::from_fn(|i| MultiTextBoxItemChanges {
                new_text: None,
                text_color_changed: false,
                new_align: self.items[i].align,
                redraw: false,
            }),
            redraw_outline: false,
            inner: self,
        };
        f(&mut updater);
        let MultiTextBoxUpdater {
            inner: _,
            items: new_items,
            redraw_outline,
        } = updater;

        if redraw_outline {
            draw_multi_outline(
                self.corner_pos,
                self.item_size,
                ITEMS as u16,
                self.outline_color,
                self.outline_type.unwrap_or(MultiOutlineType::ERASE),
            );
        }

        for (
            index,
            (
                MultiTextBoxItemChanges {
                    new_text,
                    text_color_changed,
                    new_align,
                    redraw,
                },
                item,
            ),
        ) in new_items.into_iter().zip(self.items.iter_mut()).enumerate()
        {
            if redraw {
                let dims = Rect {
                    pos: Vec2 {
                        x: self.corner_pos.x + 1 + ((self.item_size.x + 1) * index as u16),
                        y: self.corner_pos.y + 1,
                    },
                    size: self.item_size,
                };
                match (item.text.as_ref(), new_text) {
                    (None, None) => {}
                    (None, Some(text)) => {
                        super::draw_text(
                            dims,
                            item.color,
                            text.as_ref().map(AsRef::as_ref).unwrap_or_default(),
                            item.align,
                        );
                        item.text = text;
                    }
                    (Some(text), None) => {
                        super::draw_text(dims, item.color, text.as_ref(), item.align);
                    }
                    (Some(old_text), Some(new_text)) => {
                        super::overwrite_text(
                            dims,
                            item.color,
                            old_text.as_ref(),
                            item.align,
                            new_text.as_ref().map(AsRef::as_ref).unwrap_or_default(),
                            new_align,
                            text_color_changed,
                        );
                        item.text = new_text;
                    }
                }
                item.align = new_align;
            }
        }
    }

    /// Moves and redraws this without erasing it's past position first
    pub fn force_move_resize(&mut self, new_dims: Rect<u16>) {
        let new_dims = Self::inner_dims_from_size(new_dims);
        self.corner_pos = new_dims.pos;
        self.item_size = new_dims.size;
        if let Some(outline_type) = self.outline_type {
            draw_multi_outline(
                self.corner_pos,
                self.item_size,
                ITEMS as u16,
                self.outline_color,
                outline_type,
            );
        }
        let mut selected_pos = new_dims.pos + Vec2::splat(1);
        for item in &self.items {
            if let Some(text) = &item.text {
                super::draw_text(
                    Rect {
                        pos: selected_pos,
                        size: self.item_size,
                    },
                    item.color,
                    text.as_ref(),
                    item.align,
                );
            };
            selected_pos.x += self.item_size.x + 1;
        }
    }

    pub fn hide(&mut self) {
        self.update(|updater| {
            updater.clear_outline().foreach_text(|_, mut text| {
                text.clear();
            });
        });
    }

    pub fn text(&self, index: usize) -> Option<&S> {
        self.items[index].text.as_ref()
    }
}

fn draw_multi_outline(
    corner_pos: Vec2<u16>,
    item_size: Vec2<u16>,
    item_count: u16,
    color: Color,
    typ: MultiOutlineType,
) {
    queue!(
        io::stdout(),
        corner_pos.move_to(),
        style::SetForegroundColor(color),
        style::Print(typ.tl),
        style::Print(Repeat(typ.h, item_size.x)),
    )
    .unwrap();
    for _ in 1..item_count {
        queue!(
            io::stdout(),
            style::Print(typ.join_top),
            style::Print(Repeat(typ.h, item_size.x)),
        )
        .unwrap();
    }
    queue!(io::stdout(), style::Print(typ.tr)).unwrap();

    for y in (corner_pos.y + 1)..(corner_pos.y + 1 + item_size.y) {
        queue!(
            io::stdout(),
            cursor::MoveTo(corner_pos.x, y),
            style::Print(typ.v),
            cursor::MoveRight(item_size.x),
        )
        .unwrap();
        for _ in 1..item_count {
            queue!(
                io::stdout(),
                style::Print(typ.inner_v),
                cursor::MoveRight(item_size.x),
            )
            .unwrap();
        }
        queue!(io::stdout(), style::Print(typ.v)).unwrap();
    }

    queue!(
        io::stdout(),
        cursor::MoveTo(corner_pos.x, corner_pos.y + item_size.y + 1),
        style::Print(typ.bl),
        style::Print(Repeat(typ.h, item_size.x)),
    )
    .unwrap();
    for _ in 1..item_count {
        queue!(
            io::stdout(),
            style::Print(typ.join_bot),
            style::Print(Repeat(typ.h, item_size.x)),
        )
        .unwrap();
    }
    queue!(io::stdout(), style::Print(typ.br)).unwrap();
}

#[derive(Debug)]
pub struct MultiTextBoxUpdater<'a, S: AsRef<str>, const ITEMS: usize> {
    inner: &'a mut MultiTextBox<S, ITEMS>,
    items: [MultiTextBoxItemChanges<S>; ITEMS],
    redraw_outline: bool,
}

#[derive(Debug)]
pub struct MultiTextBoxItemChanges<S: AsRef<str>> {
    new_text: Option<Option<S>>,
    text_color_changed: bool,
    new_align: TextAlign,
    redraw: bool,
}

#[derive(Debug)]
pub struct MultiTextBoxItemUpdater<'a, S: AsRef<str>> {
    item: &'a mut MultiTextBoxItem<S>,
    changes: &'a mut MultiTextBoxItemChanges<S>,
}

impl<'a, S: AsRef<str>, const ITEMS: usize> MultiTextBoxUpdater<'a, S, ITEMS> {
    pub fn set_outline(&mut self, outline: MultiOutlineType) -> &mut Self {
        self.redraw_outline |= !set_and_compare(&mut self.inner.outline_type, Some(outline));
        self
    }

    pub fn clear_outline(&mut self) -> &mut Self {
        self.redraw_outline |= !set_and_compare(&mut self.inner.outline_type, None);
        self
    }

    pub fn text(&mut self, index: usize) -> MultiTextBoxItemUpdater<S> {
        MultiTextBoxItemUpdater {
            item: &mut self.inner.items[index],
            changes: &mut self.items[index],
        }
    }

    pub fn foreach_text(
        &mut self,
        mut f: impl FnMut(usize, MultiTextBoxItemUpdater<S>),
    ) -> &mut Self {
        for i in 0..ITEMS {
            f(i, self.text(i));
        }
        self
    }
}

impl<'a, S: AsRef<str>> MultiTextBoxItemUpdater<'a, S> {
    pub fn set(&mut self, text: S) -> &mut Self {
        match &self.item.text {
            Some(old_text) => self.changes.redraw |= old_text.as_ref() != text.as_ref(),
            None => self.changes.redraw = true,
        }
        self.changes.new_text = Some(Some(text));
        self
    }

    pub fn clear(&mut self) -> &mut Self {
        self.changes.redraw |= self.item.text.is_some();
        self.changes.new_text = Some(None);
        self
    }

    pub fn set_color(&mut self, color: Color) -> &mut Self {
        let changed = !set_and_compare(&mut self.item.color, color);
        self.changes.text_color_changed |= changed;
        self.changes.redraw |= changed;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiOutlineType {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,

    join_top: char,
    join_bot: char,
    inner_v: char,
}

impl MultiOutlineType {
    pub const DOUBLE_LIGHT: Self = Self {
        tl: '╔',
        tr: '╗',
        bl: '╚',
        br: '╝',
        h: '═',
        v: '║',

        join_top: '╤',
        join_bot: '╧',
        inner_v: '│',
    };

    pub const ERASE: Self = Self {
        tl: ' ',
        tr: ' ',
        bl: ' ',
        br: ' ',
        h: ' ',
        v: ' ',

        join_top: ' ',
        join_bot: ' ',
        inner_v: ' ',
    };
}
