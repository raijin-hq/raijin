/// Built-in CLI completion specs, embedded at compile time.
use std::collections::HashMap;

use crate::spec::CliSpec;

/// Load all built-in CLI specs into a HashMap keyed by command name.
pub fn load_all_specs() -> HashMap<String, CliSpec> {
    let mut specs = HashMap::new();

    let raw_specs: &[&str] = &[
        include_str!("../specs/git.json"),
        include_str!("../specs/cargo.json"),
    ];

    for raw in raw_specs {
        match serde_json::from_str::<CliSpec>(raw) {
            Ok(spec) => {
                // Register by name and all aliases
                for alias in &spec.aliases {
                    specs.insert(alias.clone(), spec.clone());
                }
                specs.insert(spec.name.clone(), spec);
            }
            Err(e) => {
                eprintln!("Failed to parse CLI spec: {}", e);
            }
        }
    }

    specs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_all_specs() {
        let specs = load_all_specs();
        assert!(specs.contains_key("git"));
        assert!(specs.contains_key("cargo"));

        let git = &specs["git"];
        assert!(!git.subcommands.is_empty());
        assert!(git.find_subcommand("commit").is_some());

        let cargo = &specs["cargo"];
        assert!(cargo.find_subcommand("build").is_some());
    }
}
