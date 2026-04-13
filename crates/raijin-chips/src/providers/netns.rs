use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Linux network namespace detection.
///
/// Linux only. Uses `ip netns identify` to detect the current network
/// namespace. Shows nothing on non-Linux or when in the default namespace.
pub struct NetnsProvider;

impl ChipProvider for NetnsProvider {
    fn id(&self) -> ChipId {
        "netns"
    }

    fn display_name(&self) -> &str {
        "Network Namespace"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        cfg!(target_os = "linux") && netns_name(ctx).is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let name = netns_name(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: name.clone(),
            icon: Some("Network"),
            tooltip: Some(format!("Network namespace: {name}")),
            ..ChipOutput::default()
        }
    }
}

fn netns_name(ctx: &ChipContext) -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    ctx.exec_cmd("ip", &["netns", "identify"])
        .map(|output| output.stdout.trim().to_string())
        .filter(|name| !name.is_empty())
}
