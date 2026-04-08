use inazuma::Oklch;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The appearance of a theme in serialized content.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceContent {
    Light,
    Dark,
}

/// Content struct for theme colors used in serialized theme definitions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ThemeColorsContent {}

/// Parses a color string into an [`Oklch`] value.
///
/// Supports `#RGB`, `#RRGGBB`, `#RRGGBBAA`, and `oklch(l c h)` formats.
pub fn try_parse_color(color: &str) -> anyhow::Result<Oklch> {
    crate::parse_color(color)
}
