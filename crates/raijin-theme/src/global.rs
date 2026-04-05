use std::sync::Arc;

use inazuma::{App, Global};

use crate::theme::Theme;

/// Global theme state, stored in the application context.
///
/// Access via `cx.global::<GlobalTheme>()` or the `ActiveTheme` trait.
pub struct GlobalTheme(pub Arc<Theme>);

impl Global for GlobalTheme {}

/// Provides convenient access to the active theme from any context
/// that can dereference to `App`.
pub trait ActiveTheme {
    /// Returns a reference to the currently active theme.
    fn theme(&self) -> &Arc<Theme>;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Arc<Theme> {
        &self.global::<GlobalTheme>().0
    }
}
