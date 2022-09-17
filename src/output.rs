use termion::color;

pub mod word_wrap;

pub fn write_fatal_error(text: &str) {
    println!("{}{text}{}", color::Fg(color::Red), color::Fg(color::Reset));
}
