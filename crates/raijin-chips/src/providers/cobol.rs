use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct CobolProvider;

impl ChipProvider for CobolProvider {
    fn id(&self) -> ChipId { "cobol" }
    fn display_name(&self) -> &str { "COBOL" }

    fn detect_extensions(&self) -> &[&str] {
        &["cbl", "cob", "CBL", "COB"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx.exec_cmd("cobc", &["--version"])
            .and_then(|o| get_cobol_version(&o.stdout))
            .unwrap_or_default();
        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Cobol"),
            tooltip: Some("GnuCOBOL version".into()),
            ..ChipOutput::default()
        }
    }
}

/// Parse version from cobc output: "cobc (GnuCOBOL) 3.1.2.0" → "3.1.2.0"
fn get_cobol_version(cobol_stdout: &str) -> Option<String> {
    Some(
        cobol_stdout
            .split_whitespace()
            .nth(2)?
            .to_string(),
    )
}
