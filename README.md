# quando

A natural-language parser for one typed line: `dentist fri 9 #health !!` reads as a title, a date and time, a list, a priority and repeats, plus the spans an editor highlights while you type. Rust, for native apps and WebAssembly (the browser and Cloudflare Workers) alike.

Not written yet: `parse` returns the line as the title.

## The target

`tests/cases.json` holds what OpenTodo's TypeScript parser (on chrono-node) returns for its phrase tests, read as of Friday 2026-11-20 14:00. `cargo test` runs every case and prints how many pass. A phrase that reads wrong gets a case before it gets a fix.

Span offsets are bytes in Rust and UTF-16 units in JavaScript; they agree while the cases are ASCII.

```sh
cargo test                                  # the scoreboard
cargo build --target wasm32-unknown-unknown # stays WebAssembly-clean
```
