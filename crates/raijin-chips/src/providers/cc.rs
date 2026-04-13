use semver::Version;

use crate::context::ChipContext;

/// Compiler detection result.
pub struct CompilerInfo {
    pub name: String,
    pub version: String,
}

/// Detect a C/C++ compiler by running commands in order and parsing output.
///
/// `commands`: List of (binary, args) to try in order (e.g., [("cc", "--version"), ("gcc", "--version")])
/// `compilers`: List of (display_name, output_hint) for name detection (e.g., [("clang", "clang"), ("gcc", "Free Software Foundation")])
pub fn detect_compiler(
    ctx: &ChipContext,
    commands: &[(&str, &[&str])],
    compilers: &[(&str, &str)],
) -> Option<CompilerInfo> {
    // Try each command until one succeeds
    let output = commands.iter().find_map(|(cmd, args)| ctx.exec_cmd(cmd, args))?;
    let stdout = &output.stdout;

    // Detect compiler name from output
    let name = compilers
        .iter()
        .find_map(|(compiler_name, compiler_hint)| {
            stdout.contains(compiler_hint).then_some(*compiler_name)
        })
        .unwrap_or("unknown")
        .to_string();

    // Extract version: find first semver-parseable word
    let version = stdout
        .split_whitespace()
        .find(|word| Version::parse(word).is_ok())
        .unwrap_or("")
        .to_string();

    Some(CompilerInfo { name, version })
}
