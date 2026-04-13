use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the Mojo programming language version.
///
/// Detection: extensions `mojo`, `🔥`
/// Version:   `mojo --version` → "mojo 24.4.0 (2cb57382)" → "24.4.0"
pub struct MojoProvider;

impl ChipProvider for MojoProvider {
    fn id(&self) -> ChipId {
        "mojo"
    }

    fn display_name(&self) -> &str {
        "Mojo"
    }

    fn detect_extensions(&self) -> &[&str] {
        &["mojo", "\u{1f525}"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("mojo", &["--version"])
            .and_then(|o| get_mojo_version(&o.stdout))
            .map(|(v, _)| v)
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Mojo"),
            tooltip: Some("Mojo version".into()),
            ..ChipOutput::default()
        }
    }
}

fn get_mojo_version(output: &str) -> Option<(String, Option<String>)> {
    let version_items: Vec<&str> = output.split_ascii_whitespace().collect();

    let (version, hash) = match version_items[..] {
        [_, version] => (version.trim().to_string(), None),
        [_, version, hash, ..] => (version.trim().to_string(), Some(hash.trim().to_string())),
        _ => {
            log::debug!("Unexpected `mojo --version` output: {output}");
            return None;
        }
    };

    Some((version, hash))
}
