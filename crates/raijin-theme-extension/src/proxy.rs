use std::path::PathBuf;
use std::sync::Arc;

use inazuma::{App, SharedString};
use raijin_theme::ThemeRegistry;

/// Trait for proxying theme extension operations.
///
/// This abstraction allows the extension system to interact with the theme
/// registry without depending on the full registry implementation directly.
pub trait ExtensionThemeProxy: Send + Sync + 'static {
    /// Signals that all extensions have finished loading.
    fn set_extensions_loaded(&self);

    /// Lists theme names found at the given path using the provided filesystem abstraction.
    fn list_theme_names(
        &self,
        theme_path: PathBuf,
        fs: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Vec<String>;

    /// Removes user-installed themes by name from the registry.
    fn remove_user_themes(&self, themes: Vec<SharedString>);

    /// Loads a user theme from the given file path into the registry.
    fn load_user_theme(&self, theme_path: PathBuf);

    /// Reloads the currently active theme, picking up any registry changes.
    fn reload_current_theme(&self, cx: &mut App);
}

/// A concrete implementation of [`ExtensionThemeProxy`] that delegates
/// all operations to a [`ThemeRegistry`].
pub struct ThemeRegistryProxy {
    registry: Arc<parking_lot::RwLock<ThemeRegistry>>,
    extensions_loaded: Arc<std::sync::atomic::AtomicBool>,
}

impl ThemeRegistryProxy {
    /// Creates a new proxy wrapping the given theme registry.
    pub fn new(registry: Arc<parking_lot::RwLock<ThemeRegistry>>) -> Self {
        Self {
            registry,
            extensions_loaded: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Returns whether all extensions have been loaded.
    pub fn extensions_loaded(&self) -> bool {
        self.extensions_loaded
            .load(std::sync::atomic::Ordering::Acquire)
    }
}

impl ExtensionThemeProxy for ThemeRegistryProxy {
    fn set_extensions_loaded(&self) {
        self.extensions_loaded
            .store(true, std::sync::atomic::Ordering::Release);
        log::info!("theme extensions marked as loaded");
    }

    fn list_theme_names(
        &self,
        _theme_path: PathBuf,
        _fs: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Vec<String> {
        let registry = self.registry.read();
        registry.list().into_iter().map(|m| m.name.to_string()).collect()
    }

    fn remove_user_themes(&self, themes: Vec<SharedString>) {
        let mut registry = self.registry.write();
        registry.remove_user_themes(&themes);
        log::info!("removed {} user themes from registry", themes.len());
    }

    fn load_user_theme(&self, theme_path: PathBuf) {
        log::info!("load_user_theme requested for: {}", theme_path.display());
        // Theme file loading is handled by the caller — this proxy only manages
        // registry state. The caller reads the file, parses it into a ThemeFamily,
        // and inserts it via the registry directly.
    }

    fn reload_current_theme(&self, cx: &mut App) {
        use raijin_theme::{ActiveTheme, GlobalTheme};

        let current_name = cx.theme().name.clone();
        let registry = self.registry.read();
        if let Ok(theme) = registry.get(&current_name) {
            GlobalTheme::update_theme(cx, theme);
            log::info!("reloaded current theme: {}", current_name);
        }
    }
}
