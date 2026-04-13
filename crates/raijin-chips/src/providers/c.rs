use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

use super::cc;

pub struct CProvider;

impl ChipProvider for CProvider {
    fn id(&self) -> ChipId { "c" }
    fn display_name(&self) -> &str { "C" }

    fn detect_extensions(&self) -> &[&str] {
        &["c", "h"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let commands: &[(&str, &[&str])] = &[
            ("cc", &["--version"]),
            ("gcc", &["--version"]),
            ("clang", &["--version"]),
        ];
        let compilers = &[("clang", "clang"), ("gcc", "Free Software Foundation")];

        let info = cc::detect_compiler(ctx, commands, compilers);
        let label = match &info {
            Some(i) if !i.version.is_empty() => format!("{} {}", i.name, i.version),
            Some(i) => i.name.clone(),
            None => String::new(),
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("C"),
            tooltip: Some("C compiler".into()),
            ..ChipOutput::default()
        }
    }
}
