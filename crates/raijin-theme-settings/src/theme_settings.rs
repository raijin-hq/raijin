use std::sync::Arc;

use inazuma::{App, AssetSource, Font, Pixels};
use inazuma_settings_framework::{Settings, SettingsStore};
use raijin_theme::{
    Appearance, DEFAULT_DARK_THEME, GlobalTheme, LoadThemes, ThemeRegistry,
    ThemeSettingsProvider, UiDensity, icon_theme::default_icon_theme,
    set_theme_settings_provider,
};

use crate::settings::{
    ThemeSettings, default_theme, reset_buffer_font_size, reset_ui_font_size,
};

struct ThemeSettingsProviderImpl;

impl ThemeSettingsProvider for ThemeSettingsProviderImpl {
    fn ui_font<'a>(&'a self, cx: &'a App) -> &'a Font {
        &ThemeSettings::get_global(cx).ui_font
    }

    fn buffer_font<'a>(&'a self, cx: &'a App) -> &'a Font {
        &ThemeSettings::get_global(cx).buffer_font
    }

    fn ui_font_size(&self, cx: &App) -> Pixels {
        ThemeSettings::get_global(cx).ui_font_size(cx)
    }

    fn buffer_font_size(&self, cx: &App) -> Pixels {
        ThemeSettings::get_global(cx).buffer_font_size(cx)
    }

    fn ui_density(&self, cx: &App) -> UiDensity {
        ThemeSettings::get_global(cx).ui_density
    }
}

/// Initialize the theme system with settings integration.
///
/// Follows Zed's pattern: `theme::init` + settings wiring.
/// The caller passes `LoadThemes::All(Box::new(Assets))` from main.
pub fn init(themes_to_load: LoadThemes, cx: &mut App) {
    let load_user_themes = matches!(&themes_to_load, LoadThemes::All(_));

    // 1. Set up ThemeRegistry as global (Zed: theme::init)
    let assets: Box<dyn AssetSource> = match themes_to_load {
        LoadThemes::JustBase => Box::new(()) as Box<dyn AssetSource>,
        LoadThemes::All(assets) => assets,
    };
    ThemeRegistry::set_global(assets, cx);

    // 2. Register ThemeSettingsProvider — without this, raijin-ui crates crash
    set_theme_settings_provider(Box::new(ThemeSettingsProviderImpl), cx);

    // 3. Load bundled TOML themes from assets
    if load_user_themes {
        let registry = ThemeRegistry::global(cx);
        registry.load_bundled_themes(registry.assets());
    }

    // 4. Load user themes from ~/.raijin/themes/
    if load_user_themes {
        let registry = ThemeRegistry::global(cx);
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
                    match std::fs::read_to_string(&theme_toml) {
                        Ok(content) => {
                            match raijin_theme::load_theme_from_toml_with_base_dir(
                                &id, &content, Some(path),
                            ) {
                                Ok(theme) => {
                                    log::info!("Loaded user theme: {} ({})", theme.name, id);
                                    registry.insert_theme(theme);
                                }
                                Err(err) => {
                                    log::warn!(
                                        "Failed to parse user theme '{}': {}",
                                        theme_toml.display(),
                                        err
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            log::warn!(
                                "Failed to read theme file '{}': {}",
                                theme_toml.display(),
                                err
                            );
                        }
                    }
                }
            }
        }
    }

    // 5. Resolve and apply configured theme
    let theme = configured_theme(cx);
    let icon_theme = default_icon_theme();
    GlobalTheme::update_theme(cx, theme);
    GlobalTheme::update_icon_theme(cx, icon_theme);

    // 6. Set up SettingsStore observer for live theme/font reloading
    let settings = ThemeSettings::get_global(cx);
    let mut prev_buffer_font_size_settings = settings.buffer_font_size_settings();
    let mut prev_ui_font_size_settings = settings.ui_font_size_settings();
    let mut prev_theme_name = settings.theme.name(Appearance::Dark);
    let mut prev_icon_theme_name = settings.icon_theme.name(Appearance::Dark);
    let mut prev_theme_overrides = (
        settings.experimental_theme_overrides.clone(),
        settings.theme_overrides.clone(),
    );

    cx.observe_global::<SettingsStore>(move |cx| {
        let settings = ThemeSettings::get_global(cx);

        let buffer_font_size_settings = settings.buffer_font_size_settings();
        let ui_font_size_settings = settings.ui_font_size_settings();
        let theme_name = settings.theme.name(Appearance::Dark);
        let icon_theme_name = settings.icon_theme.name(Appearance::Dark);
        let theme_overrides = (
            settings.experimental_theme_overrides.clone(),
            settings.theme_overrides.clone(),
        );

        if buffer_font_size_settings != prev_buffer_font_size_settings {
            prev_buffer_font_size_settings = buffer_font_size_settings;
            reset_buffer_font_size(cx);
        }

        if ui_font_size_settings != prev_ui_font_size_settings {
            prev_ui_font_size_settings = ui_font_size_settings;
            reset_ui_font_size(cx);
        }

        if theme_name != prev_theme_name || theme_overrides != prev_theme_overrides {
            prev_theme_name = theme_name;
            prev_theme_overrides = theme_overrides;
            reload_theme(cx);
        }

        if icon_theme_name != prev_icon_theme_name {
            prev_icon_theme_name = icon_theme_name;
            reload_icon_theme(cx);
        }
    })
    .detach();

    log::info!("Theme system initialized");
}

fn configured_theme(cx: &mut App) -> Arc<raijin_theme::Theme> {
    let registry = ThemeRegistry::global(cx);
    let theme_settings = ThemeSettings::get_global(cx);

    let theme_name = theme_settings.theme.name(Appearance::Dark);

    let theme = match registry.get(&theme_name.0) {
        Ok(theme) => theme,
        Err(_) => {
            log::warn!(
                "Theme '{}' not found, falling back to '{}'",
                theme_name.0,
                DEFAULT_DARK_THEME
            );
            registry
                .get(default_theme(Appearance::Dark))
                .unwrap_or_else(|_| registry.get(DEFAULT_DARK_THEME).unwrap())
        }
    };
    theme_settings.apply_theme_overrides(theme)
}

/// Reloads the current theme from settings.
pub fn reload_theme(cx: &mut App) {
    let theme = configured_theme(cx);
    GlobalTheme::update_theme(cx, theme);
    cx.refresh_windows();
}

/// Reloads the current icon theme from settings.
pub fn reload_icon_theme(cx: &mut App) {
    let registry = ThemeRegistry::global(cx);
    let icon_theme = registry
        .default_icon_theme()
        .unwrap_or_else(|_| default_icon_theme());
    GlobalTheme::update_icon_theme(cx, icon_theme);
    cx.refresh_windows();
}
