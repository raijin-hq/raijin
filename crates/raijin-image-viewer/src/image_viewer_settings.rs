pub use inazuma_settings_framework::ImageFileSizeUnit;
use inazuma_settings_framework::{RegisterSetting, Settings};

/// The settings for the image viewer.
#[derive(Clone, Debug, Default, RegisterSetting)]
pub struct ImageViewerSettings {
    /// The unit to use for displaying image file sizes.
    ///
    /// Default: "binary"
    pub unit: ImageFileSizeUnit,
}

impl Settings for ImageViewerSettings {
    fn from_settings(content: &inazuma_settings_framework::SettingsContent) -> Self {
        Self {
            unit: content.image_viewer.clone().unwrap().unit.unwrap(),
        }
    }
}
