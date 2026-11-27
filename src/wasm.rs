// The wasm export: `parse` as JSON in the shape of a `want` in tests/cases.json, with spans in UTF-16 units as JS
// strings count them. js/quando.mjs is the JS side.
use crate::{parse, DateTime, Parsed, SpanKind, Unit};
use std::fmt::Write;

/// `s` as a JSON string.
fn string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn to_json(text: &str, p: &Parsed) -> String {
    let opt = |s: &Option<String>| s.as_deref().map_or("null".into(), string);
    let date = p.date.map_or("null".into(), |d| {
        let day = format!("{:04}-{:02}-{:02}", d.year, d.month, d.day);
        string(&if p.has_time { format!("{day}T{:02}:{:02}", d.hour, d.minute) } else { day })
    });
    let repeat = p.repeat.as_ref().map_or("null".into(), |r| {
        let unit = match r.unit { Unit::Day => "day", Unit::Week => "week", Unit::Month => "month", Unit::Year => "year" };
        let mut v = format!(r#"{{"every":{},"unit":"{unit}","fromCompletion":{}"#, r.every, r.from_completion);
        if let Some(w) = r.weekday { write!(v, r#","weekday":{w}"#).unwrap() }
        if r.weekdays_only { v.push_str(r#","weekdaysOnly":true"#) }
        v + "}"
    });
    let utf16 = |b: usize| text[..b].encode_utf16().count();
    let spans: Vec<String> = p.spans.iter().map(|s| {
        let kind = match s.kind {
            SpanKind::Date => "date", SpanKind::Repeat => "repeat", SpanKind::List => "list",
            SpanKind::Tag => "tag", SpanKind::Priority => "priority",
        };
        format!(r#"{{"start":{},"end":{},"kind":"{kind}"}}"#, utf16(s.start), utf16(s.end))
    }).collect();
    let tags: Vec<String> = p.tags.iter().map(|t| string(t)).collect();
    format!(
        r#"{{"title":{},"date":{date},"list":{},"tags":[{}],"priority":{},"repeat":{repeat},"spans":[{}]}}"#,
        string(&p.title), opt(&p.list), tags.join(","), p.priority, spans.join(",")
    )
}

/// Room for `len` bytes, for JS to write into; give it back with [`quando_free`].
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn quando_alloc(len: usize) -> *mut u8 {
    std::mem::ManuallyDrop::new(Vec::<u8>::with_capacity(len)).as_mut_ptr()
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quando_free(ptr: *mut u8, len: usize) {
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
}

/// Reads the `len` UTF-8 bytes at `ptr`: the line, then each ignored text, split by newlines. Frees the input and
/// returns a buffer of a little-endian u32 length and that many bytes of JSON, to free with [`quando_free`].
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quando_parse(ptr: *mut u8, len: usize, year: i32, month: u8, day: u8, hour: u8, minute: u8) -> *mut u8 {
    let input = unsafe { String::from_utf8_unchecked(Vec::from_raw_parts(ptr, len, len)) };
    let mut lines = input.split('\n');
    let text = lines.next().unwrap_or("");
    let ignored: Vec<&str> = lines.collect();
    let json = to_json(text, &parse(text, DateTime { year, month, day, hour, minute }, &ignored));
    let mut out = (json.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(json.as_bytes());
    std::mem::ManuallyDrop::new(out.into_boxed_slice()).as_mut_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_count_utf16() {
        let now = DateTime { year: 2026, month: 11, day: 20, hour: 14, minute: 0 };
        // "é" is 2 bytes and 1 unit, "🎂" 4 bytes and 2 units: "tomorrow" starts at unit 8, byte 11
        let text = "café 🎂 tomorrow #\"x\\";
        let json = to_json(text, &parse(text, now, &[]));
        assert!(json.contains(r#"{"start":8,"end":16,"kind":"date"}"#), "{json}");
        assert!(json.contains(r#""title":"café 🎂 #\"x\\""#), "{json}");
    }
}
