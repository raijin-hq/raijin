use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Meson build system version.
///
/// Runs `meson --version` which outputs a plain version string like `1.3.1`.
///
/// Activates when `meson.build`, `meson_options.txt`, or `meson.options` is present.
pub struct MesonProvider;

impl ChipProvider for MesonProvider {
    fn id(&self) -> ChipId {
        "meson"
    }

    fn display_name(&self) -> &str {
        "Meson"
    }

    fn detect_files(&self) -> &[&str] {
        &["meson.build", "meson_options.txt", "meson.options"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("meson", &["--version"])
            .map(|o| o.stdout.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Settings"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Meson {version}"))
            },
            ..ChipOutput::default()
        }
    }
}
