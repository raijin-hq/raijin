use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct ElmProvider;

impl ChipProvider for ElmProvider {
    fn id(&self) -> ChipId {
        "elm"
    }

    fn display_name(&self) -> &str {
        "Elm"
    }

    fn detect_files(&self) -> &[&str] {
        &["elm.json", "elm-package.json", ".elm-version"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["elm"]
    }

    fn detect_folders(&self) -> &[&str] {
        &["elm-stuff"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx
            .exec_cmd("elm", &["--version"])
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Elm"),
            tooltip: Some("Elm version".into()),
            ..ChipOutput::default()
        }
    }
}
