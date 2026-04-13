use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for active sudo credentials.
///
/// Checks if the user has cached sudo credentials by running `sudo -n true`.
/// The `-n` (non-interactive) flag prevents a password prompt; it succeeds
/// only when credentials are already cached from a recent `sudo` invocation.
///
/// Only shown when sudo is actively cached. Provides awareness that
/// elevated privileges are available without re-authentication.
pub struct SudoProvider;

impl ChipProvider for SudoProvider {
    fn id(&self) -> ChipId {
        "sudo"
    }

    fn display_name(&self) -> &str {
        "Sudo"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        // `sudo -n true` exits 0 only when credentials are cached.
        // exec_cmd returns None on non-zero exit, so .is_some() suffices.
        ctx.exec_cmd("sudo", &["-n", "true"]).is_some()
    }

    fn gather(&self, _ctx: &ChipContext) -> ChipOutput {
        ChipOutput {
            id: self.id(),
            label: "sudo".into(),
            icon: Some("Shield"),
            tooltip: Some("Sudo credentials are cached — elevated privileges available".into()),
            ..ChipOutput::default()
        }
    }
}
