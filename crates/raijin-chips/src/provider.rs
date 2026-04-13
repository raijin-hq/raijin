use crate::context::ChipContext;

/// Unique identifier for a chip provider (e.g., "username", "git_branch", "rust").
pub type ChipId = &'static str;

/// A single colored text segment within a chip.
///
/// Used for multi-colored chips like git stats: `3 · +42 -13`
/// where each segment has a different color.
#[derive(Debug, Clone)]
pub struct ChipSegment {
    pub text: String,
    pub color_key: Option<&'static str>,
}

/// Output from a chip provider — everything needed to render one chip.
///
/// The rendering layer (in `raijin-terminal-view`) maps `ChipOutput` to
/// Inazuma `Chip` elements, reading colors from theme tokens.
#[derive(Debug, Clone)]
pub struct ChipOutput {
    /// Provider ID (used for theme color lookup and settings overrides).
    pub id: ChipId,
    /// Primary label text.
    pub label: String,
    /// Optional icon name (from raijin-ui IconName).
    pub icon: Option<&'static str>,
    /// Tooltip text.
    pub tooltip: Option<String>,
    /// Whether this chip is interactive (clickable, shows pointer cursor).
    pub interactive: bool,
    /// Multi-segment content (overrides `label` if present).
    /// Used for git stats and other multi-colored chips.
    pub segments: Option<Vec<ChipSegment>>,
}

impl Default for ChipOutput {
    fn default() -> Self {
        Self {
            id: "",
            label: String::new(),
            icon: None,
            tooltip: None,
            interactive: false,
            segments: None,
        }
    }
}

/// Trait for chip data providers.
///
/// Each provider is responsible for one logical chip (e.g., username, git branch, Rust version).
/// Detection uses pre-scanned `DirContents` (no I/O in `is_available`).
/// Gathering may execute commands (timeout-protected via `ctx.exec_cmd`).
///
/// # Detection Pattern
///
/// Providers specify which files, folders, and extensions trigger their activation.
/// The default `is_available()` implementation checks `ctx.dir_contents` against these.
/// Providers that are always visible (username, time) override `is_available()` directly.
pub trait ChipProvider: Send + Sync {
    /// Unique identifier for this provider.
    fn id(&self) -> ChipId;

    /// Human-readable display name (for settings UI).
    fn display_name(&self) -> &str;

    /// Files that trigger this provider (e.g., `["Cargo.toml"]`).
    fn detect_files(&self) -> &[&str] {
        &[]
    }

    /// Folders that trigger this provider (e.g., `["node_modules"]`).
    fn detect_folders(&self) -> &[&str] {
        &[]
    }

    /// File extensions that trigger this provider (e.g., `["rs"]`, without dot).
    fn detect_extensions(&self) -> &[&str] {
        &[]
    }

    /// Returns true if this chip should be shown in the current context.
    ///
    /// Default implementation: 3-level DirContents check (files → folders → extensions).
    /// Override for providers that use environment variables or are always visible.
    fn is_available(&self, ctx: &ChipContext) -> bool {
        let files = self.detect_files();
        let folders = self.detect_folders();
        let extensions = self.detect_extensions();

        // No detection criteria = not available by default
        if files.is_empty() && folders.is_empty() && extensions.is_empty() {
            return false;
        }

        ctx.dir_contents.matches(files, folders, extensions)
    }

    /// Produce the chip output.
    ///
    /// Called only when `is_available()` returns true. May execute commands
    /// via `ctx.exec_cmd()` (timeout-protected). Should be reasonably fast.
    fn gather(&self, ctx: &ChipContext) -> ChipOutput;
}

/// Helper: extract a version string from command output like "v20.11.0" or "rustc 1.77.0".
pub fn parse_version_number(output: &str) -> String {
    output
        .split_whitespace()
        .find(|word| {
            word.starts_with(|c: char| c.is_ascii_digit())
                || word.starts_with('v')
        })
        .map(|w| w.trim_start_matches('v').to_string())
        .unwrap_or_else(|| output.trim().to_string())
}
