use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct CondaProvider;

impl ChipProvider for CondaProvider {
    fn id(&self) -> ChipId { "conda" }
    fn display_name(&self) -> &str { "Conda" }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.get_env("CONDA_DEFAULT_ENV")
            .map(|v| !v.trim().is_empty() && v != "base")
            .unwrap_or(false)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let env_name = ctx.get_env("CONDA_DEFAULT_ENV").unwrap_or_default();

        // Extract just the directory name if it's a full path
        let display_name = std::path::Path::new(&env_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&env_name)
            .to_string();

        ChipOutput {
            id: self.id(),
            label: display_name,
            icon: Some("Conda"),
            tooltip: Some(format!("Conda environment: {env_name}")),
            ..ChipOutput::default()
        }
    }
}
