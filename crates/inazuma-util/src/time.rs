use std::time::Duration;

pub fn duration_alt_display(duration: Duration) -> String {
    let hours = duration.as_secs() / 3600;
    let minutes = (duration.as_secs() % 3600) / 60;
    let seconds = duration.as_secs() % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// Render a duration in milliseconds to a compact human-readable string.
///
/// Adapted from Starship (.reference/starship-master/src/utils/mod.rs) — MIT License.
///
/// Examples:
/// - `render_time(500, false)` → `"0s"`
/// - `render_time(5000, false)` → `"5s"`
/// - `render_time(65000, false)` → `"1m5s"`
/// - `render_time(3661000, false)` → `"1h1m1s"`
/// - `render_time(86400000, false)` → `"1d0h0m0s"`
/// - `render_time(5500, true)` → `"5s500ms"`
pub fn render_time(raw_millis: u128, show_millis: bool) -> String {
    match (raw_millis, show_millis) {
        (0, true) => return "0ms".into(),
        (0..=999, false) => return "0s".into(),
        _ => (),
    }

    let (millis, raw_seconds) = (raw_millis % 1000, raw_millis / 1000);
    let (seconds, raw_minutes) = (raw_seconds % 60, raw_seconds / 60);
    let (minutes, raw_hours) = (raw_minutes % 60, raw_minutes / 60);
    let (hours, days) = (raw_hours % 24, raw_hours / 24);

    let components = [(days, "d"), (hours, "h"), (minutes, "m"), (seconds, "s")];

    let result = components.iter().fold(
        String::new(),
        |acc, (component, suffix)| match component {
            0 if acc.is_empty() => acc,
            n => acc + &n.to_string() + suffix,
        },
    );

    if show_millis {
        result + &millis.to_string() + "ms"
    } else {
        result
    }
}

/// Formats an integer into a human-readable string using SI prefixes (k, M, G, T).
///
/// Adapted from Starship (.reference/starship-master/src/utils/mod.rs) — MIT License.
pub fn humanize_int(n: u64) -> String {
    if n < 1000 {
        return n.to_string();
    }

    let prefixes = ["k", "M", "G", "T"];
    let mut value = n as f64;
    let mut prefix_idx = 0;

    while value >= 1000.0 && prefix_idx < prefixes.len() {
        value /= 1000.0;
        prefix_idx += 1;
    }

    if value >= 100.0 {
        format!("{:.0}{}", value, prefixes[prefix_idx - 1])
    } else if value >= 10.0 {
        format!("{:.1}{}", value, prefixes[prefix_idx - 1])
    } else {
        format!("{:.2}{}", value, prefixes[prefix_idx - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_alt_display() {
        use duration_alt_display as f;
        assert_eq!("0s", f(Duration::from_secs(0)));
        assert_eq!("59s", f(Duration::from_secs(59)));
        assert_eq!("1m 0s", f(Duration::from_secs(60)));
        assert_eq!("10m 0s", f(Duration::from_secs(600)));
        assert_eq!("1h 0m 0s", f(Duration::from_secs(3600)));
        assert_eq!("3h 2m 1s", f(Duration::from_secs(3600 * 3 + 60 * 2 + 1)));
        assert_eq!("23h 59m 59s", f(Duration::from_secs(3600 * 24 - 1)));
        assert_eq!("100h 0m 0s", f(Duration::from_secs(3600 * 100)));
    }

    #[test]
    fn test_render_time_zero() {
        assert_eq!(render_time(0, false), "0s");
        assert_eq!(render_time(0, true), "0ms");
    }

    #[test]
    fn test_render_time_seconds() {
        assert_eq!(render_time(5000, false), "5s");
        assert_eq!(render_time(500, false), "0s");
    }

    #[test]
    fn test_render_time_minutes() {
        assert_eq!(render_time(65000, false), "1m5s");
        assert_eq!(render_time(120000, false), "2m0s");
    }

    #[test]
    fn test_render_time_hours() {
        assert_eq!(render_time(3661000, false), "1h1m1s");
    }

    #[test]
    fn test_render_time_days() {
        assert_eq!(render_time(86400000, false), "1d0h0m0s");
    }

    #[test]
    fn test_render_time_with_millis() {
        assert_eq!(render_time(5500, true), "5s500ms");
    }

    #[test]
    fn test_humanize_int() {
        assert_eq!(humanize_int(0), "0");
        assert_eq!(humanize_int(999), "999");
        assert_eq!(humanize_int(1000), "1.00k");
        assert_eq!(humanize_int(1500), "1.50k");
        assert_eq!(humanize_int(1000000), "1.00M");
    }
}
