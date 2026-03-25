pub mod defaults;

use anyhow::{Context, Result};
use inazuma::Global;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

impl Global for RaijinConfig {}

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
fn defaults_scrollback() -> u32 {
    defaults::SCROLLBACK_HISTORY
}

// ---------------------------------------------------------------------------
// Load / Save
// ---------------------------------------------------------------------------

impl RaijinConfig {
    /// Returns the config directory: `~/.config/raijin/`
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".config")
            })
            .join("raijin")
    }

    /// Returns the config file path: `~/.config/raijin/config.toml`
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
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
        let dir = Self::config_dir();
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
