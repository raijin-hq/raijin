use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for operating system information (opt-in, disabled by default).
///
/// Shows the OS name with platform-specific details:
/// - **macOS**: `"macOS"` + version from `sw_vers -productVersion` (e.g., `"macOS 15.4"`)
/// - **Linux**: Reads `/etc/os-release` for `PRETTY_NAME` (e.g., `"Ubuntu 24.04 LTS"`)
/// - **Windows**: `"Windows"` (version detection not implemented)
/// - **Other**: `std::env::consts::OS` capitalized
pub struct OsInfoProvider;

impl ChipProvider for OsInfoProvider {
    fn id(&self) -> ChipId {
        "os"
    }

    fn display_name(&self) -> &str {
        "OS"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        false // opt-in only
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let (name, icon) = get_os_info(ctx);

        ChipOutput {
            id: self.id(),
            label: name.clone(),
            icon: Some(icon),
            tooltip: Some(format!("Operating system: {name}")),
            ..ChipOutput::default()
        }
    }
}

/// Detect OS name and appropriate icon.
fn get_os_info(ctx: &ChipContext) -> (String, &'static str) {
    #[cfg(target_os = "macos")]
    {
        let version = ctx
            .exec_cmd("sw_vers", &["-productVersion"])
            .map(|o| o.stdout.trim().to_string())
            .filter(|v| !v.is_empty());

        let name = match version {
            Some(v) => format!("macOS {v}"),
            None => "macOS".to_string(),
        };
        (name, "Apple")
    }

    #[cfg(target_os = "linux")]
    {
        let _ = ctx;
        let name = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .and_then(|l| {
                        let value = l.strip_prefix("PRETTY_NAME=")?.trim();
                        let unquoted = value
                            .strip_prefix('"')
                            .and_then(|v| v.strip_suffix('"'))
                            .unwrap_or(value);
                        Some(unquoted.to_string())
                    })
            })
            .unwrap_or_else(|| "Linux".to_string());
        (name, "Laptop")
    }

    #[cfg(target_os = "windows")]
    {
        let _ = ctx;
        ("Windows".to_string(), "Monitor")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = ctx;
        let os = std::env::consts::OS;
        let mut name = os.to_string();
        if let Some(first) = name.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        (name, "Monitor")
    }
}
