# quando

Finds dates in English text. A Rust library with no dependencies, so it runs natively and in WebAssembly (the browser, Cloudflare Workers).

- `dates` finds every date in a sentence: "call me 3/12 at 5, or else friday".
- `parse` reads one todo line, `dentist fri 9 #health !!`, as a title, a date and time, a list, tags, a priority and a repeat, plus the spans an editor highlights while you type.

```sh
cargo add --git https://github.com/Reuzehagel/quando
```

## Usage

```rust
use quando::{dates, parse, DateTime, Options};

// A Friday afternoon, as the user's clock shows it. DateTime has no zone: pass local time.
let now = DateTime { year: 2026, month: 11, day: 20, hour: 14, minute: 0 };

let found = dates("call me 3/12 at 5, or else friday", now, &Options::default());
assert_eq!((found[0].start, found[0].end), (8, 17)); // "3/12 at 5"
assert_eq!(found[0].date, DateTime { year: 2026, month: 12, day: 3, hour: 5, minute: 0 });
assert_eq!(found[1].date.day, 27); // the coming Friday

let p = parse("call bank at 5 tomorrow #admin", now, &[]);
assert_eq!(p.title, "call bank");
assert_eq!(p.date, Some(DateTime { year: 2026, month: 11, day: 21, hour: 17, minute: 0 }));
assert_eq!(p.list.as_deref(), Some("admin"));
```

`parse`'s third argument lists span texts the user turned back into plain text: with `&["tomorrow"]`, "read tomorrow and today" keeps "tomorrow" in its title.

## Options

`Options::default()` turns every switch off. `parse` uses `Options::todo()`, which turns on `forward` and `bare_hour_pm`; `parse_with` takes your own.

| Field | Off | On |
|---|---|---|
| `month_first` | `3/12` is 3 December | `3/12` is 12 March (US) |
| `forward` | at 14:00, `at 9` is 09:00 today | `at 9` is tomorrow; `this week`, `this month` etc. never fall before today |
| `bare_hour_pm` | `at 5` is 05:00 | `at 5` is 17:00 (bare 1 to 7) |

`week_end` is the day `end of week` falls on: 0 is Sunday (the default), 5 is Friday. Weeks start on Monday.

## What it reads

- **Days:** `today`, `tonight`, `fri`, `next friday`, `friday of next week`, `this weekend`, `the 5th`, `27 nov`, `Dec. 21`, `3/12`, `2014-04-18`, `3 aug 27`
- **Months:** `in June`, `September 2012`, `end of month`, `last day of July`, `end of next week`
- **Past:** `last week`, `last friday`, `last night`, `5 days ago`
- **Relative:** `in 3 days`, `in a couple of weeks`, `next month`, `3 weeks from now`, `2 days before friday`, `the day after tomorrow`
- **Moments:** `now`, `in half an hour`, `in 1.5 hours`, `+15min`, `in 1d, 2hr, and 5min`
- **Times:** `at 5`, `9 p.m.`, `15:30`, `noon`, `fri 9`, `tomorrow morning`, `11 at night`
- **Ranges:** `monday - friday`, `10 - 12 August`, `tomorrow until 5pm`, `8 - 11pm`, `between 3:30-4:30pm`
- **Repeats:** `daily`, `every monday 9`, `every other week`, `every weekday`, `after done`
- **Markers:** `#list`, `@tag`, `!` to `!!!`

It leaves alone what only looks like a date: "call may", "boil eggs 10 minutes", "call monday to confirm" (no range), "buy tomorrow's paper". `tests/cases.json` has every reading in full.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release --target wasm32-unknown-unknown
QUANDO_MISSES=1 cargo test --test bench -- --nocapture   # benchmark score and every phrase it disagrees on
```

- `tests/cases.json`: todo lines and what `parse` returns for each, as of Friday 2026-11-20 14:00. A misread phrase gets a case before it gets a fix.
- `tests/bench.json`: the 569 English phrases from chrono-node's test suite (MIT). quando agrees on 424 (75%); `tests/bench.rs` fails below `FLOOR`, so raise it when the score goes up. Agreement isn't correctness: a phrase quando reads better belongs in `cases.json`.

## License

MIT
