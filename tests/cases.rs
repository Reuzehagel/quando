// Runs tests/cases.json (parity with OpenTodo) and tests/quando.json (our own); each fails with a scoreboard.
use quando::{dates, parse, DateTime, Options, Parsed, SpanKind, Unit};
use serde_json::{json, Value};

/// `dates` finds every date in free text; the default options read as chrono-node does, `todo()` as `parse` does.
#[test]
fn options() {
    let now = datetime("2026-11-20T14:00");
    fn found<'a>(s: &'a str, o: &Options) -> Vec<(&'a str, DateTime)> {
        dates(s, datetime("2026-11-20T14:00"), o).iter().map(|f| (&s[f.start..f.end], f.date)).collect()
    }
    let general = Options::default();
    assert_eq!(found("call me 3/12 at 5, or else friday", &general), [("3/12 at 5", datetime("2027-03-12T05:00")), ("friday", datetime("2026-11-27T00:00"))]);
    assert_eq!(found("call me 3/12 at 5", &Options::todo()), [("3/12 at 5", datetime("2026-12-03T17:00"))]);
    // a passed time stays today, and "this month" is its 1st, unless looking forward
    assert_eq!(found("at 9", &general), [("at 9", datetime("2026-11-20T09:00"))]);
    assert_eq!(found("this month", &general), [("this month", datetime("2026-11-01T00:00"))]);
    assert_eq!(found("this month", &Options::todo()), [("this month", datetime("2026-11-20T00:00"))]);
    // only one order is a date, so the option doesn't matter
    assert_eq!(found("5/31", &Options::todo())[0].1, datetime("2027-05-31T00:00"));
    assert_eq!(found("31/5", &general)[0].1, datetime("2027-05-31T00:00"));
    assert!(dates("tomorrow's paper", now, &general).is_empty());
    assert_eq!(found("+15min", &general), [("+15min", datetime("2026-11-20T14:15"))]);
}

fn datetime(s: &str) -> DateTime {
    let n = |r: std::ops::Range<usize>| s[r].parse::<i32>().unwrap();
    DateTime { year: n(0..4), month: n(5..7) as u8, day: n(8..10) as u8, hour: n(11..13) as u8, minute: n(14..16) as u8 }
}

/// The same shape as the `want` objects in cases.json.
fn to_json(p: &Parsed) -> Value {
    let date = p.date.map(|d| {
        let day = format!("{:04}-{:02}-{:02}", d.year, d.month, d.day);
        if p.has_time { format!("{day}T{:02}:{:02}", d.hour, d.minute) } else { day }
    });
    let unit = |u: Unit| format!("{u:?}").to_lowercase();
    let repeat = p.repeat.as_ref().map(|r| {
        let mut v = json!({ "every": r.every, "unit": unit(r.unit), "fromCompletion": r.from_completion });
        if let Some(w) = r.weekday { v["weekday"] = json!(w) }
        if r.weekdays_only { v["weekdaysOnly"] = json!(true) }
        if let Some(d) = r.day { v["day"] = json!(d) }
        v
    });
    let kind = |k: SpanKind| format!("{k:?}").to_lowercase();
    json!({
        "title": p.title, "date": date, "list": p.list, "tags": p.tags, "priority": p.priority, "repeat": repeat,
        "spans": p.spans.iter().map(|s| json!({ "start": s.start, "end": s.end, "kind": kind(s.kind) })).collect::<Vec<_>>(),
    })
}

/// What OpenTodo's TypeScript parser returns: quando matches it, except where quando.json reads a phrase differently.
#[test]
fn opentodo() {
    let own: Value = serde_json::from_str(include_str!("quando.json")).unwrap();
    let differs: Vec<&Value> = own["cases"].as_array().unwrap().iter().map(|c| &c["input"]).collect();
    run(include_str!("cases.json"), &differs)
}

/// quando's own: phrases OpenTodo doesn't test, or reads differently.
#[test]
fn quando() {
    run(include_str!("quando.json"), &[])
}

/// A month on from the 31st is the last day of a shorter month, not the start of the one after.
#[test]
fn months_clamp() {
    let date = |s, now| parse(s, datetime(now), &[]).date.map(|d| (d.year, d.month, d.day));
    assert_eq!(date("next month", "2027-01-31T10:00"), Some((2027, 2, 28)));
    assert_eq!(date("in 1 year", "2028-02-29T10:00"), Some((2029, 2, 28)));
}

/// Weeks start on Monday: on a weekend, "next week" is the coming one and "this weekend" is today.
#[test]
fn weekends() {
    let date = |s, now| parse(s, datetime(now), &[]).date.map(|d| (d.month, d.day));
    let sunday = "2026-11-22T10:00";
    assert_eq!(date("next week", sunday), Some((11, 23)));
    assert_eq!(date("next friday", sunday), Some((11, 27)));
    assert_eq!(date("this weekend", sunday), Some((11, 22)));
    assert_eq!(date("next weekend", sunday), Some((11, 28)));
    let saturday = "2026-11-21T10:00";
    assert_eq!(date("this weekend", saturday), Some((11, 21)));
    assert_eq!(date("next weekend", saturday), Some((11, 28)));
    assert_eq!(date("next saturday", saturday), Some((11, 28)));
    // "this week", "this month", "this year" start before today, so they are today
    for s in ["this week", "this month", "this year"] {
        assert_eq!(date(s, sunday), Some((11, 22)), "{s}");
    }
    assert_eq!(parse("pay rent this month", datetime(sunday), &[]).title, "pay rent");
}

/// "last" looks back from today, and an amount counts from the day it names.
#[test]
fn behind_and_from() {
    let date = |s, now| parse(s, datetime(now), &[]).date.map(|d| (d.month, d.day));
    let saturday = "2026-11-21T10:00";
    assert_eq!(date("last friday", saturday), Some((11, 20)));
    // today's own name is a week away, either way
    assert_eq!(date("last saturday", saturday), Some((11, 14)));
    assert_eq!(date("saturday", saturday), Some((11, 28)));
    assert_eq!(date("last month", "2027-03-31T10:00"), Some((2, 28)));
    assert_eq!(date("a month after 31 jan", saturday), Some((2, 28)));
    assert_eq!(date("day before 1 march", "2028-01-10T10:00"), Some((2, 29)));
}

/// Edges the phrase files can't reach, since they share one `now`.
#[test]
fn edges() {
    let at = |s, now| parse(s, datetime(now), &[]).date.map(|d| (d.year, d.month, d.day, d.hour));
    // a day no month has yet: February is skipped
    assert_eq!(at("the 30th", "2027-01-31T10:00"), Some((2027, 3, 30, 0)));
    assert_eq!(at("end of february", "2028-01-10T10:00"), Some((2028, 2, 29, 0)));
    // a date that is today stays today; the day before is next year's
    assert_eq!(at("nov 20", "2026-11-20T14:00"), Some((2026, 11, 20, 0)));
    assert_eq!(at("nov 19", "2026-11-20T14:00"), Some((2027, 11, 19, 0)));
    // a time that is now has passed
    assert_eq!(at("at 14:00", "2026-11-20T14:00"), Some((2026, 11, 21, 14)));
    assert_eq!(at("at 23:59", "2026-12-31T23:58"), Some((2026, 12, 31, 23)));
    assert_eq!(at("12am", "2026-12-31T14:00"), Some((2027, 1, 1, 0)));
    assert_eq!(at("in 10 hours", "2026-12-31T14:00"), Some((2027, 1, 1, 0)));
    assert_eq!(at("pay 2026-02-30", "2026-11-20T14:00"), None);
}

/// Anything typed gives an answer, never a panic: spans land on char boundaries whatever the bytes.
#[test]
fn never_panics() {
    let now = datetime("2026-11-20T14:00");
    for s in ["", " ", "#", "@", "!!!!", "at", "at at 5", "every", "every other", "in 3", "3/", "/12", "31/2/2026", "nov", "29 feb 2027",
        "every 99999999999 days", "at 99", "25:00", "12am", "0pm", "ünïcödé #lïst @tâg ! morgen", "日本語 #リスト 明日\u{3000}!!", "tomorrow’s", "a\u{3000}!!\u{3000}b",
        "+", "in", "in half", "in half an", "in 1d,", "in 1d, and", "twenty-", "twenty-ninth", "thirty-ninth of may", "the", "next", "end of", "99999999mo later", "+1y",
        "-", "to", "monday -", "monday to", "- friday", "10 -", "10 to", "aug 10 to", "day", "day after", "the day before", "last", "past 2", "last day of",
        "end of the", "end of the next", "a few", "in a couple", "in about", "1-", "15sep-", "2024-", "2024/aug/", "9-jan-", "aug 5 –", "aug 5 – 7"] {
        let p = parse(s, now, &[]);
        for sp in &p.spans {
            assert!(s.is_char_boundary(sp.start) && s.is_char_boundary(sp.end), "{s:?}");
        }
    }
}

fn run(json: &str, skip: &[&Value]) {
    let file: Value = serde_json::from_str(json).unwrap();
    let now = datetime(file["now"].as_str().unwrap());
    let cases: Vec<&Value> = file["cases"].as_array().unwrap().iter().filter(|c| !skip.contains(&&c["input"])).collect();
    let mut failed = Vec::new();
    for c in &cases {
        let ignored: Vec<&str> = c["ignored"].as_array().map(|a| a.iter().map(|v| v.as_str().unwrap()).collect()).unwrap_or_default();
        let got = to_json(&parse(c["input"].as_str().unwrap(), now, &ignored));
        if got != c["want"] {
            failed.push(format!("{}\n  want {}\n  got  {}", c["input"], c["want"], got));
        }
    }
    assert!(failed.is_empty(), "{}/{} cases pass\n\n{}", cases.len() - failed.len(), cases.len(), failed.join("\n"));
}
