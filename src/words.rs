// The line as words: runs of letters and digits, with byte offsets. What lies between two words decides how they join.

pub struct Word {
    pub start: usize,
    pub end: usize,
    pub lower: String,
}

/// A line and its words. Word indexes past the end read as nothing, so readers can look ahead freely.
pub struct Text<'a> {
    pub s: &'a str,
    pub ws: Vec<Word>,
}

impl<'a> Text<'a> {
    pub fn new(s: &'a str) -> Text<'a> {
        let mut ws = Vec::new();
        let mut start = None;
        for (i, c) in s.char_indices().chain([(s.len(), ' ')]) {
            match (c.is_alphanumeric(), start) {
                (true, None) => start = Some(i),
                (false, Some(st)) => {
                    ws.push(Word { start: st, end: i, lower: s[st..i].to_lowercase() });
                    start = None;
                }
                _ => {}
            }
        }
        Text { s, ws }
    }

    pub fn len(&self) -> usize {
        self.ws.len()
    }

    /// Word `j`, lowercased.
    pub fn at(&self, j: usize) -> Option<&str> {
        self.ws.get(j).map(|w| w.lower.as_str())
    }

    /// What lies between word `j - 1` and word `j`.
    pub fn gap(&self, j: usize) -> Option<&'a str> {
        let (a, b) = (self.ws.get(j.checked_sub(1)?)?, self.ws.get(j)?);
        Some(&self.s[a.end..b.start])
    }

    /// Word `j` follows the one before after only whitespace.
    pub fn spaced(&self, j: usize) -> bool {
        self.gap(j).is_some_and(|g| !g.is_empty() && g.chars().all(char::is_whitespace))
    }

    /// Word `j` is `word`, after only whitespace.
    pub fn is(&self, j: usize, word: &str) -> bool {
        self.spaced(j) && self.at(j) == Some(word)
    }

    /// Word `j`, when only a space or a comma (and spaces) separates it from the word before.
    pub fn joined(&self, j: usize) -> Option<usize> {
        self.gap(j).filter(|g| !g.is_empty() && matches!(g.trim(), "" | ",")).map(|_| j)
    }

    /// Exactly `mark` lies between word `j - 1` and word `j`.
    pub fn sep(&self, j: usize, mark: &str) -> bool {
        self.gap(j) == Some(mark)
    }

    /// The text before word `j`.
    pub fn before(&self, j: usize) -> &'a str {
        &self.s[..self.ws[j].start]
    }
}

impl std::ops::Index<usize> for Text<'_> {
    type Output = Word;
    fn index(&self, j: usize) -> &Word {
        &self.ws[j]
    }
}

/// 0 is Sunday.
pub fn weekday(w: &str) -> Option<u8> {
    const NAMES: [&[&str]; 7] = [
        &["sun", "sunday"],
        &["mon", "monday"],
        &["tue", "tues", "tuesday"],
        &["wed", "wednesday"],
        &["thu", "thur", "thurs", "thursday"],
        &["fri", "friday"],
        &["sat", "saturday"],
    ];
    NAMES.iter().position(|n| n.contains(&w)).map(|d| d as u8)
}

pub fn month(w: &str) -> Option<u8> {
    const NAMES: [&str; 12] = ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
    let w = if w == "sept" { "sep" } else { w };
    NAMES.iter().position(|n| w.len() >= 3 && (*n == w || (w.len() == 3 && n.starts_with(w)))).map(|m| m as u8 + 1)
}
