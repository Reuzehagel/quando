//! quando: one typed line in, a task out. "dentist fri 9 #health" reads as a title, a date, a list,
//! and the spans an editor highlights while you type.
//!
//! Not written yet: `parse` returns the whole line as the title. `tests/cases.json` is the target.

/// A wall-clock time with no zone: quando reads the line in the zone `now` is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    Date,
    Repeat,
    List,
    Tag,
    Priority,
}

/// A part of the line that was read as something; `start..end` are byte offsets into the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: SpanKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repeat {
    pub every: u32,
    pub unit: Unit,
    /// 0 is Sunday.
    pub weekday: Option<u8>,
    pub weekdays_only: bool,
    /// "after done": the next date counts from completing it, not from the last date.
    pub from_completion: bool,
    /// Day of the month a monthly or yearly repeat was set on, so a 31st clamped to the 28th goes back to the 31st.
    pub day: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub title: String,
    pub date: Option<DateTime>,
    pub has_time: bool,
    pub list: Option<String>,
    pub tags: Vec<String>,
    /// 0 to 3, from `!` to `!!!`.
    pub priority: u8,
    pub repeat: Option<Repeat>,
    pub spans: Vec<Span>,
}

/// Reads `text` as of `now`. `ignored` holds span texts the user turned back into plain text ("buy tomorrow's paper").
pub fn parse(text: &str, _now: DateTime, _ignored: &[&str]) -> Parsed {
    Parsed {
        title: text.trim().to_string(),
        date: None,
        has_time: false,
        list: None,
        tags: Vec::new(),
        priority: 0,
        repeat: None,
        spans: Vec::new(),
    }
}
