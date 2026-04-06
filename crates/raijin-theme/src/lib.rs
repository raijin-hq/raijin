mod accent;
mod colors;
mod global;
mod loader;
mod players;
mod refinement;
mod registry;
mod scale;
mod status;
mod syntax;
mod theme;

pub use accent::AccentColors;
pub use colors::ThemeColors;
pub use global::{ActiveTheme, GlobalTheme};
pub use loader::{load_theme_from_toml, load_theme_from_toml_with_base_dir, parse_color};
pub use players::{PlayerColor, PlayerColors};
pub use refinement::ThemeColorsRefinement;
pub use registry::{ThemeMeta, ThemeRegistry};
pub use scale::{ColorScale, ColorScaleStep};
pub use status::{StatusColors, StatusStyle};
pub use syntax::SyntaxTheme;
pub use theme::{Appearance, Theme, ThemeBackgroundImage, ThemeFamily, ThemeStyles};

/// The name of the default dark theme.
pub const DEFAULT_DARK_THEME: &str = "Raijin Dark";

/// The name of the default light theme.
pub const DEFAULT_LIGHT_THEME: &str = "Raijin Dark";
