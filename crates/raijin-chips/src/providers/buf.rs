use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Buf (protobuf tooling) version.
///
/// Runs `buf --version` which outputs a plain version string like `1.28.1`.
///
/// Activates when `buf.yaml`, `buf.gen.yaml`, `buf.work.yaml`, or `.proto` files
/// are present.
pub struct BufProvider;

impl ChipProvider for BufProvider {
    fn id(&self) -> ChipId {
        "buf"
    }

    fn display_name(&self) -> &str {
        "Buf"
    }

    fn detect_files(&self) -> &[&str] {
        &["buf.yaml", "buf.gen.yaml", "buf.work.yaml"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["proto"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("buf", &["--version"])
            .map(|o| o.stdout.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Package"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Buf {version}"))
            },
            ..ChipOutput::default()
        }
    }
}
