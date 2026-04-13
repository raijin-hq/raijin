use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for last command execution duration.
///
/// Shows human-readable duration when the last command exceeded a configurable
/// threshold (default: 2000ms / 2 seconds).
///
/// Duration formatting tiers:
/// - Under 1 second: not shown (below threshold)
/// - Under 60 seconds: `"5s"`, `"32s"`
/// - Under 60 minutes: `"2m 30s"`, `"15m 7s"`
/// - 60 minutes and above: `"1h 5m"`, `"3h 22m"`
///

pub struct CmdDurationProvider;

/// Minimum command duration in milliseconds before the chip is shown.
const MIN_DURATION_MS: u64 = 2000;

impl ChipProvider for CmdDurationProvider {
    fn id(&self) -> ChipId {
        "cmd_duration"
    }

    fn display_name(&self) -> &str {
        "Command Duration"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.last_duration_ms
            .is_some_and(|ms| ms >= MIN_DURATION_MS)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let ms = ctx.last_duration_ms.unwrap_or(0);
        let label = format_duration(ms);

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Timer"),
            tooltip: Some(format_duration_tooltip(ms)),
            ..ChipOutput::default()
        }
    }
}

/// Format milliseconds into a human-readable duration string.
///
/// Follows the same formatting conventions as `render_time`:
/// - `< 60s`: whole seconds only (`"5s"`, `"32s"`)
/// - `< 60m`: minutes and seconds (`"2m 30s"`)
/// - `>= 60m`: hours and minutes (`"1h 5m"`)
fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;

    if total_secs < 60 {
        format!("{total_secs}s")
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if secs == 0 {
            format!("{mins}m")
        } else {
            format!("{mins}m {secs}s")
        }
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {mins}m")
        }
    }
}

/// Format a detailed tooltip with exact milliseconds.
fn format_duration_tooltip(ms: u64) -> String {
    let human = format_duration(ms);
    format!("Last command took {human} ({ms}ms)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(5000), "5s");
        assert_eq!(format_duration(32000), "32s");
        assert_eq!(format_duration(59999), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60_000), "1m");
        assert_eq!(format_duration(150_000), "2m 30s");
        assert_eq!(format_duration(907_000), "15m 7s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3_600_000), "1h");
        assert_eq!(format_duration(3_900_000), "1h 5m");
        assert_eq!(format_duration(12_120_000), "3h 22m");
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(999), "0s");
    }

    #[test]
    fn test_format_duration_exact_minute() {
        assert_eq!(format_duration(120_000), "2m");
    }
}
