use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Nix shell environment detection.
///
/// Detection: `IN_NIX_SHELL` or `NIX_SHELL_PACKAGES` env vars, or
///            heuristic PATH check for `/nix/store` entries.
/// Label:     "pure", "impure", or "nix" (unknown type)
pub struct NixShellProvider;

impl ChipProvider for NixShellProvider {
    fn id(&self) -> ChipId {
        "nix_shell"
    }

    fn display_name(&self) -> &str {
        "Nix Shell"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        detect_shell_type(ctx).is_some()
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let shell_type = detect_shell_type(ctx);
        let label = match shell_type {
            Some(NixShellType::Pure) => "pure".to_string(),
            Some(NixShellType::Impure) => "impure".to_string(),
            Some(NixShellType::Unknown) => "nix".to_string(),
            None => String::new(),
        };

        let shell_name = ctx.get_env("name").unwrap_or_default();
        let full_label = if shell_name.is_empty() {
            label
        } else {
            format!("{label} ({shell_name})")
        };

        ChipOutput {
            id: self.id(),
            label: full_label,
            icon: Some("Nix"),
            tooltip: Some("Nix shell environment".into()),
            ..ChipOutput::default()
        }
    }
}

enum NixShellType {
    Pure,
    Impure,
    Unknown,
}

fn detect_shell_type(ctx: &ChipContext) -> Option<NixShellType> {
    let shell_type = ctx.get_env("IN_NIX_SHELL");
    match shell_type.as_deref() {
        Some("pure") => return Some(NixShellType::Pure),
        Some("impure") => return Some(NixShellType::Impure),
        _ => {}
    }

    if ctx.has_env("NIX_SHELL_PACKAGES") {
        return Some(NixShellType::Unknown);
    }

    in_new_nix_shell(ctx).map(|()| NixShellType::Unknown)
}

fn in_new_nix_shell(ctx: &ChipContext) -> Option<()> {
    let path = ctx.get_env("PATH").or_else(|| std::env::var("PATH").ok())?;

    std::env::split_paths(&path)
        .any(|path| path.starts_with("/nix/store"))
        .then_some(())
}
