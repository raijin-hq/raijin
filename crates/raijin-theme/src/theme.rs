use std::path::{Path, PathBuf};
use std::sync::Arc;

use inazuma::{App, Global, Oklch, Pixels, SharedString, WindowAppearance, WindowBackgroundAppearance, px};
use serde::{Deserialize, Serialize};

use crate::accent::AccentColors;
use crate::colors::ThemeColors;
use crate::players::PlayerColors;
use crate::status::StatusColors;
use crate::syntax::SyntaxTheme;
use crate::system::SystemColors;

/// Whether a theme is light or dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Appearance {
    /// A light theme with dark text on light backgrounds.
    Light,
    /// A dark theme with light text on dark backgrounds.
    #[default]
    Dark,
}

impl Appearance {
    /// Returns whether the appearance is light.
    pub fn is_light(&self) -> bool {
        matches!(self, Self::Light)
    }
}

impl From<WindowAppearance> for Appearance {
    fn from(value: WindowAppearance) -> Self {
        match value {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
        }
    }
}

/// The rounding radius for client-side window decorations.
pub const CLIENT_SIDE_DECORATION_ROUNDING: Pixels = px(10.0);

/// The shadow/inset size for client-side window decorations.
pub const CLIENT_SIDE_DECORATION_SHADOW: Pixels = px(10.0);

/// Tracks the system's current appearance (light or dark).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemAppearance(pub Appearance);

#[derive(derive_more::Deref, derive_more::DerefMut, Default)]
struct GlobalSystemAppearance(SystemAppearance);

impl Global for GlobalSystemAppearance {}

impl SystemAppearance {
    /// Initializes the [`SystemAppearance`] for the application.
    pub fn init(cx: &mut App) {
        *cx.default_global::<GlobalSystemAppearance>() =
            GlobalSystemAppearance(SystemAppearance(cx.window_appearance().into()));
    }

    /// Returns the global [`SystemAppearance`].
    pub fn global(cx: &App) -> Self {
        cx.global::<GlobalSystemAppearance>().0
    }

    /// Returns a mutable reference to the global [`SystemAppearance`].
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<GlobalSystemAppearance>()
    }
}

/// Which themes should be loaded. This is used primarily for testing.
pub enum LoadThemes {
    /// Only load the base theme.
    JustBase,
    /// Load all of the built-in themes.
    All(Box<dyn inazuma::AssetSource>),
}

/// The complete set of styles for a theme.
#[derive(Clone, Debug)]
pub struct ThemeStyles {
    /// The background appearance of the window.
    pub window_background_appearance: WindowBackgroundAppearance,
    /// System colors (transparent, traffic lights).
    pub system: SystemColors,
    /// Accent colors for cycling UI elements.
    pub accents: AccentColors,
    /// All UI colors.
    pub colors: ThemeColors,
    /// Status indicator colors.
    pub status: StatusColors,
    /// Syntax highlighting theme.
    pub syntax: Arc<SyntaxTheme>,
    /// Player/collaborator colors.
    pub players: PlayerColors,
    /// Optional background image.
    pub background_image: Option<ThemeBackgroundImage>,
}

/// Background image configuration within a theme.
#[derive(Clone, Debug)]
pub struct ThemeBackgroundImage {
    /// Path to the image, relative to the themes directory.
    pub path: String,
    /// Opacity 0–100 (like Warp). Default: 15.
    pub opacity: u32,
}

impl ThemeBackgroundImage {
    /// Resolves the background image to an absolute path and normalized opacity.
    ///
    /// `base_dir` is the directory containing the theme file — relative paths
    /// in `background_image.path` are resolved against this directory.
    pub fn resolve(&self, base_dir: Option<&Path>) -> Option<(PathBuf, f32)> {
        let path = PathBuf::from(&self.path);

        let resolved = if path.is_absolute() {
            path
        } else if let Some(base) = base_dir {
            base.join(&path)
        } else {
            log::warn!("Cannot resolve relative background image path without base_dir: {}", self.path);
            return None;
        };

        if resolved.exists() {
            let opacity = (self.opacity as f32 / 100.0).clamp(0.0, 1.0);
            Some((resolved, opacity))
        } else {
            log::warn!("Background image not found: {}", resolved.display());
            None
        }
    }
}

/// A single theme with an identity, appearance, and styles.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Unique identifier for this theme.
    pub id: String,
    /// Display name of the theme.
    pub name: SharedString,
    /// Whether this is a light or dark theme.
    pub appearance: Appearance,
    /// The complete style definitions.
    pub styles: ThemeStyles,
    /// Base directory for resolving relative asset paths (e.g. background images).
    ///
    /// For user themes loaded from `~/.raijin/themes/my-theme/theme.toml`, this is
    /// the directory containing `theme.toml`. For bundled themes, this is `None`.
    pub base_dir: Option<PathBuf>,
}

impl Theme {
    /// Returns the system colors.
    #[inline(always)]
    pub fn system(&self) -> &SystemColors {
        &self.styles.system
    }

    /// Returns the accent colors.
    #[inline(always)]
    pub fn accents(&self) -> &AccentColors {
        &self.styles.accents
    }

    /// Returns the theme's colors.
    #[inline(always)]
    pub fn colors(&self) -> &ThemeColors {
        &self.styles.colors
    }

    /// Returns the theme's status colors.
    #[inline(always)]
    pub fn status(&self) -> &StatusColors {
        &self.styles.status
    }

    /// Returns the theme's syntax theme.
    #[inline(always)]
    pub fn syntax(&self) -> &Arc<SyntaxTheme> {
        &self.styles.syntax
    }

    /// Returns the theme's player colors.
    #[inline(always)]
    pub fn players(&self) -> &PlayerColors {
        &self.styles.players
    }

    /// Returns the appearance (light or dark).
    #[inline(always)]
    pub fn appearance(&self) -> Appearance {
        self.appearance
    }

    /// Returns the window background appearance.
    #[inline(always)]
    pub fn window_background_appearance(&self) -> WindowBackgroundAppearance {
        self.styles.window_background_appearance
    }

    /// Whether this theme is dark.
    #[inline(always)]
    pub fn is_dark(&self) -> bool {
        matches!(self.appearance, Appearance::Dark)
    }

    /// Darkens a color based on the current appearance.
    ///
    /// `light_amount` is used for light themes, `dark_amount` for dark themes.
    /// The lightness is reduced by the given amount (OKLCH L channel).
    pub fn darken(&self, color: Oklch, light_amount: f32, dark_amount: f32) -> Oklch {
        let amount = match self.appearance {
            Appearance::Light => light_amount,
            Appearance::Dark => dark_amount,
        };
        Oklch {
            l: (color.l - amount).max(0.0),
            c: color.c,
            h: color.h,
            a: color.a,
        }
    }
}

/// A family of related themes (e.g. "Raijin" with Dark and Light variants).
#[derive(Clone, Debug)]
pub struct ThemeFamily {
    /// Unique identifier for this family.
    pub id: String,
    /// Display name of the family.
    pub name: SharedString,
    /// Author of the theme family.
    pub author: SharedString,
    /// The themes in this family.
    pub themes: Vec<Theme>,
    /// The color scales used by the themes in the family.
    pub scales: crate::scale::ColorScales,
}
