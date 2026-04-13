use raijin_editor::EditorSettings;
use inazuma::Pixels;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use inazuma_settings_framework::{RegisterSetting, Settings, StatusStyle};
use raijin_ui::{px, ScrollbarVisibility, ShowScrollbar};
use raijin_workspace::dock::DockPosition;

/// Convert from the editor's `ShowScrollbar` (which is `pub(crate)` in `raijin_editor`)
/// to `raijin_ui::ShowScrollbar` by matching on the `Debug` representation.
fn editor_show_scrollbar_to_ui(cx: &raijin_ui::App) -> ShowScrollbar {
    let editor_value = format!("{:?}", EditorSettings::get_global(cx).scrollbar.show);
    match editor_value.as_str() {
        "System" => ShowScrollbar::System,
        "Always" => ShowScrollbar::Always,
        "Never" => ShowScrollbar::Never,
        _ => ShowScrollbar::Auto,
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ScrollbarSettings {
    pub show: Option<ShowScrollbar>,
}

#[derive(Debug, Clone, PartialEq, RegisterSetting)]
pub struct GitPanelSettings {
    pub button: bool,
    pub dock: DockPosition,
    pub default_width: Pixels,
    pub status_style: StatusStyle,
    pub file_icons: bool,
    pub folder_icons: bool,
    pub scrollbar: ScrollbarSettings,
    pub fallback_branch_name: String,
    pub sort_by_path: bool,
    pub collapse_untracked_diff: bool,
    pub tree_view: bool,
    pub diff_stats: bool,
    pub show_count_badge: bool,
    pub starts_open: bool,
}

impl ScrollbarVisibility for GitPanelSettings {
    fn visibility(&self, cx: &raijin_ui::App) -> ShowScrollbar {
        // TODO: This PR should have defined Editor's `scrollbar.axis`
        // as an Option<ScrollbarAxis>, not a ScrollbarAxes as it would allow you to
        // `.unwrap_or(EditorSettings::get_global(cx).scrollbar.show)`.
        //
        // Once this is fixed we can extend the GitPanelSettings with a `scrollbar.axis`
        // so we can show each axis based on the settings.
        //
        // We should fix this. PR: https://github.com/raijin-industries/raijin/pull/19495
        self.scrollbar
            .show
            .unwrap_or_else(|| editor_show_scrollbar_to_ui(cx))
    }
}

impl Settings for GitPanelSettings {
    fn from_settings(content: &inazuma_settings_framework::SettingsContent) -> Self {
        let git_panel = content.git_panel.clone().unwrap();
        Self {
            button: git_panel.button.unwrap(),
            dock: git_panel.dock.unwrap().into(),
            default_width: px(git_panel.default_width.unwrap()),
            status_style: git_panel.status_style.unwrap(),
            file_icons: git_panel.file_icons.unwrap(),
            folder_icons: git_panel.folder_icons.unwrap(),
            scrollbar: ScrollbarSettings {
                show: git_panel.scrollbar.unwrap().show.map(Into::into),
            },
            fallback_branch_name: git_panel.fallback_branch_name.unwrap(),
            sort_by_path: git_panel.sort_by_path.unwrap(),
            collapse_untracked_diff: git_panel.collapse_untracked_diff.unwrap(),
            tree_view: git_panel.tree_view.unwrap(),
            diff_stats: git_panel.diff_stats.unwrap(),
            show_count_badge: git_panel.show_count_badge.unwrap(),
            starts_open: git_panel.starts_open.unwrap(),
        }
    }
}
