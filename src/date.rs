// Reading a date phrase: parts (a day, a time, a part of the day, a year) joined by spaces, commas, "on", "of", "by",
// "before" or "after",
// as in "Tuesday, 10 January", "12th of July at 19:00" or "tonight at 8".
//
// Reading order: `read_date` → `phrase`, which reads each part with `read_part` (`read_day` → `read_day_any`,
// `read_time`, `part_of_day`) and a range's end with `range_end`. `resolve` → `day_of` then turns the match into a
// `DateTime` against `now`.
use crate::calendar::{civil_from_days, days_from_civil, valid, weekday_of};
use crate::words::{month, weekday, Text};
use crate::{DateTime, Options};

enum Day {
    /// A calendar date; without a year it is the next one from today.
    On { year: Option<i32>, month: u8, day: u8 },
    /// "in June", "September 2012": the 1st. Without a year, this month or the next one of that name.
    Month { year: Option<i32>, month: u8 },
    /// "end of July", "end of February 2024": the last day of that month.
    EndOf { year: Option<i32>, month: u8 },
    /// "end of month", "end of next month": the last day, this many months on.
    EndOfMonth(i64),
    /// "end of week", "end of next week": the options' last day of the week, this many weeks on.
    EndOfWeek(i64),
    /// "end of this year": 31 December, this many years on.
    EndOfYear(i64),
    Weekday(u8),
    /// "last friday": the one before today.
    LastWeekday(u8),
    /// "next friday", "friday of last week": that day in the week this many weeks on. Weeks start on Monday.
    WeekOf(u8, i64),
    /// "this weekend", "next weekend": the Saturday of this week or the next.
    Weekend(i64),
    /// "this week", "this month", "this year": its first day.
    This(Period),
    /// "the 5th": the next 5th of a month, today included.
    Ordinal(u8),
    InDays(i64),
    /// "in 3 months", "in 1 day 2 hours", "now": months first (the same day, clamped: a month on from 31 Jan is
    /// 28 Feb), then days, then minutes. With hours, minutes or seconds it is a moment, so it carries its own time.
    Ahead(Amount),
    /// "2 weeks after yesterday", "the day before friday": the amount counts from that day.
    From(Amount, Box<Day>),
}

enum Period {
    Week,
    Month,
    Year,
}

#[derive(Clone, Copy, Default)]
struct Amount {
    months: i64,
    days: i64,
    minutes: i64,
    moment: bool,
}

impl Amount {
    fn times(self, n: i64) -> Amount {
        Amount { months: self.months * n, days: self.days * n, minutes: self.minutes * n, moment: self.moment }
    }

    fn plus(self, o: Amount) -> Amount {
        Amount { months: self.months + o.months, days: self.days + o.days, minutes: self.minutes + o.minutes, moment: self.moment || o.moment }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Morning,
    Afternoon,
    Evening,
    Night,
}

struct Time {
    hour: u8,
    minute: u8,
    /// am or pm was written, or the time names its half ("noon").
    meridiem: bool,
    /// Written as "05:30": a morning on purpose.
    zero: bool,
}

/// A date phrase: `start..end` are byte offsets into the text it was read from.
pub struct DateMatch {
    day: Option<Day>,
    time: Option<Time>,
    part: Option<Part>,
    pub start: usize,
    pub end: usize,
}

impl DateMatch {
    /// Only a time: "at 9", not "friday at 9".
    pub fn time_only(&self) -> bool {
        self.day.is_none()
    }
}

/// The date phrase at word `i`. With `day_known` the day is said elsewhere, so a bare hour is a time: the "9" in
/// "every monday 9".
pub fn read_date(t: &Text, i: usize, o: &Options, day_known: bool) -> Option<DateMatch> {
    let mut m = read_phrase(t, i, o).or_else(|| day_known.then(|| bare_time(t, i)).flatten())?;
    if possessive(&t.s[m.end..]) {
        return None;
    }
    // "+15min" keeps its sign
    m.start -= t.before(i).ends_with('+') as usize;
    Some(m)
}

/// "tomorrow's paper" is a title, not a date.
fn possessive(after: &str) -> bool {
    let s = after.strip_prefix("'s").or_else(|| after.strip_prefix("’s"));
    s.is_some_and(|s| !s.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

fn read_phrase(t: &Text, i: usize, o: &Options) -> Option<DateMatch> {
    // not from inside a number: the "6/2018" in "50/6/2018", the "1" in "10.1"; "...Thursday" is no number
    if let [.., digit, mark] = t.before(i).as_bytes()
        && digit.is_ascii_digit()
        && b"/-.:".contains(mark)
    {
        return None;
    }
    phrase(t, i, o)
}

/// The phrase at word `i`, which may follow a number as a range's end: the "4:30pm" in "3:30-4:30pm".
fn phrase(t: &Text, i: usize, o: &Options) -> Option<DateMatch> {
    // nor from the middle of a date already turned down: the "February 2014" in "29 February 2014"
    if month(&t[i].lower).is_some() && t.spaced(i) && day_number(&t[i - 1].lower).is_some() {
        return None;
    }
    let mut m = DateMatch { day: None, time: None, part: None, start: t[i].start, end: 0 };
    let (mut j, mut after) = (i, i);
    while let Some(k) = read_part(t, j, &mut m, o) {
        (m.end, after) = (t[k - 1].end, k);
        // "9 a.m. tomorrow": the dot ends "a.m.", not the phrase
        let dotted = t.sep(k - 1, ".") && t.at(k - 1) == Some("m") && t.gap(k).is_some_and(|g| g.trim() == "." && g.len() > 1);
        let Some(next) = t.joined(k).or(dotted.then_some(k)) else { break };
        // "12th of July", "at midnight on 12th August", "tomorrow by 5pm"
        j = if matches!(t[next].lower.as_str(), "on" | "of" | "by" | "before" | "after") {
            match t.joined(next + 1) {
                Some(after) => after,
                None => break,
            }
        } else {
            next
        };
    }
    // "9 p.m." takes its last dot
    let b = t.s.as_bytes();
    if m.end >= 2 && b[m.end - 2] == b'.' && b[m.end - 1].eq_ignore_ascii_case(&b'm') && b.get(m.end) == Some(&b'.') {
        m.end += 1;
    }
    // a lone hour starts a range whose end says the half of the day: "8 - 11pm", "10 - 11 at night"
    if after == i {
        let (tm, k) = read_time(t, i, true)?;
        let to = range_end(t, k).and_then(|e| phrase(t, e, o))?;
        if to.day.is_some() || !(to.part.is_some() || to.time.as_ref().is_some_and(|x| x.meridiem)) {
            return None;
        }
        (m.time, m.end, after) = (Some(tm), t[k - 1].end, k);
    }
    // a range is its start, and the span runs to its end: "monday - friday", "10pm to 2am"; after a time, a bare
    // hour ends one, but only with no word after it: "1pm-3", not "5pm - 3 kids"
    // not into a number either: "at 6.5 kilograms", "4/13/1"
    let into_number = |end: usize| matches!(&b[end..], [mark, digit, ..] if b"/-.:".contains(mark) && digit.is_ascii_digit());
    let to = range_end(t, after).and_then(|k| {
        phrase(t, k, o).or_else(|| {
            let (time, e) = read_time(t, k, true).filter(|&(_, e)| m.time.is_some() && t.joined(e).is_none() && !counted(t, e) && !into_number(t[e - 1].end))?;
            Some(DateMatch { day: None, time: Some(time), part: None, start: t[k].start, end: t[e - 1].end })
        })
    });
    if to.is_none() && into_number(m.end) {
        return None;
    }
    if let Some(to) = to {
        m.end = to.end;
        // the start borrows what only the end says: the year in "Aug 10 - Nov 12, 2013", the day in "9am to 5pm, Tuesday",
        // the time in "tomorrow until 5pm" (a task is due by then)
        match (&mut m.day, to.day) {
            (Some(Day::On { year: y @ None, .. }), Some(Day::On { year, .. })) => *y = year,
            (d @ None, day) => *d = day,
            _ => {}
        }
        if m.time.is_none() {
            (m.time, m.part) = (to.time, m.part.or(to.part));
        } else if let (Some(a), Some(z)) = (&mut m.time, &to.time)
            && !a.meridiem
            && m.part.is_none()
        {
            if z.meridiem {
                // the same half as the end, unless the start is the later hour: "8 - 11pm" is 20:00, "11-3pm" 11:00, "11-3am" 23:00
                let pm = (z.hour >= 12) == (a.hour % 12 <= z.hour % 12);
                (a.hour, a.meridiem) = (a.hour % 12 + if pm { 12 } else { 0 }, true);
            } else {
                m.part = to.part;
            }
        }
    }
    // the year arrives after the date, so check the date again: "29 February 2014" isn't one
    if let Some(Day::On { year: Some(y), month, day }) = m.day
        && !valid(y, month, day)
    {
        return None;
    }
    (m.day.is_some() || m.time.is_some()).then_some(m)
}

/// The word a range's end starts at, when word `k` follows a dash or is "to", "until", "till" or "through".
fn range_end(t: &Text, k: usize) -> Option<usize> {
    if matches!(t.gap(k)?.trim(), "-" | "–" | "—") {
        return Some(k);
    }
    let word = matches!(t.at(k)?, "to" | "until" | "till" | "through");
    (word && t.spaced(k) && t.spaced(k + 1)).then_some(k + 1)
}

/// The word after "August 10", or after the day its range runs to: the "22" in "August 10 - 22".
fn past_range(t: &Text, k: usize) -> usize {
    let day = |j: usize| day_number(t.at(j)?).map(|_| j + 1).or_else(|| ordinal(t, j).map(|(_, k)| k));
    range_end(t, k).and_then(day).filter(|&e| !counted(t, e)).unwrap_or(k)
}

/// Whether the number before word `k` counts something: the "30" in "nov 27 - 30 euro", "- 30%", "- 30 minutes".
fn counted(t: &Text, k: usize) -> bool {
    // shortcut: a fixed list of what people count, so an unlisted one ("30 bottles") still reads as a range's end
    const COUNTS: [&str; 22] = [
        "euro", "euros", "eur", "dollar", "dollars", "usd", "pound", "pounds", "gbp", "cents", "percent", "people", "persons", "guests",
        "pages", "items", "pieces", "pcs", "times", "km", "kg", "x",
    ];
    t.s[t[k - 1].end..].starts_with(['%', '€', '$', '£']) || (t.spaced(k) && t.at(k).is_some_and(|w| COUNTS.contains(&w) || unit(w).is_some()))
}

fn bare_time(t: &Text, i: usize) -> Option<DateMatch> {
    let (time, k) = read_time(t, i, true)?;
    Some(DateMatch { day: None, time: Some(time), part: None, start: t[i].start, end: t[k - 1].end })
}

/// Reads one part at word `j` into `m`, if it fills something `m` lacks; returns the word after it.
fn read_part(t: &Text, j: usize, m: &mut DateMatch, o: &Options) -> Option<usize> {
    let w = t.at(j)?;

    // "tonight", "this morning", "last night": a day and a part at once
    if m.day.is_none() && m.part.is_none() {
        let both = match w {
            "tonight" => Some((0, Part::Night, j + 1)),
            "last" if t.is(j + 1, "night") => Some((-1, Part::Night, j + 2)),
            "this" if t.spaced(j + 1) => part_word(&t[j + 1].lower).filter(|p| *p != Part::Night).map(|p| (0, p, j + 2)),
            _ => None,
        };
        if let Some((n, p, k)) = both {
            (m.day, m.part) = (Some(Day::InDays(n)), Some(p));
            return Some(k);
        }
    }
    // "friday of next week", "friday this week", "tuesday, last week"
    if let Some(Day::Weekday(d)) = m.day
        && t.is(j + 1, "week")
    {
        let n = match w {
            "this" => 0,
            "next" => 1,
            "last" | "past" => -1,
            _ => return None,
        };
        m.day = Some(Day::WeekOf(d, n));
        return Some(j + 2);
    }
    // a day; a weekday gives way to a date ("Tuesday, 10 January")
    if matches!(m.day, None | Some(Day::Weekday(_)))
        && let Some((d, k)) = read_day(t, j, o)
        && (m.day.is_none() || matches!(d, Day::On { .. })) {
            m.day = Some(d);
            return Some(k);
        }
    // a year after a date that has none: "Jan 1, 2020", "June of 2022"
    if let Some(Day::On { year: y @ None, .. } | Day::Month { year: y @ None, .. } | Day::EndOf { year: y @ None, .. }) = &mut m.day
        && let Some(n) = year(w)
    {
        *y = Some(n);
        return Some(j + 1);
    }
    // "3 Aug 96"
    if let Some(Day::On { year: y @ None, .. }) = &mut m.day
        && let Some(n) = short_year(t, j)
    {
        *y = Some(n);
        return Some(j + 1);
    }
    if m.time.is_none() && !matches!(m.day, Some(Day::Ahead(Amount { moment: true, .. }))) {
        // after a day or a part of the day, a bare hour is a time: "fri 9", "tonight 8"
        if let Some((tm, k)) = read_time(t, j, m.day.is_some() || m.part.is_some()) {
            m.time = Some(tm);
            return Some(k);
        }
    }
    // a part of the day, only after a day or a time: "tomorrow morning", "6 in the morning", "11 at night"
    if m.part.is_none()
        && (m.day.is_some() || m.time.is_some())
        && let Some((p, k)) = part_of_day(t, j)
    {
        m.part = Some(p);
        return Some(k);
    }
    None
}

/// "morning", "in the afternoon", "at night", "tonight" at word `j`.
fn part_of_day(t: &Text, j: usize) -> Option<(Part, usize)> {
    match t.at(j)? {
        "tonight" => Some((Part::Night, j + 1)),
        "at" if t.is(j + 1, "night") => Some((Part::Night, j + 2)),
        "in" if t.is(j + 1, "the") && t.spaced(j + 2) => part_word(&t[j + 2].lower).filter(|p| *p != Part::Night).map(|p| (p, j + 3)),
        w => part_word(w).map(|p| (p, j + 1)),
    }
}

fn part_word(w: &str) -> Option<Part> {
    Some(match w {
        "morning" => Part::Morning,
        "afternoon" => Part::Afternoon,
        "evening" => Part::Evening,
        "night" => Part::Night,
        _ => return None,
    })
}

/// One of a unit: "day", "hours", and the short forms "d", "h", "m", "w", "mo", "y".
fn unit(w: &str) -> Option<Amount> {
    let (months, days, minutes, moment) = match w {
        "d" | "day" | "days" => (0, 1, 0, false),
        "w" | "wk" | "wks" | "week" | "weeks" => (0, 7, 0, false),
        "mo" | "month" | "months" => (1, 0, 0, false),
        "y" | "yr" | "yrs" | "year" | "years" => (12, 0, 0, false),
        "h" | "hr" | "hrs" | "hour" | "hours" => (0, 0, 60, true),
        "m" | "min" | "mins" | "minute" | "minutes" => (0, 0, 1, true),
        // seconds are below the minute: they make it a moment, and nothing more
        "s" | "sec" | "secs" | "second" | "seconds" => (0, 0, 0, true),
        _ => return None,
    };
    Some(Amount { months, days, minutes, moment })
}

/// A whole-day unit as (months, days): "week" is (0, 7), "yrs" is (12, 0). Hours and minutes aren't one.
pub fn calendar_unit(w: &str) -> Option<(i64, i64)> {
    unit(w).filter(|a| !a.moment).map(|a| (a.months, a.days))
}

/// An amount of time: "3 days", "2h", "half an hour", "1d, 2hr, and 5min". Returns the word after it.
fn read_amount(t: &Text, i: usize) -> Option<(Amount, usize)> {
    let term = |j: usize| -> Option<(Amount, usize)> {
        let w = t.at(j)?;
        if w == "half" && t.spaced(j + 1) && matches!(t.at(j + 1), Some("an" | "a")) && t.is(j + 2, "hour") {
            return Some((Amount { minutes: 30, moment: true, ..Amount::default() }, j + 3));
        }
        // "2h", "15min", "3mo"
        let digits = w.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && digits < w.len() {
            return Some((unit(&w[digits..])?.times(number(&w[..digits], 3)?.into()), j + 1));
        }
        // "1.5 hours": a tenth of an hour is a whole number of minutes, a tenth of a day isn't
        if let Some(n) = number(w, 3).filter(|_| t.sep(j + 1, "."))
            && let Some(f) = t.at(j + 1).and_then(|f| number(f, 1))
            && let Some(u) = t.at(j + 2).filter(|_| t.spaced(j + 2)).and_then(unit).filter(|u| u.months == 0 && u.days == 0)
        {
            return Some((Amount { minutes: u.minutes * (n * 10 + f) as i64 / 10, ..u }, j + 3));
        }
        // "a few days", "a couple of weeks", "a couple days", "several hours"
        let j = if w == "a" && t.spaced(j + 1) && matches!(t.at(j + 1), Some("few" | "couple")) { j + 1 } else { j };
        let (n, j) = match t.at(j)? {
            "few" => (3, j),
            "several" => (7, j),
            "couple" => (2, if t.is(j + 1, "of") { j + 1 } else { j }),
            w => (count(w)?, j),
        };
        Some((unit(t.at(j + 1).filter(|_| t.spaced(j + 1))?)?.times(n), j + 2))
    };
    let (mut total, mut k) = term(i)?;
    // more terms after a space, a comma or "and"
    while t.joined(k).is_some() {
        let j = if t.at(k) == Some("and") && t.spaced(k + 1) { k + 1 } else { k };
        let Some((a, next)) = term(j) else { break };
        (total, k) = (total.plus(a), next);
    }
    Some((total, k))
}

/// "first" to "thirty-first", as a day of the month. Returns the word after it.
fn ordinal(t: &Text, i: usize) -> Option<(u8, usize)> {
    const ORD: [&str; 20] = [
        "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "eleventh", "twelfth",
        "thirteenth", "fourteenth", "fifteenth", "sixteenth", "seventeenth", "eighteenth", "nineteenth", "twentieth",
    ];
    let one = |w: &str| ORD.iter().position(|o| *o == w).map(|n| n as u8 + 1);
    let w = t.at(i)?;
    if w == "thirtieth" {
        return Some((30, i + 1));
    }
    let tens = match w {
        "twenty" => 20,
        "thirty" => 30,
        _ => return one(w).map(|d| (d, i + 1)),
    };
    // "twenty-fourth", "twenty fourth"
    let gap = t.gap(i + 1)?;
    if gap.is_empty() || !matches!(gap.trim(), "" | "-") {
        return None;
    }
    let d = tens + one(t.at(i + 1)?).filter(|&d| d < 10)?;
    (d <= 31).then_some((d, i + 2))
}

fn number(w: &str, max_len: usize) -> Option<u32> {
    (!w.is_empty() && w.len() <= max_len && w.bytes().all(|b| b.is_ascii_digit())).then(|| w.parse().unwrap())
}

/// "3", "a", "five": a count.
fn count(w: &str) -> Option<i64> {
    const WORDS: [&str; 13] = ["a", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    match w {
        "an" => Some(1),
        _ => WORDS.iter().position(|n| *n == w).map(|n| n.max(1) as i64).or_else(|| number(w, 3).map(i64::from)),
    }
}

/// The digits of "27th".
fn strip_ordinal(w: &str) -> Option<&str> {
    ["st", "nd", "rd", "th"].iter().find_map(|s| w.strip_suffix(s))
}

/// "27", "27th": a day of the month.
fn day_number(w: &str) -> Option<u8> {
    number(strip_ordinal(w).unwrap_or(w), 2).filter(|d| (1..=31).contains(d)).map(|d| d as u8)
}

fn year(w: &str) -> Option<i32> {
    (w.len() == 4).then(|| number(w, 4)).flatten().map(|y| y as i32)
}

/// "96" is 1996 and "27" is 2027, but only where the phrase ends and never an hour: "dec 5 50 invitations" is a count.
fn short_year(t: &Text, j: usize) -> Option<i32> {
    let w = t.at(j)?;
    // nor after a date in numbers: "14/4 90"
    let named = j > 0 && !t.before(j - 1).ends_with('/');
    let n = number(w, 2).filter(|&n| named && n > 23 && t.joined(j + 1).is_none())? as i32;
    Some(if n < 50 { 2000 + n } else { 1900 + n })
}

/// "2 weeks after yesterday", "the day before friday", "day after tomorrow": whole days on from another day.
fn offset(t: &Text, i: usize, o: &Options) -> Option<(Day, usize)> {
    // "day after tomorrow" is one day without saying so
    let (a, k) = if t[i].lower == "day" { (unit("day")?, i + 1) } else { read_amount(t, i)? };
    let sign = match t.at(k)? {
        "after" => 1,
        "before" => -1,
        _ => return None,
    };
    if a.moment || !t.spaced(k) || !t.spaced(k + 1) {
        return None;
    }
    let (base, end) = read_day(t, k + 1, o)?;
    Some((Day::From(a.times(sign), Box::new(base)), end))
}

/// The day part, and the index of the word after it. A date that can't exist ("31/2") isn't one.
fn read_day(t: &Text, i: usize, o: &Options) -> Option<(Day, usize)> {
    // 2000 was a leap year, so a 29 Feb without a year passes
    read_day_any(t, i, o).filter(|(d, _)| !matches!(*d, Day::On { year, month, day } if !valid(year.unwrap_or(2000), month, day)))
}

fn read_day_any(t: &Text, i: usize, o: &Options) -> Option<(Day, usize)> {
    let w = t.at(i)?;
    match w {
        "now" => return Some((Day::Ahead(Amount { moment: true, ..Amount::default() }), i + 1)),
        "today" => return Some((Day::InDays(0), i + 1)),
        "tomorrow" => return Some((Day::InDays(1), i + 1)),
        "yesterday" => return Some((Day::InDays(-1), i + 1)),
        "in" | "within" | "after" if t.spaced(i + 1) => {
            // "in about 5 hours"
            let j = if matches!(t.at(i + 1), Some("about" | "around")) && t.spaced(i + 2) { i + 2 } else { i + 1 };
            return read_amount(t, j).map(|(a, k)| (Day::Ahead(a), k));
        }
        "this" if t.spaced(i + 1) => {
            return match t.at(i + 1)? {
                "weekend" => Some((Day::Weekend(0), i + 2)),
                "week" => Some((Day::This(Period::Week), i + 2)),
                "month" => Some((Day::This(Period::Month), i + 2)),
                "year" => Some((Day::This(Period::Year), i + 2)),
                w => Some((Day::Weekday(weekday(w)?), i + 2)),
            };
        }
        // "the 5th", "the 5th of December"; a plain "the 2" is a count, not a day
        "the" if t.spaced(i + 1) => {
            if let Some((d @ (Day::On { .. } | Day::From(..)), k)) = read_day_any(t, i + 1, o) {
                return Some((d, k));
            }
            let w = t.at(i + 1)?;
            return strip_ordinal(w).and(day_number(w)).map(|d| (Day::Ordinal(d), i + 2));
        }
        // "next week" is next Monday (on a Sunday, tomorrow); "in a week" is seven days
        "next" if t.spaced(i + 1) => match t.at(i + 1)? {
            "week" => return Some((Day::Weekday(1), i + 2)),
            "weekend" => return Some((Day::Weekend(1), i + 2)),
            w if weekday(w).is_some() => return Some((Day::WeekOf(weekday(w)?, 1), i + 2)),
            "day" => return Some((Day::InDays(1), i + 2)),
            w @ ("month" | "year" | "hour") => return Some((Day::Ahead(unit(w)?), i + 2)),
            // "next 2 weeks"
            _ => return read_amount(t, i + 1).map(|(a, k)| (Day::Ahead(a), k)),
        },
        "end" if t.is(i + 1, "of") && t.spaced(i + 2) => return end_of(t, i + 2),
        // "last day of July" is the end of it
        "last" if t.is(i + 1, "day") && t.is(i + 2, "of") && t.spaced(i + 3) => return end_of(t, i + 3),
        // behind us, since a task can be overdue: "last week" is seven days ago, "last friday" the one before today
        "last" | "past" if t.spaced(i + 1) => match t.at(i + 1)? {
            w if weekday(w).is_some() => return Some((Day::LastWeekday(weekday(w)?), i + 2)),
            "weekend" => return Some((Day::Weekend(-1), i + 2)),
            w @ ("day" | "week" | "month" | "year") => return Some((Day::Ahead(unit(w)?.times(-1)), i + 2)),
            // "last 2 weeks"
            _ => return read_amount(t, i + 1).map(|(a, k)| (Day::Ahead(a.times(-1)), k)),
        },
        _ => {}
    }
    if let Some(found) = offset(t, i, o) {
        return Some(found);
    }
    // "+15min", "5 days ago", "2 days later", "3 weeks from now"; a bare "10 minutes" is a length, not a date
    if let Some((a, k)) = read_amount(t, i) {
        if t.before(i).ends_with('+') {
            return Some((Day::Ahead(a), k));
        }
        let (sign, k) = match t.at(k).filter(|_| t.spaced(k)) {
            Some("ago") => (-1, k + 1),
            Some("later") => (1, k + 1),
            Some("from") if t.is(k + 1, "now") => (1, k + 2),
            _ => (0, k),
        };
        if sign != 0 {
            return Some((Day::Ahead(a.times(sign)), k));
        }
    }
    if let Some(d) = weekday(w) {
        return Some((Day::Weekday(d), i + 1));
    }
    // "nov 27", "Dec. 21", "september 2012", and "in June" (a month alone is a name: "call may")
    if let Some(m) = month(w) {
        let dotted = t.gap(i + 1).is_some_and(|g| g.len() > 1 && g.trim_start_matches('.').chars().all(char::is_whitespace));
        let next = (t.spaced(i + 1) || dotted).then(|| t.at(i + 1)).flatten();
        if let Some(day) = next.and_then(day_number) {
            return Some((Day::On { year: None, month: m, day }, past_range(t, i + 2)));
        }
        // "May eighth", "May twenty-fourth"
        if let Some((day, k)) = next.and_then(|_| ordinal(t, i + 1)) {
            return Some((Day::On { year: None, month: m, day }, past_range(t, k)));
        }
        if let Some(y) = next.and_then(year) {
            return Some((Day::Month { year: Some(y), month: m }, i + 2));
        }
        let after_in = t.spaced(i) && t.at(i - 1) == Some("in");
        return after_in.then_some((Day::Month { year: None, month: m }, i + 1));
    }
    // "15Sep", as a flight itinerary writes it
    let digits = w.bytes().take_while(u8::is_ascii_digit).count();
    if let (Some(day), Some(m)) = (day_number(&w[..digits]), month(&w[digits..])) {
        return Some((Day::On { year: None, month: m, day }, i + 1));
    }
    // "27 nov", "12th of July", "twenty-fourth of May"
    if let Some((day, k)) = day_number(w).map(|d| (d, i + 1)).or_else(|| ordinal(t, i)) {
        let k = if t.is(k, "of") { k + 1 } else { k };
        // "10-August-2012", "06/Nov/2023": the same mark brings the year
        let mark = ["-", "/"].into_iter().find(|s| t.sep(k, s));
        if let Some(m) = (t.spaced(k) || mark.is_some()).then(|| t.at(k).and_then(month)).flatten() {
            let y = mark.filter(|s| t.sep(k + 1, s)).and_then(|_| t.at(k + 1).and_then(year));
            return Some((Day::On { year: y, month: m, day }, k + 1 + y.is_some() as usize));
        }
        // "10 - 22 August": the 10th of that August
        if let Some((Day::On { year, month, .. }, end)) = range_end(t, k).and_then(|j| read_day(t, j, o)) {
            return Some((Day::On { year, month, day }, end));
        }
    }
    // "2024 August", "2018 March 18"
    if let Some(y) = year(w).filter(|y| (1900..=2100).contains(y))
        && t.spaced(i + 1)
        && let Some(m) = t.at(i + 1).and_then(month)
    {
        return Some(match t.at(i + 2).filter(|_| t.spaced(i + 2)).and_then(day_number) {
            Some(day) => (Day::On { year: Some(y), month: m, day }, i + 3),
            None => (Day::Month { year: Some(y), month: m }, i + 2),
        });
    }
    // "2014-04-18", "2012/8/10", "2012/Aug/10"; with a month's name the day can be left out: "2024-Aug"
    if let Some(y) = year(w)
        && let Some(mark) = ["-", "/", "."].into_iter().find(|s| t.sep(i + 1, s))
    {
        let name = t.at(i + 1).and_then(month);
        let d = t.sep(i + 2, mark).then(|| t.at(i + 2).and_then(|w| number(w, 2))).flatten();
        if let (Some(m), Some(d)) = (name.or_else(|| t.at(i + 1).and_then(|w| number(w, 2)).map(|m| m as u8)), d) {
            return Some((Day::On { year: Some(y), month: m, day: d as u8 }, i + 3));
        }
        if let Some(m) = name {
            return Some((Day::Month { year: Some(y), month: m }, i + 2));
        }
    }
    // "3/12" is 3 December; "5/31" can only be 31 May; "9/2012" is a month
    let a = number(w, 2)? as u8;
    if !t.sep(i + 1, "/") {
        return None;
    }
    let b = t.at(i + 1)?;
    if let Some(y) = year(b) {
        return (1..=12).contains(&a).then_some((Day::Month { year: Some(y), month: a }, i + 2));
    }
    let b = number(b, 2)? as u8;
    let y = t.sep(i + 2, "/").then(|| t.at(i + 2).and_then(year)).flatten();
    if t.sep(i + 2, "/") && y.is_none() {
        return None;
    }
    // the order the options prefer, unless only the other one is a date
    let first = if o.month_first { (b, a) } else { (a, b) };
    let (day, month) = if valid(y.unwrap_or(2000), first.1, first.0) { first } else { (first.1, first.0) };
    Some((Day::On { year: y, month, day }, i + 2 + y.is_some() as usize))
}

/// After "end of": "month", "the month", "next month", "this year", "July".
fn end_of(t: &Text, i: usize) -> Option<(Day, usize)> {
    // "end of the next month"
    let i = if t.at(i) == Some("the") && t.spaced(i + 1) && matches!(t.at(i + 1), Some("next" | "last" | "past")) { i + 1 } else { i };
    let (shift, k) = match t.at(i)? {
        "the" | "this" => (0, i + 1),
        "next" => (1, i + 1),
        "last" | "past" => (-1, i + 1),
        _ => (0, i),
    };
    if k > i && !t.spaced(k) {
        return None;
    }
    match t.at(k)? {
        "month" => Some((Day::EndOfMonth(shift), k + 1)),
        "week" => Some((Day::EndOfWeek(shift), k + 1)),
        "year" => Some((Day::EndOfYear(shift), k + 1)),
        w if k == i => Some((Day::EndOf { year: None, month: month(w)? }, k + 1)),
        _ => None,
    }
}

/// "at 5", "5pm", "15:30", "10:30 am", "noon", and a bare "9" when `bare` (after a day). Returns the index of the word after it.
fn read_time(t: &Text, i: usize, bare: bool) -> Option<(Time, usize)> {
    let mut j = i;
    let with_at = t.at(j) == Some("at") && t.spaced(j + 1);
    if with_at {
        j += 1;
    }
    match t.at(j)? {
        "noon" | "midday" => return Some((Time { hour: 12, minute: 0, meridiem: true, zero: false }, j + 1)),
        "midnight" => return Some((Time { hour: 0, minute: 0, meridiem: true, zero: false }, j + 1)),
        _ => {}
    }
    let split = |w: &str| ["am", "pm"].iter().find_map(|s| w.strip_suffix(s).map(|d| (d.to_string(), Some(*s))));
    let (hour_text, mut suffix) = split(&t[j].lower).unwrap_or((t[j].lower.clone(), None));
    let mut hour = number(&hour_text, 2)?;
    let mut minute = None;
    // "12.30" could be a number ("12.53%"); it is a time only where a bare hour would be
    let colon = t.sep(j + 1, ":");
    // "15:30", "12.30"; seconds are read and dropped: "20:32:13"
    if suffix.is_none() && matches!(t.gap(j + 1), Some(":" | ".")) {
        let (m, s) = split(&t[j + 1].lower).unwrap_or((t[j + 1].lower.clone(), None));
        if m.len() == 2 {
            minute = Some(number(&m, 2)?);
            suffix = s;
            j += 1;
            if suffix.is_none() && t.sep(j + 1, ":") && t[j + 1].lower.len() == 2 && number(&t[j + 1].lower, 2).is_some() {
                j += 1;
            }
        }
    }
    if suffix.is_none() && t.spaced(j + 1) && matches!(t.at(j + 1), Some("am" | "pm")) {
        suffix = Some(if t[j + 1].lower == "am" { "am" } else { "pm" });
        j += 1;
    }
    // "9 p.m."
    if suffix.is_none() && t.spaced(j + 1) && matches!(t.at(j + 1), Some("a" | "p")) && t.sep(j + 2, ".") && t.at(j + 2) == Some("m") {
        suffix = Some(if t[j + 1].lower == "a" { "am" } else { "pm" });
        j += 2;
    }
    // a lone number is only a time with something around it: "at 9", "9:00", "9pm", "fri 9", "9 in the morning"
    let part_follows = t.spaced(j + 1) && part_of_day(t, j + 1).is_some_and(|(_, k)| k > j + 2 || t.at(j + 1) == Some("tonight"));
    if !(with_at || bare || (colon && minute.is_some()) || suffix.is_some() || part_follows) {
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

fn last_day(y: i32, m: u8) -> u8 {
    (28..=31).rev().find(|&d| valid(y, m, d)).unwrap()
}

/// The month `n` months from now's: (year, month).
fn months_on(now: DateTime, n: i64) -> (i32, u8) {
    let months = now.year as i64 * 12 + now.month as i64 - 1 + n;
    (months.div_euclid(12) as i32, (months.rem_euclid(12) + 1) as u8)
}

/// The day an amount lands on, minutes aside: months first, the same day clamped, then days.
fn ahead_days(now: DateTime, a: Amount) -> i64 {
    let (y, mo) = months_on(now, a.months);
    days_from_civil(y, mo, now.day.min(last_day(y, mo))) + a.days
}

/// The phrase as a moment, and whether it has a time.
pub fn resolve(m: DateMatch, now: DateTime, o: &Options) -> (DateTime, bool) {
    let today = days_from_civil(now.year, now.month, now.day);
    if let Some(Day::Ahead(a @ Amount { moment: true, .. })) = m.day {
        let at = ahead_days(now, a) * 1440 + now.hour as i64 * 60 + now.minute as i64 + a.minutes;
        let (year, month, day) = civil_from_days(at.div_euclid(1440));
        let (hour, minute) = ((at.rem_euclid(1440) / 60) as u8, (at.rem_euclid(60)) as u8);
        return (DateTime { year, month, day, hour, minute }, true);
    }
    let (mut hour, minute) = m.time.as_ref().map_or((0, 0), |t| (t.hour, t.minute));
    // "tonight at 12", "midnight tonight": the midnight that ends the day, so the next one's 00:00
    let late = m.part == Some(Part::Night) && m.time.as_ref().is_some_and(|t| t.hour == 0 || (t.hour == 12 && !t.meridiem));
    if late {
        hour = 0;
    }
    if let Some(t) = m.time.as_ref().filter(|t| !t.meridiem && !late) {
        let pm = match m.part {
            Some(Part::Morning) => false,
            Some(Part::Afternoon | Part::Evening) => t.hour < 12,
            // "11 at night" is 23:00, "1 at night" is 01:00
            Some(Part::Night) => (6..12).contains(&t.hour),
            None => o.bare_hour_pm && !t.zero && (1..=7).contains(&t.hour),
        };
        if pm {
            hour += 12;
        }
    }
    let days = match m.day {
        Some(d) => day_of(d, now, o) + late as i64,
        // "at 5": today, unless it has passed and the options look forward
        None if late || (o.forward && (hour, minute) <= (now.hour, now.minute)) => today + 1,
        None => today,
    };
    let (year, month, day) = civil_from_days(days);
    (DateTime { year, month, day, hour, minute }, m.time.is_some())
}

/// The day a phrase names, in days since 1970.
fn day_of(d: Day, now: DateTime, o: &Options) -> i64 {
    let today = days_from_civil(now.year, now.month, now.day);
    // looking forward, a period already begun is today: "this weekend" on a Sunday, "in November" in November.
    // "end of last week" asks for the past, and a year said ("November 2026") means that month's 1st.
    let begun = o.forward && matches!(d, Day::Weekend(0..) | Day::This(_) | Day::EndOfWeek(0..) | Day::Month { year: None, .. });
    // the month of a name: this one, or the next with that name
    let named = |year: Option<i32>, month: u8| year.unwrap_or(if month >= now.month { now.year } else { now.year + 1 });
    let monday = today - (weekday_of(today) as i64 + 6).rem_euclid(7);
    let days = match d {
        Day::InDays(n) => today + n,
        Day::WeekOf(d, n) => monday + 7 * n + (d as i64 + 6).rem_euclid(7),
        Day::Weekend(n) => monday + 5 + 7 * n,
        Day::This(Period::Week) => monday,
        Day::This(Period::Month) => days_from_civil(now.year, now.month, 1),
        Day::This(Period::Year) => days_from_civil(now.year, 1, 1),
        Day::EndOfWeek(n) => monday + 7 * n + (o.week_end as i64 + 6).rem_euclid(7),
        Day::Ordinal(d) => (0..13)
            .map(|k| months_on(now, k))
            .filter(|&(y, mo)| valid(y, mo, d))
            .map(|(y, mo)| days_from_civil(y, mo, d))
            .find(|&x| x >= today)
            .unwrap_or(today),
        Day::Ahead(a) => ahead_days(now, a),
        Day::From(a, base) => {
            let (year, month, day) = civil_from_days(day_of(*base, now, o));
            ahead_days(DateTime { year, month, day, ..now }, a)
        }
        // "fri" on a Friday means next week
        Day::Weekday(d) => today + ((d as i64 - weekday_of(today) as i64 + 6).rem_euclid(7) + 1),
        // and "last fri" on a Friday means last week
        Day::LastWeekday(d) => today - ((weekday_of(today) as i64 - d as i64 + 6).rem_euclid(7) + 1),
        Day::On { year: Some(y), month, day } => days_from_civil(y, month, day),
        // the next one from today; a 29 Feb can be years away
        Day::On { year: None, month, day } => (now.year..now.year + 8)
            .filter(|&y| valid(y, month, day))
            .map(|y| days_from_civil(y, month, day))
            .find(|&d| d >= today)
            .unwrap_or(today),
        Day::Month { year, month } => days_from_civil(named(year, month), month, 1),
        Day::EndOf { year, month } => {
            let y = named(year, month);
            days_from_civil(y, month, last_day(y, month))
        }
        Day::EndOfMonth(n) => {
            let (y, mo) = months_on(now, n);
            days_from_civil(y, mo, last_day(y, mo))
        }
        Day::EndOfYear(n) => days_from_civil(now.year + n as i32, 12, 31),
    };
    if begun { days.max(today) } else { days }
}
