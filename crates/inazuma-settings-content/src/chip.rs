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
}
