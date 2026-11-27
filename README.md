# quando

A natural-language parser for one typed line: `dentist fri 9 #health !!` reads as a title, a date and time, a list, a priority and repeats, plus the spans an editor highlights while you type. Rust with no runtime dependencies, for native apps and WebAssembly (the browser and Cloudflare Workers) alike.

```rust
let p = quando::parse("call bank at 5 tomorrow #admin", now, &[]);
// p.title == "call bank", p.date == 2026-11-21 17:00, p.list == Some("admin")
```

`now` is a wall-clock time: quando reads the line in whatever zone `now` is in. `ignored` holds span texts the user turned back into plain text ("buy tomorrow's paper").

## What it reads

- Days: `today`, `tomorrow`, weekdays (`fri`, `friday`), `in 3 days`, `in 2 weeks`, `27 nov`, `nov 27th`, `27 nov 2027`, `3/12` (day first), `3/12/2026`.
- Times: `at 5`, `5pm`, `10:30 am`, `15:30`, `05:30`, and a bare hour after a day (`fri 9`). A bare 1 to 7 is the afternoon; `5am` and `05:30` mean what they say. A time on its own is today, or tomorrow once it has passed.
- Repeats: `daily`, `weekly`, `monthly`, `yearly`, `every monday`, `every other week`, `every 3 days`, `every weekday`, and `after done`.
- Markers: `#list`, `@tag`, `!` to `!!!`.

## The tests

- `tests/cases.json`: what OpenTodo's TypeScript parser (on chrono-node) returns for its phrase tests, as of Friday 2026-11-20 14:00. quando matches all of it. Regenerate with `node tools/export-cases.ts tests/cases.json` from a sibling `../opentodo` checkout.
- `tests/quando.json`: phrases OpenTodo doesn't test, or reads differently. A case here overrides the one in `cases.json` with the same input. The differences: `fri 5:30` is 17:30 here, 05:30 there; `standup every weekday at 9` on a Friday afternoon starts Monday here, Saturday there.

A phrase that reads wrong gets a case before it gets a fix. Span offsets are bytes in Rust and UTF-16 units in JavaScript; they agree while a line is ASCII.

```sh
cargo test
cargo build --release --target wasm32-unknown-unknown
```
