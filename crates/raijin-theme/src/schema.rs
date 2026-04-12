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
