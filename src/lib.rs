//! quando: dates in typed English. [`dates`] finds every date in a sentence ("call me tomorrow at 5"), and
//! [`parse`] reads one todo line ("dentist fri 9 #health") as a title, a date, a list, and the spans an editor
//! highlights while you type.

mod calendar;
mod date;
mod todo;
mod words;

use date::{read_date, resolve};
pub use todo::{parse, parse_with, Parsed, Repeat, Unit};
use words::Text;

/// A wall-clock time with no zone: quando reads the line in the zone `now` is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

/// The readings that depend on who is typing. The default reads dates day first, as Europe writes them, and lets
/// them fall in the past; [`Options::todo`] reads as a todo app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    /// "3/12" is 12 March, as the US writes it, not 3 December. A date that only reads one way ("5/31", "31/5")
    /// reads that way regardless.
    pub month_first: bool,
    /// Never before today: a time that has passed is tomorrow, "this month" is today rather than the 1st.
    pub forward: bool,
    /// A bare hour from 1 to 7 ("at 5", "fri 5") is the afternoon. "5am" and "05:00" still mean what they say.
    pub bare_hour_pm: bool,
    /// The day "end of week" falls on, 0 being Sunday (the default); 5 for a working week. Weeks start on Monday.
    pub week_end: u8,
}

impl Options {
    /// For a todo app: a task is due ahead, and nobody plans one for 5 in the morning.
    pub const fn todo() -> Options {
        Options { month_first: false, forward: true, bare_hour_pm: true, week_end: 0 }
    }
}

/// A date found in text: `start..end` are byte offsets into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    pub start: usize,
    pub end: usize,
    pub date: DateTime,
    pub has_time: bool,
}

/// Every date in `text`, in order, as of `now`.
pub fn dates(text: &str, now: DateTime, o: &Options) -> Vec<Found> {
    let t = Text::new(text);
    let mut found = Vec::new();
    let mut i = 0;
    while i < t.len() {
        let Some(m) = read_date(&t, i, o, false) else {
            i += 1;
            continue;
        };
        let (start, end) = (m.start, m.end);
        let (date, has_time) = resolve(m, now, o);
        found.push(Found { start, end, date, has_time });
        while i < t.len() && t[i].start < end {
            i += 1;
        }
    }
    found
}
