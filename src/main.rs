use argh::FromArgs;

mod debug;
mod flashcards;
mod input;
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
