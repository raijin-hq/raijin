use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};
use raijin_battery as battery;

pub struct BatteryProvider;

impl ChipProvider for BatteryProvider {
    fn id(&self) -> ChipId { "battery" }
    fn display_name(&self) -> &str { "Battery" }
    fn is_available(&self, _ctx: &ChipContext) -> bool { true }
    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let (label, icon) = gather_battery(ctx).unwrap_or((String::new(), "Battery"));
        ChipOutput {
            id: self.id(), label,
            icon: Some(icon),
            tooltip: Some("Battery status".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_battery(ctx: &ChipContext) -> Option<(String, &'static str)> {
    let status = get_battery_status(ctx)?;
    let icon = match status.state {
        battery::State::Charging => "BatteryCharging",
        _ => "Battery",
    };
    Some((format!("{}%", status.percentage.round()), icon))
}

fn get_battery_status(ctx: &ChipContext) -> Option<BatteryStatus> {
    let battery_info = ctx.battery_info_provider.get_battery_info()?;
    if battery_info.energy_full == 0.0 {
        None
    } else {
        let battery = BatteryStatus {
            percentage: battery_info.energy / battery_info.energy_full * 100.0,
            state: battery_info.state,
        };
        log::trace!("Battery status: {battery:?}");
        Some(battery)
    }
}

/// the merge returns Charging if at least one is charging
///                   Discharging if at least one is Discharging
///                   Full if both are Full or one is Full and the other Unknown
///                   Empty if both are Empty or one is Empty and the other Unknown
///                   Unknown otherwise
fn merge_battery_states(state1: battery::State, state2: battery::State) -> battery::State {
    use battery::State::{Charging, Discharging, Unknown};
    if state1 == Charging || state2 == Charging {
        Charging
    } else if state1 == Discharging || state2 == Discharging {
        Discharging
    } else if state1 == state2 {
        state1
    } else if state1 == Unknown {
        state2
    } else if state2 == Unknown {
        state1
    } else {
        Unknown
    }
}

pub struct BatteryInfo {
    energy: f32,
    energy_full: f32,
    state: battery::State,
}

#[derive(Debug)]
struct BatteryStatus {
    percentage: f32,
    state: battery::State,
}

pub trait BatteryInfoProvider {
    fn get_battery_info(&self) -> Option<BatteryInfo>;
}

pub struct BatteryInfoProviderImpl;

impl BatteryInfoProvider for BatteryInfoProviderImpl {
    fn get_battery_info(&self) -> Option<BatteryInfo> {
        let battery_manager = battery::Manager::new().ok()?;
        let batteries = battery_manager.batteries().ok()?;
        Some(
            batteries
                .filter_map(|battery| match battery {
                    Ok(battery) => {
                        log::trace!("Battery found: {battery:?}");

                        let charge_rate = battery.state_of_charge().value;
                        let energy_full = battery.energy_full().value;
                        Some(BatteryInfo {
                            energy: charge_rate * energy_full,
                            energy_full,
                            state: battery.state(),
                        })
                    }
                    Err(e) => {
                        let level = if cfg!(target_os = "linux") {
                            log::Level::Info
                        } else {
                            log::Level::Warn
                        };
                        log::log!(level, "Unable to access battery information:\n{}", &e);
                        None
                    }
                })
                .fold(
                    BatteryInfo {
                        energy: 0.0,
                        energy_full: 0.0,
                        state: battery::State::Unknown,
                    },
                    |mut acc, x| {
                        acc.energy += x.energy;
                        acc.energy_full += x.energy_full;
                        acc.state = merge_battery_states(acc.state, x.state);
                        acc
                    },
                ),
        )
    }
}

