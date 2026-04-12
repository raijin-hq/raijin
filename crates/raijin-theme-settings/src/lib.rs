mod schema;
mod settings;
mod theme_settings;

pub use schema::{
    deserialize_user_theme, status_colors_refinement, syntax_overrides, theme_colors_refinement,
    ThemeColorsContent, ThemeContent, ThemeFamilyContent, ThemeStyleContent, StatusColorsContent,
};
pub use settings::{
    BufferLineHeight, FontFamilyName, IconThemeName, IconThemeSelection, ThemeAppearanceMode,
    ThemeName, ThemeSelection, ThemeSettings, adjust_buffer_font_size, adjust_ui_font_size,
    appearance_to_mode, clamp_font_size, default_theme, reset_buffer_font_size,
    observe_buffer_font_size_adjustment, reset_ui_font_size, set_mode, set_theme, setup_ui_font,
};
pub use theme_settings::{init, reload_icon_theme, reload_theme};
pub use raijin_theme::UiDensity;
