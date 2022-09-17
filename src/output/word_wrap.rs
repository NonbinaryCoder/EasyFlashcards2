use std::{borrow::Cow, mem};

pub struct WordWrap<'a> {
    text: &'a str,
    max_length: usize,
}

impl<'a> WordWrap<'a> {
    /// # Panics
    ///
    /// Panics if `max_length` is less than 2
    pub fn new(text: &'a str, max_length: usize) -> Self {
        assert!(max_length >= 2);
        Self { text, max_length }
    }
}

impl<'a> Iterator for WordWrap<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut len = 0;
        for word in SplitKeepWhitespace::new(self.text) {
            let word_len = word.chars().count();
            if len + word_len > self.max_length {
                return Some(if len > 0 {
                    let (ret, new_text) = self.text.split_at(len);
                    self.text = new_text.trim_start();
                    ret.into()
                } else {
                    let mut ret = String::with_capacity(self.max_length);
                    self.text
                        .chars()
                        .take(self.max_length - 1)
                        .for_each(|c| ret.push(c));
                    self.text = &self.text[ret.len()..];
                    ret.push('-');
                    ret.into()
                });
            } else {
                len += word_len;
            }
        }
        self.text
            .chars()
            .any(|c| !c.is_whitespace())
            .then(|| mem::take(&mut self.text).into())
    }
}

pub struct SplitKeepWhitespace<'a> {
    text: &'a str,
}

impl<'a> SplitKeepWhitespace<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }
}

impl<'a> Iterator for SplitKeepWhitespace<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let mut found_non_whitespace = false;
        for (index, char) in self.text.char_indices() {
            if char.is_whitespace() {
                if found_non_whitespace {
                    let ret;
                    (ret, self.text) = self.text.split_at(index);
                    return Some(ret);
                }
            } else {
                found_non_whitespace = true;
            }
        }
        found_non_whitespace.then(|| mem::take(&mut self.text))
    }
}
