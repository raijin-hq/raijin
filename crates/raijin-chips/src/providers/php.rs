use std::path::Path;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for PHP runtime version.
///
/// Detection (mirrors standard PHP module):
/// - Files: `composer.json`, `.php-version`
/// - Extensions: `.php`
///
/// Uses `php -r 'echo PHP_MAJOR_VERSION."."...;'` for precise version
/// without the noise of `php --version` output. Falls back to parsing
/// `php --version` if the inline eval fails.
pub struct PhpProvider;

impl ChipProvider for PhpProvider {
    fn id(&self) -> ChipId {
        "php"
    }

    fn display_name(&self) -> &str {
        "PHP"
    }

    fn detect_files(&self) -> &[&str] {
        &["composer.json", ".php-version"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["php"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_php_version(ctx);

        let label = version.clone().unwrap_or_default();

        let tooltip = match &version {
            Some(ver) => format!("PHP {ver}"),
            None => "PHP".into(),
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Php"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Resolve the PHP version string.
///
/// Priority order:
/// 1. `.php-version` file (phpenv local config)
/// 2. `php -r` inline version constants (precise, no extra output)
/// 3. `php --version` output parsing (fallback)
fn get_php_version(ctx: &ChipContext) -> Option<String> {
    // 1. .php-version file (phpenv local config)
    if let Some(ver) = read_php_version_file(&ctx.cwd) {
        return Some(ver);
    }

    // 2. php -r with version constants (cleanest output, standard pattern)
    if let Some(output) = ctx.exec_cmd(
        "php",
        &["-nr", "echo PHP_MAJOR_VERSION.\".\".PHP_MINOR_VERSION.\".\".PHP_RELEASE_VERSION;"],
    ) {
        let ver = output.stdout.trim().to_string();
        if !ver.is_empty() && ver.contains('.') {
            return Some(ver);
        }
    }

    // 3. Fall back to php --version output parsing
    if let Some(output) = ctx.exec_cmd("php", &["--version"]) {
        return parse_php_version_output(&output.stdout);
    }

    None
}

/// Parse version from `php --version` output.
///
/// Output format: "PHP 8.3.0 (cli) (built: Nov 21 2023 18:40:40) (NTS)"
/// Extracts "8.3.0" from the first line.
fn parse_php_version_output(output: &str) -> Option<String> {
    let first_line = output.lines().next()?;
    // "PHP 8.3.0 (cli) ..." → split → find version token
    for token in first_line.split_whitespace() {
        if token.starts_with(|c: char| c.is_ascii_digit()) && token.contains('.') {
            return Some(token.to_string());
        }
    }
    None
}

/// Read `.php-version` file from CWD (phpenv local config).
fn read_php_version_file(cwd: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join(".php-version")).ok()?;
    let line = content
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#'))?;
    if line.is_empty() {
        return None;
    }
    Some(line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_php_83() {
        assert_eq!(
            parse_php_version_output("PHP 8.3.0 (cli) (built: Nov 21 2023 18:40:40) (NTS)"),
            Some("8.3.0".to_string()),
        );
    }

    #[test]
    fn parse_php_74() {
        assert_eq!(
            parse_php_version_output("PHP 7.4.33 (cli) (built: Sep  2 2023 08:03:46) ( NTS )"),
            Some("7.4.33".to_string()),
        );
    }

    #[test]
    fn parse_php_81_with_zts() {
        assert_eq!(
            parse_php_version_output("PHP 8.1.27 (cli) (built: Dec 19 2023 20:35:55) (ZTS)"),
            Some("8.1.27".to_string()),
        );
    }

    #[test]
    fn parse_empty_output() {
        assert_eq!(parse_php_version_output(""), None);
    }

    #[test]
    fn parse_malformed_output() {
        assert_eq!(parse_php_version_output("Some random text"), None);
    }
}
