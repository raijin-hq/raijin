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
