use std::collections::HashMap;
use std::sync::Arc;

use inazuma::{App, Global, SharedString};
use raijin_theme::{
    GlobalTheme, ThemeColorsRefinement, ThemeFamily, ThemeRegistry, fallback_theme,
};

/// Controls how the theme appearance is determined.
#[derive(Clone, Debug)]
pub enum ThemeAppearanceMode {
    /// Follow the OS-level light/dark setting.
    System,
    /// Always use a light theme.
    Light,
    /// Always use a dark theme.
    Dark,
}

/// Describes which theme(s) to use.
#[derive(Clone, Debug)]
pub enum ThemeSelection {
    /// A single theme used regardless of system appearance.
    Static(SharedString),
    /// Separate themes for light and dark system appearance.
    Dynamic {
        mode: ThemeAppearanceMode,
        light: SharedString,
        dark: SharedString,
    },
}

/// Application-wide theme settings, stored as a global in the Inazuma app context.
///
/// Manages the active theme selection, per-theme color overrides, and an optional
/// experimental override layer applied on top of everything.
pub struct ThemeSettings {
    /// The current theme selection strategy.
    pub theme: ThemeSelection,
    /// Per-theme color overrides keyed by theme name.
    pub theme_overrides: HashMap<String, ThemeColorsRefinement>,
    /// An experimental override layer applied on top of the resolved theme.
    pub experimental_theme_overrides: Option<ThemeColorsRefinement>,
}

impl Global for ThemeSettings {}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            theme: ThemeSelection::Static(SharedString::from("Raijin Dark")),
            theme_overrides: HashMap::new(),
            experimental_theme_overrides: None,
        }
    }
}

/// Initializes the theme system.
///
/// 1. Creates a `ThemeRegistry` with the fallback theme family loaded.
/// 2. Sets the registry as a global on the app context.
/// 3. Sets `ThemeSettings` as a global with its default values.
/// 4. Sets the `GlobalTheme` to the fallback theme.
pub fn init(cx: &mut App) {
    let fallback = fallback_theme();
    let family = ThemeFamily {
        id: "raijin".into(),
        name: SharedString::from("Raijin"),
        author: "Raijin".into(),
        themes: vec![fallback.clone()],
    };

    let mut registry = ThemeRegistry::new();
    registry.insert_theme_families(vec![family]);

    cx.set_global(registry);
    cx.set_global(ThemeSettings::default());
    cx.set_global(GlobalTheme(Arc::new(fallback)));

    log::info!("Theme system initialized with fallback theme");
}

/// Reloads the active theme from the current `ThemeSettings` and `ThemeRegistry`.
///
/// Resolves the theme name from `ThemeSettings`, looks it up in the registry,
/// and updates `GlobalTheme`. Falls back to the hardcoded fallback theme if
/// the selected theme is not found in the registry.
pub fn reload_theme(cx: &mut App) {
    let settings = cx.global::<ThemeSettings>();
    let theme_name = match &settings.theme {
        ThemeSelection::Static(name) => name.clone(),
        ThemeSelection::Dynamic { mode, light, dark } => match mode {
            ThemeAppearanceMode::Light => light.clone(),
            ThemeAppearanceMode::Dark => dark.clone(),
            ThemeAppearanceMode::System => {
                // Default to dark when system appearance detection is not yet implemented.
                dark.clone()
            }
        },
    };

    let registry = cx.global::<ThemeRegistry>();
    let theme = match registry.get(&theme_name) {
        Ok(theme) => theme,
        Err(err) => {
            log::warn!(
                "Failed to load theme '{}': {}. Using fallback.",
                theme_name,
                err
            );
            Arc::new(fallback_theme())
        }
    };

    cx.set_global(GlobalTheme(theme));
    log::info!("Theme reloaded: {}", theme_name);
}
