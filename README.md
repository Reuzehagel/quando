# quando

Finds dates in English text. A Rust library with no dependencies, so it runs natively and in WebAssembly (the browser, Cloudflare Workers).

It does two jobs:

- `dates` finds every date in a sentence: "call me 3/12 at 5, or else friday".
- `parse` reads one todo line, `dentist fri 9 #health !!`, as a title, a date and time, a list, tags, a priority and a repeat. It also returns the spans an editor highlights while you type.

```sh
cargo add --git https://github.com/Reuzehagel/quando
```

## Usage

```rust
use quando::{dates, parse, DateTime, Options};

// A Friday afternoon, as the user's clock shows it
let now = DateTime { year: 2026, month: 11, day: 20, hour: 14, minute: 0 };

for f in dates("call me 3/12 at 5, or else friday", now, &Options::default()) {
    println!("{}..{} {:?} has_time={}", f.start, f.end, f.date, f.has_time);
}
// 8..17   3 December 05:00, has_time=true
// 27..33  27 November (the coming Friday), has_time=false

let p = parse("call bank at 5 tomorrow #admin", now, &[]);
assert_eq!(p.title, "call bank");
assert_eq!(p.date, Some(DateTime { year: 2026, month: 11, day: 21, hour: 17, minute: 0 }));
assert_eq!(p.list.as_deref(), Some("admin"));
```

`DateTime` has no time zone. quando reads the text in the zone `now` is in, so pass the user's local time.

What comes back:

- `dates` returns a `Vec<Found>`, in order: `start..end` (byte offsets), `date` and `has_time`.
- `parse` returns a `Parsed`:
  - `title`: the line with everything read taken out.
  - `date`, `has_time`.
  - `list` (`#list`), `tags` (`@tag`), `priority` (0 to 3, from `!` to `!!!`).
  - `repeat`: every how many days, weeks, months or years, on which weekday, weekdays only, or counted from when the task is done.
  - `spans`: what each part of the line was read as.

`parse`'s third argument holds span texts the user turned back into plain text. With `&["tomorrow"]`, "read tomorrow and today" keeps "tomorrow" in its title and reads "today" as the date.

## Options

Some readings depend on who is typing:

- `Options::default()` turns every switch off.
- `Options::todo()` turns on `forward` and `bare_hour_pm`. `parse` uses it.
- `parse_with` takes options of your own.

| Field | Off | On |
|---|---|---|
| `month_first` | `3/12` is 3 December, as Europe writes it | `3/12` is 12 March, as the US writes it |
| `forward` | at 14:00, `at 9` is 09:00 today and `this month` is the 1st | `at 9` is tomorrow; `this week`, `this weekend`, `this month`, `end of week` and `in November` (said in November) are never before today |
| `bare_hour_pm` | `at 5` is 05:00 | `at 5` is 17:00 (a bare 1 to 7); `5am` and `05:00` still mean what they say |

`week_end` is the day `end of week` falls on: 0 is Sunday (the default), 5 is a working week's Friday. Weeks start on Monday.

A date that can only be read one way (`5/31`, `31/5`) is read that way whatever `month_first` says.

## What it reads

The readings below are `parse`'s, with `Options::todo()`. The options above change only what their row says.

- **Days:** `today`, `tomorrow`, `yesterday`, `tonight`, `this morning`.
  - Weekdays: `fri`, `friday`, `this wednesday`. `next friday` is the Friday of next week.
  - Weekends: `this weekend` and `next weekend` are the Saturday. On a Sunday, this weekend is today.
  - Days of the month: `the 5th` is the next 5th, today included. Also `the 5th of December`.
  - Dates with month names: `27 nov`, `nov 27th`, `12th of July`, `Dec. 21`, `27 nov 2027`, `Jan 1, 2020`, `9-jan-2027`, `15sep`.
  - Numeric dates: `3/12` (day first), `5/31` (month first, since day first can't be a date), `3/12/2026`, `2014-04-18`, `2027/1/9`.
  - A two-digit year counts only at the end of the phrase, where it can't be an hour: `3 aug 27` is 2027, but in "dec 5 50 invitations" the 50 is a count.
- **Months:** `in June`, `September 2012`, `9/2012`, `2024-Aug`.
  - Ends: `end of month`, `end of next month`, `end of July`, `last day of July`, `end of this year`, `end of week`, `end of next week`.
  - A month name alone is a word, not a date: "call may".
- **The past:** `last week`, `last month`, `last 2 weeks` (that long ago), `last friday` (the one before today). A task can be overdue.
- **Counting from another day:** `the day after tomorrow`, `2 days before friday`, `a week after 3/12`. Whole days only: in "arrive 15 minutes before", nothing is a date.
- **Ranges:** `monday - friday`, `10pm to 2am`, `10 - 12 August`, `aug 10 to 22, 2027`.
  - The separator is a dash, `to`, `until`, `till` or `through`.
  - The date is the start, and the span runs to the end. The start borrows a year, a day or a time that only the end gives: `tomorrow until 5pm` is tomorrow at 17:00.
  - The end must be a date itself, so "call monday to confirm" keeps "to confirm" in its title. A number that counts something (`nov 27 - 30 euro`) is no range's end.
- **Relative:** `in 3 days`, `in five days`, `in a week`, `in 3 months`, `next month`, `next year`, `next 2 weeks`, `2 days later`, `3w later`, `3 weeks from now`, `after a year`, `5 days ago`, `in a few days` (3), `in a couple of weeks`.
  - `next week` is next Monday, like a date picker's button.
  - A month on keeps the day of the month, clamped: a month on from 31 January is 28 February.
- **Moments:** `now`, `in an hour`, `in half an hour`, `in 2h`, `within 20 minutes`, `+15min`, `in about 2 hours`, `in 1d, 2hr, and 5min`. These count from now, to the minute.
  - An amount needs a word that makes it a date: `in`, `within`, `after`, `+`, `later`, `ago` or `from now`. In "boil eggs 10 minutes", nothing is a date.
- **Ordinal words:** `May eighth`, `May twenty-fourth`, `the twenty-fourth of May`. Only next to a month: "read the second half" stays text.
- **Times:** `at 5`, `5pm`, `10:30 am`, `15:30`, `05:30`, `at 12.30`, `noon`, `midnight`, and a bare hour after a day: `fri 9`.
  - A time on its own is today, or tomorrow once it has passed.
- **Parts of the day:** `tomorrow morning`, `6 in the morning`, `11 at night`, `11 tonight`, `this afternoon at 3`.
  - `tonight at 12` and `midnight tonight` are the midnight at the end of today.
- **Joining parts:** spaces, commas, `at`, `on`, `of`, `by`, `before` and `after` join parts into one date: `Tuesday, 10 January`, `tomorrow by 5pm`, `at midnight on 12th August`.
  - A leading `on`, `by` or `in` goes with the date, so "dentist on friday" is titled "dentist".
- **Repeats:** `daily`, `weekly`, `monthly`, `yearly`, `every monday`, `every other week`, `every 3 days`, `every 2 wks`, `every weekday`, and `after done`.
  - A repeat with no date starts on the first day it allows.
  - A bare hour right after a repeat is its time: `every monday 9`.
- **Markers:** `#list` and `@tag` at the start of a word, and `!` to `!!!` standing on their own: "wow! great" has no priority.
- **Not dates:** "buy tomorrow's paper" keeps its title, since a possessive is a word, not a date.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release --target wasm32-unknown-unknown
QUANDO_MISSES=1 cargo test --test bench -- --nocapture   # the benchmark score, and every phrase it disagrees on
```

- `tests/cases.json` holds todo lines and everything `parse` returns for them, as of Friday 2026-11-20 14:00. A phrase that reads wrong gets a case here before it gets a fix.
- `tests/bench.json` is the benchmark: the 569 English phrases from chrono-node's test suite (MIT), with chrono-node's reading of each under a todo app's rules.
  - `tests/bench.rs` fails if fewer than `FLOOR` phrases agree, so the score only goes up. Raise `FLOOR` when it does.
  - The benchmark measures agreement, not correctness. A phrase quando reads better belongs in `cases.json`.
  - chrono-node leaves a leading "on", "by" or "in" out of the date, and the benchmark counts that as agreeing.

## Benchmark

| Added | Agrees |
|---|---|
| first parser | 120/569 (21%) |
| parts joined, parts of the day, months, ISO dates | 251/569 (44%) |
| next friday, weekends, the 5th | 260/569 (46%) |
| amounts (half an hour, 2h, 1d 2hr 5min), ordinal words | 303/569 (53%) |
| ranges, last week, the day after tomorrow, a few days, 9-jan-2027, two-digit years | 404/569 (71%) |
