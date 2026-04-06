use std::borrow::Cow;

use inazuma::{AssetSource, Result, SharedString};

/// Raijin-specific bundled assets: themes, fonts, keymaps.
///
/// Embedded at compile time from the repository's `assets/` directory via RustEmbed.
/// Implements `AssetSource` with fallback to `inazuma_component_assets::Assets`
/// for component icons and UI fonts.
#[derive(rust_embed::RustEmbed)]
#[folder = "../../assets"]
#[include = "themes/**/*"]
#[include = "fonts/**/*"]
#[include = "keymaps/**/*"]
pub struct RaijinAssets;

/// Composite asset source: Raijin assets first, then inazuma-component assets as fallback.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        // Try Raijin assets first (themes, fonts, keymaps)
        if let Some(file) = RaijinAssets::get(path) {
            return Ok(Some(file.data));
        }

        // Fall back to inazuma-component assets (icons, UI fonts)
        inazuma_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        // Collect from both sources, Raijin first
        let mut results: Vec<SharedString> = RaijinAssets::iter()
            .filter(|p| p.starts_with(path))
            .map(|p| p.into())
            .collect();

        // Add component assets
        if let Ok(component_assets) = inazuma_component_assets::Assets.list(path) {
            results.extend(component_assets);
        }

        Ok(results)
    }
}
