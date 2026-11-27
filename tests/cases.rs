// Runs tests/cases.json (parity with OpenTodo) and tests/quando.json (our own); each fails with a scoreboard.
use quando::{parse, DateTime, Parsed, SpanKind, Unit};
use serde_json::{json, Value};

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
}

/// Anything typed gives an answer, never a panic: spans land on char boundaries whatever the bytes.
#[test]
fn never_panics() {
    let now = datetime("2026-11-20T14:00");
    for s in ["", " ", "#", "@", "!!!!", "at", "at at 5", "every", "every other", "in 3", "3/", "/12", "31/2/2026", "nov", "29 feb 2027",
        "every 99999999999 days", "at 99", "25:00", "12am", "0pm", "ünïcödé #lïst @tâg ! morgen", "日本語 #リスト 明日\u{3000}!!", "tomorrow’s", "a\u{3000}!!\u{3000}b",
        "+", "in", "in half", "in half an", "in 1d,", "in 1d, and", "twenty-", "twenty-ninth", "thirty-ninth of may", "the", "next", "end of", "99999999mo later", "+1y"] {
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
