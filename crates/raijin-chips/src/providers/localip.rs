use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the local IP address (opt-in, disabled by default).
///
/// Platform support:
/// - **macOS**: Uses `ipconfig getifaddr en0` (Wi-Fi) or `en1` fallback.
/// - **Linux**: Parses `hostname -I` for the first non-loopback IPv4 address.
///
/// This chip is opt-in because IP lookups can be slow on some network configurations.
pub struct LocalipProvider;

impl ChipProvider for LocalipProvider {
    fn id(&self) -> ChipId {
        "localip"
    }

    fn display_name(&self) -> &str {
        "Local IP"
    }

    fn is_available(&self, _ctx: &ChipContext) -> bool {
        false // opt-in only
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let ip = get_local_ip(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: ip.clone(),
            icon: Some("Network"),
            tooltip: if ip.is_empty() {
                Some("Local IP address (not detected)".into())
            } else {
                Some(format!("Local IP: {ip}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Detect the local IPv4 address.
fn get_local_ip(ctx: &ChipContext) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        // Try Wi-Fi (en0) first, then Ethernet (en1)
        for iface in &["en0", "en1", "en2"] {
            if let Some(output) = ctx.exec_cmd("ipconfig", &["getifaddr", iface]) {
                let ip = output.stdout.trim().to_string();
                if !ip.is_empty() {
                    return Some(ip);
                }
            }
        }
        None
    }

    #[cfg(target_os = "linux")]
    {
        ctx.exec_cmd("hostname", &["-I"]).and_then(|output| {
            output
                .stdout
                .split_whitespace()
                .find(|ip| ip.contains('.')) // first IPv4
                .map(|ip| ip.to_string())
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = ctx;
        None
    }
}
