mod accent;
mod colors;
mod default_colors;
mod fallback_themes;
mod font_family_cache;
mod global;
pub mod icon_theme;
mod icon_theme_schema;
mod loader;
mod players;
mod refinement;
mod registry;
mod scale;
mod schema;
mod status;
mod syntax;
mod system;
mod theme;
mod theme_settings_provider;
mod ui_density;

pub use accent::AccentColors;
pub use colors::ThemeColors;
pub use default_colors::*;
pub use fallback_themes::{raijin_default_themes, apply_status_color_defaults, apply_theme_color_defaults};
pub use font_family_cache::FontFamilyCache;
pub use global::{ActiveTheme, GlobalTheme};
pub use icon_theme::IconTheme;
pub use icon_theme_schema::IconThemeFamilyContent;
pub use loader::{load_theme_from_toml, load_theme_from_toml_with_base_dir, parse_color};
pub use players::{PlayerColor, PlayerColors};
pub use colors::{ThemeColorsRefinement, ThemeColorField};
pub use registry::{GlobalThemeRegistry, IconThemeNotFoundError, ThemeMeta, ThemeNotFoundError, ThemeRegistry};
pub use scale::{ColorScale, ColorScaleStep};
pub use schema::{AppearanceContent, ThemeColorsContent};
pub use status::{DiagnosticColors, StatusColors, StatusColorsRefinement, StatusStyle, StatusStyleRefinement};
pub use syntax::SyntaxTheme;
pub use system::SystemColors;
pub use theme::{Appearance, LoadThemes, Theme, ThemeBackgroundImage, ThemeFamily, ThemeStyles};
pub use theme_settings_provider::{ThemeSettingsProvider, set_theme_settings_provider, theme_settings};
pub use ui_density::UiDensity;

/// The name of the default dark theme.
pub const DEFAULT_DARK_THEME: &str = "Raijin Dark";

/// The name of the default light theme.
pub const DEFAULT_LIGHT_THEME: &str = "Raijin Light";
