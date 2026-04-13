use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct CmakeProvider;

impl ChipProvider for CmakeProvider {
    fn id(&self) -> ChipId { "cmake" }
    fn display_name(&self) -> &str { "CMake" }

    fn detect_files(&self) -> &[&str] {
        &["CMakeLists.txt", "CMakePresets.json"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let version = ctx.exec_cmd("cmake", &["--version"])
            .and_then(|o| parse_cmake_version(&o.stdout))
            .unwrap_or_default();
        ChipOutput {
            id: self.id(),
            label: version,
            icon: Some("Cmake"),
            tooltip: Some("CMake version".into()),
            ..ChipOutput::default()
        }
    }
}


fn parse_cmake_version(cmake_version: &str) -> Option<String> {
    Some(
        cmake_version
            //split into ["cmake" "version" "3.10.2", ...]
            .split_whitespace()
            // get down to "3.10.2"
            .nth(2)?
            .to_string(),
    )
}
