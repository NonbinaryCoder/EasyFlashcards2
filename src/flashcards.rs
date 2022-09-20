use std::{
    fmt::{Display, Write},
    fs, io,
    ops::{Index, IndexMut},
    path::Path,
    str::FromStr,
};

use crossterm::{
    cursor, queue,
    style::{self, Attribute, Color},
};
use rand::seq::SliceRandom;
use smallvec::{smallvec, SmallVec};

use crate::{
    output::{self, word_wrap::WordWrap, Repeat},
    vec2::Vec2,
};

#[derive(Debug, Default, Clone)]
pub struct Set {
    pub recall_t: RecallSettings,
    pub recall_d: RecallSettings,
    pub cards: Vec<Flashcard>,
}

impl Set {
    /// Loads the set from the path specified, printing error information if it cannot
    /// be loaded
    pub fn load_from_file_path(path: &Path) -> Option<Self> {
        match fs::read_to_string(path) {
            Ok(f) => match Set::from_str(&f) {
                Ok(set) => Some(set),
                Err(errors) => {
                    let mut s = String::new();
                    for error in errors {
                        writeln!(s, "{error}").unwrap();
                    }
                    output::write_fatal_error(&s);
                    None
                }
            },
            Err(err) => {
                output::write_fatal_error(&format!("Unable to open set: {err}"));
                None
            }
        }
    }
}

impl FromStr for Set {
    type Err = Vec<ParseBlockError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        impl RecallSettings {
            fn update_from_lines<'a>(
                &mut self,
                line_number: u32,
                lines: &mut impl Iterator<Item = (u32, &'a str)>,
                errors: &mut Vec<ParseBlockError>,
            ) {
                let mut inner_errors = Vec::new();

                for (line_number, line) in lines {
                    match line {
                        "matching" => self.matching = true,
                        "text" => self.text = true,
                        "" => break,
                        _ => inner_errors.push(ParseRecallTypeError::UnknownSetting {
                            name: line.to_owned(),
                            line_number,
                        }),
                    }
                }

                if !inner_errors.is_empty() {
                    errors.push(ParseBlockError::ParseRecallTypeErrors {
                        errors: inner_errors,
                        line_number,
                    });
                }
            }
        }

        fn flashcard_from_lines<'a>(
            first_line_number: u32,
            first_line: &str,
            lines: &mut impl Iterator<Item = (u32, &'a str)>,
        ) -> Result<Flashcard, Vec<ParseFlashcardItemError>> {
            fn trim(s: &str) -> &str {
                s.chars()
                    .next()
                    .map(|c| if c.is_ascii_whitespace() { &s[1..] } else { s })
                    .unwrap_or(s)
            }

            let mut card = Flashcard::empty();
            let mut errors = Vec::new();

            let mut parse_line = |line_number, line: &str| {
                if line.is_empty() {
                    true
                } else {
                    match line.split_once(':') {
                        Some(("T", term)) => card[Side::Term].push(trim(term).to_owned()),
                        Some(("D", definition)) => {
                            card[Side::Definition].push(trim(definition).to_owned())
                        }
                        Some((tag, _)) => errors.push(ParseFlashcardItemError::UnknownTag {
                            tag: tag.to_owned(),
                            line_number,
                        }),
                        None => errors.push(ParseFlashcardItemError::MissingTag { line_number }),
                    }
                    false
                }
            };

            if !parse_line(first_line_number, first_line) {
                for (line_number, line) in lines {
                    if parse_line(line_number, line) {
                        break;
                    }
                }
            }

            if errors.is_empty() && card.is_valid() {
                Ok(card)
            } else {
                if !card.term.is_valid() {
                    errors.push(ParseFlashcardItemError::MissingSide(Side::Term))
                };
                if !card.definition.is_valid() {
                    errors.push(ParseFlashcardItemError::MissingSide(Side::Definition))
                };
                Err(errors)
            }
        }

        let mut recall_t = RecallSettings::default();
        let mut recall_d = RecallSettings::default();
        let mut cards = Vec::new();

        let mut errors = Vec::new();

        let mut lines = (1..).zip(s.lines().map(str::trim));
        while let Some((line_number, line)) = lines.next() {
            if line.is_empty() {
                continue;
            } else if line.starts_with('[') {
                match line {
                    "[recall_t]" => {
                        recall_t.update_from_lines(line_number, &mut lines, &mut errors)
                    }
                    "[recall_d]" => {
                        recall_d.update_from_lines(line_number, &mut lines, &mut errors)
                    }
                    _ => {
                        errors.push(ParseBlockError::UnknownBlock {
                            name: line.to_owned(),
                            line_number,
                        });
                        for (_, line) in lines.by_ref() {
                            if line.is_empty() {
                                break;
                            }
                        }
                    }
                }
            } else {
                match flashcard_from_lines(line_number, line, &mut lines) {
                    Ok(card) => cards.push(card),
                    Err(err) => {
                        if !err.is_empty() {
                            errors.push(ParseBlockError::ParseFlashcardErrors {
                                errors: err,
                                line_number,
                            })
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(Set {
                recall_t,
                recall_d,
                cards,
            })
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug)]
pub enum ParseBlockError {
    UnknownBlock {
        name: String,
        line_number: u32,
    },
    ParseRecallTypeErrors {
        errors: Vec<ParseRecallTypeError>,
        line_number: u32,
    },
    ParseFlashcardErrors {
        errors: Vec<ParseFlashcardItemError>,
        line_number: u32,
    },
}

impl Display for ParseBlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseBlockError::*;
        match self {
            UnknownBlock { name, line_number } => {
                writeln!(f, "Unknown block {name:?} on line {line_number}")?
            }
            ParseRecallTypeErrors {
                errors,
                line_number,
            } => {
                writeln!(f, "Unable to parse recall settings on line {line_number}:")?;
                for error in errors {
                    writeln!(f, "  {error}")?;
                }
            }
            ParseFlashcardErrors {
                errors,
                line_number,
            } => {
                writeln!(f, "Unable to parse flashcard on line {line_number}:")?;
                for error in errors {
                    writeln!(f, "  {error}")?;
                }
            }
        };
        Ok(())
    }
}

#[derive(Debug)]
pub enum ParseRecallTypeError {
    UnknownSetting { name: String, line_number: u32 },
}

impl Display for ParseRecallTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseRecallTypeError::*;
        match self {
            UnknownSetting { name, line_number } => {
                write!(f, "Unknown setting {name:?} on line {line_number}")
            }
        }
    }
}

#[derive(Debug)]
pub enum ParseFlashcardItemError {
    MissingTag { line_number: u32 },
    UnknownTag { tag: String, line_number: u32 },
    MissingSide(Side),
}

impl Display for ParseFlashcardItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseFlashcardItemError::*;
        match self {
            MissingTag { line_number } => write!(f, "Missing tag on line {line_number}"),
            UnknownTag { tag, line_number } => {
                write!(f, "Unknown tag {tag:?} on line {line_number}")
            }
            MissingSide(side) => write!(f, "Missing {side}"),
        }
    }
}

#[macro_export]
macro_rules! load_set {
    ($path:expr) => {
        match Set::load_from_file_path($path) {
            Some(set) => set,
            None => return,
        }
    };
}

#[derive(Debug, Default, Clone)]
pub struct RecallSettings {
    matching: bool,
    text: bool,
}

#[derive(Debug, Clone)]
pub struct Flashcard {
    pub term: FlashcardText,
    pub definition: FlashcardText,
}

impl Flashcard {
    const fn empty() -> Self {
        Self {
            term: FlashcardText::empty(),
            definition: FlashcardText::empty(),
        }
    }

    /// Returns true if this is valid.  Invalid cards should not be allowed to escape
    ///
    /// A flashcard is valid if it has at least 1 term and at least 1 definition
    fn is_valid(&self) -> bool {
        self.term.is_valid() && self.definition.is_valid()
    }

    /// Draws a flashcard on screen.  Does not flush stdout
    ///
    /// # Panics
    ///
    /// Panics if size is not at least 5x3
    pub fn draw(&self, position: Vec2<u16>, size: Vec2<u16>, side: Side, bold: bool) {
        assert!(size.x >= 5 && size.y >= 3);

        let mut stdout = io::stdout();
        let (c_t_l, c_t_r, c_b_l, c_b_r, l_h, l_v) = if !bold {
            ('┏', '┓', '┗', '┛', '━', '┃')
        } else {
            queue!(stdout, style::SetAttribute(Attribute::Bold)).unwrap();
            ('╔', '╗', '╚', '╝', '═', '║')
        };

        let lines = {
            let mut lines = WordWrap::new(self[side].random(), size.x as usize - 2);
            let mut vec = Vec::from_iter(lines.by_ref().take(size.y as usize - 2));
            if lines.next().is_some() {
                if let Some(line) = vec.last_mut() {
                    let line = line.to_mut();
                    let mut len = line.chars().count();
                    while len > (size.x - 5) as usize {
                        line.pop();
                        len -= 1;
                    }
                    line.push_str("...");
                }
            }
            vec
        };
        let lines_start = ((size.y as usize - 2) / 2) - (lines.len() / 2);

        queue!(
            stdout,
            position.move_to(),
            style::SetForegroundColor(side.color()),
            style::Print(c_t_l),
            style::Print(Repeat(l_h, size.x - 2)),
            style::Print(c_t_r),
        )
        .unwrap();
        for line in 0..(size.y as usize - 2) {
            queue!(
                stdout,
                cursor::MoveDown(1),
                cursor::MoveToColumn(position.x),
                style::Print(l_v),
            )
            .unwrap();
            if line >= lines_start {
                if let Some(line) = lines.get(line - lines_start) {
                    let offset = ((size.x - 2) / 2) - (line.chars().count() as u16 / 2) + 1;
                    queue!(
                        stdout,
                        cursor::MoveToColumn(position.x + offset),
                        style::Print(line)
                    )
                    .unwrap();
                }
            }
            queue!(
                stdout,
                cursor::MoveToColumn(position.x + size.x - 1),
                style::Print(l_v),
            )
            .unwrap();
        }
        queue!(
            stdout,
            cursor::MoveDown(1),
            cursor::MoveToColumn(position.x),
            style::SetForegroundColor(side.color()),
            style::Print(c_b_l),
            style::Print(Repeat(l_h, size.x - 2)),
            style::Print(c_b_r),
        )
        .unwrap();
        if bold {
            queue!(stdout, style::SetAttribute(Attribute::NormalIntensity)).unwrap();
        }
    }
}

impl Index<Side> for Flashcard {
    type Output = FlashcardText;

    fn index(&self, index: Side) -> &Self::Output {
        match index {
            Side::Term => &self.term,
            Side::Definition => &self.definition,
        }
    }
}

impl IndexMut<Side> for Flashcard {
    fn index_mut(&mut self, index: Side) -> &mut Self::Output {
        match index {
            Side::Term => &mut self.term,
            Side::Definition => &mut self.definition,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlashcardText(SmallVec<[String; 1]>);

impl FlashcardText {
    const fn empty() -> Self {
        FlashcardText(SmallVec::new_const())
    }

    /// Returns true if this is valid
    ///
    /// A flashcard text is valid if it has at least 1 value
    fn is_valid(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn push(&mut self, val: String) {
        self.0.push(val);
    }

    pub fn random(&self) -> &str {
        self.0.choose(&mut rand::thread_rng()).unwrap()
    }
}

impl FlashcardText {
    pub fn new(text: String) -> Self {
        Self(smallvec![text])
    }
}

impl From<String> for FlashcardText {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for FlashcardText {
    fn from(s: &str) -> Self {
        Self::new(s.to_owned())
    }
}

impl From<&[&str]> for FlashcardText {
    fn from(list: &[&str]) -> Self {
        Self(list.iter().map(|&s| s.to_owned()).collect())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Side {
    Term,
    Definition,
}

impl Side {
    pub fn color(self) -> Color {
        use Side::*;
        match self {
            Term => Color::Blue,
            Definition => Color::Green,
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Side::*;
        match self {
            Term => write!(f, "term"),
            Definition => write!(f, "definition"),
        }
    }
}
