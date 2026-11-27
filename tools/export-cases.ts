// Runs OpenTodo's parser (a sibling checkout at ../opentodo) on its phrase tests and writes tests/cases.json: node tools/export-cases.ts tests/cases.json
import { writeFileSync } from "node:fs"
import { parse } from "../../opentodo/src/quick-add/parse.ts"

const NOW = new Date("2026-11-20T14:00:00") // a Friday afternoon
const inputs: [string, string[]?][] = [
  ["dentist tomorrow at 11"], ["dentist tomorrow at 1"], ["dentist tomorrow at 12"], ["dentist fri 9"], ["fri"],
  ["call bank at 9"], ["call bank at 5"], ["call bank at 5am"], ["call bank 05:30"], ["call bank 5:30"],
  ["pay rent nov 27 at 15:30"], ["pay rent 27 nov"], ["pay rent 3/12"], ["pay rent 23 nov #work"], ["in 3 days"],
  ["buy tomorrow's paper"], ["milk #groceries @errands !!"], ["email #work tomorrow 10am"],
  ["bins every monday"], ["bins every friday"], ["bins 23 nov every monday"], ["water plants every 3 days after done"],
  ["standup every weekday at 9"], ["bins every friday at 9"], ["bins every friday at 5"], ["review monthly"], ["meet at #1 spot"], ["wow! great"],
  ["dentist next month"], ["in 3 months"], ["renew passport next year"], ["review next week"], ["review next week at 10"], ["review in a week"],
  ["read tomorrow and today", ["tomorrow"]],
]

const pad = (n: number) => String(n).padStart(2, "0")
const local = (d: Date, time: boolean) =>
  `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` + (time ? `T${pad(d.getHours())}:${pad(d.getMinutes())}` : "")

const cases = inputs.map(([input, ignored = []]) => {
  const p = parse(input, NOW, ignored)
  return {
    input,
    ...(ignored.length ? { ignored } : {}),
    want: {
      title: p.title,
      date: p.date && local(p.date, p.hasTime),
      list: p.list,
      tags: p.tags,
      priority: p.priority,
      repeat: p.repeat,
      spans: p.spans.map(({ start, end, kind }) => ({ start, end, kind })),
    },
  }
})

writeFileSync(process.argv[2], JSON.stringify({ now: "2026-11-20T14:00", cases }, null, 2) + "\n")
console.log(`${cases.length} cases`)
