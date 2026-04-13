use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use inazuma_settings_macros::{MergeFrom, with_fallible_options};
use inazuma_collections::HashMap;

/// Content for the `[chip]` section in settings.toml.
///
/// Controls the context chips displayed in the terminal input area.
/// Chips detect project context (languages, DevOps tools, git state)
/// and display relevant information as compact, colored labels.
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ChipSettingsContent {
    /// Ordered list of chip IDs to display.
    /// Use `"*"` as a wildcard to include remaining detected chips in default order.
    ///
    /// Default: ["username", "hostname", "directory", "time", "shell", "git_branch", "git_status", "*"]
    pub layout: Option<Vec<String>>,

    /// Whether to show icons on chips.
    ///
    /// Default: true
    pub show_icons: Option<bool>,

    /// Whether to show labels on chips.
    ///
    /// Default: true
    pub show_labels: Option<bool>,

    /// Command execution timeout in milliseconds.
    /// Commands (e.g., `rustc --version`) that exceed this are killed.
    ///
    /// Default: 500
    pub command_timeout: Option<u64>,

    /// CWD directory scan timeout in milliseconds.
    /// Protects against slow filesystems (NFS, SSHFS).
    ///
    /// Default: 30
    pub scan_timeout: Option<u64>,

    /// Per-chip overrides, keyed by chip ID.
    pub overrides: Option<HashMap<String, ChipOverrideContent>>,

    /// Kubernetes provider configuration.
    pub kubernetes: Option<KubernetesChipContent>,

    /// AWS provider configuration.
    pub aws: Option<AwsChipContent>,

    /// Directory provider configuration.
    pub directory: Option<DirectoryChipContent>,

    /// Git status provider configuration.
    pub git_status: Option<GitStatusChipContent>,

    /// Python provider configuration.
    pub python: Option<PythonChipContent>,
}

/// Per-chip configuration override.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ChipOverrideContent {
    /// Disable this chip.
    pub disabled: Option<bool>,

    /// Override text color (OKLCH string, e.g., "oklch(0.7 0.15 207)").
    pub color: Option<String>,

    /// Override icon name.
    pub icon: Option<String>,

    /// Version format: "major", "major.minor", or "full".
    pub version_format: Option<String>,

    /// Override detection files.
    pub detect_files: Option<Vec<String>>,

    /// Override detection extensions.
    pub detect_extensions: Option<Vec<String>>,

    /// Override detection folders.
    pub detect_folders: Option<Vec<String>>,

    /// Override detection environment variables.
    pub detect_env_vars: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Provider-specific config sub-sections
// ---------------------------------------------------------------------------

/// Kubernetes chip configuration.
///
/// ```toml
/// [chip.kubernetes]
/// context_aliases = { "docker-desktop" = "local" }
/// show_namespace = true
/// ```
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct KubernetesChipContent {
    /// Map of context name patterns to aliases (supports regex).
    /// Precedence: `contexts` Vec first, then `context_aliases` as fallback.
    pub context_aliases: Option<HashMap<String, String>>,

    /// Map of user name patterns to aliases (supports regex).
    pub user_aliases: Option<HashMap<String, String>>,

    /// Context-specific configuration with pattern matching.
    /// Takes precedence over `context_aliases`/`user_aliases`.
    pub contexts: Option<Vec<KubernetesContextContent>>,

    /// Show namespace in chip label.
    ///
    /// Default: true
    pub show_namespace: Option<bool>,

    /// Show user in chip label.
    ///
    /// Default: false
    pub show_user: Option<bool>,

    /// Show cluster in chip label.
    ///
    /// Default: false
    pub show_cluster: Option<bool>,
}

/// A context-specific configuration entry for Kubernetes.
/// Matched by pattern against the current context name.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct KubernetesContextContent {
    /// Regex pattern to match against the context name.
    pub context_pattern: Option<String>,

    /// Regex pattern to match against the user name.
    pub user_pattern: Option<String>,

    /// Replacement alias for the matched context.
    pub context_alias: Option<String>,

    /// Replacement alias for the matched user.
    pub user_alias: Option<String>,
}

/// AWS chip configuration.
///
/// ```toml
/// [chip.aws]
/// region_aliases = { "us-east-1" = "use1" }
/// force_display = false
/// ```
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct AwsChipContent {
    /// Map of region names to short aliases.
    pub region_aliases: Option<HashMap<String, String>>,

    /// Map of profile names to short aliases.
    pub profile_aliases: Option<HashMap<String, String>>,

    /// Show the chip even without valid credentials.
    ///
    /// Default: false
    pub force_display: Option<bool>,

    /// Symbol shown when credentials are expired.
    ///
    /// Default: "expired"
    pub expiration_symbol: Option<String>,
}

/// Directory chip configuration.
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct DirectoryChipContent {
    /// Maximum number of parent directories to show.
    pub truncation_length: Option<usize>,

    /// Truncate to the root of the current git repo.
    ///
    /// Default: false
    pub truncate_to_repo: Option<bool>,
}

/// Git status chip configuration.
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct GitStatusChipContent {
    /// Show stash count.
    ///
    /// Default: false
    pub show_stash: Option<bool>,

    /// Show ahead/behind count relative to upstream.
    ///
    /// Default: false
    pub show_ahead_behind: Option<bool>,
}

/// Python chip configuration.
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct PythonChipContent {
    /// Show virtualenv name in chip label.
    ///
    /// Default: true
    pub show_virtualenv: Option<bool>,

    /// Show pyenv version name instead of detected version.
    ///
    /// Default: false
    pub pyenv_version_name: Option<bool>,
}
