use std::io;

use crossterm::{
    cursor, queue,
    style::{self, Color},
};

use crate::{output::Repeat, vec2::Vec2};

#[derive(Debug)]
pub struct Footer {
    vals: [u32; 4],
    width: u16,
    y: u16,
}

impl Footer {
    /// Constructs this with `count` black items and renders this
    pub fn new(count: u32, term_size: Vec2<u16>) -> Self {
        let this = Footer {
            vals: [count, 0, 0, 0],
            width: term_size.x,
            y: term_size.y - 1,
        };
        this.render();
        this
    }

    /// Moves an item from `curr` to `dst` and renders this
    pub fn r#move(&mut self, curr: FooterColor, dst: FooterColor) {
        self.vals[dst as usize] += 1;
        self.vals[curr as usize] -= 1;
        self.render();
    }

    pub fn resize(&mut self, term_size: Vec2<u16>) {
        self.width = term_size.x;
        self.y = term_size.y - 1;
        self.render();
    }

    #[rustfmt::skip]
    pub fn render(&self) {
        queue!(io::stdout(), cursor::MoveTo(0, self.y)).unwrap();

        let count = self.vals.into_iter().sum::<u32>() as f32;
        let width = self.width as f32;

        fn print_section(amount: u32, width: u16, color: Color) {
            let amount = format!("{amount}");
            let amount = &amount[..(width as usize).min(amount.len())];
            let pad = width - amount.len() as u16;
            let left_pad = pad / 2;
            let right_pad = pad - left_pad;
            queue!(
                io::stdout(),
                style::SetBackgroundColor(color),
                style::Print(Repeat(' ', left_pad)),
                style::Print(amount),
                style::Print(Repeat(' ', right_pad)),
            )
            .unwrap();
        }

        let width = |val: u32| (((val as f32) / count) * width) as u16;
        let green_width  = width(self.vals[FooterColor::Green  as usize]);
        let yellow_width = width(self.vals[FooterColor::Yellow as usize]);
        let red_width    = width(self.vals[FooterColor::Red    as usize]);

        print_section(self.vals[FooterColor::Green as usize],  green_width,  Color::Green);
        print_section(self.vals[FooterColor::Yellow as usize], yellow_width, Color::Yellow);
        print_section(self.vals[FooterColor::Red as usize], red_width, Color::Red);
        print_section(
            self.vals[FooterColor::Black as usize],
            self.width
                .saturating_sub(green_width)
                .saturating_sub(yellow_width)
                .saturating_sub(red_width),
            Color::Black,
        );

        queue!(io::stdout(), style::SetBackgroundColor(Color::Reset)).unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FooterColor {
    Black = 0,
    Red = 1,
    Yellow = 2,
    Green = 3,
}
