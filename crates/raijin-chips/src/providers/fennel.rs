use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct FennelProvider;

impl ChipProvider for FennelProvider {
    fn id(&self) -> ChipId {
        "fennel"
    }

    fn display_name(&self) -> &str {
        "Fennel"
    }

    fn detect_extensions(&self) -> &[&str] {
        &["fnl"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("fennel", &["--version"])
            .and_then(|o| {
                let combined = format!("{}{}", o.stdout, o.stderr);
                parse_fennel_version(&combined)
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Fennel"),
            tooltip: Some("Fennel version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse fennel version from output like "Fennel 1.2.1 on PUC Lua 5.4"
fn parse_fennel_version(fennel_version: &str) -> Option<String> {
    let version = fennel_version
        .split_whitespace()
        .nth(1)?;

    Some(version.to_string())
}
