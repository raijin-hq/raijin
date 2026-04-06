use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use inazuma::{AssetSource, Global, SharedString};

use crate::loader::load_theme_from_toml;
use crate::theme::{Theme, ThemeFamily};

/// Metadata about a registered theme, used for listing without loading full theme data.
#[derive(Clone, Debug)]
pub struct ThemeMeta {
    /// The theme's unique identifier.
    pub id: String,
    /// The theme's display name.
    pub name: SharedString,
    /// The theme's appearance (light/dark).
    pub appearance: crate::theme::Appearance,
}

/// A registry that stores and retrieves themes by display name or id.
///
/// Themes are stored by display name. An additional id→name index allows
/// lookup by theme file stem (e.g. "raijin-dark" → "Raijin Dark").
#[derive(Clone, Debug)]
pub struct ThemeRegistry {
    themes: HashMap<SharedString, Arc<Theme>>,
    /// Maps theme id (file stem) to display name for dual-key lookup.
    id_index: HashMap<String, SharedString>,
}

impl ThemeRegistry {
    /// Creates an empty theme registry.
    pub fn new() -> Self {
        Self {
            themes: HashMap::new(),
            id_index: HashMap::new(),
        }
    }

    /// Retrieves a theme by display name or id.
    ///
    /// Tries display name first, then falls back to id lookup.
    pub fn get(&self, key: &SharedString) -> Result<Arc<Theme>> {
        // Try direct name lookup
        if let Some(theme) = self.themes.get(key) {
            return Ok(theme.clone());
        }

        // Try id→name lookup
        if let Some(name) = self.id_index.get(key.as_ref()) {
            if let Some(theme) = self.themes.get(name) {
                return Ok(theme.clone());
            }
        }

        Err(anyhow!("theme not found: {}", key))
    }

    /// Returns metadata for all registered themes.
    pub fn list(&self) -> Vec<ThemeMeta> {
        self.themes
            .values()
            .map(|theme| ThemeMeta {
                id: theme.id.clone(),
                name: theme.name.clone(),
                appearance: theme.appearance,
            })
            .collect()
    }

    /// Inserts a single theme into the registry, indexed by both name and id.
    pub fn insert_theme(&mut self, theme: Theme) {
        let name = theme.name.clone();
        let id = theme.id.clone();
        self.themes.insert(name.clone(), Arc::new(theme));
        self.id_index.insert(id, name);
    }

    /// Inserts all themes from the given theme families into the registry.
    pub fn insert_theme_families(&mut self, families: impl IntoIterator<Item = ThemeFamily>) {
        for family in families {
            for theme in family.themes {
                self.insert_theme(theme);
            }
        }
    }

    /// Loads all bundled themes from the asset source.
    ///
    /// Expects directory-per-theme structure: `themes/{id}/theme.toml`.
    /// Lists `themes/` via `AssetSource`, finds all `theme.toml` files,
    /// parses them, and inserts into the registry.
    pub fn load_bundled_themes(&mut self, assets: &dyn AssetSource) {
        let all_paths = match assets.list("themes/") {
            Ok(paths) => paths,
            Err(err) => {
                log::error!("Failed to list bundled theme assets: {err}");
                return;
            }
        };

        // Filter for theme.toml files: "themes/raijin-dark/theme.toml"
        for path in all_paths {
            if !path.ends_with("/theme.toml") {
                continue;
            }

            let bytes = match assets.load(&path) {
                Ok(Some(bytes)) => bytes,
                Ok(None) => {
                    log::warn!("Bundled theme file not found: {path}");
                    continue;
                }
                Err(err) => {
                    log::error!("Failed to load bundled theme '{path}': {err}");
                    continue;
                }
            };

            let content = match std::str::from_utf8(&bytes) {
                Ok(s) => s,
                Err(err) => {
                    log::error!("Bundled theme '{path}' is not valid UTF-8: {err}");
                    continue;
                }
            };

            // Extract id from path: "themes/raijin-dark/theme.toml" → "raijin-dark"
            let id = path
                .trim_start_matches("themes/")
                .trim_end_matches("/theme.toml");

            match load_theme_from_toml(id, content) {
                Ok(theme) => {
                    log::info!("Loaded bundled theme: {} ({})", theme.name, id);
                    self.insert_theme(theme);
                }
                Err(err) => {
                    log::error!("Failed to parse bundled theme '{path}': {err}");
                }
            }
        }
    }

    /// Removes themes by name from the registry.
    pub fn remove_themes(&mut self, names: &[SharedString]) {
        for name in names {
            if let Some(theme) = self.themes.remove(name) {
                self.id_index.remove(&theme.id);
            }
        }
    }
}

impl Global for ThemeRegistry {}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
