// Runs every case in cases.json and fails with a scoreboard until all of them pass.
use quando::{parse, DateTime, Parsed, SpanKind, Unit};
use serde_json::{json, Value};

fn datetime(s: &str) -> DateTime {
    let n = |r: std::ops::Range<usize>| s[r].parse::<i32>().unwrap();
    DateTime { year: n(0..4) as i32, month: n(5..7) as u8, day: n(8..10) as u8, hour: n(11..13) as u8, minute: n(14..16) as u8 }
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

#[test]
fn cases() {
    let file: Value = serde_json::from_str(include_str!("cases.json")).unwrap();
    let now = datetime(file["now"].as_str().unwrap());
    let cases = file["cases"].as_array().unwrap();
    let mut failed = Vec::new();
    for c in cases {
        let ignored: Vec<&str> = c["ignored"].as_array().map(|a| a.iter().map(|v| v.as_str().unwrap()).collect()).unwrap_or_default();
        let got = to_json(&parse(c["input"].as_str().unwrap(), now, &ignored));
        if got != c["want"] {
            failed.push(format!("{}\n  want {}\n  got  {}", c["input"], c["want"], got));
        }
    }
    assert!(failed.is_empty(), "{}/{} cases pass\n\n{}", cases.len() - failed.len(), cases.len(), failed.join("\n"));
}
