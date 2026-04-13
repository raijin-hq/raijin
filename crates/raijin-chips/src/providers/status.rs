use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for last command exit status.
///
/// Only shown when the exit code is non-zero. Maps well-known exit codes
/// to human-readable signal names following Unix conventions:
///
/// - Exit codes 129-165 map to signals (128 + signal number):
///   130 → INT, 137 → KILL, 139 → SEGV, 143 → TERM
/// - Special exit codes: 126 → NOPERM, 127 → NOTFOUND
/// - All others show the numeric code with a failure indicator.
///

pub struct StatusProvider;

impl ChipProvider for StatusProvider {
    fn id(&self) -> ChipId {
        "status"
    }

    fn display_name(&self) -> &str {
        "Exit Status"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.last_exit_code.is_some_and(|c| c != 0)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let code = ctx.last_exit_code.unwrap_or(1);
        let label = format_exit_code(code);
        let tooltip = format_exit_tooltip(code);

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("AlertCircle"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Format an exit code for display.
///
/// If the exit code maps to a known signal, show the signal name.
/// Otherwise show the numeric code.
fn format_exit_code(code: i32) -> String {
    if let Some(signal_name) = exit_code_to_signal_name(code) {
        signal_name.to_string()
    } else if let Some(meaning) = common_meaning(code) {
        meaning.to_string()
    } else {
        format!("\u{2718} {code}")
    }
}

/// Format a detailed tooltip for the exit code.
fn format_exit_tooltip(code: i32) -> String {
    if let Some(signal_name) = exit_code_to_signal_name(code) {
        let signal_num = code - 128;
        format!("Exit code {code} (SIG{signal_name}, signal {signal_num})")
    } else if let Some(meaning) = common_meaning(code) {
        format!("Exit code {code} ({meaning})")
    } else {
        format!("Exit code {code}")
    }
}

/// Map exit codes > 128 to signal names (exit_code = 128 + signal_number).
fn exit_code_to_signal_name(code: i32) -> Option<&'static str> {
    if code <= 128 {
        return None;
    }
    let signal = code - 128;
    match signal {
        1 => Some("HUP"),
        2 => Some("INT"),
        3 => Some("QUIT"),
        4 => Some("ILL"),
        5 => Some("TRAP"),
        6 => Some("IOT"),
        7 => Some("BUS"),
        8 => Some("FPE"),
        9 => Some("KILL"),
        10 => Some("USR1"),
        11 => Some("SEGV"),
        12 => Some("USR2"),
        13 => Some("PIPE"),
        14 => Some("ALRM"),
        15 => Some("TERM"),
        16 => Some("STKFLT"),
        17 => Some("CHLD"),
        18 => Some("CONT"),
        19 => Some("STOP"),
        20 => Some("TSTP"),
        21 => Some("TTIN"),
        22 => Some("TTOU"),
        _ => None,
    }
}

/// Map common non-signal exit codes to human-readable meanings.
fn common_meaning(code: i32) -> Option<&'static str> {
    match code {
        1 => Some("ERROR"),
        2 => Some("USAGE"),
        126 => Some("NOPERM"),
        127 => Some("NOTFOUND"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_names() {
        assert_eq!(format_exit_code(130), "INT");
        assert_eq!(format_exit_code(137), "KILL");
        assert_eq!(format_exit_code(139), "SEGV");
        assert_eq!(format_exit_code(143), "TERM");
        assert_eq!(format_exit_code(129), "HUP");
        assert_eq!(format_exit_code(134), "IOT");
    }

    #[test]
    fn test_common_meanings() {
        assert_eq!(format_exit_code(1), "ERROR");
        assert_eq!(format_exit_code(126), "NOPERM");
        assert_eq!(format_exit_code(127), "NOTFOUND");
    }

    #[test]
    fn test_unknown_exit_code() {
        assert_eq!(format_exit_code(42), "\u{2718} 42");
        assert_eq!(format_exit_code(255), "\u{2718} 255");
    }

    #[test]
    fn test_tooltip_signal() {
        assert_eq!(
            format_exit_tooltip(130),
            "Exit code 130 (SIGINT, signal 2)"
        );
    }

    #[test]
    fn test_tooltip_common() {
        assert_eq!(format_exit_tooltip(127), "Exit code 127 (NOTFOUND)");
    }

    #[test]
    fn test_tooltip_unknown() {
        assert_eq!(format_exit_tooltip(42), "Exit code 42");
    }
}
