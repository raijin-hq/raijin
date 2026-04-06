use std::collections::HashMap;

use inazuma::{App, Global, SharedString};
use raijin_theme::{
    DEFAULT_DARK_THEME, GlobalTheme, ThemeColorsRefinement, ThemeRegistry,
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
            theme: ThemeSelection::Static(SharedString::from(DEFAULT_DARK_THEME)),
            theme_overrides: HashMap::new(),
            experimental_theme_overrides: None,
        }
    }
}

/// Initializes the theme system.
///
/// 1. Parses all bundled TOML themes (compiled into the binary).
/// 2. Scans `~/.raijin/themes/*.toml` for user themes.
/// 3. Builds the `ThemeRegistry` with all themes.
/// 4. Reads the selected theme from `RaijinSettings`.
/// 5. Sets `ThemeRegistry`, `ThemeSettings`, and `GlobalTheme` as globals.
pub fn init(cx: &mut App) {
    let mut registry = ThemeRegistry::new();

    // --- Load bundled themes via asset pipeline ---
    registry.load_bundled_themes(cx.asset_source().as_ref());

    // --- Load user themes from ~/.raijin/themes/ ---
    // One format: each theme is a directory with theme.toml inside.
    //   ~/.raijin/themes/my-theme/theme.toml (+ optional assets like wallpaper.png)
    let themes_dir = raijin_settings::RaijinSettings::themes_dir();
    if themes_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&themes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let theme_toml = path.join("theme.toml");
                if !theme_toml.exists() {
                    continue;
                }

                let id = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let toml_path = theme_toml;
                let base_dir = path;

                match std::fs::read_to_string(&toml_path) {
                    Ok(content) => {
                        match raijin_theme::load_theme_from_toml_with_base_dir(
                            &id,
                            &content,
                            Some(base_dir),
                        ) {
                            Ok(theme) => {
                                log::info!(
                                    "Loaded user theme: {} ({}) from {}",
                                    theme.name,
                                    id,
                                    toml_path.display()
                                );
                                registry.insert_theme(theme);
                            }
                            Err(err) => {
                                log::warn!(
                                    "Failed to parse user theme '{}': {}",
                                    toml_path.display(),
                                    err
                                );
                            }
                        }
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to read theme file '{}': {}",
                            toml_path.display(),
                            err
                        );
                    }
                }
            }
        }
    }

    // --- Resolve initial theme from settings ---
    let theme_name = cx
        .try_global::<raijin_settings::RaijinSettings>()
        .map(|s| SharedString::from(s.theme.theme_name().to_string()))
        .unwrap_or_else(|| SharedString::from(DEFAULT_DARK_THEME));

    let theme = registry
        .get(&theme_name)
        .unwrap_or_else(|_| {
            log::warn!(
                "Configured theme '{}' not found, falling back to '{}'",
                theme_name,
                DEFAULT_DARK_THEME,
            );
            registry
                .get(&SharedString::from(DEFAULT_DARK_THEME))
                .unwrap_or_else(|_| {
                    let themes = registry.list();
                    let first = themes
                        .first()
                        .expect("No themes available in registry — cannot initialize theme system");
                    registry.get(&first.name).unwrap()
                })
        });

    let selection = match cx.try_global::<raijin_settings::RaijinSettings>() {
        Some(s) => match s.theme.mode {
            raijin_settings::ThemeMode::System => ThemeSelection::Dynamic {
                mode: ThemeAppearanceMode::System,
                light: SharedString::from(s.theme.light.clone()),
                dark: SharedString::from(s.theme.dark.clone()),
            },
            raijin_settings::ThemeMode::Light => ThemeSelection::Dynamic {
                mode: ThemeAppearanceMode::Light,
                light: SharedString::from(s.theme.light.clone()),
                dark: SharedString::from(s.theme.dark.clone()),
            },
            raijin_settings::ThemeMode::Dark => ThemeSelection::Static(theme_name.clone()),
        },
        None => ThemeSelection::Static(SharedString::from(DEFAULT_DARK_THEME)),
    };

    cx.set_global(registry);
    cx.set_global(ThemeSettings {
        theme: selection,
        ..ThemeSettings::default()
    });
    cx.set_global(GlobalTheme(theme));

    log::info!("Theme system initialized with '{}'", theme_name);
}

/// Reloads the active theme from the current `ThemeSettings` and `ThemeRegistry`.
///
/// Resolves the theme name from `ThemeSettings`, looks it up in the registry,
/// and updates `GlobalTheme`. Falls back to "Raijin Dark" if the selected
/// theme is not found in the registry.
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
                "Failed to load theme '{}': {}. Falling back to '{}'.",
                theme_name,
                err,
                DEFAULT_DARK_THEME,
            );
            registry
                .get(&SharedString::from(DEFAULT_DARK_THEME))
                .unwrap_or_else(|_| {
                    let themes = registry.list();
                    let first = themes
                        .first()
                        .expect("No themes in registry");
                    registry.get(&first.name).unwrap()
                })
        }
    };

    cx.set_global(GlobalTheme(theme));
    cx.refresh_windows();
    log::info!("Theme reloaded: {}", theme_name);
}
