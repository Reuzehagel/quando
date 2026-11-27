// The benchmark: the English phrases from chrono-node's own tests, read by OpenTodo's parser (chrono plus OpenTodo's rules).
// node tools/export-bench.ts <chrono checkout> tests/bench.json   (OpenTodo checked out at ../opentodo)
// shortcut: agreement with OpenTodo is the score, not correctness; a phrase quando reads better moves to quando.json.
import { readdirSync, readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import { parse } from "../../opentodo/src/quick-add/parse.ts"

const [chronoDir, out] = process.argv.slice(2)
const dir = join(chronoDir, "test", "en")
const CALL = /test(?:SingleCase|UnexpectedResult)\(\s*chrono[\w.]*\s*,\s*("(?:[^"\\\n]|\\.)*")/g

const phrases = new Set<string>()
for (const f of readdirSync(dir).filter((f) => f.endsWith(".test.ts") && f !== "en_timezone_exp.test.ts")) {
  for (const m of readFileSync(join(dir, f), "utf8").matchAll(CALL)) {
    const s = JSON.parse(m[1]) as string
    // one line, as it would be typed
    if (s.trim() && !s.includes("\n") && s.length <= 80) phrases.add(s.trim())
  }
}

const NOW = new Date("2026-11-20T14:00:00") // a Friday afternoon
const pad = (n: number) => String(n).padStart(2, "0")
const local = (d: Date, time: boolean) =>
  `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` + (time ? `T${pad(d.getHours())}:${pad(d.getMinutes())}` : "")

const cases = [...phrases].map((input) => {
  const p = parse(input, NOW)
  const span = p.spans.find((s) => s.kind === "date")
  // a repeat's first date isn't a date phrase; the benchmark is about dates
  return { input, date: span && p.date ? local(p.date, p.hasTime) : null, span: span ? [span.start, span.end] : null }
})

writeFileSync(out, JSON.stringify({ now: "2026-11-20T14:00", source: "chrono-node test/en (MIT, Wanasit Tanakitrungruang)", cases }, null, 1) + "\n")
console.log(`${cases.length} phrases, ${cases.filter((c) => c.date).length} with a date`)
