use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Perl runtime version.
///
/// Detection: `Makefile.PL`, `Build.PL`, `cpanfile`, `cpanfile.snapshot`,
///   `META.json`, `META.yml`, `.perl-version`, `.pl`, `.pm`, `.pod` files.
/// Version: Uses `perl -e 'printf q#%vd#,$^V;'` which outputs
///   the version directly like `5.38.0`. Falls back to parsing `perl --version`.
///

pub struct PerlProvider;

impl ChipProvider for PerlProvider {
    fn id(&self) -> ChipId {
        "perl"
    }

    fn display_name(&self) -> &str {
        "Perl"
    }

    fn detect_files(&self) -> &[&str] {
        &[
            "Makefile.PL",
            "Build.PL",
            "cpanfile",
            "cpanfile.snapshot",
            "META.json",
            "META.yml",
            ".perl-version",
        ]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["pl", "pm", "pod"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        // standard pattern: use printf to get clean version output
        let version = ctx
            .exec_cmd("perl", &["-e", "printf q#%vd#,$^V;"])
            .map(|o| o.stdout.trim().to_string())
            .filter(|v| !v.is_empty() && v.chars().next().map_or(false, |c| c.is_ascii_digit()))
            .or_else(|| {
                // Fallback: parse `perl --version` output
                ctx.exec_cmd("perl", &["--version"])
                    .and_then(|o| parse_perl_version(&o.stdout))
            })
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version.clone(),
            icon: Some("Perl"),
            tooltip: if version.is_empty() {
                None
            } else {
                Some(format!("Perl {version}"))
            },
            ..ChipOutput::default()
        }
    }
}

/// Parse Perl version from `perl --version` output.
///
/// Input: `This is perl 5, version 38, subversion 0 (v5.38.0) built for ...`
/// Output: `Some("5.38.0")`
///
/// Looks for the `(vX.Y.Z)` pattern in the output.
fn parse_perl_version(output: &str) -> Option<String> {
    // Look for (vX.Y.Z) pattern
    for word in output.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| c == '(' || c == ')');
        if let Some(version) = trimmed.strip_prefix('v') {
            if version.contains('.')
                && version
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.')
            {
                return Some(version.to_string());
            }
        }
    }
    None
}
