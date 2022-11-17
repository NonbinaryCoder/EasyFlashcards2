use std::{
    ops::{Index, IndexMut},
    rc::Rc,
};

use rand::{seq::SliceRandom, Rng};

use crate::flashcards::{Flashcard, RecallSettings, Set, Side};

use self::fail_count::FailCount;

use super::footer::{Footer, FooterColor};

#[derive(Debug)]
pub struct Item<'a> {
    pub card: &'a Flashcard,
    pub side: Side,
    pub next_study_type: StudyType,
    pub footer_color: FooterColor,
    pub match_fails: FailCount,
    pub text_fails: FailCount,
}

#[derive(Debug)]
pub struct FailedItem<'a> {
    pub card: &'a Flashcard,
    pub side: Side,
    pub match_fails: FailCount,
    pub text_fails: FailCount,
}

impl<'a> From<&Item<'a>> for FailedItem<'a> {
    fn from(item: &Item<'a>) -> Self {
        FailedItem {
            card: item.card,
            side: item.side,
            match_fails: item.match_fails,
            text_fails: item.text_fails,
        }
    }
}

impl<'a> FailedItem<'a> {
    pub fn total_fails(&self) -> FailCount {
        self.match_fails + self.text_fails
    }
}

/// A token representing an item in a card list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token(usize);

#[derive(Debug)]
pub struct RefToken<'a, 'b> {
    list: &'a CardList<'b>,
    token: Token,
}

impl<'a, 'b> RefToken<'a, 'b> {
    pub fn token(&self) -> Token {
        self.token
    }

    pub fn item(&self) -> &Item<'b> {
        &self.list[self.token]
    }

    pub fn tup(&self) -> (&Item<'b>, Token) {
        (self.item(), self.token())
    }
}

#[derive(Debug)]
pub struct CardList<'a> {
    cards: Vec<Item<'a>>,
    removed: Vec<FailedItem<'a>>,
    set: &'a Set,
}

impl<'a> CardList<'a> {
    pub fn from_set(set: &'a Set) -> Self {
        let term_start = StudyType::first(&set.recall_t);
        let definition_start = StudyType::first(&set.recall_d);
        let count = [term_start.is_some(), definition_start.is_some()]
            .into_iter()
            .filter(|b| *b)
            .count();
        let mut cards = Vec::with_capacity(count * set.cards.len());

        let mut extend_cards = |start, side| {
            if let Some(next_study_type) = start {
                cards.extend(set.cards.iter().map(|card| Item {
                    card,
                    side,
                    next_study_type,
                    footer_color: FooterColor::Black,
                    match_fails: FailCount::ZERO,
                    text_fails: FailCount::ZERO,
                }))
            }
        };

        extend_cards(term_start, Side::Term);
        extend_cards(definition_start, Side::Definition);

        Self {
            removed: Vec::new(),
            cards,
            set,
        }
    }

    pub fn next_unstudied(&self, last: Option<Token>) -> Option<RefToken<'_, 'a>> {
        if self.cards.is_empty() {
            None
        } else {
            let last = last.map(|s| s.0).unwrap_or(usize::MAX);
            let mut index = last;
            let mut counter = 0;
            while index == last && counter < 12 {
                index = rand::thread_rng().gen_range(0..self.cards.len());
                counter += 1;
            }
            Some(RefToken {
                list: self,
                token: Token(index),
            })
        }
    }

    pub fn progress(&mut self, card: Token, footer: &mut Footer) {
        let index = card.0;
        let card = &self.cards[index];
        let old_color = card.footer_color;
        let new_color = match card
            .next_study_type
            .progress(self.recall_settings(card.side))
        {
            (Some(next_study_type), color) => {
                let card = &mut self.cards[index];
                card.next_study_type = next_study_type;
                card.footer_color = color;
                color
            }
            (None, color) => {
                let card = self.cards.swap_remove(index);
                if card.match_fails.has_failed() || card.text_fails.has_failed() {
                    self.removed.push((&card).into());
                }
                color
            }
        };
        footer.r#move(old_color, new_color);
    }

    pub fn regress(&mut self, card: Token, footer: &mut Footer) {
        let index = card.0;
        let card = &self.cards[index];
        let old_color = card.footer_color;
        if let Some((next_study_type, new_color)) = card
            .next_study_type
            .regress(self.recall_settings(card.side))
        {
            let card = &mut self.cards[index];
            card.next_study_type = next_study_type;
            card.footer_color = new_color;
            footer.r#move(old_color, new_color);
        }
    }

    pub fn fail(&mut self, card: Token) {
        let card = &mut self.cards[card.0];
        match card.next_study_type {
            StudyType::Matching(_) => card.match_fails.inc(),
            StudyType::Text(_) => card.text_fails.inc(),
        }
    }

    pub fn matching_answers_for(&self, card: &Item<'a>) -> [Rc<str>; 4] {
        let mut answers = [None, None, None, None];
        answers[0] = Some(card.card[card.side].display().clone());
        let mut rng = rand::thread_rng();
        for i in 1..4 {
            for _ in 0..12 {
                answers[i] = Some(
                    self.set.cards.choose(&mut rng).unwrap()[card.side]
                        .display()
                        .clone(),
                );
                if !answers[..i].contains(&answers[i]) {
                    break;
                }
            }
        }
        let mut answers = answers.map(Option::unwrap);
        answers.shuffle(&mut rng);
        answers
    }

    pub fn recall_settings(&self, side: Side) -> &RecallSettings {
        self.set.recall_settings(side)
    }

    pub fn remaining_unstudied(&self) -> usize {
        self.cards.len()
    }

    pub fn fails(&mut self) -> &mut [FailedItem<'a>] {
        self.removed.extend(self.cards.iter().filter_map(|item| {
            (item.match_fails.has_failed() || item.text_fails.has_failed()).then_some(item.into())
        }));
        &mut self.removed
    }
}

impl<'a> Index<Token> for CardList<'a> {
    type Output = Item<'a>;

    fn index(&self, index: Token) -> &Self::Output {
        &self.cards[index.0]
    }
}

impl<'a> IndexMut<Token> for CardList<'a> {
    fn index_mut(&mut self, index: Token) -> &mut Self::Output {
        &mut self.cards[index.0]
    }
}

/// Progression:
/// ```no_run
/// match (matching, text) {
///     (false, false) => [],
///     (true, false) => [M0, M1], // Black, Yellow
///     (false, true) => [T0, T1], // Black, Yellow
///     (true, true) => [M0, T0, T1], // Black, Red, Yellow
/// }
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum StudyType {
    Matching(bool),
    Text(bool),
}

impl StudyType {
    #[rustfmt::skip]
    fn first(recall_settings: &RecallSettings) -> Option<Self> {
        use StudyType::*;
        match (recall_settings.matching, recall_settings.text) {
            (false, false) => None,
            (true,  _)     => Some(Matching(false)),
            (false, true)  => Some(Text(false)),
        }
    }

    #[rustfmt::skip]
    fn progress(self, recall_settings: &RecallSettings) -> (Option<StudyType>, FooterColor) {
        use StudyType::*;
        match (self, recall_settings.matching, recall_settings.text) {
            (Matching(false), true,  false) => (Some(Matching(true)), FooterColor::Yellow),
            (Matching(true),  true,  false) => (None,                 FooterColor::Green),

            (Text(false),     false, true)  => (Some(Text(true)),     FooterColor::Yellow),
            (Text(true),      false, true)  => (None,                 FooterColor::Green),

            (Matching(false), true,  true)  => (Some(Text(false)),    FooterColor::Red),
            (Text(false),     true,  true)  => (Some(Text(true)),     FooterColor::Yellow),
            (Text(true),      true,  true)  => (None,                 FooterColor::Green),

            (s, m, t) => unreachable!("Bad progression: {s:?} with matching = {m} and text = {t}"),
        }
    }

    #[rustfmt::skip]
    fn regress(self, recall_settings: &RecallSettings) -> Option<(StudyType, FooterColor)> {
        use StudyType::*;
        match (self, recall_settings.matching, recall_settings.text) {
            (Matching(true), true,  false) => Some((Matching(false), FooterColor::Black)),

            (Text(true),     false, true)  => Some((Text(false),     FooterColor::Black)),

            (Text(false),    true,  true)  => Some((Matching(false), FooterColor::Black)),
            (Text(true),     true,  true)  => Some((Text(false),     FooterColor::Red)),

            _ => None,
        }
    }
}

pub mod fail_count {
    use std::{fmt, ops::Add};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct FailCount(u8);

    impl fmt::Display for FailCount {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if self.0 > 100 {
                write!(f, "100+")
            } else {
                write!(f, "{}", self.0)
            }
        }
    }

    impl fmt::Debug for FailCount {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if self.0 > 100 {
                write!(f, "100+")
            } else {
                write!(f, "{}", self.0)
            }
        }
    }

    impl FailCount {
        pub const ZERO: Self = Self(0);

        pub fn inc(&mut self) {
            self.0 = self.0.saturating_add(1);
        }

        pub fn has_failed(&self) -> bool {
            self.0 > 0
        }

        pub fn count(&self) -> u8 {
            self.0
        }
    }

    impl Add<FailCount> for FailCount {
        type Output = FailCount;

        fn add(self, rhs: FailCount) -> Self::Output {
            FailCount(self.0.saturating_add(rhs.0))
        }
    }
}
