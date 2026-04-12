use inazuma::Pixels;
use inazuma_settings_framework::{RegisterSetting, Settings};
use raijin_ui::px;
use raijin_workspace::dock::DockPosition;

#[derive(Debug, RegisterSetting)]
pub struct CollaborationPanelSettings {
    pub button: bool,
    pub dock: DockPosition,
    pub default_width: Pixels,
}

#[derive(Debug, RegisterSetting)]
pub struct NotificationPanelSettings {
    pub button: bool,
    pub dock: DockPosition,
    pub default_width: Pixels,
    pub show_count_badge: bool,
}

impl Settings for CollaborationPanelSettings {
    fn from_settings(content: &inazuma_settings_framework::SettingsContent) -> Self {
        let panel = content.collaboration_panel.as_ref().unwrap();

        Self {
            button: panel.button.unwrap(),
            dock: panel.dock.unwrap().into(),
            default_width: panel.default_width.map(px).unwrap(),
        }
    }
}

impl Settings for NotificationPanelSettings {
    fn from_settings(content: &inazuma_settings_framework::SettingsContent) -> Self {
        let panel = content.notification_panel.as_ref().unwrap();
        return Self {
            button: panel.button.unwrap(),
            dock: panel.dock.unwrap().into(),
            default_width: panel.default_width.map(px).unwrap(),
            show_count_badge: panel.show_count_badge.unwrap(),
        };
    }
}
