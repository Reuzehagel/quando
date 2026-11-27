# quando

A natural-language parser for one typed line: `dentist fri 9 #health !!` reads as a title, a date and time, a list, a priority and repeats, plus the spans an editor highlights while you type. Rust with no runtime dependencies, for native apps and WebAssembly (the browser and Cloudflare Workers) alike.

```rust
let p = quando::parse("call bank at 5 tomorrow #admin", now, &[]);
// p.title == "call bank", p.date == 2026-11-21 17:00, p.list == Some("admin")
```

`now` is a wall-clock time: quando reads the line in whatever zone `now` is in. `ignored` holds span texts the user turned back into plain text ("buy tomorrow's paper").

## What it reads

- Days: `today`, `tomorrow`, `yesterday`, `tonight`, `this morning`, weekdays (`fri`, `friday`, `this wednesday`), `next friday` (that day in next week; weeks start on Monday), `this weekend`, `next weekend` (the Saturday; on a Sunday, this weekend is today), `the 5th` (the next 5th, today included), `the 5th of December`, `27 nov`, `nov 27th`, `12th of July`, `Dec. 21`, `27 nov 2027`, `Jan 1, 2020`, `3/12` (day first), `5/31` (month first when day first can't be a date), `3/12/2026`, `2014-04-18`.
- Months: `in June`, `September 2012`, `9/2012`, `end of month`, `end of next month`, `end of July`, `end of this year`. A month name alone stays text: "call may".
- Relative: `in 3 days`, `in five days`, `in a week`, `in 3 months`, `next week` (next Monday, as a date picker's button), `next month`, `next year`, `next 2 weeks`, `2 days later`, `3 weeks from now`, `after a year`, `5 days ago`. Months keep the day, clamped: a month on from 31 Jan is 28 Feb.
- Moments: `now`, `in an hour`, `in 30 minutes`: from now, to the minute.
- Times: `at 5`, `5pm`, `10:30 am`, `15:30`, `05:30`, `at 12.30`, `noon`, `midnight`, and a bare hour after a day (`fri 9`). A bare 1 to 7 is the afternoon; `5am` and `05:30` mean what they say. A time on its own is today, or tomorrow once it has passed.
- Parts of the day: `tomorrow morning`, `6 in the morning`, `11 at night`, `11 tonight`, `this afternoon at 3`.
- Parts join with spaces, commas, `at`, `on`, `of`, `by`, `before`, `after`: `Tuesday, 10 January`, `tomorrow by 5pm`, `at midnight on 12th August`. A leading `on`, `by` or `in` joins the date, so "dentist on friday" is titled "dentist" (chrono leaves it out; the benchmark counts that as agreeing).
- Repeats: `daily`, `weekly`, `monthly`, `yearly`, `every monday`, `every other week`, `every 3 days`, `every weekday`, and `after done`. A repeat with only a time starts on a day it allows.
- Markers: `#list`, `@tag`, `!` to `!!!`.

## The tests

- `tests/cases.json`: what OpenTodo's TypeScript parser (on chrono-node) returns for its phrase tests, as of Friday 2026-11-20 14:00. quando matches all of it. Regenerate with `node tools/export-cases.ts tests/cases.json` from a sibling `../opentodo` checkout.
- `tests/quando.json`: phrases OpenTodo doesn't test, or reads differently (`fri 5:30` is 17:30 here, 05:30 there). A case here overrides the one in `cases.json` with the same input.
- `tests/bench.json`: the benchmark. The 569 English phrases from chrono-node's own test suite (MIT), as OpenTodo reads them. `tests/bench.rs` fails if quando agrees on fewer than its `FLOOR`, so the score only goes up. It measures agreement, not correctness: a phrase quando reads better belongs in `quando.json`. Regenerate with `node tools/export-bench.ts <chrono checkout> tests/bench.json`.

A phrase that reads wrong gets a case before it gets a fix. Span offsets are bytes in Rust and UTF-16 units in JavaScript; they agree while a line is ASCII.

```sh
cargo test
QUANDO_MISSES=1 cargo test --test bench -- --nocapture   # the score, and every phrase it disagrees on
cargo build --release --target wasm32-unknown-unknown
```

## Benchmark

| | agrees with OpenTodo |
|---|---|
| first parser | 120/569 (21%) |
| parts joined, parts of the day, months, ISO dates | 251/569 (44%) |
| next friday, weekends, the 5th | 260/569 (46%) |
