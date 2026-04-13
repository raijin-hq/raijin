use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct GuixShellProvider;

impl ChipProvider for GuixShellProvider {
    fn id(&self) -> ChipId {
        "guix_shell"
    }

    fn display_name(&self) -> &str {
        "Guix Shell"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("GUIX_ENVIRONMENT")
    }

    fn gather(&self, _ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: "guix".to_string(),
            icon: Some("Code"),
            tooltip: Some("Inside a Guix shell environment".into()),
            ..ChipOutput::default()
        }
    }
}
