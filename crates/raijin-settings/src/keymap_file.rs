use std::borrow::Cow;
use std::rc::Rc;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use inazuma::{App, DummyKeyboardMapper, KeyBinding, KeyBindingContextPredicate};
use serde::Deserialize;

/// Bundled assets for settings and keymaps — embedded at compile time.
/// Like Zed's `SettingsAssets`, each crate embeds only the assets it needs.
#[derive(rust_embed::RustEmbed)]
#[folder = "../../assets"]
#[include = "keymaps/*"]
pub struct SettingsAssets;

/// Loads a bundled asset as a UTF-8 string, panicking if not found.
fn asset_str(path: &str) -> Cow<'static, str> {
    match SettingsAssets::get(path).expect(path).data {
        Cow::Borrowed(bytes) => Cow::Borrowed(std::str::from_utf8(bytes).unwrap()),
        Cow::Owned(bytes) => Cow::Owned(String::from_utf8(bytes).unwrap()),
    }
}

/// A parsed keymap file containing one or more keybinding sections.
///
/// Each section has an optional context predicate and a set of keystroke→action bindings.
/// Sections are processed in order — later bindings for the same keystroke override earlier ones.
#[derive(Debug, Deserialize)]
pub struct KeymapFile {
    #[serde(rename = "section")]
    sections: Vec<KeymapSection>,
}

/// A single section of keybindings with an optional context restriction.
#[derive(Debug, Deserialize)]
struct KeymapSection {
    /// Context predicate string (e.g. "Terminal", "RaijinInput", "" for global).
    #[serde(default)]
    context: String,
    /// Ordered map of keystroke → action name.
    #[serde(default)]
    bindings: IndexMap<String, KeymapAction>,
}

/// A keymap action — either a simple action name or an action with parameters.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum KeymapAction {
    /// Simple action name: `"terminal::Copy"`
    Simple(String),
    /// Action with parameters: `{ action = "...", data = {...} }`
    WithData {
        action: String,
        data: serde_json::Value,
    },
}

impl KeymapFile {
    /// Parses a keymap from TOML content.
    pub fn load(content: &str) -> Result<Self> {
        toml::from_str(content).context("failed to parse keymap TOML")
    }

    /// Loads a keymap from a bundled asset path (e.g. "keymaps/default-macos.toml").
    pub fn load_asset(asset_path: &str, cx: &App) -> Result<Vec<KeyBinding>> {
        let content = asset_str(asset_path);
        let keymap = Self::load(&content)?;
        Ok(keymap.into_bindings(cx))
    }

    /// Converts the parsed keymap into a list of `KeyBinding` objects.
    ///
    /// Uses `cx.build_action()` to resolve action names to concrete action types.
    /// Invalid bindings are logged and skipped.
    pub fn into_bindings(self, cx: &App) -> Vec<KeyBinding> {
        let mut bindings = Vec::new();

        for section in self.sections {
            let context_predicate = if section.context.is_empty() {
                None
            } else {
                match KeyBindingContextPredicate::parse(&section.context) {
                    Ok(predicate) => Some(Rc::new(predicate)),
                    Err(err) => {
                        log::warn!(
                            "Invalid keymap context predicate '{}': {err}",
                            section.context
                        );
                        continue;
                    }
                }
            };

            for (keystroke, action_def) in section.bindings {
                let (action_name, action_data) = match &action_def {
                    KeymapAction::Simple(name) => (name.as_str(), None),
                    KeymapAction::WithData { action, data } => {
                        (action.as_str(), Some(data.clone()))
                    }
                };

                let action = match cx.build_action(action_name, action_data) {
                    Ok(action) => action,
                    Err(err) => {
                        log::warn!(
                            "Unknown action '{}' for keystroke '{}': {err}",
                            action_name,
                            keystroke
                        );
                        continue;
                    }
                };

                match KeyBinding::load(
                    &keystroke,
                    action,
                    context_predicate.clone(),
                    false,
                    None,
                    &DummyKeyboardMapper,
                ) {
                    Ok(binding) => bindings.push(binding),
                    Err(err) => {
                        log::warn!(
                            "Invalid keystroke '{}' for action '{}': {err}",
                            keystroke,
                            action_name
                        );
                    }
                }
            }
        }

        bindings
    }
}

/// Loads the default platform keymap + user keymap, merges them, and applies to the app.
///
/// Default keymap is loaded from bundled assets via `SettingsAssets` (compile-time embedded).
/// User keymap is loaded from `~/.raijin/keymap.toml` if it exists.
/// User bindings are appended after defaults — later bindings take priority.
pub fn load_default_and_user_keymap(cx: &mut App) {
    let mut all_bindings = Vec::new();

    // Load default keymap from bundled assets (compile-time, like Zed's SettingsAssets)
    let default_keymap_path = if cfg!(target_os = "macos") {
        "keymaps/default-macos.toml"
    } else if cfg!(target_os = "windows") {
        "keymaps/default-windows.toml"
    } else {
        "keymaps/default-linux.toml"
    };

    match KeymapFile::load_asset(default_keymap_path, cx) {
        Ok(bindings) => {
            log::info!(
                "Loaded {} default keybindings from {}",
                bindings.len(),
                default_keymap_path
            );
            all_bindings.extend(bindings);
        }
        Err(err) => {
            log::error!("Failed to load default keymap: {err}");
        }
    }

    // Load user keymap from ~/.raijin/keymap.toml (if exists)
    let user_keymap_path = raijin_paths::keymap_file().clone();
    if user_keymap_path.exists() {
        match std::fs::read_to_string(&user_keymap_path) {
            Ok(content) => match KeymapFile::load(&content) {
                Ok(keymap) => {
                    let bindings = keymap.into_bindings(cx);
                    log::info!(
                        "Loaded {} user keybindings from {}",
                        bindings.len(),
                        user_keymap_path.display()
                    );
                    all_bindings.extend(bindings);
                }
                Err(err) => {
                    log::warn!(
                        "Failed to parse user keymap '{}': {err}",
                        user_keymap_path.display()
                    );
                }
            },
            Err(err) => {
                log::warn!(
                    "Failed to read user keymap '{}': {err}",
                    user_keymap_path.display()
                );
            }
        }
    }

    cx.bind_keys(all_bindings);
    log::info!("Keybindings applied");
}
