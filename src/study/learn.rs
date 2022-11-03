use std::path::PathBuf;

use argh::FromArgs;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};

use crate::{
    flashcards::Set,
    load_set,
    output::{self, TerminalSettings},
    vec2::Vec2,
};

use self::world::World;

mod world;

/// Learn a set
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "learn")]
pub struct Entry {
    /// the set to learn
    #[argh(positional)]
    set: PathBuf,
}

impl Entry {
    pub fn run(self) {
        let set = load_set!(&self.set);
        if set.cards.is_empty() {
            output::write_fatal_error("Set must have at least 1 card to learn");
            return;
        }

        let mut term_settings = TerminalSettings::new();
        term_settings
            .enable_raw_mode()
            .enter_alternate_screen()
            .hide_cursor();

        let mut world = World::new(
            terminal::size()
                .expect("unable to get terminal size")
                .into(),
            &set,
        );

        loop {
            match event::read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers,
                    ..
                }) if modifiers.contains(KeyModifiers::CONTROL) => {
                    term_settings.clear();
                    println!("Exited with ctrl-c");
                    return;
                }
                Event::Resize(x, y) => world.resize(Vec2::new(x, y)),
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    ..
                }) => {
                    if world.key_pressed(c).is_break() {
                        break;
                    }
                }
                _ => {}
            }
        }

        term_settings.clear();
    }
}
