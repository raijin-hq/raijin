//! Internationalization for Raijin.
//!
//! Provides the `t!()` macro for runtime translations with English as the
//! base language and German as the second language. Extensible to any language
//! by adding entries to the YAML files in `locales/`.
//!
//! # Usage
//!
//! ```ignore
//! use raijin_i18n::t;
//!
//! let label = t!("Dialog.ok");       // "OK" (en) / "OK" (de)
//! let month = t!("Calendar.month.January"); // "January" / "Januar"
//! ```
//!
//! # Adding a new language
//!
//! Edit `locales/ui.yml` and add a new language key under each entry:
//!
//! ```yaml
//! Dialog:
//!   ok:
//!     en: OK
//!     de: OK
//!     fr: OK        # ← new language
//! ```
//!
//! # Switching the active language at runtime
//!
//! ```ignore
//! raijin_i18n::set_locale("de");
//! ```

// Initialize translations from this crate's locales/ directory.
rust_i18n::i18n!("locales", fallback = "en");

/// Translate a key using the active locale.
///
/// This is a function wrapper around the `rust_i18n::t!()` macro so that
/// other crates can call it without needing `i18n!()` initialization.
pub fn translate(key: &str) -> String {
    _rust_i18n_translate(rust_i18n::locale().as_ref(), key).to_string()
}

/// Translate a key with explicit locale.
pub fn translate_with_locale(locale: &str, key: &str) -> String {
    _rust_i18n_translate(locale, key).to_string()
}

/// The `t!()` macro for translations. Works across crate boundaries.
///
/// ```ignore
/// use raijin_i18n::t;
/// let s = t!("Dialog.ok"); // returns String
/// ```
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::translate($key)
    };
    ($key:expr, $($arg:tt)*) => {
        $crate::translate($key)
    };
}

/// Set the active locale at runtime (e.g. `"en"`, `"de"`, `"zh-CN"`).
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale);
}

/// Get the currently active locale.
pub fn locale() -> String {
    rust_i18n::locale().to_string()
}

/// List all available locales compiled into the binary.
pub fn available_locales() -> Vec<&'static str> {
    rust_i18n::available_locales!()
}
