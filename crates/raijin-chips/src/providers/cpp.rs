use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

use super::cc;

pub struct CppProvider;

impl ChipProvider for CppProvider {
    fn id(&self) -> ChipId { "cpp" }
    fn display_name(&self) -> &str { "C++" }

    fn detect_extensions(&self) -> &[&str] {
        &["cpp", "cc", "cxx", "c++", "hpp", "hh", "hxx", "h++", "tcc"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let commands: &[(&str, &[&str])] = &[
            ("c++", &["--version"]),
            ("g++", &["--version"]),
            ("clang++", &["--version"]),
        ];
        let compilers = &[("clang++", "clang"), ("g++", "Free Software Foundation")];

        let info = cc::detect_compiler(ctx, commands, compilers);
        let label = match &info {
            Some(i) if !i.version.is_empty() => format!("{} {}", i.name, i.version),
            Some(i) => i.name.clone(),
            None => String::new(),
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Cpp"),
            tooltip: Some("C++ compiler".into()),
            ..ChipOutput::default()
        }
    }
}
