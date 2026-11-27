// The line as words: runs of letters and digits, with byte offsets. What lies between two words decides how they join.

pub struct Word {
    pub start: usize,
    pub end: usize,
    pub lower: String,
}

pub fn words(s: &str) -> Vec<Word> {
    let mut found = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices().chain([(s.len(), ' ')]) {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(st)) => {
                found.push(Word { start: st, end: i, lower: s[st..i].to_lowercase() });
                start = None;
            }
            _ => {}
        }
    }
    found
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

/// The gap between two words is only whitespace.
pub fn spaced(rest: &str, a: &Word, b: &Word) -> bool {
    let gap = &rest[a.end..b.start];
    !gap.is_empty() && gap.chars().all(char::is_whitespace)
}
