use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for battery level and charging state.
///
/// Platform support:
/// - **macOS**: Parses `pmset -g batt` for percentage and charging/discharging state.
/// - **Linux**: Reads `/sys/class/power_supply/BAT0/capacity` and `status`.
///
/// Display format:
/// - Charging: `"42%"` with charging icon
/// - Discharging: `"42%"` with battery icon
/// - Full / AC power: `"100%"` or not shown
///
/// Returns an empty label if no battery is detected or on unsupported platforms.
pub struct BatteryProvider;

impl ChipProvider for BatteryProvider {
    fn id(&self) -> ChipId {
        "battery"
    }

    fn display_name(&self) -> &str {
        "Battery"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        get_battery_info(ctx).is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let (percentage, charging) = get_battery_info(ctx).unwrap_or((0, false));

        let icon = if charging {
            "BatteryCharging"
        } else if percentage > 75 {
            "BatteryFull"
        } else if percentage > 25 {
            "BatteryMedium"
        } else {
            "BatteryLow"
        };

        let label = format!("{percentage}%");
        let state = if charging { "charging" } else { "discharging" };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some(icon),
            tooltip: Some(format!("Battery {percentage}% ({state})")),
            ..ChipOutput::default()
        }
    }
}

/// Returns `(percentage, is_charging)` or None if no battery is detected.
fn get_battery_info(ctx: &ChipContext) -> Option<(u8, bool)> {
    #[cfg(target_os = "macos")]
    {
        get_battery_info_macos(ctx)
    }

    #[cfg(target_os = "linux")]
    {
        get_battery_info_linux(ctx)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = ctx;
        None
    }
}

/// Parse battery info from macOS `pmset -g batt`.
///
/// Output format:
/// ```text
/// Now drawing from 'Battery Power'
///  -InternalBattery-0 (id=...)    56%; discharging; 3:45 remaining
/// ```
#[cfg(target_os = "macos")]
fn get_battery_info_macos(ctx: &ChipContext) -> Option<(u8, bool)> {
    let output = ctx.exec_cmd("pmset", &["-g", "batt"])?;
    parse_pmset_output(&output.stdout)
}

fn parse_pmset_output(output: &str) -> Option<(u8, bool)> {
    for line in output.lines() {
        // Battery lines contain a percentage like "56%"
        if let Some(pct_pos) = line.find('%') {
            let before = &line[..pct_pos];
            let num_start = before
                .rfind(|c: char| !c.is_ascii_digit())
                .map(|i| i + 1)
                .unwrap_or(0);
            let num_str = &before[num_start..];
            if let Ok(pct) = num_str.parse::<u8>() {
                let charging = line.contains("charging") && !line.contains("discharging");
                return Some((pct, charging));
            }
        }
    }
    None
}

/// Read battery info from Linux sysfs.
#[cfg(target_os = "linux")]
fn get_battery_info_linux(_ctx: &ChipContext) -> Option<(u8, bool)> {
    use std::path::Path;

    // Try common battery paths
    for bat in &["BAT0", "BAT1", "BAT", "battery"] {
        let base = Path::new("/sys/class/power_supply").join(bat);
        if !base.exists() {
            continue;
        }

        let capacity = std::fs::read_to_string(base.join("capacity"))
            .ok()?
            .trim()
            .parse::<u8>()
            .ok()?;

        let status = std::fs::read_to_string(base.join("status"))
            .ok()
            .unwrap_or_default();

        let charging = status.trim().eq_ignore_ascii_case("charging");

        return Some((capacity, charging));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pmset_discharging() {
        let output = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=4653155)\t56%; discharging; 3:45 remaining present: true\n";
        assert_eq!(parse_pmset_output(output), Some((56, false)));
    }

    #[test]
    fn test_parse_pmset_charging() {
        let output = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=4653155)\t89%; charging; 0:30 remaining present: true\n";
        assert_eq!(parse_pmset_output(output), Some((89, true)));
    }

    #[test]
    fn test_parse_pmset_full() {
        let output = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=4653155)\t100%; charged; present: true\n";
        assert_eq!(parse_pmset_output(output), Some((100, false)));
    }

    #[test]
    fn test_parse_pmset_no_battery() {
        let output = "Now drawing from 'AC Power'\n";
        assert_eq!(parse_pmset_output(output), None);
    }
}
