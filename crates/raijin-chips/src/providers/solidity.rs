use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Solidity compiler version.
///
/// Detection: `sol` extension.
/// Version:   `solc --version` — parses version from multi-line output.
///
/// Supports both `solc` (native) and `solcjs` (JavaScript) compilers.
/// `solc --version` outputs "Version: 0.8.16+commit.07a7930e.Linux.g++"
/// `solcjs --version` outputs "0.8.15+commit.e14f2714.Emscripten.clang"
pub struct SolidityProvider;

impl ChipProvider for SolidityProvider {
    fn id(&self) -> ChipId {
        "solidity"
    }

    fn display_name(&self) -> &str {
        "Solidity"
    }

    fn detect_extensions(&self) -> &[&str] {
        &["sol"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_solidity_version(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Solidity"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Solidity {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Try `solc --version` first, fall back to `solcjs --version`.
fn get_solidity_version(ctx: &ChipContext) -> Option<String> {
    let compilers = ["solc", "solcjs"];
    for compiler in &compilers {
        if let Some(output) = ctx.exec_cmd(compiler, &["--version"]) {
            let combined = format!("{}\n{}", output.stdout, output.stderr);
            if let Some(version) = parse_solidity_version(&combined) {
                return Some(version);
            }
        }
    }
    None
}

/// Parse solidity compiler version output.
///
/// `solc --version` multi-line output contains a line like:
///   "Version: 0.8.16+commit.07a7930e.Linux.g++"
/// `solcjs --version` single-line output:
///   "0.8.15+commit.e14f2714.Emscripten.clang"
///
/// We look for a token containing '+' and extract the part before it,
/// or fall back to the 8th whitespace-separated token (solc format).
fn parse_solidity_version(version: &str) -> Option<String> {
    // Try solc format first: split all whitespace tokens, 8th token is version
    let version_var = match version.split_whitespace().nth(7) {
        Some(c) => c.split_terminator('+').next()?,
        None => version.trim().split_terminator('+').next()?,
    };

    if version_var.is_empty() {
        return None;
    }

    Some(version_var.to_string())
}
