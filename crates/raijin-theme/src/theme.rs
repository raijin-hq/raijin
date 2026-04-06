use std::path::{Path, PathBuf};

use inazuma::SharedString;

use crate::colors::ThemeColors;
use crate::players::PlayerColor;
use crate::status::StatusColors;
use crate::syntax::SyntaxTheme;

/// Whether a theme is light or dark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Appearance {
    /// A light theme with dark text on light backgrounds.
    Light,
    /// A dark theme with light text on dark backgrounds.
    Dark,
}

/// The complete set of styles for a theme.
#[derive(Clone, Debug)]
pub struct ThemeStyles {
    /// All UI colors.
    pub colors: ThemeColors,
    /// Status indicator colors.
    pub status: StatusColors,
    /// Syntax highlighting theme.
    pub syntax: SyntaxTheme,
    /// Player/collaborator colors.
    pub players: Vec<PlayerColor>,
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
    /// Returns a reference to the theme's colors.
    pub fn colors(&self) -> &ThemeColors {
        &self.styles.colors
    }

    /// Returns a reference to the theme's status colors.
    pub fn status(&self) -> &StatusColors {
        &self.styles.status
    }

    /// Returns a reference to the theme's syntax theme.
    pub fn syntax(&self) -> &SyntaxTheme {
        &self.styles.syntax
    }

    /// Returns a reference to the theme's player colors.
    pub fn players(&self) -> &[PlayerColor] {
        &self.styles.players
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
    pub author: String,
    /// The themes in this family.
    pub themes: Vec<Theme>,
}
