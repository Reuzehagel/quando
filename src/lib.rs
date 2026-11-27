//! quando: one typed line in, a task out. "dentist fri 9 #health" reads as a title, a date, a list,
//! and the spans an editor highlights while you type.

mod calendar;
mod words;

use calendar::{days_from_civil, valid, weekday_of};
use words::{month, weekday, words, Word};

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

/// Reads `text` as of `now`. `ignored` holds span texts the user turned back into plain text ("buy tomorrow's paper").
pub fn parse(text: &str, now: DateTime, ignored: &[&str]) -> Parsed {
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
        let Some(m) = read_date(&line.rest, &ws, i) else { continue };
        if possessive(&text[m.end..]) || !line.take(ws[i].start, m.end, SpanKind::Date) {
            continue;
        }
        let time_only = m.day.is_none();
        let (mut d, has_time) = resolve(m, now);
        // "every weekday at 9" on a Friday afternoon starts Monday, not Saturday
        if let Some(r) = repeat.as_ref().filter(|_| time_only) {
            let mut days = days_from_civil(d.year, d.month, d.day);
            for _ in 0..7 {
                if r.allows(weekday_of(days)) {
                    break;
                }
                days += 1;
            }
            (d.year, d.month, d.day) = calendar::civil_from_days(days);
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

/// The gap between two words is only whitespace.
fn spaced(rest: &str, a: &Word, b: &Word) -> bool {
    let gap = &rest[a.end..b.start];
    !gap.is_empty() && gap.chars().all(char::is_whitespace)
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

enum Day {
    /// A calendar date; without a year it is the next one from today.
    On { year: Option<i32>, month: u8, day: u8 },
    Weekday(u8),
    InDays(i64),
}

struct Time {
    hour: u8,
    minute: u8,
    /// am or pm was written.
    meridiem: bool,
    /// Written as "05:30": a morning on purpose.
    zero: bool,
}

struct DateMatch {
    day: Option<Day>,
    time: Option<Time>,
    end: usize,
}

/// A date phrase starting at word `i`: a day, a time, or both in either order.
fn read_date(rest: &str, ws: &[Word], i: usize) -> Option<DateMatch> {
    let then = |j: usize| (j < ws.len() && spaced(rest, &ws[j - 1], &ws[j])).then_some(j);
    if let Some((day, j)) = read_day(rest, ws, i) {
        // after a day, a bare hour is a time: "fri 9"
        if let Some((time, k)) = then(j).and_then(|j| read_time(rest, ws, j, true)) {
            return Some(DateMatch { day: Some(day), time: Some(time), end: ws[k - 1].end });
        }
        return Some(DateMatch { day: Some(day), time: None, end: ws[j - 1].end });
    }
    let (time, j) = read_time(rest, ws, i, false)?;
    if let Some((day, k)) = then(j).and_then(|j| read_day(rest, ws, j)) {
        return Some(DateMatch { day: Some(day), time: Some(time), end: ws[k - 1].end });
    }
    Some(DateMatch { day: None, time: Some(time), end: ws[j - 1].end })
}

fn number(w: &str, max_len: usize) -> Option<u32> {
    (!w.is_empty() && w.len() <= max_len && w.bytes().all(|b| b.is_ascii_digit())).then(|| w.parse().unwrap())
}

/// "27", "27th": a day of the month.
fn day_number(w: &str) -> Option<u8> {
    let digits = ["st", "nd", "rd", "th"].iter().find_map(|s| w.strip_suffix(s)).unwrap_or(w);
    number(digits, 2).filter(|d| (1..=31).contains(d)).map(|d| d as u8)
}

/// The day part, and the index of the word after it. A date that can't exist ("31/2") isn't one.
fn read_day(rest: &str, ws: &[Word], i: usize) -> Option<(Day, usize)> {
    // 2000 was a leap year, so a 29 Feb without a year passes
    read_day_any(rest, ws, i).filter(|(d, _)| !matches!(*d, Day::On { year, month, day } if !valid(year.unwrap_or(2000), month, day)))
}

fn read_day_any(rest: &str, ws: &[Word], i: usize) -> Option<(Day, usize)> {
    let at = |j: usize| (j < ws.len()).then(|| ws[j].lower.as_str());
    let spaced_to = |j: usize| j < ws.len() && spaced(rest, &ws[j - 1], &ws[j]);
    let year_after = |j: usize| spaced_to(j).then(|| number(&ws[j].lower, 4).filter(|_| ws[j].lower.len() == 4)).flatten();
    let w = at(i)?;
    match w {
        "today" => return Some((Day::InDays(0), i + 1)),
        "tomorrow" => return Some((Day::InDays(1), i + 1)),
        "in" if spaced_to(i + 1) && spaced_to(i + 2) => {
            let n = number(at(i + 1)?, 3)? as i64;
            let per = match at(i + 2)? {
                "day" | "days" => 1,
                "week" | "weeks" => 7,
                _ => return None,
            };
            return Some((Day::InDays(n * per), i + 3));
        }
        _ => {}
    }
    if let Some(d) = weekday(w) {
        return Some((Day::Weekday(d), i + 1));
    }
    // "nov 27", "nov 27 2027"
    if let Some(m) = month(w) {
        let day = spaced_to(i + 1).then(|| day_number(&ws[i + 1].lower)).flatten()?;
        let year = year_after(i + 2);
        return Some((Day::On { year: year.map(|y| y as i32), month: m, day }, i + 2 + year.is_some() as usize));
    }
    // "27 nov", "27 nov 2027"
    if let Some(day) = day_number(w)
        && let Some(m) = spaced_to(i + 1).then(|| month(&ws[i + 1].lower)).flatten() {
            let year = year_after(i + 2);
            return Some((Day::On { year: year.map(|y| y as i32), month: m, day }, i + 2 + year.is_some() as usize));
        }
    // "3/12", "3/12/2026": day first
    let slash = |j: usize| j < ws.len() && &rest[ws[j - 1].end..ws[j].start] == "/";
    let day = number(w, 2)?;
    let m = slash(i + 1).then(|| number(&ws[i + 1].lower, 2)).flatten().filter(|m| (1..=12).contains(m))?;
    let year = slash(i + 2).then(|| number(&ws[i + 2].lower, 4).filter(|_| ws[i + 2].lower.len() == 4)).flatten();
    Some((Day::On { year: year.map(|y| y as i32), month: m as u8, day: day as u8 }, i + 2 + year.is_some() as usize))
}

/// "at 5", "5pm", "15:30", "10:30 am", and a bare "9" when `bare` (after a day). Returns the index of the word after it.
fn read_time(rest: &str, ws: &[Word], i: usize, bare: bool) -> Option<(Time, usize)> {
    let mut j = i;
    let with_at = ws[j].lower == "at" && j + 1 < ws.len() && spaced(rest, &ws[j], &ws[j + 1]);
    if with_at {
        j += 1;
    }
    let split = |w: &str| ["am", "pm"].iter().find_map(|s| w.strip_suffix(s).map(|d| (d.to_string(), Some(*s))));
    let (hour_text, mut suffix) = split(&ws[j].lower).unwrap_or((ws[j].lower.clone(), None));
    let mut hour = number(&hour_text, 2)?;
    let mut minute = None;
    if suffix.is_none() && j + 1 < ws.len() && &rest[ws[j].end..ws[j + 1].start] == ":" {
        let (m, s) = split(&ws[j + 1].lower).unwrap_or((ws[j + 1].lower.clone(), None));
        if m.len() == 2 {
            minute = Some(number(&m, 2)?);
            suffix = s;
            j += 1;
        }
    }
    if suffix.is_none() && j + 1 < ws.len() && spaced(rest, &ws[j], &ws[j + 1]) && matches!(ws[j + 1].lower.as_str(), "am" | "pm") {
        suffix = Some(if ws[j + 1].lower == "am" { "am" } else { "pm" });
        j += 1;
    }
    if !(with_at || bare || minute.is_some() || suffix.is_some()) {
        return None;
    }
    if let Some(s) = suffix {
        if !(1..=12).contains(&hour) {
            return None;
        }
        hour = hour % 12 + if s == "pm" { 12 } else { 0 };
    }
    let minute = minute.unwrap_or(0);
    if hour > 23 || minute > 59 {
        return None;
    }
    let zero = hour_text.len() == 2 && hour_text.starts_with('0') && suffix.is_none();
    Some((Time { hour: hour as u8, minute: minute as u8, meridiem: suffix.is_some(), zero }, j + 1))
}

/// "tomorrow's paper" is a title, not a date.
fn possessive(after: &str) -> bool {
    let s = after.strip_prefix("'s").or_else(|| after.strip_prefix("’s"));
    s.is_some_and(|s| !s.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

/// The phrase as a moment, and whether it has a time.
fn resolve(m: DateMatch, now: DateTime) -> (DateTime, bool) {
    let today = days_from_civil(now.year, now.month, now.day);
    let (mut hour, minute) = m.time.as_ref().map_or((0, 0), |t| (t.hour, t.minute));
    // A bare "5" is 17:00, since nobody plans tasks for 05:00; "5am" and "05:00" mean what they say.
    if m.time.as_ref().is_some_and(|t| !t.meridiem && !t.zero && (1..=7).contains(&t.hour)) {
        hour += 12;
    }
    let days = match m.day {
        Some(Day::InDays(n)) => today + n,
        // "fri" on a Friday means next week
        Some(Day::Weekday(d)) => today + ((d as i64 - weekday_of(today) as i64 + 6).rem_euclid(7) + 1),
        Some(Day::On { year: Some(y), month, day }) => days_from_civil(y, month, day),
        // the next one from today; a 29 Feb can be years away
        Some(Day::On { year: None, month, day }) => (now.year..now.year + 8)
            .filter(|&y| valid(y, month, day))
            .map(|y| days_from_civil(y, month, day))
            .find(|&d| d >= today)
            .unwrap_or(today),
        // "at 5": today, unless it has passed
        None if (hour, minute) <= (now.hour, now.minute) => today + 1,
        None => today,
    };
    let (year, month, day) = calendar::civil_from_days(days);
    (DateTime { year, month, day, hour, minute }, m.time.is_some())
}

/// With a repeat and no date: today, or the repeat's next weekday ("every friday" on a Friday starts today).
fn first_occurrence(r: &Repeat, now: DateTime) -> DateTime {
    let today = days_from_civil(now.year, now.month, now.day);
    let days = today + r.weekday.map_or(0, |d| (d as i64 - weekday_of(today) as i64).rem_euclid(7));
    let (year, month, day) = calendar::civil_from_days(days);
    DateTime { year, month, day, hour: 0, minute: 0 }
}
