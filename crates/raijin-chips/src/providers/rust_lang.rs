use std::fs;
use std::path::Path;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider, parse_version_number};

/// Chip provider for Rust toolchain version.
///
/// Follows rustup-aware detection strategy to avoid triggering
/// toolchain downloads through the cargo proxy. Detection order:
///
/// 1. `$RUSTUP_TOOLCHAIN` environment variable
/// 2. `rust-toolchain` or `rust-toolchain.toml` in CWD or parent directories
/// 3. `~/.rustup/settings.toml` — override list, then default toolchain
/// 4. Direct rustc binary at `~/.rustup/toolchains/{toolchain}/bin/rustc`
/// 5. Fallback to `rustc --version` (may trigger proxy download)
pub struct RustProvider;

impl ChipProvider for RustProvider {
    fn id(&self) -> ChipId {
        "rust"
    }

    fn display_name(&self) -> &str {
        "Rust"
    }

    fn detect_files(&self) -> &[&str] {
        &["Cargo.toml", "rust-toolchain", "rust-toolchain.toml"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["rs"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = resolve_rust_version(ctx);

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Cog"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Rust {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Resolve the Rust version using a rustup-aware strategy.
///
/// Avoids triggering toolchain downloads by checking env vars and files
/// before falling back to `rustc --version` through the cargo proxy.
fn resolve_rust_version(ctx: &ChipContext) -> String {
    // Strategy 1: Check $RUSTUP_TOOLCHAIN env var
    if let Some(toolchain) = env_rustup_toolchain(ctx)
        && let Some(version) = version_from_toolchain(ctx, &toolchain)
    {
        return version;
    }

    // Strategy 2: Check rust-toolchain / rust-toolchain.toml files
    if let Some(toolchain) = find_toolchain_file(&ctx.cwd)
        && let Some(version) = version_from_toolchain(ctx, &toolchain)
    {
        return version;
    }

    // Strategy 3: Check ~/.rustup/settings.toml for overrides and default
    if let Some(toolchain) = rustup_settings_toolchain(ctx)
        && let Some(version) = version_from_toolchain(ctx, &toolchain)
    {
        return version;
    }

    // Strategy 4: Fall back to `rustc --version` (may trigger proxy download)
    if let Some(output) = ctx.exec_cmd("rustc", &["--version"]) {
        return format_rustc_version(&output.stdout);
    }

    String::new()
}

/// Read `$RUSTUP_TOOLCHAIN` from the environment.
fn env_rustup_toolchain(ctx: &ChipContext) -> Option<String> {
    // First check our captured env snapshot
    ctx.get_env("RUSTUP_TOOLCHAIN")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        // Fall back to direct env check (RUSTUP_TOOLCHAIN may not be in our snapshot)
        .or_else(|| {
            std::env::var("RUSTUP_TOOLCHAIN")
                .ok()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
        })
}

/// Given a toolchain name (e.g., "stable", "nightly", "1.77.0"), resolve the
/// Rust version by running the toolchain-specific rustc binary directly.
///
/// This avoids the cargo proxy and prevents toolchain downloads.
fn version_from_toolchain(ctx: &ChipContext, toolchain: &str) -> Option<String> {
    // Try direct binary at ~/.rustup/toolchains/{toolchain}/bin/rustc
    if let Some(rustup_home) = rustup_home_dir() {
        let rustc_path = rustup_home
            .join("toolchains")
            .join(toolchain)
            .join("bin")
            .join("rustc");

        if rustc_path.exists() {
            let rustc_str = rustc_path.to_string_lossy().to_string();
            if let Some(output) = ctx.exec_cmd(&rustc_str, &["--version"]) {
                return Some(format_rustc_version(&output.stdout));
            }
        }

        // The toolchain name might be a short alias like "stable" or "nightly".
        // Rustup stores toolchains with the full triple, e.g., "stable-x86_64-apple-darwin".
        // Scan for a matching directory prefix.
        let toolchains_dir = rustup_home.join("toolchains");
        if let Ok(entries) = fs::read_dir(&toolchains_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(toolchain)
                    && name[toolchain.len()..].starts_with('-')
                    && entry.file_type().is_ok_and(|ft| ft.is_dir())
                {
                    let rustc_path = toolchains_dir.join(&name).join("bin").join("rustc");
                    if rustc_path.exists() {
                        let rustc_str = rustc_path.to_string_lossy().to_string();
                        if let Some(output) = ctx.exec_cmd(&rustc_str, &["--version"]) {
                            return Some(format_rustc_version(&output.stdout));
                        }
                    }
                    break;
                }
            }
        }
    }

    // If the toolchain string itself looks like a version, return it directly.
    // Handles cases like channel = "1.77.0" in rust-toolchain.toml.
    if toolchain
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
    {
        return Some(toolchain.to_string());
    }

    None
}

/// Find `rust-toolchain` or `rust-toolchain.toml` in CWD or parent directories.
///
/// Follows rustup's override precedence:
/// - `rust-toolchain` can be a plain one-line toolchain name OR TOML format
/// - `rust-toolchain.toml` must be TOML format with `[toolchain] channel = "..."`
///
/// Reference: https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file
fn find_toolchain_file(start: &Path) -> Option<String> {
    let mut dir = start;
    loop {
        // Check `rust-toolchain` (plain text or TOML)
        let plain_path = dir.join("rust-toolchain");
        if plain_path.is_file()
            && let Some(channel) = read_toolchain_channel(&plain_path, false)
        {
            return Some(channel);
        }

        // Check `rust-toolchain.toml` (TOML only)
        let toml_path = dir.join("rust-toolchain.toml");
        if toml_path.is_file()
            && let Some(channel) = read_toolchain_channel(&toml_path, true)
        {
            return Some(channel);
        }

        dir = match dir.parent() {
            Some(parent) => parent,
            None => break,
        };
    }
    None
}

/// Parse the channel from a rust-toolchain file.
///
/// `only_toml`: if true, only parse TOML format (for `.toml` extension).
/// If false, a single-line file is treated as a plain toolchain name.
fn read_toolchain_channel(path: &Path, only_toml: bool) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();

    if trimmed.is_empty() {
        return None;
    }

    // Single non-empty line without TOML markers → plain toolchain name
    // Only valid for `rust-toolchain`, not `rust-toolchain.toml`
    let line_count = contents.lines().count();
    if line_count == 1 && !only_toml && !trimmed.contains('[') {
        return Some(trimmed.to_owned());
    }

    // Parse as TOML: [toolchain] channel = "..."
    parse_toolchain_toml(&contents)
}

/// Parse `[toolchain] channel` from TOML content.
fn parse_toolchain_toml(contents: &str) -> Option<String> {
    // Minimal TOML parsing — avoids pulling in a full TOML crate dependency.
    // The format is well-defined: [toolchain]\nchannel = "value"
    let mut in_toolchain_section = false;

    for line in contents.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_toolchain_section = line == "[toolchain]";
            continue;
        }

        if in_toolchain_section && line.starts_with("channel") {
            // Parse: channel = "value" or channel = 'value'
            if let Some((_key, value)) = line.split_once('=') {
                let value = value.trim();
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .or_else(|| {
                        value.strip_prefix('\'').and_then(|v| v.strip_suffix('\''))
                    });

                if let Some(channel) = value {
                    let channel = channel.trim();
                    if !channel.is_empty() {
                        return Some(channel.to_owned());
                    }
                }
            }
        }
    }

    None
}

/// Check `~/.rustup/settings.toml` for override and default toolchain.
///
/// Checks two things in order:
/// 1. Override for the current CWD (most specific path match)
/// 2. Default toolchain setting
fn rustup_settings_toolchain(ctx: &ChipContext) -> Option<String> {
    let rustup_home = rustup_home_dir()?;
    let settings_path = rustup_home.join("settings.toml");
    let contents = fs::read_to_string(settings_path).ok()?;

    // Check for path-specific override first
    if let Some(toolchain) = find_settings_override(&contents, &ctx.cwd) {
        return Some(toolchain);
    }

    // Fall back to default_toolchain
    find_settings_default_toolchain(&contents)
}

/// Find a CWD-specific toolchain override in rustup settings.toml.
///
/// The `[overrides]` section maps absolute paths to toolchain names.
/// We find the most specific (longest) matching path prefix.
fn find_settings_override(contents: &str, cwd: &Path) -> Option<String> {
    let mut in_overrides = false;
    let mut best_match: Option<(usize, String)> = None;
    let cwd_str = cwd.to_string_lossy();

    for line in contents.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_overrides = line == "[overrides]";
            continue;
        }

        if !in_overrides {
            continue;
        }

        // Parse: "/path/to/dir" = "toolchain-name"
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            let path = key
                .strip_prefix('"')
                .and_then(|k| k.strip_suffix('"'))
                .unwrap_or(key);

            let toolchain = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(value);

            if cwd_str.starts_with(path) {
                let path_len = path.len();
                if best_match
                    .as_ref()
                    .is_none_or(|(len, _)| path_len > *len)
                {
                    best_match = Some((path_len, toolchain.to_owned()));
                }
            }
        }
    }

    best_match.map(|(_, toolchain)| toolchain)
}

/// Extract `default_toolchain` from rustup settings.toml.
fn find_settings_default_toolchain(contents: &str) -> Option<String> {
    // Avoid matching inside [overrides] or other sections.
    // default_toolchain is a top-level key.
    let mut in_top_level = true;

    for line in contents.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_top_level = false;
            continue;
        }

        if in_top_level && line.starts_with("default_toolchain")
            && let Some((_key, value)) = line.split_once('=')
        {
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| {
                    value.strip_prefix('\'').and_then(|v| v.strip_suffix('\''))
                })
                .unwrap_or(value);

            let value = value.trim();
            if !value.is_empty() && value != "none" {
                return Some(value.to_owned());
            }
        }
    }

    None
}

/// Get the rustup home directory.
///
/// Checks `$RUSTUP_HOME`, falls back to `~/.rustup`.
fn rustup_home_dir() -> Option<std::path::PathBuf> {
    if let Ok(rustup_home) = std::env::var("RUSTUP_HOME") {
        let path = std::path::PathBuf::from(rustup_home);
        if path.is_dir() {
            return Some(path);
        }
    }

    std::env::var("HOME")
        .ok()
        .map(|home| std::path::PathBuf::from(home).join(".rustup"))
        .filter(|p: &std::path::PathBuf| p.is_dir())
}

/// Format `rustc --version` output into a clean version string.
///
/// Handles various formats:
/// - `"rustc 1.77.0 (aedd173a2 2024-03-17)"` → `"1.77.0"`
/// - `"rustc 1.78.0-nightly (b139669f3 2024-03-10)"` → `"1.78.0-nightly"`
/// - `"rustc 1.77.0-beta.3 (..."` → `"1.77.0-beta.3"`
/// - `"rustc 1.77.0\n"` → `"1.77.0"`
fn format_rustc_version(output: &str) -> String {
    let output = output.trim();

    // Split "rustc 1.77.0 (hash date)" and take the version token
    output
        .split_whitespace()
        .nth(if output.starts_with("rustc") { 1 } else { 0 })
        .map(|version| {
            // Strip trailing parenthesized hash if somehow attached
            version
                .find('(')
                .map_or(version, |i| &version[..i])
                .trim()
                .to_string()
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| parse_version_number(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_rustc_version_stable() {
        assert_eq!(
            format_rustc_version("rustc 1.77.0 (aedd173a2 2024-03-17)"),
            "1.77.0"
        );
    }

    #[test]
    fn test_format_rustc_version_nightly() {
        assert_eq!(
            format_rustc_version("rustc 1.78.0-nightly (b139669f3 2024-03-10)"),
            "1.78.0-nightly"
        );
    }

    #[test]
    fn test_format_rustc_version_beta() {
        assert_eq!(
            format_rustc_version("rustc 1.77.0-beta.3 (2bc1d406d 2024-03-10)"),
            "1.77.0-beta.3"
        );
    }

    #[test]
    fn test_format_rustc_version_bare() {
        assert_eq!(format_rustc_version("rustc 1.77.0\n"), "1.77.0");
    }

    #[test]
    fn test_format_rustc_version_just_version() {
        assert_eq!(format_rustc_version("1.77.0"), "1.77.0");
    }

    #[test]
    fn test_format_rustc_version_empty() {
        assert_eq!(format_rustc_version(""), "");
    }

    #[test]
    fn test_parse_toolchain_toml_basic() {
        let toml = r#"
[toolchain]
channel = "1.77.0"
"#;
        assert_eq!(parse_toolchain_toml(toml), Some("1.77.0".to_owned()));
    }

    #[test]
    fn test_parse_toolchain_toml_nightly() {
        let toml = r#"
[toolchain]
channel = "nightly-2024-03-10"
components = ["rustfmt", "clippy"]
"#;
        assert_eq!(
            parse_toolchain_toml(toml),
            Some("nightly-2024-03-10".to_owned())
        );
    }

    #[test]
    fn test_parse_toolchain_toml_with_other_sections() {
        let toml = r#"
[other]
channel = "should-not-match"

[toolchain]
channel = "stable"

[another]
channel = "also-not"
"#;
        assert_eq!(parse_toolchain_toml(toml), Some("stable".to_owned()));
    }

    #[test]
    fn test_parse_toolchain_toml_no_channel() {
        let toml = r#"
[toolchain]
components = ["rustfmt"]
"#;
        assert_eq!(parse_toolchain_toml(toml), None);
    }

    #[test]
    fn test_parse_toolchain_toml_empty() {
        assert_eq!(parse_toolchain_toml(""), None);
    }

    #[test]
    fn test_read_toolchain_channel_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rust-toolchain");
        fs::write(&path, "1.77.0").unwrap();
        assert_eq!(
            read_toolchain_channel(&path, false),
            Some("1.77.0".to_owned())
        );
    }

    #[test]
    fn test_read_toolchain_channel_toml_only_rejects_plain() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rust-toolchain.toml");
        fs::write(&path, "1.77.0").unwrap();
        // Single-line plain text is rejected when only_toml=true
        assert_eq!(read_toolchain_channel(&path, true), None);
    }

    #[test]
    fn test_read_toolchain_channel_toml_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rust-toolchain.toml");
        fs::write(&path, "[toolchain]\nchannel = \"nightly\"").unwrap();
        assert_eq!(
            read_toolchain_channel(&path, true),
            Some("nightly".to_owned())
        );
    }

    #[test]
    fn test_find_settings_default_toolchain() {
        let settings = r#"
default_host_triple = "x86_64-apple-darwin"
default_toolchain = "stable"
version = "12"

[overrides]
"/some/path" = "nightly"
"#;
        assert_eq!(
            find_settings_default_toolchain(settings),
            Some("stable".to_owned())
        );
    }

    #[test]
    fn test_find_settings_default_toolchain_none() {
        let settings = r#"
default_host_triple = "x86_64-apple-darwin"
default_toolchain = "none"
version = "12"
"#;
        assert_eq!(find_settings_default_toolchain(settings), None);
    }

    #[test]
    fn test_find_settings_override() {
        let settings = r#"
default_toolchain = "stable"
version = "12"

[overrides]
"/home/user/project-a" = "nightly"
"/home/user/project-b" = "beta"
"/home/user/project-b/sub" = "1.70.0"
"#;
        // Exact match
        assert_eq!(
            find_settings_override(settings, Path::new("/home/user/project-a")),
            Some("nightly".to_owned())
        );
        // Subdirectory matches parent
        assert_eq!(
            find_settings_override(settings, Path::new("/home/user/project-a/src")),
            Some("nightly".to_owned())
        );
        // More specific override wins
        assert_eq!(
            find_settings_override(settings, Path::new("/home/user/project-b/sub/deep")),
            Some("1.70.0".to_owned())
        );
        // No match
        assert_eq!(
            find_settings_override(settings, Path::new("/home/other")),
            None
        );
    }

    #[test]
    fn test_find_toolchain_file_traverses_parents() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("child");
        fs::create_dir(&child).unwrap();
        fs::write(
            dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.80.0\"",
        )
        .unwrap();

        assert_eq!(
            find_toolchain_file(&child),
            Some("1.80.0".to_owned())
        );
    }
}
