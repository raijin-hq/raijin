use std::sync::Arc;

use inazuma::{App, BorrowAppContext, Global};

use crate::icon_theme::IconTheme;
use crate::theme::Theme;

/// The active theme and icon theme, stored globally in the application context.
///
/// Access via `GlobalTheme::theme(cx)` / `GlobalTheme::icon_theme(cx)` or the
/// `ActiveTheme` trait.
pub struct GlobalTheme {
    theme: Arc<Theme>,
    icon_theme: Arc<IconTheme>,
}

impl Global for GlobalTheme {}

impl GlobalTheme {
    /// Creates a new [`GlobalTheme`] with the given theme and icon theme.
    pub fn new(theme: Arc<Theme>, icon_theme: Arc<IconTheme>) -> Self {
        Self { theme, icon_theme }
    }

    /// Updates the active theme.
    pub fn update_theme(cx: &mut App, theme: Arc<Theme>) {
        cx.update_global::<Self, _>(|this, _| this.theme = theme);
    }

    /// Updates the active icon theme.
    pub fn update_icon_theme(cx: &mut App, icon_theme: Arc<IconTheme>) {
        cx.update_global::<Self, _>(|this, _| this.icon_theme = icon_theme);
    }

    /// Returns the active theme.
    pub fn theme(cx: &App) -> &Arc<Theme> {
        &cx.global::<Self>().theme
    }

    /// Returns the active icon theme.
    pub fn icon_theme(cx: &App) -> &Arc<IconTheme> {
        &cx.global::<Self>().icon_theme
    }
}

/// Provides convenient access to the active theme from any context
/// that can dereference to `App`.
pub trait ActiveTheme {
    /// Returns a reference to the currently active theme.
    fn theme(&self) -> &Arc<Theme>;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Arc<Theme> {
        GlobalTheme::theme(self)
    }
}
