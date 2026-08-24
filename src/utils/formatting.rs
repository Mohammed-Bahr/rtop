/// Format a byte count using binary units (KB = 1024 B), e.g. `10.9 GB`.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Format a bytes-per-second rate, e.g. `2.41 MB/s`.
pub fn format_rate(bytes_per_sec: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec.max(0.0) as u64))
}

/// Format a duration in seconds as `[Dd ]HH:MM:SS`.
pub fn format_duration(total_secs: u64) -> String {
    let days = total_secs / 86_400;
    let rem = total_secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    if days > 0 {
        format!("{days}d {h:02}:{m:02}:{s:02}")
    } else {
        format!("{h:02}:{m:02}:{s:02}")
    }
}

/// Truncate a string to `max` chars, appending an ellipsis when cut.
/// Operates on chars, not bytes, so multi-byte UTF-8 is handled safely.
pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let cut: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536 * 1024), "1.5 MB");
        assert_eq!(format_bytes(11_700_000_000), "10.9 GB");
    }

    #[test]
    fn rates() {
        assert_eq!(format_rate(420.0 * 1024.0), "420.0 KB/s");
    }

    #[test]
    fn durations() {
        assert_eq!(format_duration(59), "00:00:59");
        assert_eq!(format_duration(3661), "01:01:01");
        assert_eq!(format_duration(90_000), "1d 01:00:00");
    }

    #[test]
    fn truncation() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 5), "abcd…");
        // Multi-byte characters are counted per char, not per byte.
        assert_eq!(truncate("héllo wörld", 6), "héllo…");
    }
}
