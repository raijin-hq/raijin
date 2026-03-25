//! WASM stub implementation for highlighter module.
//! Provides empty/no-op implementations since tree-sitter is not available in WASM.
//!
//! Note: diagnostics.rs is available in WASM, only syntax highlighting requires stubs.

use inazuma::{HighlightStyle, SharedString};
use std::ops::Range;
use std::time::Duration;

// Syntax highlighter stub
pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    pub fn new(_language: impl AsRef<str>) -> Self {
        Self
    }

    pub fn highlight(&self, _text: &ropey::Rope) -> Vec<(Range<usize>, HighlightStyle)> {
        Vec::new()
    }

    pub fn styles(
        &self,
        _range: &Range<usize>,
        _theme: &HighlightTheme,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        Vec::new()
    }

    pub fn update(
        &mut self,
        _edit: Option<crate::input::InputEdit>,
        _text: &ropey::Rope,
        _timeout: Option<Duration>,
    ) -> bool {
        // No-op in WASM
        true
    }

    pub fn language(&self) -> &SharedString {
        static EMPTY: SharedString = SharedString::new_static("");
        &EMPTY
    }

    pub fn text(&self) -> &ropey::Rope {
        static EMPTY_ROPE: LazyLock<ropey::Rope> = LazyLock::new(ropey::Rope::new);
        &EMPTY_ROPE
    }

    pub fn tree(&self) -> Option<&crate::input::Tree> {
        None
    }
}

// Language enum stub
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Unknown,
}

impl Language {
    pub fn from_str(_name: &str) -> Self {
        Language::Unknown
    }

    pub fn name(&self) -> &'static str {
        "unknown"
    }

    pub fn config(&self) -> LanguageConfig {
        LanguageConfig {
            name: "unknown".into(),
        }
    }

    pub fn all() -> impl Iterator<Item = Self> {
        std::iter::once(Language::Unknown)
    }
}

// Language config stub (without tree_sitter::Language)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageConfig {
    pub name: SharedString,
}

// Re-export theme types from registry module (which will be conditionally compiled)
// For WASM, we create minimal stubs here
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
    Underline,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, JsonSchema, Serialize, Deserialize)]
#[repr(u16)]
pub enum FontWeightContent {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    Normal = 400,
    Medium = 500,
    Semibold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct ThemeStyle {
    pub color: Option<inazuma::Hsla>,
    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeightContent>,
}

impl From<ThemeStyle> for HighlightStyle {
    fn from(style: ThemeStyle) -> Self {
        HighlightStyle {
            color: style.color,
            font_weight: style.font_weight.map(|w| match w {
                FontWeightContent::Thin => inazuma::FontWeight::THIN,
                FontWeightContent::ExtraLight => inazuma::FontWeight::EXTRA_LIGHT,
                FontWeightContent::Light => inazuma::FontWeight::LIGHT,
                FontWeightContent::Normal => inazuma::FontWeight::NORMAL,
                FontWeightContent::Medium => inazuma::FontWeight::MEDIUM,
                FontWeightContent::Semibold => inazuma::FontWeight::SEMIBOLD,
                FontWeightContent::Bold => inazuma::FontWeight::BOLD,
                FontWeightContent::ExtraBold => inazuma::FontWeight::EXTRA_BOLD,
                FontWeightContent::Black => inazuma::FontWeight::BLACK,
            }),
            font_style: style.font_style.map(|s| match s {
                FontStyle::Normal => inazuma::FontStyle::Normal,
                FontStyle::Italic => inazuma::FontStyle::Italic,
                FontStyle::Underline => inazuma::FontStyle::Normal,
            }),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct SyntaxColors {
    // Minimal stub - actual fields are in native registry.rs
    // Adding commonly accessed fields to avoid compilation errors
    #[serde(rename = "link_text")]
    pub link_text: Option<ThemeStyle>,
}

impl SyntaxColors {
    pub fn style(&self, _name: &str) -> Option<HighlightStyle> {
        None
    }

    pub fn style_for_index(&self, _index: usize) -> Option<HighlightStyle> {
        None
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct StatusColors {
    // Minimal stub
}

impl StatusColors {
    pub fn error(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn error_background(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn error_border(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn warning(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn warning_background(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn warning_border(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn info(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn info_background(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn info_border(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn success(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn success_background(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn success_border(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn hint(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn hint_background(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }

    pub fn hint_border(&self, _cx: &inazuma::App) -> inazuma::Hsla {
        inazuma::Hsla::default()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct HighlightThemeStyle {
    pub editor_background: Option<inazuma::Hsla>,
    pub editor_foreground: Option<inazuma::Hsla>,
    pub editor_active_line: Option<inazuma::Hsla>,
    pub editor_line_number: Option<inazuma::Hsla>,
    pub editor_active_line_number: Option<inazuma::Hsla>,
    pub editor_invisible: Option<inazuma::Hsla>,
    #[serde(flatten)]
    pub status: StatusColors,
    #[serde(rename = "syntax")]
    pub syntax: SyntaxColors,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
pub struct HighlightTheme {
    pub name: String,
    #[serde(default)]
    pub appearance: crate::ThemeMode,
    pub style: HighlightThemeStyle,
}

impl std::ops::Deref for HighlightTheme {
    type Target = SyntaxColors;

    fn deref(&self) -> &Self::Target {
        &self.style.syntax
    }
}

impl HighlightTheme {
    pub fn default_dark() -> std::sync::Arc<Self> {
        use crate::DEFAULT_THEME_COLORS;
        DEFAULT_THEME_COLORS[&crate::ThemeMode::Dark].1.clone()
    }

    pub fn default_light() -> std::sync::Arc<Self> {
        use crate::DEFAULT_THEME_COLORS;
        DEFAULT_THEME_COLORS[&crate::ThemeMode::Light].1.clone()
    }
}

// Language registry stub
pub struct LanguageRegistry {
    languages: Mutex<HashMap<SharedString, LanguageConfig>>,
}

impl LanguageRegistry {
    pub fn singleton() -> &'static LazyLock<LanguageRegistry> {
        static INSTANCE: LazyLock<LanguageRegistry> = LazyLock::new(|| LanguageRegistry {
            languages: Mutex::new(HashMap::new()),
        });
        &INSTANCE
    }

    pub fn register(&self, lang: &str, config: &LanguageConfig) {
        self.languages
            .lock()
            .unwrap()
            .insert(lang.to_string().into(), config.clone());
    }

    pub fn languages(&self) -> Vec<SharedString> {
        self.languages.lock().unwrap().keys().cloned().collect()
    }

    pub fn language(&self, name: &str) -> Option<LanguageConfig> {
        self.languages.lock().unwrap().get(name).cloned()
    }
}
