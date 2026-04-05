use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use inazuma::{Global, SharedString};

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

/// A registry that stores and retrieves themes by name.
#[derive(Clone, Debug)]
pub struct ThemeRegistry {
    themes: HashMap<SharedString, Arc<Theme>>,
}

impl ThemeRegistry {
    /// Creates an empty theme registry.
    pub fn new() -> Self {
        Self {
            themes: HashMap::new(),
        }
    }

    /// Retrieves a theme by name.
    pub fn get(&self, name: &SharedString) -> Result<Arc<Theme>> {
        self.themes
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("theme not found: {}", name))
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

    /// Inserts all themes from the given theme families into the registry.
    pub fn insert_theme_families(&mut self, families: impl IntoIterator<Item = ThemeFamily>) {
        for family in families {
            for theme in family.themes {
                self.themes
                    .insert(theme.name.clone(), Arc::new(theme));
            }
        }
    }

    /// Removes themes by name from the registry.
    pub fn remove_themes(&mut self, names: &[SharedString]) {
        for name in names {
            self.themes.remove(name);
        }
    }
}

impl Global for ThemeRegistry {}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
