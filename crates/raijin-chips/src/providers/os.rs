use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for operating system identification.
///
/// Detection: Always available.
/// Label:     OS name from `std::env::consts::OS` (macOS, Linux, Windows, etc.)
pub struct OsProvider;

impl ChipProvider for OsProvider {
    fn id(&self) -> ChipId {
        "os"
    }

    fn display_name(&self) -> &str {
        "Operating System"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        true
    }

    fn gather(&self, _ctx: &ChipContext) -> ChipOutput {
        let os_name = get_os_name();

        ChipOutput {
            id: self.id(),
            label: os_name.clone(),
            icon: Some("Monitor"),
            tooltip: Some(format!("OS: {os_name}")),
            ..ChipOutput::default()
        }
    }
}

fn get_os_name() -> String {
    match std::env::consts::OS {
        "macos" => "macOS".to_string(),
        "linux" => "Linux".to_string(),
        "windows" => "Windows".to_string(),
        "freebsd" => "FreeBSD".to_string(),
        "openbsd" => "OpenBSD".to_string(),
        "netbsd" => "NetBSD".to_string(),
        "dragonfly" => "DragonFly".to_string(),
        "ios" => "iOS".to_string(),
        "android" => "Android".to_string(),
        other => other.to_string(),
    }
}
