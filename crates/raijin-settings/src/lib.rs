pub mod keymap_file;

use inazuma::WindowColorspace;
use inazuma_collections::HashMap;
use inazuma_settings_content::{
    AlternateScroll, AppearanceColorspace, CursorShapeContent,
    GeneralInputMode, GeneralWorkingDirectory, Shell, TerminalBlink,
};
use inazuma_settings_content::ChipOverrideContent;
use inazuma_settings_framework::{RegisterSetting, Settings};
use std::path::PathBuf;
use std::time::Duration;

// Re-export content types for convenience
pub use inazuma_settings_content::{
    AppearanceColorspace as Colorspace, GeneralInputMode as InputMode,
    GeneralWorkingDirectory as WorkingDirectory, ResolvedSymbolMap, SymbolMapEntry,
};

/// Resolved general settings — shell, working directory, input mode, scrollback, etc.
///
/// Access via `GeneralSettings::get_global(cx)`.
#[derive(Debug, Clone, RegisterSetting)]
pub struct GeneralSettings {
    pub shell: Shell,
    pub working_directory: GeneralWorkingDirectory,
    pub input_mode: GeneralInputMode,
    pub scrollback_history: usize,
    pub alternate_scroll: AlternateScroll,
    pub option_as_meta: bool,
    pub copy_on_select: bool,
    pub keep_selection_on_copy: bool,
    pub scroll_multiplier: f32,
    pub env: HashMap<String, String>,
}

impl Settings for GeneralSettings {
    fn from_settings(content: &inazuma_settings_content::SettingsContent) -> Self {
        let general = content.general.clone().unwrap_or_default();
        GeneralSettings {
            shell: general.shell.unwrap_or_default(),
            working_directory: general.working_directory.unwrap_or_default(),
            input_mode: general.input_mode.unwrap_or_default(),
            scrollback_history: general.scrollback_history.unwrap_or(10_000),
            alternate_scroll: general.alternate_scroll.unwrap_or(AlternateScroll::On),
            option_as_meta: general.option_as_meta.unwrap_or(false),
            copy_on_select: general.copy_on_select.unwrap_or(false),
            keep_selection_on_copy: general.keep_selection_on_copy.unwrap_or(false),
            scroll_multiplier: general.scroll_multiplier.unwrap_or(1.0),
            env: general.env.unwrap_or_default(),
        }
    }
}

impl GeneralSettings {
    /// Convert the working directory setting to an absolute path.
    pub fn resolve_working_directory(&self) -> PathBuf {
        match &self.working_directory {
            GeneralWorkingDirectory::Home => {
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
            }
            GeneralWorkingDirectory::Previous => {
                // Session persistence for last CWD will be implemented via raijin-session
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
            }
            GeneralWorkingDirectory::Custom(path) => {
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

/// Resolved appearance settings — font, cursor, contrast, colorspace, symbol maps.
///
/// Access via `AppearanceSettings::get_global(cx)`.
#[derive(Debug, Clone, RegisterSetting)]
pub struct AppearanceSettings {
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub cursor_style: CursorShapeContent,
    pub cursor_blink: TerminalBlink,
    pub minimum_contrast: f32,
    pub window_colorspace: WindowColorspace,
    pub symbol_map: Vec<ResolvedSymbolMap>,
}

impl Settings for AppearanceSettings {
    fn from_settings(content: &inazuma_settings_content::SettingsContent) -> Self {
        let appearance = content.appearance.clone().unwrap_or_default();

        let symbol_map = appearance
            .symbol_map
            .unwrap_or_default()
            .iter()
            .filter_map(|entry| entry.resolve())
            .collect();

        let window_colorspace = match appearance.window_colorspace.unwrap_or_default() {
            AppearanceColorspace::Srgb => WindowColorspace::Srgb,
            AppearanceColorspace::DisplayP3 => WindowColorspace::DisplayP3,
            AppearanceColorspace::Native => WindowColorspace::Native,
        };

        AppearanceSettings {
            font_family: appearance
                .font_family
                .map(|f| f.0.to_string())
                .unwrap_or_else(|| "DankMono Nerd Font Mono".to_string()),
            font_size: appearance
                .font_size
                .map(|s| s.0)
                .unwrap_or(15.0),
            line_height: appearance.line_height.unwrap_or(1.6),
            cursor_style: appearance.cursor_style.unwrap_or(CursorShapeContent::Bar),
            cursor_blink: appearance
                .cursor_blink
                .unwrap_or(TerminalBlink::TerminalControlled),
            minimum_contrast: appearance.minimum_contrast.unwrap_or(45.0),
            window_colorspace,
            symbol_map,
        }
    }
}

/// Resolved chip settings — layout, timeouts, per-chip overrides.
///
/// Access via `ChipSettings::get_global(cx)`.
// ---------------------------------------------------------------------------
// Chip Settings (resolved)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, RegisterSetting)]
pub struct ChipSettings {
    pub layout: Vec<String>,
    pub show_icons: bool,
    pub show_labels: bool,
    pub command_timeout: Duration,
    pub scan_timeout: Duration,
    pub overrides: HashMap<String, ChipOverrideContent>,
    pub kubernetes: KubernetesChipConfig,
    pub aws: AwsChipConfig,
    pub directory: DirectoryChipConfig,
    pub git_status: GitStatusChipConfig,
    pub python: PythonChipConfig,
}

impl Settings for ChipSettings {
    fn from_settings(content: &inazuma_settings_content::SettingsContent) -> Self {
        let chip = content.chip.clone().unwrap_or_default();
        let k8s = chip.kubernetes.unwrap_or_default();
        let aws = chip.aws.unwrap_or_default();
        let dir = chip.directory.unwrap_or_default();
        let git = chip.git_status.unwrap_or_default();
        let py = chip.python.unwrap_or_default();

        ChipSettings {
            layout: chip.layout.unwrap_or_else(|| {
                vec![
                    "username".into(), "hostname".into(), "directory".into(),
                    "time".into(), "shell".into(), "git_branch".into(),
                    "git_status".into(), "*".into(),
                ]
            }),
            show_icons: chip.show_icons.unwrap_or(true),
            show_labels: chip.show_labels.unwrap_or(true),
            command_timeout: Duration::from_millis(chip.command_timeout.unwrap_or(500)),
            scan_timeout: Duration::from_millis(chip.scan_timeout.unwrap_or(30)),
            overrides: chip.overrides.unwrap_or_default(),
            kubernetes: KubernetesChipConfig {
                context_aliases: k8s.context_aliases.unwrap_or_default(),
                user_aliases: k8s.user_aliases.unwrap_or_default(),
                contexts: k8s.contexts.unwrap_or_default().into_iter().map(|c| {
                    KubernetesContextConfig {
                        context_pattern: c.context_pattern.unwrap_or_default(),
                        user_pattern: c.user_pattern,
                        context_alias: c.context_alias,
                        user_alias: c.user_alias,
                    }
                }).collect(),
                show_namespace: k8s.show_namespace.unwrap_or(true),
                show_user: k8s.show_user.unwrap_or(false),
                show_cluster: k8s.show_cluster.unwrap_or(false),
            },
            aws: AwsChipConfig {
                region_aliases: aws.region_aliases.unwrap_or_default(),
                profile_aliases: aws.profile_aliases.unwrap_or_default(),
                force_display: aws.force_display.unwrap_or(false),
                expiration_symbol: aws.expiration_symbol.unwrap_or_else(|| "expired".into()),
            },
            directory: DirectoryChipConfig {
                truncation_length: dir.truncation_length.unwrap_or(usize::MAX),
                truncate_to_repo: dir.truncate_to_repo.unwrap_or(false),
            },
            git_status: GitStatusChipConfig {
                show_stash: git.show_stash.unwrap_or(false),
                show_ahead_behind: git.show_ahead_behind.unwrap_or(false),
            },
            python: PythonChipConfig {
                show_virtualenv: py.show_virtualenv.unwrap_or(true),
                pyenv_version_name: py.pyenv_version_name.unwrap_or(false),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Resolved per-provider config structs (non-Option, with defaults)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct KubernetesChipConfig {
    pub context_aliases: HashMap<String, String>,
    pub user_aliases: HashMap<String, String>,
    pub contexts: Vec<KubernetesContextConfig>,
    pub show_namespace: bool,
    pub show_user: bool,
    pub show_cluster: bool,
}

#[derive(Debug, Clone)]
pub struct KubernetesContextConfig {
    pub context_pattern: String,
    pub user_pattern: Option<String>,
    pub context_alias: Option<String>,
    pub user_alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AwsChipConfig {
    pub region_aliases: HashMap<String, String>,
    pub profile_aliases: HashMap<String, String>,
    pub force_display: bool,
    pub expiration_symbol: String,
}

#[derive(Debug, Clone)]
pub struct DirectoryChipConfig {
    pub truncation_length: usize,
    pub truncate_to_repo: bool,
}

#[derive(Debug, Clone)]
pub struct GitStatusChipConfig {
    pub show_stash: bool,
    pub show_ahead_behind: bool,
}

#[derive(Debug, Clone)]
pub struct PythonChipConfig {
    pub show_virtualenv: bool,
    pub pyenv_version_name: bool,
}
