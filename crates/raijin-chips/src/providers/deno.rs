use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for Deno runtime version.
///
/// Detection: `deno.json`, `deno.jsonc`, `deno.lock`
/// Version:   `deno -V` → "deno 1.8.3" → "1.8.3"
pub struct DenoProvider;

impl ChipProvider for DenoProvider {
    fn id(&self) -> ChipId {
        "deno"
    }

    fn display_name(&self) -> &str {
        "Deno"
    }

    fn detect_files(&self) -> &[&str] {
        &["deno.json", "deno.jsonc", "deno.lock"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("deno", &["-V"])
            .and_then(|o| parse_deno_version(&o.stdout))
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Deno"),
            tooltip: Some("Deno version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse `deno -V` output: "deno 1.8.3\n" → "1.8.3"
///
/// The first line is always "deno <version>". standard splits on whitespace
/// and takes the second token.
fn parse_deno_version(output: &str) -> Option<String> {
    output.split_whitespace().nth(1).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stable() {
        assert_eq!(
            parse_deno_version("deno 1.8.3\nv8 9.0.257.3\ntypescript 4.2.2\n"),
            Some("1.8.3".to_string()),
        );
    }

    #[test]
    fn parse_canary() {
        assert_eq!(
            parse_deno_version("deno 1.40.0+bc4553a\n"),
            Some("1.40.0+bc4553a".to_string()),
        );
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse_deno_version(""), None);
    }
}
