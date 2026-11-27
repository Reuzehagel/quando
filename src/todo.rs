// The todo layer: a typed line as a task. The title is what's left once the date, repeat, `#list`, `@tag` and `!` are read.
use crate::calendar::{civil_from_days, days_from_civil, weekday_of};
use crate::date::{read_date, resolve};
use crate::words::{spaced, weekday, words, Word};
use crate::{possessive, DateTime, Options, Span, SpanKind};

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

impl Repeat {
    /// Whether the repeat can fall on this weekday (0 is Sunday).
    pub fn allows(&self, weekday: u8) -> bool {
        self.weekday.is_none_or(|d| d == weekday) && !(self.weekdays_only && (weekday == 0 || weekday == 6))
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

    let mut repeat = None;
    let ws = words(&line.rest);
    for i in 0..ws.len() {
        if let Some((end, r)) = read_repeat(&line.rest, &ws, i)
            && line.take(ws[i].start, end, SpanKind::Repeat) {
                repeat = Some(r);
                break;
            }
    }

    let mut date = None;
    let ws = words(&line.rest);
    for i in 0..ws.len() {
        let Some(m) = read_date(&line.rest, &ws, i, o) else { continue };
        // "dentist on friday" is a dentist: the word leading into the date goes with it
        let lead = i > 0 && matches!(ws[i - 1].lower.as_str(), "on" | "by" | "in") && spaced(&line.rest, &ws[i - 1], &ws[i]);
        let start = if lead { ws[i - 1].start } else { ws[i].start };
        // "+15min" keeps its sign
        let start = if line.rest[..start].ends_with('+') { start - 1 } else { start };
        if possessive(&text[m.end..]) || !line.take(start, m.end, SpanKind::Date) {
            continue;
        }
        let time_only = m.day.is_none();
        let (mut d, has_time) = resolve(m, now, o);
        // "every weekday at 9" on a Friday afternoon starts Monday, not Saturday
        if let Some(r) = repeat.as_ref().filter(|_| time_only) {
            let mut days = days_from_civil(d.year, d.month, d.day);
            for _ in 0..7 {
                if r.allows(weekday_of(days)) {
                    break;
                }
                days += 1;
            }
            (d.year, d.month, d.day) = civil_from_days(days);
        }
        date = Some((d, has_time));
        break;
    }
    let has_time = date.is_some_and(|(_, t)| t);
    let mut date = date.map(|(d, _)| d);
    if date.is_none() {
        date = repeat.as_ref().map(|r| first_occurrence(r, now));
    }

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
fn read_repeat(rest: &str, ws: &[Word], i: usize) -> Option<(usize, Repeat)> {
    let plain = |every, unit| Repeat { every, unit, weekday: None, weekdays_only: false, from_completion: false, day: None };
    let (mut r, mut last) = match ws[i].lower.as_str() {
        "daily" => (plain(1, Unit::Day), i),
        "weekly" => (plain(1, Unit::Week), i),
        "monthly" => (plain(1, Unit::Month), i),
        "yearly" => (plain(1, Unit::Year), i),
        "every" => {
            let mut j = i + 1;
            let next = |j: usize| (j < ws.len() && spaced(rest, &ws[j - 1], &ws[j])).then(|| ws[j].lower.as_str());
            let (mut n, mut other) = (1, false);
            match next(j)? {
                "other" => (other, j) = (true, j + 1),
                w if w.bytes().all(|b| b.is_ascii_digit()) => (n, j) = (w.parse().ok()?, j + 1),
                _ => {}
            }
            let w = next(j)?;
            let single = w.strip_suffix('s').unwrap_or(w);
            let unit = |u| plain(if other { 2 } else { n }, u);
            let r = match single {
                "day" => unit(Unit::Day),
                "week" => unit(Unit::Week),
                "month" => unit(Unit::Month),
                "year" => unit(Unit::Year),
                "weekday" => Repeat { weekdays_only: true, ..plain(1, Unit::Day) },
                _ => {
                    let d = weekday(w).or_else(|| weekday(single))?;
                    Repeat { weekday: Some(d), ..plain(if other { 2 } else { 1 }, Unit::Week) }
                }
            };
            (r, j)
        }
        _ => return None,
    };
    let after = |k: usize, word: &str| k < ws.len() && spaced(rest, &ws[k - 1], &ws[k]) && ws[k].lower == word;
    if after(last + 1, "after") && after(last + 2, "done") {
        r.from_completion = true;
        last += 2;
    }
    Some((ws[last].end, r))
}

/// With a repeat and no date: today, or the repeat's next weekday ("every friday" on a Friday starts today).
fn first_occurrence(r: &Repeat, now: DateTime) -> DateTime {
    let today = days_from_civil(now.year, now.month, now.day);
    let days = today + r.weekday.map_or(0, |d| (d as i64 - weekday_of(today) as i64).rem_euclid(7));
    let (year, month, day) = civil_from_days(days);
    DateTime { year, month, day, hour: 0, minute: 0 }
}
