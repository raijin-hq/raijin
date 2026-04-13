use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for system memory usage (opt-in, disabled by default).
///
/// Platform support:
/// - **macOS**: Runs `vm_stat` to parse page statistics and `sysctl hw.memsize` for total memory.
///   Calculates used memory from active + wired + compressed pages.
/// - **Linux**: Reads `/proc/meminfo` for MemTotal and MemAvailable.
///
/// Display: `"3.2/16.0 GiB"` or `"45%"` depending on space.
pub struct MemoryUsageProvider;

impl ChipProvider for MemoryUsageProvider {
    fn id(&self) -> ChipId {
        "memory_usage"
    }

    fn display_name(&self) -> &str {
        "Memory Usage"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        false // opt-in only — user must enable in config
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let (label, tooltip) = get_memory_info(ctx).unwrap_or_else(|| {
            ("mem".into(), "System memory usage".into())
        });

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("MemoryStick"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Returns (label, tooltip) for memory usage.
fn get_memory_info(ctx: &ChipContext) -> Option<(String, String)> {
    #[cfg(target_os = "macos")]
    {
        get_memory_info_macos(ctx)
    }

    #[cfg(target_os = "linux")]
    {
        get_memory_info_linux(ctx)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = ctx;
        None
    }
}

/// Get memory info on macOS using `vm_stat` and `sysctl hw.memsize`.
#[cfg(target_os = "macos")]
fn get_memory_info_macos(ctx: &ChipContext) -> Option<(String, String)> {
    let vm_stat = ctx.exec_cmd("vm_stat", &[])?;
    let sysctl = ctx.exec_cmd("sysctl", &["-n", "hw.memsize"])?;

    let total_bytes: u64 = sysctl.stdout.trim().parse().ok()?;

    // Parse vm_stat page counts
    let page_size = parse_vm_stat_value(&vm_stat.stdout, "page size of ")?;
    let active = parse_vm_stat_value(&vm_stat.stdout, "Pages active:")?;
    let wired = parse_vm_stat_value(&vm_stat.stdout, "Pages wired down:")?;
    let compressed = parse_vm_stat_value(&vm_stat.stdout, "Pages occupied by compressor:")
        .unwrap_or(0);

    let used_bytes = (active + wired + compressed) * page_size;
    let total_gib = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_gib = used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let pct = (used_bytes as f64 / total_bytes as f64 * 100.0) as u32;

    let label = format!("{used_gib:.1}/{total_gib:.0} GiB");
    let tooltip = format!("Memory: {used_gib:.1} / {total_gib:.1} GiB ({pct}% used)");

    Some((label, tooltip))
}

/// Parse a numeric value from vm_stat output lines.
#[cfg(target_os = "macos")]
fn parse_vm_stat_value(output: &str, key: &str) -> Option<u64> {
    for line in output.lines() {
        if line.contains(key) {
            // Extract the number, stripping trailing period and whitespace
            let num_str: String = line
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            let num_str = num_str.trim_end_matches('.');
            return num_str.parse().ok();
        }
    }
    None
}

/// Get memory info on Linux by reading /proc/meminfo.
#[cfg(target_os = "linux")]
fn get_memory_info_linux(_ctx: &ChipContext) -> Option<(String, String)> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;

    let total_kb = parse_meminfo_value(&meminfo, "MemTotal:")?;
    let available_kb = parse_meminfo_value(&meminfo, "MemAvailable:")?;
    let used_kb = total_kb.saturating_sub(available_kb);

    let total_gib = total_kb as f64 / (1024.0 * 1024.0);
    let used_gib = used_kb as f64 / (1024.0 * 1024.0);
    let pct = (used_kb as f64 / total_kb as f64 * 100.0) as u32;

    let label = format!("{used_gib:.1}/{total_gib:.0} GiB");
    let tooltip = format!("Memory: {used_gib:.1} / {total_gib:.1} GiB ({pct}% used)");

    Some((label, tooltip))
}

/// Parse a kB value from /proc/meminfo.
#[cfg(target_os = "linux")]
fn parse_meminfo_value(meminfo: &str, key: &str) -> Option<u64> {
    for line in meminfo.lines() {
        if line.starts_with(key) {
            let value = line[key.len()..].trim().trim_end_matches(" kB").trim();
            return value.parse().ok();
        }
    }
    None
}
