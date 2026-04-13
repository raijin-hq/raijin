use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Vagrant presence.
///
/// Detects `Vagrantfile` in the current directory.
/// Shows a simple "vagrant" label as an indicator.
pub struct VagrantProvider;

impl ChipProvider for VagrantProvider {
    fn id(&self) -> ChipId {
        "vagrant"
    }

    fn display_name(&self) -> &str {
        "Vagrant"
    }

    fn detect_files(&self) -> &[&str] {
        &["Vagrantfile"]
    }

    fn gather(&self, _ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: "vagrant".into(),
            icon: Some("Box"),
            tooltip: Some("Vagrantfile detected".into()),
            ..ChipOutput::default()
        }
    }
}
