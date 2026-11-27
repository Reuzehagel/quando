# quando

A natural-language parser for one typed line: `dentist fri 9 #health !!` reads as a title, a date and time, a list, a priority and repeats, plus the spans an editor highlights while you type. Rust with no runtime dependencies, for native apps and WebAssembly (the browser and Cloudflare Workers) alike.

```rust
let p = quando::parse("call bank at 5 tomorrow #admin", now, &[]);
// p.title == "call bank", p.date == 2026-11-21 17:00, p.list == Some("admin")
```

`now` is a wall-clock time: quando reads the line in whatever zone `now` is in. `ignored` holds span texts the user turned back into plain text ("buy tomorrow's paper").

## What it reads

- Days: `today`, `tomorrow`, `yesterday`, `tonight`, `this morning`, weekdays (`fri`, `friday`, `this wednesday`), `next friday` (that day in next week; weeks start on Monday), `this weekend`, `next weekend` (the Saturday; on a Sunday, this weekend is today), `the 5th` (the next 5th, today included), `the 5th of December`, `27 nov`, `nov 27th`, `12th of July`, `Dec. 21`, `27 nov 2027`, `Jan 1, 2020`, `3/12` (day first), `5/31` (month first when day first can't be a date), `3/12/2026`, `2014-04-18`, `2027/1/9`, `9-jan-2027`, `15sep`. A two-digit year counts only where the phrase ends and it can't be an hour: `3 aug 27` is 2027, "dec 5 50 invitations" is a count.
- Months: `in June`, `September 2012`, `9/2012`, `2024-Aug`, `end of month`, `end of next month`, `end of July`, `end of this year`, `last day of July`. A month name alone stays text: "call may".
- Behind: `last week`, `last month`, `last 2 weeks` (that long ago), `last friday` (the one before today). A task can be overdue.
- From another day: `the day after tomorrow`, `2 days before friday`, `a week after 3/12`. Whole days only: "arrive 15 minutes before" stays text.
- Ranges: `monday - friday`, `10pm to 2am`, `10 - 12 August`, `aug 10 to 22, 2027`, with a dash, `to`, `until`, `till` or `through`. The date is the start and the span runs to the end; the start borrows a year or a day only the end says. The end must be a date itself: "call monday to confirm" keeps its title.
- Relative: `in 3 days`, `in five days`, `in a week`, `in 3 months`, `next week` (next Monday, as a date picker's button), `next month`, `next year`, `next 2 weeks`, `2 days later`, `3w later`, `3 weeks from now`, `after a year`, `5 days ago`, `in a few days` (3), `in a couple of weeks`, `in about 2 hours`. Months keep the day, clamped: a month on from 31 Jan is 28 Feb.
- Moments: `now`, `in an hour`, `in half an hour`, `in 2h`, `within 20 minutes`, `+15min`, `in 1 day 2 hours`, `in 1d, 2hr, and 5min`: from now, to the minute. An amount needs `in`, `within`, `after`, `+`, `later`, `ago` or `from now`: "boil eggs 10 minutes" stays text.
- Ordinal words: `May eighth`, `May twenty-fourth`, `the twenty-fourth of May`. Only beside a month: "read the second half" stays text.
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
| amounts (half an hour, 2h, 1d 2hr 5min), ordinal words | 303/569 (53%) |
| ranges, last week, the day after tomorrow, a few days, 9-jan-2027, two-digit years | 404/569 (71%) |
