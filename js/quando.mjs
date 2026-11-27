// quando from JS: `const { parse } = await load(wasmBytesOrModule)`, then
// `parse("dentist fri 9", new Date())` gives { title, date, list, tags, priority, repeat, spans } as in tests/cases.json:
// `date` is "2026-11-21T11:00", or "2026-11-21" with no time; spans count UTF-16 units, as JS strings do.
export async function load(wasm) {
  const result = await WebAssembly.instantiate(wasm);
  const e = (result.instance ?? result).exports;
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();

  // `now` is read in its local zone; `ignored` holds span texts the user turned back into plain text.
  function parse(text, now, ignored = []) {
    const input = encoder.encode([text, ...ignored].map((s) => s.replaceAll("\n", " ")).join("\n"));
    const ptr = e.quando_alloc(input.length);
    new Uint8Array(e.memory.buffer, ptr, input.length).set(input);
    const out = e.quando_parse(ptr, input.length, now.getFullYear(), now.getMonth() + 1, now.getDate(), now.getHours(), now.getMinutes());
    const len = new DataView(e.memory.buffer).getUint32(out, true);
    const json = decoder.decode(new Uint8Array(e.memory.buffer, out + 4, len));
    e.quando_free(out, len + 4);
    return JSON.parse(json);
  }
  return { parse };
}
