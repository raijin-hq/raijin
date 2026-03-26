/// CLI spec definitions — describes the structure of CLI commands.
use serde::Deserialize;

/// A complete CLI tool specification.
#[derive(Debug, Clone, Deserialize)]
pub struct CliSpec {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub subcommands: Vec<CliSpec>,
    #[serde(default)]
    pub options: Vec<CliOption>,
    #[serde(default)]
    pub args: Vec<CliArg>,
}

/// A CLI option/flag definition.
#[derive(Debug, Clone, Deserialize)]
pub struct CliOption {
    /// All names for this option, e.g. ["-v", "--verbose"]
    pub names: Vec<String>,
    pub description: Option<String>,
    /// Whether this option takes a value argument.
    #[serde(default)]
    pub takes_arg: bool,
    /// Display name for the argument value, e.g. "FILE", "PATH".
    pub arg_name: Option<String>,
    /// Template for argument value completion.
    pub arg_template: Option<ArgTemplate>,
    /// Whether this option can be specified multiple times.
    #[serde(default)]
    pub is_repeatable: bool,
}

/// A positional CLI argument definition.
#[derive(Debug, Clone, Deserialize)]
pub struct CliArg {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<ArgTemplate>,
    #[serde(default)]
    pub is_optional: bool,
    #[serde(default)]
    pub is_variadic: bool,
}

/// Template for argument completion — tells the completer what kind of values to suggest.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgTemplate {
    Filepaths,
    Folders,
    History,
    GitBranches,
    GitTags,
    GitRemotes,
    GitFiles,
    EnvVars,
    ProcessIds,
    Custom(Vec<String>),
}

impl CliSpec {
    /// Find a subcommand by name (exact match or alias).
    pub fn find_subcommand(&self, name: &str) -> Option<&CliSpec> {
        self.subcommands.iter().find(|s| {
            s.name == name || s.aliases.iter().any(|a| a == name)
        })
    }

    /// Find an option by name (any of its names).
    pub fn find_option(&self, name: &str) -> Option<&CliOption> {
        self.options.iter().find(|o| o.names.iter().any(|n| n == name))
    }
}
