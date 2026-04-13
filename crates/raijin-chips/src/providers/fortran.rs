use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

pub struct FortranProvider;

impl ChipProvider for FortranProvider {
    fn id(&self) -> ChipId {
        "fortran"
    }

    fn display_name(&self) -> &str {
        "Fortran"
    }

    fn detect_extensions(&self) -> &[&str] {
        &[
            "f", "F", "for", "FOR", "ftn", "FTN", "f77", "F77", "f90", "F90", "f95", "F95",
            "f03", "F03", "f08", "F08", "f18", "F18",
        ]
    }

    fn detect_files(&self) -> &[&str] {
        &["fpm.toml"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let commands: &[(&str, &[&str])] = &[
            ("gfortran", &["--version"]),
            ("flang", &["--version"]),
            ("flang-new", &["--version"]),
        ];

        let compiler_output = commands
            .iter()
            .find_map(|(cmd, args)| ctx.exec_cmd(cmd, args));

        let (version, name) = compiler_output
            .as_ref()
            .map(|o| {
                let stdout = &o.stdout;
                let compiler_name = if stdout.contains("GNU") {
                    "gfortran"
                } else if stdout.contains("flang-new") || stdout.contains("flang") {
                    "flang"
                } else {
                    ""
                };

                let version = stdout
                    .split_whitespace()
                    .find(|word| semver::Version::parse(word).is_ok())
                    .unwrap_or("")
                    .to_string();

                (version, compiler_name)
            })
            .unwrap_or_default();

        let label = if !version.is_empty() && !name.is_empty() {
            format!("{version}-{name}")
        } else if !version.is_empty() {
            version
        } else {
            String::new()
        };

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Fortran"),
            tooltip: Some("Fortran compiler version".into()),
            ..ChipOutput::default()
        }
    }
}
