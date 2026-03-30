pub mod defaults;

use anyhow::{Context, Result};
use inazuma::{Global, Hsla, rgb};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

impl Global for RaijinConfig {}

// ---------------------------------------------------------------------------
// Resolved Theme — parsed colors + derived values, set as Global
// ---------------------------------------------------------------------------

impl Global for ResolvedTheme {}

/// Parsed theme with Hsla colors, ready for rendering.
///
/// Loaded once at startup from the theme TOML, set as `Global`.
/// All UI code reads colors from here via `cx.global::<ResolvedTheme>()`.
#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    // --- Base colors (from theme file) ---
    pub accent: Hsla,
    pub background: Hsla,
    pub foreground: Hsla,
    pub error: Hsla,

    // --- Derived colors (computed from base) ---
    /// Block/sticky header background — base bg with configurable alpha
    pub block_bg: Hsla,
    /// Selected block highlight — accent at low alpha
    pub selected_bg: Hsla,
    /// Sticky header hover — accent at medium alpha
    pub sticky_hover_bg: Hsla,
    /// Block header metadata text — foreground at low alpha
    pub metadata_fg: Hsla,
    /// Command text in headers
    pub command_fg: Hsla,

    // --- Background image ---
    pub background_image: Option<(PathBuf, f32)>,
}

impl ResolvedTheme {
    /// Resolve a theme into parsed Hsla values with derived colors.
    pub fn from_theme(theme: &RaijinTheme) -> Self {
        let accent = parse_hex_color(&theme.accent).unwrap_or(rgb(0x00BFFF).into());
        let background = parse_hex_color(&theme.background).unwrap_or(rgb(0x121212).into());
        let foreground = parse_hex_color(&theme.foreground).unwrap_or(rgb(0xf1f1f1).into());
        let error = parse_hex_color(&theme.error).unwrap_or(rgb(0xff5f5f).into());
        let block_alpha = (theme.block_opacity as f32 / 100.0).clamp(0.0, 1.0);

        // Block bg: background color with configurable alpha
        let mut block_bg = background;
        block_bg.a = block_alpha;

        // Selected bg: accent at 8% alpha
        let mut selected_bg = accent;
        selected_bg.a = 0.08;

        // Sticky hover: accent at 15% alpha
        let mut sticky_hover_bg = accent;
        sticky_hover_bg.a = 0.15;

        // Metadata fg: foreground at 35% alpha
        let mut metadata_fg = foreground;
        metadata_fg.a = 0.35;

        // Command fg: full foreground
        let command_fg = foreground;

        let background_image = theme.resolve_background_image();

        Self {
            accent,
            background,
            foreground,
            error,
            block_bg,
            selected_bg,
            sticky_hover_bg,
            metadata_fg,
            command_fg,
            background_image,
        }
    }

    /// Create a default resolved theme (no theme file loaded).
    pub fn default_theme() -> Self {
        Self::from_theme(&RaijinTheme::default())
    }
}

/// Parse a hex color string like "#00BFFF" into Hsla.
fn parse_hex_color(hex: &str) -> Option<Hsla> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(rgb(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)).into())
}

// ---------------------------------------------------------------------------
// Config structs
// ---------------------------------------------------------------------------

/// Root configuration for Raijin.
///
/// Stored at `~/.config/raijin/config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RaijinConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub terminal: TerminalConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default)]
    pub working_directory: WorkingDirectory,
    #[serde(default)]
    pub input_mode: InputMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default = "defaults_theme")]
    pub theme: String,
    #[serde(default = "defaults_font_family")]
    pub font_family: String,
    #[serde(default = "defaults_font_size")]
    pub font_size: f64,
    #[serde(default = "defaults_line_height")]
    pub line_height: f64,
    /// Symbol maps: map Unicode ranges to specific font families.
    /// Useful for Nerd Font icons, Powerline glyphs, etc.
    ///
    /// Example in config.toml:
    /// ```toml
    /// [[appearance.symbol_map]]
    /// start = "E0B0"
    /// end = "E0D7"
    /// font_family = "Symbols Nerd Font Mono"
    /// ```
    #[serde(default)]
    pub symbol_map: Vec<SymbolMapEntry>,
}

/// Maps a Unicode codepoint range to a specific font family.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMapEntry {
    /// Start of Unicode range (hex, e.g. "E0B0").
    pub start: String,
    /// End of Unicode range (hex, e.g. "E0D7").
    pub end: String,
    /// Font family to use for characters in this range.
    pub font_family: String,
}

/// Parsed symbol map entry with resolved codepoint range.
#[derive(Debug, Clone)]
pub struct ResolvedSymbolMap {
    pub start: u32,
    pub end: u32,
    pub font_family: String,
}

impl SymbolMapEntry {
    /// Parse hex start/end into a resolved entry.
    pub fn resolve(&self) -> Option<ResolvedSymbolMap> {
        let start = u32::from_str_radix(&self.start, 16).ok()?;
        let end = u32::from_str_radix(&self.end, 16).ok()?;
        Some(ResolvedSymbolMap {
            start,
            end,
            font_family: self.font_family.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

/// A Raijin theme file loaded from `~/.raijin/themes/{name}.toml`.
///
/// Defines colors, background image, and terminal ANSI colors.
/// Same concept as Warp's `.yml` theme files in `~/.warp/themes/`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RaijinTheme {
    /// Accent color for UI elements (hex, e.g. "#00BFFF").
    #[serde(default = "defaults::theme_accent")]
    pub accent: String,
    /// Terminal background color (hex).
    #[serde(default = "defaults::theme_background")]
    pub background: String,
    /// Terminal foreground color (hex).
    #[serde(default = "defaults::theme_foreground")]
    pub foreground: String,
    /// Error indicator color (hex).
    #[serde(default = "defaults::theme_error")]
    pub error: String,
    /// Opacity for block/sticky header backgrounds (0–100).
    /// Controls how much the background image shows through.
    #[serde(default = "defaults::theme_block_opacity")]
    pub block_opacity: u32,
    /// Optional background image.
    #[serde(default)]
    pub background_image: Option<ThemeBackgroundImage>,
    /// Terminal ANSI colors.
    #[serde(default)]
    pub terminal_colors: Option<ThemeTerminalColors>,
}

/// ANSI terminal color palette.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeTerminalColors {
    #[serde(default)]
    pub normal: ThemeAnsiColors,
    #[serde(default)]
    pub bright: ThemeAnsiColors,
}

/// 8 ANSI colors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeAnsiColors {
    #[serde(default)]
    pub black: Option<String>,
    #[serde(default)]
    pub red: Option<String>,
    #[serde(default)]
    pub green: Option<String>,
    #[serde(default)]
    pub yellow: Option<String>,
    #[serde(default)]
    pub blue: Option<String>,
    #[serde(default)]
    pub magenta: Option<String>,
    #[serde(default)]
    pub cyan: Option<String>,
    #[serde(default)]
    pub white: Option<String>,
}

/// Background image configuration within a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeBackgroundImage {
    /// Path to the image, relative to `~/.raijin/themes/`.
    pub path: String,
    /// Opacity 0–100 (like Warp).
    #[serde(default = "default_bg_image_opacity")]
    pub opacity: u32,
}

fn default_bg_image_opacity() -> u32 {
    15
}

impl RaijinTheme {
    /// Load a theme by name from `~/.raijin/themes/{name}.toml`.
    pub fn load(name: &str) -> Option<Self> {
        let path = RaijinConfig::themes_dir().join(format!("{name}.toml"));
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Resolve the background image to an absolute path.
    pub fn resolve_background_image(&self) -> Option<(PathBuf, f32)> {
        let bg = self.background_image.as_ref()?;
        let path = PathBuf::from(&bg.path);

        let resolved = if path.is_absolute() {
            path
        } else {
            RaijinConfig::themes_dir().join(&path)
        };

        if resolved.exists() {
            let opacity = (bg.opacity as f32 / 100.0).clamp(0.0, 1.0);
            Some((resolved, opacity))
        } else {
            log::warn!("Background image not found: {}", resolved.display());
            None
        }
    }
}

impl ResolvedSymbolMap {
    /// Check if a character falls in this range and return the font family.
    pub fn match_char(&self, c: char) -> Option<&str> {
        let cp = c as u32;
        if cp >= self.start && cp <= self.end {
            Some(&self.font_family)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    #[serde(default = "defaults_scrollback")]
    pub scrollback_history: u32,
    #[serde(default)]
    pub cursor_style: CursorStyle,
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Where the terminal shell starts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkingDirectory {
    /// User home directory ($HOME).
    #[default]
    Home,
    /// Last used directory from previous session.
    Previous,
    /// A fixed custom path.
    Custom(String),
}

/// Terminal input mode.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    /// Raijin mode: shell prompt suppressed, context chips visible.
    #[default]
    Raijin,
    /// Shell PS1 mode: native prompt (Starship, P10k) visible.
    ShellPs1,
}

/// Terminal cursor style.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    #[default]
    Beam,
    Block,
    Underline,
}

// ---------------------------------------------------------------------------
// Defaults (for serde)
// ---------------------------------------------------------------------------

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: defaults::THEME.to_string(),
            font_family: defaults::FONT_FAMILY.to_string(),
            font_size: defaults::FONT_SIZE,
            line_height: defaults::LINE_HEIGHT,
            symbol_map: Vec::new(),
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            scrollback_history: defaults::SCROLLBACK_HISTORY,
            cursor_style: CursorStyle::default(),
        }
    }
}

fn defaults_theme() -> String {
    defaults::THEME.to_string()
}
fn defaults_font_family() -> String {
    defaults::FONT_FAMILY.to_string()
}
fn defaults_font_size() -> f64 {
    defaults::FONT_SIZE
}
fn defaults_line_height() -> f64 {
    defaults::LINE_HEIGHT
}
fn defaults_scrollback() -> u32 {
    defaults::SCROLLBACK_HISTORY
}

// ---------------------------------------------------------------------------
// Load / Save
// ---------------------------------------------------------------------------

impl RaijinConfig {
    /// Returns the Raijin home directory: `~/.raijin/`
    pub fn home_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".raijin")
    }

    /// Returns the themes directory: `~/.raijin/themes/`
    pub fn themes_dir() -> PathBuf {
        Self::home_dir().join("themes")
    }

    /// Returns the config file path: `~/.raijin/config.toml`
    pub fn config_path() -> PathBuf {
        Self::home_dir().join("config.toml")
    }

    /// Load config from disk. Creates default if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            let config = Self::default();
            config.save().ok();
            return Ok(config);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;

        let config: Self = toml::from_str(&content).unwrap_or_else(|e| {
            log::warn!("Failed to parse config, using defaults: {e}");
            Self::default()
        });

        Ok(config)
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        let dir = Self::home_dir();
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create config dir {}", dir.display()))?;

        let path = Self::config_path();
        let content = toml::to_string_pretty(self)
            .context("failed to serialize config")?;

        fs::write(&path, content)
            .with_context(|| format!("failed to write config to {}", path.display()))?;

        log::info!("Config saved to {}", path.display());
        Ok(())
    }

    /// Resolve the effective working directory as an absolute path.
    pub fn resolve_working_directory(&self) -> PathBuf {
        match &self.general.working_directory {
            WorkingDirectory::Home => {
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
            }
            WorkingDirectory::Previous => {
                // TODO: implement session persistence for last CWD
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
            }
            WorkingDirectory::Custom(path) => {
                let expanded = if path.starts_with('~') {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&path[2..])
                    } else {
                        PathBuf::from(path)
                    }
                } else {
                    PathBuf::from(path)
                };

                if expanded.is_dir() {
                    expanded
                } else {
                    log::warn!(
                        "Custom working directory does not exist: {}, falling back to home",
                        expanded.display()
                    );
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
                }
            }
        }
    }
}
