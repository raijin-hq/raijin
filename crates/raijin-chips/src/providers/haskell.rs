use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Haskell (GHC) version.
///
/// Detection: `stack.yaml`, `cabal.project`, `.hs`, `.cabal` files.
/// Version resolution order:
///   1. `$GHC_VERSION` environment variable
///   2. `stack ghc -- --numeric-version` (for Stack projects)
///   3. `ghc --numeric-version` (direct GHC)
///
/// For Stack projects, also reads `stack.yaml` for the resolver/snapshot.
pub struct HaskellProvider;

impl ChipProvider for HaskellProvider {
    fn id(&self) -> ChipId {
        "haskell"
    }

    fn display_name(&self) -> &str {
        "Haskell"
    }

    fn detect_files(&self) -> &[&str] {
        &["stack.yaml", "cabal.project"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["hs", "cabal"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let is_stack = ctx.dir_contents.has_file("stack.yaml");
        let snapshot = if is_stack {
            read_stack_snapshot(&ctx.cwd)
        } else {
            None
        };

        // Get GHC version: env var -> stack -> direct
        let ghc_version = get_ghc_version(ctx, is_stack);

        // Primary label: snapshot if Stack project, otherwise GHC version
        let label = snapshot
            .clone()
            .or_else(|| ghc_version.clone())
            .unwrap_or_default();

        let tooltip = match (&ghc_version, &snapshot) {
            (Some(ghc), Some(snap)) => Some(format!("GHC {ghc} ({snap})")),
            (Some(ghc), None) => Some(format!("GHC {ghc}")),
            (None, Some(snap)) => Some(format!("Stack {snap}")),
            _ => None,
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Haskell"),
            tooltip,
            ..ChipOutput::default()
        }
    }
}

/// Get GHC version from environment, Stack, or direct invocation.
fn get_ghc_version(ctx: &ChipContext, is_stack: bool) -> Option<String> {
    // 1. Environment variable
    if let Some(env_ver) = ctx.get_env("GHC_VERSION") {
        let v = env_ver.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    // 2. Stack-managed GHC
    if is_stack
        && let Some(output) = ctx.exec_cmd("stack", &["ghc", "--", "--numeric-version"])
    {
        let v = output.stdout.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    // 3. Direct GHC
    ctx.exec_cmd("ghc", &["--numeric-version"])
        .map(|o| o.stdout.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Read the resolver/snapshot from `stack.yaml`.
///
/// Looks for `resolver:` or `snapshot:` and returns values like
/// `lts-22.0`, `nightly-2024-01-01`, `ghc-9.6.3`.
fn read_stack_snapshot(cwd: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join("stack.yaml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        let value = if let Some(rest) = trimmed.strip_prefix("resolver:") {
            rest.trim()
        } else if let Some(rest) = trimmed.strip_prefix("snapshot:") {
            rest.trim()
        } else {
            continue;
        };

        // Skip URLs and complex values
        if value.is_empty() || value.starts_with("http") || value.starts_with('{') {
            continue;
        }

        // Accept lts-*, nightly-*, ghc-* patterns
        if value.starts_with("lts")
            || value.starts_with("nightly")
            || value.starts_with("ghc")
        {
            return Some(value.to_string());
        }
    }
    None
}
