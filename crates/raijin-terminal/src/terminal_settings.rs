use inazuma_collections::HashMap;
use inazuma_settings_framework::Settings;
pub use inazuma_settings_framework::CondaManager;
use raijin_task::Shell;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Terminal cursor shape.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Beam,
}

/// Alternate scroll mode.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlternateScroll {
    #[default]
    On,
    Off,
}

/// Virtual environment detection settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VenvSettings {
    #[default]
    Off,
    On {
        activate_script: Option<String>,
        directories: Option<Vec<String>>,
        #[serde(default)]
        conda_manager: CondaManager,
    },
}

/// View into the active venv settings.
pub struct VenvSettingsView<'a> {
    pub activate_script: Option<&'a str>,
    pub directories: Option<&'a [String]>,
    pub conda_manager: CondaManager,
}

impl VenvSettings {
    pub fn as_option(&self) -> Option<VenvSettingsView<'_>> {
        match self {
            VenvSettings::Off => None,
            VenvSettings::On {
                activate_script,
                directories,
                conda_manager,
            } => Some(VenvSettingsView {
                activate_script: activate_script.as_deref(),
                directories: directories.as_deref(),
                conda_manager: *conda_manager,
            }),
        }
    }
}

/// Terminal settings.
#[derive(Clone, Debug)]
pub struct TerminalSettings {
    pub shell: Shell,
    pub env: HashMap<String, String>,
    pub cursor_shape: CursorShape,
    pub alternate_scroll: AlternateScroll,
    pub max_scroll_history_lines: Option<usize>,
    pub detect_venv: VenvSettings,
    pub path_hyperlink_regexes: Vec<String>,
    pub path_hyperlink_timeout_ms: u64,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: Shell::System,
            env: HashMap::default(),
            cursor_shape: CursorShape::default(),
            alternate_scroll: AlternateScroll::default(),
            max_scroll_history_lines: None,
            detect_venv: VenvSettings::default(),
            path_hyperlink_regexes: Vec::new(),
            path_hyperlink_timeout_ms: 500,
        }
    }
}

impl Settings for TerminalSettings {
    fn from_settings(_content: &inazuma_settings_framework::SettingsContent) -> Self {
        TerminalSettings::default()
    }
}
