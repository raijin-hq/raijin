use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active Spack environment.
///
/// Availability: `$SPACK_ENV` environment variable is set.
/// Label: environment name (last path component if it's a path).
pub struct SpackProvider;

impl ChipProvider for SpackProvider {
    fn id(&self) -> ChipId {
        "spack"
    }

    fn display_name(&self) -> &str {
        "Spack"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("SPACK_ENV")
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let raw_env = ctx.get_env("SPACK_ENV").unwrap_or_default();
        let name = truncate_path(&raw_env);

        ChipOutput {
            id: self.id(),
            label: name.clone(),
            icon: Some("Package"),
            tooltip: if name.is_empty() {
                None
            } else {
                Some(format!("Spack environment: {name}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Truncate a potentially long path to just the last component.
///
/// "/some/really/long/path/my_env" → "my_env"
/// "my_env" → "my_env"
fn truncate_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}
