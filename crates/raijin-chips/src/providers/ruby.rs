use std::path::Path;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Ruby runtime version.
///
/// Detection priority (mirrors Ruby module):
/// 1. `$RBENV_VERSION` environment variable (rbenv override)
/// 2. `.ruby-version` file in CWD (rbenv/rvm/asdf local version)
/// 3. `ruby --version` command output
///
/// Also activates when `$RUBY_VERSION` or `$RBENV_VERSION` env vars are set,
/// even without Ruby project files in the directory.
pub struct RubyProvider;

impl ChipProvider for RubyProvider {
    fn id(&self) -> ChipId {
        "ruby"
    }

    fn display_name(&self) -> &str {
        "Ruby"
    }

    fn detect_files(&self) -> &[&str] {
        &["Gemfile", ".ruby-version", "Rakefile"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["rb", "gemspec"]
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        // Active rbenv or ruby version env means Ruby is relevant
        if ctx.has_env("RBENV_VERSION") || ctx.has_env("RUBY_VERSION") {
            return true;
        }

        let files = self.detect_files();
        let folders = self.detect_folders();
        let extensions = self.detect_extensions();
        ctx.dir_contents.matches(files, folders, extensions)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = get_ruby_version(ctx);

        let label = version.clone().unwrap_or_default();

        let tooltip = match &version {
            Some(ver) => format!("Ruby {ver}"),
            None => "Ruby".into(),
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Gem"),
            tooltip: Some(tooltip),
            ..ChipOutput::default()
        }
    }
}

/// Resolve the Ruby version string.
///
/// Priority order :
/// 1. `$RBENV_VERSION` — if set, the user pinned a version via rbenv
/// 2. `.ruby-version` file — rbenv/rvm/asdf local version file
/// 3. `ruby --version` — actual installed Ruby version
fn get_ruby_version(ctx: &ChipContext) -> Option<String> {
    // 1. rbenv environment variable
    if let Some(rbenv_ver) = ctx.get_env("RBENV_VERSION") {
        let ver = rbenv_ver.trim();
        if !ver.is_empty() {
            return Some(ver.to_string());
        }
    }

    // 2. .ruby-version file (rbenv/rvm/asdf local config)
    if let Some(ver) = read_ruby_version_file(&ctx.cwd) {
        return Some(ver);
    }

    // 3. ruby --version
    exec_ruby_version(ctx)
}

/// Execute `ruby --version` and parse the version.
///
/// Output format: "ruby 3.3.0 (2023-12-25 revision 5124f9ac75) [arm64-darwin23]"
/// Also handles: "ruby 2.6.0p0 (2018-12-25 revision 66547) [x86_64-linux]"
fn exec_ruby_version(ctx: &ChipContext) -> Option<String> {
    let output = ctx.exec_cmd("ruby", &["--version"])?;
    parse_ruby_version_output(&output.stdout)
}

/// Parse version from `ruby --version` output.
///
/// Handles formats:
/// - "ruby 3.3.0 (2023-12-25 ...)" → "3.3.0"
/// - "ruby 2.6.0p0 (2018-12-25 ...)" → "2.6.0" (strips patch level)
/// - "ruby 2.1.10p492 (2016-04-01 ...)" → "2.1.10"
fn parse_ruby_version_output(output: &str) -> Option<String> {
    let version_token = output.split_whitespace().nth(1)?;
    // Strip patch level suffix: "2.6.0p0" → "2.6.0", "3.3.0" stays "3.3.0"
    let version = version_token.split('p').next()?;
    if version.is_empty() {
        return None;
    }
    Some(version.to_string())
}

/// Read `.ruby-version` file from CWD.
///
/// Contains one version per line; we take the first non-empty, non-comment line.
fn read_ruby_version_file(cwd: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cwd.join(".ruby-version")).ok()?;
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
    fn parse_ruby_3() {
        assert_eq!(
            parse_ruby_version_output("ruby 3.3.0 (2023-12-25 revision 5124f9ac75) [arm64-darwin23]"),
            Some("3.3.0".to_string()),
        );
    }

    #[test]
    fn parse_ruby_2_with_patch() {
        assert_eq!(
            parse_ruby_version_output("ruby 2.5.1p57 (2018-03-29 revision 63029) [x86_64-linux-gnu]"),
            Some("2.5.1".to_string()),
        );
    }

    #[test]
    fn parse_ruby_2_1_with_patch() {
        assert_eq!(
            parse_ruby_version_output("ruby 2.1.10p492 (2016-04-01 revision 54464) [x86_64-darwin19.0]"),
            Some("2.1.10".to_string()),
        );
    }

    #[test]
    fn parse_ruby_2_7() {
        assert_eq!(
            parse_ruby_version_output("ruby 2.7.0p0 (2019-12-25 revision 647ee6f091) [x86_64-linux-musl]"),
            Some("2.7.0".to_string()),
        );
    }

    #[test]
    fn parse_empty_output() {
        assert_eq!(parse_ruby_version_output(""), None);
    }

    #[test]
    fn parse_malformed_output() {
        assert_eq!(parse_ruby_version_output("ruby"), None);
    }
}
