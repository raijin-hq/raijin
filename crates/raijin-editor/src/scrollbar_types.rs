use inazuma::{App, Global};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use inazuma_settings_framework::Settings;

/// When to show the scrollbar in the editor.
///
/// Default: auto
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShowScrollbar {
    /// Show the scrollbar if there's important information or
    /// follow the system's configured behavior.
    #[default]
    Auto,
    /// Match the system's configured behavior.
    System,
    /// Always show the scrollbar.
    Always,
    /// Never show the scrollbar.
    Never,
}

impl From<inazuma_settings_framework::ShowScrollbar> for ShowScrollbar {
    fn from(value: inazuma_settings_framework::ShowScrollbar) -> Self {
        match value {
            inazuma_settings_framework::ShowScrollbar::Auto => ShowScrollbar::Auto,
            inazuma_settings_framework::ShowScrollbar::System => ShowScrollbar::System,
            inazuma_settings_framework::ShowScrollbar::Always => ShowScrollbar::Always,
            inazuma_settings_framework::ShowScrollbar::Never => ShowScrollbar::Never,
        }
    }
}

pub trait GlobalSetting {
    fn get_value(cx: &App) -> &Self;
}

impl<T: Settings> GlobalSetting for T {
    fn get_value(cx: &App) -> &T {
        T::get_global(cx)
    }
}

pub trait ScrollbarVisibility: GlobalSetting + 'static {
    fn visibility(&self, cx: &App) -> ShowScrollbar;
}

#[derive(Default)]
pub struct ScrollbarAutoHide(pub bool);

impl ScrollbarAutoHide {
    pub fn should_hide(&self) -> bool {
        self.0
    }
}

impl Global for ScrollbarAutoHide {}
