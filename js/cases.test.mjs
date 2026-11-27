// Every line in tests/cases.json reads as its `want` through the wasm build, plus a line JS counts differently.
import { readFileSync } from "node:fs";
import { deepStrictEqual } from "node:assert";
import { load } from "./quando.mjs";

const root = new URL("../", import.meta.url);
const { parse } = await load(readFileSync(new URL("target/wasm32-unknown-unknown/release/quando.wasm", root)));
const file = JSON.parse(readFileSync(new URL("tests/cases.json", root), "utf8"));
const now = new Date(file.now);

for (const c of file.cases) deepStrictEqual(parse(c.input, now, c.ignored), c.want, c.input);

const text = "café 🎂 tomorrow";
const [span] = parse(text, now).spans;
deepStrictEqual(text.slice(span.start, span.end), "tomorrow");
console.log(`${file.cases.length + 1} cases agree`);
