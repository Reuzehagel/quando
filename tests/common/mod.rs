use quando::DateTime;

/// "2026-11-20T14:00".
pub fn datetime(s: &str) -> DateTime {
    let n = |r: std::ops::Range<usize>| s[r].parse::<i32>().unwrap();
    DateTime { year: n(0..4), month: n(5..7) as u8, day: n(8..10) as u8, hour: n(11..13) as u8, minute: n(14..16) as u8 }
}

/// "2026-11-20T14:00", or "2026-11-20" without a time.
pub fn stamp(d: DateTime, has_time: bool) -> String {
    let day = format!("{:04}-{:02}-{:02}", d.year, d.month, d.day);
    if has_time { format!("{day}T{:02}:{:02}", d.hour, d.minute) } else { day }
}
