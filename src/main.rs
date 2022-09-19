use argh::FromArgs;

mod debug;
mod flashcards;
mod output;
mod study;
mod vec2;

/// "Simple" flashcards app
#[derive(Debug, FromArgs)]
struct EasyFlashCards {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Debug(debug::Entry),
    Flashcards(study::flashcards::Entry),
}

fn main() {
    match argh::from_env::<EasyFlashCards>().subcommand {
        Subcommand::Debug(cmd) => cmd.run(),
        Subcommand::Flashcards(cmd) => cmd.run(),
    }
}

#[macro_export]
macro_rules! up {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Up,
            ..
        })
    };
}

#[macro_export]
macro_rules! down {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Down,
            ..
        })
    };
}

#[macro_export]
macro_rules! left {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Left,
            ..
        })
    };
}

#[macro_export]
macro_rules! right {
    () => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Right,
            ..
        })
    };
}
