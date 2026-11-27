// How often quando reads chrono-node's English test phrases as OpenTodo does. The score is agreement, not
// correctness (see tools/export-bench.ts), and it only goes up: raise FLOOR when it does.
// `cargo test --test bench -- --nocapture` prints the score; set QUANDO_MISSES=1 to list every disagreement.
use quando::{parse, DateTime, SpanKind};
use serde_json::Value;

const FLOOR: usize = 304;

#[test]
fn bench() {
    let file: Value = serde_json::from_str(include_str!("bench.json")).unwrap();
    let n = |s: &str, r: std::ops::Range<usize>| s[r].parse::<i32>().unwrap();
    let now = file["now"].as_str().unwrap();
    let now = DateTime { year: n(now, 0..4), month: n(now, 5..7) as u8, day: n(now, 8..10) as u8, hour: n(now, 11..13) as u8, minute: n(now, 14..16) as u8 };
    let show = std::env::var_os("QUANDO_MISSES").is_some();

    let (mut same, mut missed, mut extra, mut differs) = (0, Vec::new(), Vec::new(), Vec::new());
    let cases = file["cases"].as_array().unwrap();
    for c in cases {
        let input = c["input"].as_str().unwrap();
        let p = parse(input, now, &[]);
        let span = p.spans.iter().find(|s| s.kind == SpanKind::Date);
        let date = span.and(p.date).map(|d| {
            let day = format!("{:04}-{:02}-{:02}", d.year, d.month, d.day);
            if p.has_time { format!("{day}T{:02}:{:02}", d.hour, d.minute) } else { day }
        });
        let want = c["date"].as_str();
        // quando's span takes a leading "on", "by" or "in" so the title reads well; chrono's leaves it out
        let span = span.map(|s| {
            let lead = c["span"][0].as_u64().map(|w| w as usize).filter(|&w| w > s.start && matches!(input[s.start..w].trim().to_lowercase().as_str(), "on" | "by" | "in"));
            quando::Span { start: lead.unwrap_or(s.start), ..s.clone() }
        });
        let span = span.as_ref();
        let line = |got: &str| format!("{input:?}\n    want {} {}\n    got  {got}", want.unwrap_or("-"), c["span"]);
        match (want, &date) {
            (None, None) => same += 1,
            (Some(w), Some(d)) if w == d && c["span"] == serde_json::json!([span.unwrap().start, span.unwrap().end]) => same += 1,
            (Some(_), None) => missed.push(line("-")),
            (None, Some(d)) => extra.push(line(d)),
            (Some(_), Some(d)) => differs.push(line(&format!("{d} [{},{}]", span.unwrap().start, span.unwrap().end))),
        }
    }

    println!("{same}/{} agree ({:.0}%)", cases.len(), 100.0 * same as f64 / cases.len() as f64);
    println!("{} missed (no date in quando), {} read differently, {} extra (a date only quando sees)", missed.len(), differs.len(), extra.len());
    if show {
        for (name, list) in [("missed", &missed), ("differs", &differs), ("extra", &extra)] {
            println!("\n== {name}");
            list.iter().for_each(|l| println!("{l}"));
        }
    }
    assert!(same >= FLOOR, "the benchmark fell from {FLOOR} to {same}: QUANDO_MISSES=1 cargo test --test bench -- --nocapture");
}
