pub mod defaults;
pub mod keymap_file;
pub mod watcher;

use anyhow::{Context, Result};
use inazuma::{Global, WindowColorspace};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

impl Global for RaijinSettings {}

// ---------------------------------------------------------------------------
// Settings structs
// ---------------------------------------------------------------------------

/// Root settings for Raijin.
///
/// Stored at `~/.raijin/settings.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RaijinSettings {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub appearance: AppearanceSettings,
    #[serde(default)]
    pub terminal: TerminalSettings,
    /// Theme selection — which theme(s) to use.
    ///
    /// ```toml
    /// [theme]
    /// mode = "dark"
    /// light = "Catppuccin Latte"
    /// dark = "Raijin Dark"
    /// ```
    #[serde(default)]
    pub theme: ThemeConfig,
}

/// Theme selection configuration.
///
/// Supports static (single mode) or dynamic (system appearance aware):
/// - `mode = "dark"` — always use the `dark` theme
/// - `mode = "light"` — always use the `light` theme
/// - `mode = "system"` — follow OS appearance setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// How the theme appearance is determined.
    #[serde(default)]
    pub mode: ThemeMode,
    /// Theme name for dark mode.
    #[serde(default = "default_dark_theme")]
    pub dark: String,
    /// Theme name for light mode.
    #[serde(default = "default_light_theme")]
    pub light: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: ThemeMode::default(),
            dark: default_dark_theme(),
            light: default_light_theme(),
        }
    }
}

impl ThemeConfig {
    /// Resolves the effective theme name based on the mode.
    pub fn theme_name(&self) -> &str {
        match self.mode {
            ThemeMode::Light => &self.light,
            ThemeMode::Dark => &self.dark,
            ThemeMode::System => {
                // Default to dark until system appearance detection is implemented.
                &self.dark
            }
        }
    }
}

/// How the theme appearance is determined.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    /// Follow the OS light/dark setting.
    System,
    /// Always light.
    Light,
    /// Always dark.
    #[default]
    Dark,
}

fn default_dark_theme() -> String {
    "Raijin Dark".to_string()
}

fn default_light_theme() -> String {
    "Raijin Dark".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default)]
    pub working_directory: WorkingDirectory,
    #[serde(default)]
    pub input_mode: InputMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    #[serde(default = "defaults_font_family")]
    pub font_family: String,
    #[serde(default = "defaults_font_size")]
    pub font_size: f64,
    #[serde(default = "defaults_line_height")]
    pub line_height: f64,
    /// Window colorspace for the Metal rendering layer.
    /// Controls how colors are interpreted on wide-gamut (P3) displays.
    ///
    /// - `srgb` (default): Explicit sRGB tagging prevents oversaturation on P3 displays.
    /// - `display_p3`: Enable the wider P3 gamut for richer colors.
    /// - `native`: Use the display's native colorspace without explicit tagging.
    #[serde(default)]
    pub window_colorspace: WindowColorspace,
    /// Symbol maps: map Unicode ranges to specific font families.
    /// Useful for Nerd Font icons, Powerline glyphs, etc.
    ///
    /// Example in settings.toml:
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
// (Theme structs removed — all visual theming now lives in raijin-theme)
// ---------------------------------------------------------------------------

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
pub struct TerminalSettings {
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

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            font_family: defaults::FONT_FAMILY.to_string(),
            font_size: defaults::FONT_SIZE,
            line_height: defaults::LINE_HEIGHT,
            window_colorspace: WindowColorspace::default(),
            symbol_map: Vec::new(),
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            scrollback_history: defaults::SCROLLBACK_HISTORY,
            cursor_style: CursorStyle::default(),
        }
    }
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

impl RaijinSettings {
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

    /// Returns the settings file path: `~/.raijin/settings.toml`
    pub fn settings_path() -> PathBuf {
        Self::home_dir().join("settings.toml")
    }

    /// Returns the keymap file path: `~/.raijin/keymap.toml`
    pub fn keymap_path() -> PathBuf {
        Self::home_dir().join("keymap.toml")
    }

    /// Load settings from disk. Creates default if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::settings_path();

        if !path.exists() {
            let settings = Self::default();
            settings.save().ok();
            return Ok(settings);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read settings at {}", path.display()))?;

        let settings: Self = toml::from_str(&content).unwrap_or_else(|e| {
            log::warn!("Failed to parse settings, using defaults: {e}");
            Self::default()
        });

        Ok(settings)
    }

    /// Save settings to disk.
    pub fn save(&self) -> Result<()> {
        let dir = Self::home_dir();
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create settings dir {}", dir.display()))?;

        let path = Self::settings_path();
        let content = toml::to_string_pretty(self)
            .context("failed to serialize settings")?;

        fs::write(&path, content)
            .with_context(|| format!("failed to write settings to {}", path.display()))?;

        log::info!("Settings saved to {}", path.display());
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
