// How often quando reads chrono-node's English test phrases as bench.json records them. The score is agreement, not
// correctness, and it only goes up: raise FLOOR when it does. A phrase quando reads better belongs in cases.json.
// `cargo test --test bench -- --nocapture` prints the score; set QUANDO_MISSES=1 to list every disagreement.
mod common;

use common::{datetime, stamp};
use quando::{parse, SpanKind};
use serde_json::Value;

const FLOOR: usize = 404;

#[test]
fn bench() {
    let file: Value = serde_json::from_str(include_str!("bench.json")).unwrap();
    let now = datetime(file["now"].as_str().unwrap());
    let show = std::env::var_os("QUANDO_MISSES").is_some();

    let (mut same, mut missed, mut extra, mut differs) = (0, Vec::new(), Vec::new(), Vec::new());
    let cases = file["cases"].as_array().unwrap();
    for c in cases {
        let input = c["input"].as_str().unwrap();
        let p = parse(input, now, &[]);
        let span = p.spans.iter().find(|s| s.kind == SpanKind::Date);
        let date = span.and(p.date).map(|d| stamp(d, p.has_time));
        let want = c["date"].as_str();
        // quando's span takes a leading "on", "by" or "in" so the title reads well; chrono's leaves it out
        let span = span.map(|s| {
            let lead = c["span"][0].as_u64().map(|w| w as usize).filter(|&w| w > s.start && matches!(input[s.start..w].trim().to_lowercase().as_str(), "on" | "by" | "in"));
            (lead.unwrap_or(s.start), s.end)
        });
        let line = |got: &str| format!("{input:?}\n    want {} {}\n    got  {got}", want.unwrap_or("-"), c["span"]);
        match (want, &date, span) {
            (None, None, _) => same += 1,
            (Some(w), Some(d), Some((a, b))) if w == d && c["span"] == serde_json::json!([a, b]) => same += 1,
            (Some(_), None, _) => missed.push(line("-")),
            (None, Some(d), _) => extra.push(line(d)),
            (Some(_), Some(d), span) => differs.push(line(&format!("{d} {span:?}"))),
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
