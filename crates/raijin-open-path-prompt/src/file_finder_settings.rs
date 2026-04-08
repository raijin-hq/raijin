use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use inazuma_settings_framework::{RegisterSetting, Settings};

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, RegisterSetting)]
pub struct FileFinderSettings {
    pub file_icons: bool,
    pub modal_max_width: FileFinderWidth,
    pub skip_focus_for_active_in_search: bool,
    pub include_ignored: Option<bool>,
    pub include_channels: bool,
}

impl Settings for FileFinderSettings {
    fn from_settings(content: &inazuma_settings_framework::SettingsContent) -> Self {
        let file_finder = content.file_finder.as_ref().unwrap();

        Self {
            file_icons: file_finder.file_icons.unwrap(),
            modal_max_width: file_finder.modal_max_width.unwrap().into(),
            skip_focus_for_active_in_search: file_finder.skip_focus_for_active_in_search.unwrap(),
            include_ignored: match file_finder.include_ignored.unwrap() {
                inazuma_settings_framework::IncludeIgnoredContent::All => Some(true),
                inazuma_settings_framework::IncludeIgnoredContent::Indexed => Some(false),
                inazuma_settings_framework::IncludeIgnoredContent::Smart => None,
            },
            include_channels: file_finder.include_channels.unwrap(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FileFinderWidth {
    #[default]
    Small,
    Medium,
    Large,
    XLarge,
    Full,
}

impl From<inazuma_settings_framework::FileFinderWidthContent> for FileFinderWidth {
    fn from(content: inazuma_settings_framework::FileFinderWidthContent) -> Self {
        match content {
            inazuma_settings_framework::FileFinderWidthContent::Small => FileFinderWidth::Small,
            inazuma_settings_framework::FileFinderWidthContent::Medium => FileFinderWidth::Medium,
            inazuma_settings_framework::FileFinderWidthContent::Large => FileFinderWidth::Large,
            inazuma_settings_framework::FileFinderWidthContent::XLarge => FileFinderWidth::XLarge,
            inazuma_settings_framework::FileFinderWidthContent::Full => FileFinderWidth::Full,
        }
    }
}
