// The todo layer: a typed line as a task. The title is what's left once the date, repeat, `#list`, `@tag` and `!` are read.
use crate::calendar::{civil_from_days, days_from_civil, weekday_of};
use crate::date::{calendar_unit, read_date, resolve};
use crate::words::{weekday, Text};
use crate::{DateTime, Options, Span, SpanKind};

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
}

impl Repeat {
    /// Whether the repeat can fall on this weekday (0 is Sunday).
    pub fn allows(&self, weekday: u8) -> bool {
        self.weekday.is_none_or(|d| d == weekday) && !(self.weekdays_only && (weekday == 0 || weekday == 6))
    }

    /// The first day the repeat can fall on, from `d`'s day on.
    fn first_from(&self, d: DateTime) -> DateTime {
        let days = days_from_civil(d.year, d.month, d.day);
        let (year, month, day) = civil_from_days((days..days + 7).find(|&x| self.allows(weekday_of(x))).unwrap_or(days));
        DateTime { year, month, day, ..d }
    }
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

/// Reads `text` as of `now`, as a todo app would: see [`Options::todo`]. `ignored` holds span texts the user turned
/// back into plain text ("buy tomorrow's paper").
pub fn parse(text: &str, now: DateTime, ignored: &[&str]) -> Parsed {
    parse_with(text, now, ignored, &Options::todo())
}

/// [`parse`] with your own reading of dates.
pub fn parse_with(text: &str, now: DateTime, ignored: &[&str], o: &Options) -> Parsed {
    let mut line = Line { text, rest: text.to_string(), spans: Vec::new(), ignored };
    let (mut list, mut tags, mut priority) = (None, Vec::new(), 0);

    for (start, end) in markers(text, '#') {
        if line.take(start, end, SpanKind::List) {
            list = Some(text[start + 1..end].to_string());
        }
    }
    for (start, end) in markers(text, '@') {
        if line.take(start, end, SpanKind::Tag) {
            tags.push(text[start + 1..end].to_string());
        }
    }
    for (start, end) in bangs(text) {
        if line.take(start, end, SpanKind::Priority) {
            priority = (end - start) as u8;
        }
    }

    let (mut repeat, mut repeat_end) = (None, None);
    let rest = line.rest.clone();
    let t = Text::new(&rest);
    for i in 0..t.len() {
        if let Some((end, r)) = read_repeat(&t, i)
            && line.take(t[i].start, end, SpanKind::Repeat)
        {
            (repeat, repeat_end) = (Some(r), Some(end));
            break;
        }
    }

    let mut date = None;
    let rest = line.rest.clone();
    let t = Text::new(&rest);
    for i in 0..t.len() {
        // right after a repeat, a bare hour is its time: "every monday 9"
        let after_repeat = repeat_end.is_some_and(|e| e < t[i].start && rest[e..t[i].start].trim().is_empty());
        let Some(m) = read_date(&t, i, o, after_repeat) else { continue };
        // "dentist on friday" is a dentist: the word leading into the date goes with it
        let lead = t.spaced(i) && matches!(t.at(i - 1), Some("on" | "by" | "in" | "between"));
        let start = if lead { t[i - 1].start } else { m.start };
        if !line.take(start, m.end, SpanKind::Date) {
            continue;
        }
        let time_only = m.time_only();
        let (d, has_time) = resolve(m, now, o);
        // "every weekday at 9" on a Friday afternoon starts Monday, not Saturday
        let d = repeat.as_ref().filter(|_| time_only).map_or(d, |r| r.first_from(d));
        date = Some((d, has_time));
        break;
    }
    let has_time = date.is_some_and(|(_, t)| t);
    // with a repeat and no date: the first day from today it can fall on ("every friday" on a Friday starts today)
    let midnight = DateTime { hour: 0, minute: 0, ..now };
    let date = date.map(|(d, _)| d).or_else(|| repeat.as_ref().map(|r| r.first_from(midnight)));

    let mut spans = line.spans;
    spans.sort_by_key(|s| s.start);
    let title = text
        .char_indices()
        .map(|(i, c)| if spans.iter().any(|s| (s.start..s.end).contains(&i)) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Parsed { title, date, has_time, list, tags, priority, repeat, spans }
}

struct Line<'a> {
    text: &'a str,
    /// `text` with the parts already read blanked out, so offsets stay valid and nothing is read twice.
    rest: String,
    spans: Vec<Span>,
    ignored: &'a [&'a str],
}

impl Line<'_> {
    fn take(&mut self, start: usize, end: usize, kind: SpanKind) -> bool {
        if self.ignored.contains(&&self.text[start..end]) {
            return false;
        }
        self.spans.push(Span { start, end, kind });
        // ';' rather than spaces, so a blanked part never reads as the gap between two words
        self.rest.replace_range(start..end, &";".repeat(end - start));
        true
    }
}

/// `#name` or `@name` at the start of the line or after a space: (start, end) of each.
fn markers(text: &str, sign: char) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut prev = ' ';
    for (i, c) in text.char_indices() {
        if c == sign && prev.is_whitespace() {
            let name = text[i + 1..].find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-')).unwrap_or(text.len() - i - 1);
            if name > 0 {
                found.push((i, i + 1 + name));
            }
        }
        prev = c;
    }
    found
}

/// `!`, `!!` or `!!!` standing on its own.
fn bangs(text: &str) -> Vec<(usize, usize)> {
    text.split_whitespace()
        .filter(|part| (1..=3).contains(&part.len()) && part.bytes().all(|b| b == b'!'))
        .map(|part| {
            let at = part.as_ptr() as usize - text.as_ptr() as usize;
            (at, at + part.len())
        })
        .collect()
}

/// "every monday", "every 3 days after done", "every other week", "daily": the end of the phrase and its rule.
fn read_repeat(t: &Text, i: usize) -> Option<(usize, Repeat)> {
    let plain = |every, unit| Repeat { every, unit, weekday: None, weekdays_only: false, from_completion: false };
    let (mut r, mut last) = match t.at(i)? {
        "daily" => (plain(1, Unit::Day), i),
        "weekly" => (plain(1, Unit::Week), i),
        "monthly" => (plain(1, Unit::Month), i),
        "yearly" => (plain(1, Unit::Year), i),
        "every" => {
            let mut j = i + 1;
            let next = |j: usize| t.spaced(j).then(|| t.at(j)).flatten();
            let (mut n, mut other) = (1, false);
            match next(j)? {
                "other" => (other, j) = (true, j + 1),
                w if w.bytes().all(|b| b.is_ascii_digit()) => (n, j) = (w.parse().ok()?, j + 1),
                _ => {}
            }
            let w = next(j)?;
            let single = w.strip_suffix('s').unwrap_or(w);
            let every = |u| plain(if other { 2 } else { n }, u);
            let unit = calendar_unit(w).map(|u| match u {
                (0, 1) => Unit::Day,
                (0, 7) => Unit::Week,
                (1, 0) => Unit::Month,
                _ => Unit::Year,
            });
            let r = match (single, unit) {
                ("weekday", _) => Repeat { weekdays_only: true, ..plain(1, Unit::Day) },
                (_, Some(u)) => every(u),
                _ => Repeat { weekday: Some(weekday(w).or_else(|| weekday(single))?), ..every(Unit::Week) },
            };
            (r, j)
        }
        _ => return None,
    };
    if t.is(last + 1, "after") && t.is(last + 2, "done") {
        r.from_completion = true;
        last += 2;
    }
    Some((t[last].end, r))
}
