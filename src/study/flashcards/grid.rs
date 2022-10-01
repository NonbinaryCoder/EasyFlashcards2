use std::{
    io::{self, Write},
    ops::{Index, IndexMut},
};

use crate::{
    flashcards::Side,
    output::{BoxOutline, TextBox},
    vec2::Vec2,
};

#[derive(Debug)]
pub struct FlashcardGrid<'a> {
    card_count: Vec2<u16>,
    card_size: Vec2<u16>,
    offset: Vec2<u16>,
    selected: Vec2<u16>,
    /// The cards that can currently be seen.
    /// The length of this is equal to `self.card_count.area()`
    cards: Vec<Option<(&'a str, Side)>>,
}

impl<'a> FlashcardGrid<'a> {
    #[must_use]
    pub fn new(card_count: Vec2<u16>) -> Self {
        FlashcardGrid {
            card_count,
            card_size: Vec2::new(5, 3),
            offset: Vec2::ZERO,
            selected: Vec2::ZERO,
            cards: vec![None; card_count.area() as usize],
        }
    }

    pub fn fill_from_text(&mut self, cards_iter: impl Iterator<Item = &'a str>) -> &mut Self {
        cards_iter
            .take(self.card_count.area() as usize)
            .map(|text| Some((text, Side::Term)))
            .enumerate()
            .for_each(|(index, value)| self.cards[index] = value);
        self
    }

    pub fn fill_from_cards(
        &mut self,
        cards_iter: impl Iterator<Item = (&'a str, Side)>,
    ) -> &mut Self {
        self.cards.clear();
        let area = self.card_count.area() as usize;
        self.cards.extend(cards_iter.take(area).map(Some));
        self.cards.resize(area, None);
        self
    }

    #[must_use]
    fn card_printer(&self) -> TextBox {
        let mut card_printer = TextBox::new();
        card_printer.text_align_h = crate::output::TextAlignH::Center;
        card_printer.text_align_v = crate::output::TextAlignV::Center;
        card_printer.size = self.card_size;
        card_printer
    }

    /// Resizes and prints this
    pub fn size_to(&mut self, term_size: Vec2<u16>) -> &mut Self {
        let card_size = Some(term_size / self.card_count).filter(|s| s.x >= 5 && s.y >= 3);
        if let Some(card_size) = card_size {
            self.card_size = card_size;
            self.offset = (term_size - (self.card_count * card_size)) / Vec2::splat(2);
            self.print();
        } else {
            self.card_size = Vec2::new(5, 3);
            self.offset = Vec2::ZERO;
        }
        self
    }

    fn print_at<'b>(&self, pos: Vec2<u16>, printer: &'b mut TextBox) -> &'b mut TextBox {
        printer.pos(pos * self.card_size + self.offset)
    }

    fn print_card<'b>(&self, pos: Vec2<u16>, printer: &'b mut TextBox) -> &'b mut TextBox {
        let index = pos.index_row_major(self.card_count.x as usize);
        if let Some((text, side)) = self.cards[index] {
            self.print_at(pos, printer)
                .outline(outline_type(pos == self.selected))
                .color(side.color())
                .draw_outline_and_text(text);
        }
        printer
    }

    pub fn print(&self) -> &Self {
        use crossterm::{queue, terminal};
        queue!(io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
        let mut printer = self.card_printer();
        for pos in Vec2::ZERO.positions_between(self.card_count) {
            self.print_card(pos, &mut printer);
        }
        io::stdout().flush().unwrap();
        self
    }

    pub fn update(&mut self, f: impl FnOnce(&mut FlashcardGridUpdater<'a, '_>)) {
        let old_cards = self.cards.clone();
        let old_selected = self.selected;
        let mut updater = FlashcardGridUpdater(self);
        f(&mut updater);

        let mut printer = self.card_printer();
        for pos in Vec2::ZERO.positions_between(self.card_count) {
            let index = pos.index_row_major(self.card_count.x as usize);
            match (old_cards[index], self.cards[index]) {
                (Some((old_text, old_side)), Some((text, side))) => {
                    let color_changed = old_side != side;
                    let redraw_outline =
                        ((pos == old_selected) != (pos == self.selected)) || color_changed;
                    let redraw_text = old_text != text || color_changed;
                    if redraw_outline || redraw_text {
                        self.print_at(pos, &mut printer)
                            .outline(outline_type(pos == self.selected))
                            .color(side.color());
                        if redraw_outline {
                            printer.draw_outline();
                        }
                        if redraw_text {
                            printer.overwrite_text(old_text, text);
                        }
                    }
                }
                (Some((old_text, _)), None) => {
                    self.print_at(pos, &mut printer)
                        .outline(Some(BoxOutline::ERASE))
                        .draw_outline()
                        .overwrite_text(old_text, "");
                }
                (None, Some(_)) => {
                    self.print_card(pos, &mut printer);
                }
                (None, None) => {}
            }
        }
        io::stdout().flush().unwrap();
    }
}

fn outline_type(selected: bool) -> Option<BoxOutline> {
    Some(match selected {
        true => BoxOutline::DOUBLE,
        false => BoxOutline::HEAVY,
    })
}

#[derive(Debug)]
pub struct FlashcardGridUpdater<'a, 'b>(&'b mut FlashcardGrid<'a>);

impl<'a, 'b> FlashcardGridUpdater<'a, 'b> {
    pub fn card_count(&self) -> Vec2<u16> {
        self.0.card_count
    }

    pub fn selected(&self) -> Vec2<u16> {
        self.0.selected
    }

    pub fn selected_mut(&mut self) -> &mut Vec2<u16> {
        &mut self.0.selected
    }

    pub fn set_selected(&mut self, selected: Vec2<u16>) {
        self.0.selected = selected;
    }

    pub fn fill_from_cards(
        &mut self,
        cards_iter: impl Iterator<Item = (&'a str, Side)>,
    ) -> &mut Self {
        self.0.fill_from_cards(cards_iter);
        self
    }
}

impl<'a, 'b> Index<Vec2<u16>> for FlashcardGridUpdater<'a, 'b> {
    type Output = Option<(&'a str, Side)>;

    fn index(&self, index: Vec2<u16>) -> &Self::Output {
        &self.0.cards[index.index_row_major(self.0.card_count.x as usize)]
    }
}

impl<'a, 'b> IndexMut<Vec2<u16>> for FlashcardGridUpdater<'a, 'b> {
    fn index_mut(&mut self, index: Vec2<u16>) -> &mut Self::Output {
        &mut self.0.cards[index.index_row_major(self.0.card_count.x as usize)]
    }
}
