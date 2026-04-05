use std::sync::Arc;

use inazuma::{
    App, Context, DismissEvent, EventEmitter, Focusable, FocusHandle, Render, SharedString,
    Window, div, px, ParentElement, Styled, IntoElement, StatefulInteractiveElement, prelude::*,
};
use raijin_theme::{ActiveTheme, Appearance, GlobalTheme, ThemeMeta, ThemeRegistry};

/// A modal picker for selecting themes from the registry.
///
/// On open, captures the current theme so it can revert on dismiss.
/// Supports fuzzy filtering by name and displays light/dark appearance tags.
pub struct ThemeSelector {
    /// All available themes from the registry.
    themes: Vec<ThemeMeta>,
    /// Filtered themes matching the current query.
    filtered: Vec<usize>,
    /// Current search query string.
    query: String,
    /// Index of the currently highlighted item in the filtered list.
    selected_index: usize,
    /// The theme that was active when the selector opened (for reverting on dismiss).
    original_theme: Arc<raijin_theme::Theme>,
    /// Shared theme registry for looking up full theme data.
    registry: Arc<parking_lot::RwLock<ThemeRegistry>>,
    /// Focus handle for this view.
    focus_handle: FocusHandle,
}

impl EventEmitter<DismissEvent> for ThemeSelector {}

impl ThemeSelector {
    /// Creates a new theme selector, capturing the current theme for revert-on-dismiss.
    pub fn new(
        registry: Arc<parking_lot::RwLock<ThemeRegistry>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let original_theme = cx.theme().clone();
        let themes = registry.read().list();
        let filtered: Vec<usize> = (0..themes.len()).collect();

        Self {
            themes,
            filtered,
            query: String::new(),
            selected_index: 0,
            original_theme,
            registry,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Updates the search query and refilters the theme list.
    pub fn set_query(&mut self, query: String, _cx: &mut Context<Self>) {
        self.query = query;
        self.refilter();
        self.selected_index = 0;
    }

    /// Moves selection up by one item.
    pub fn select_prev(&mut self, _cx: &mut Context<Self>) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Moves selection down by one item.
    pub fn select_next(&mut self, _cx: &mut Context<Self>) {
        if self.selected_index + 1 < self.filtered.len() {
            self.selected_index += 1;
        }
    }

    /// Confirms the currently selected theme, applying it globally.
    pub fn confirm(&mut self, cx: &mut Context<Self>) {
        if let Some(&idx) = self.filtered.get(self.selected_index) {
            let meta = &self.themes[idx];
            let name = meta.name.clone();
            let registry = self.registry.read();
            if let Ok(theme) = registry.get(&name) {
                cx.update_global::<GlobalTheme, _>(|global, _cx| {
                    global.0 = theme;
                });
            }
        }
        cx.emit(DismissEvent);
    }

    /// Dismisses the selector and reverts to the original theme.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.update_global::<GlobalTheme, _>(|global, _cx| {
            global.0 = self.original_theme.clone();
        });
        cx.emit(DismissEvent);
    }

    /// Returns the currently selected theme meta, if any.
    pub fn selected_theme(&self) -> Option<&ThemeMeta> {
        self.filtered
            .get(self.selected_index)
            .map(|&idx| &self.themes[idx])
    }

    /// Refilters the theme list based on the current query using case-insensitive substring matching.
    fn refilter(&mut self) {
        let query_lower = self.query.to_lowercase();
        self.filtered = self
            .themes
            .iter()
            .enumerate()
            .filter(|(_, meta)| {
                if query_lower.is_empty() {
                    return true;
                }
                let name_lower = meta.name.to_lowercase();
                fuzzy_match(&name_lower, &query_lower)
            })
            .map(|(i, _)| i)
            .collect();
    }
}

/// Simple fuzzy match: all characters in the pattern appear in order in the haystack.
fn fuzzy_match(haystack: &str, pattern: &str) -> bool {
    let mut haystack_chars = haystack.chars();
    for pattern_char in pattern.chars() {
        loop {
            match haystack_chars.next() {
                Some(c) if c == pattern_char => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

impl Focusable for ThemeSelector {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ThemeSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = cx;
        let mut items = Vec::new();

        for (display_idx, &theme_idx) in self.filtered.iter().enumerate() {
            let meta = &self.themes[theme_idx];
            let is_selected = display_idx == self.selected_index;
            let appearance_label: SharedString = match meta.appearance {
                Appearance::Light => "Light".into(),
                Appearance::Dark => "Dark".into(),
            };

            let row = div()
                .flex()
                .flex_row()
                .justify_between()
                .px(px(8.0))
                .py(px(4.0))
                .when(is_selected, |el| {
                    el.bg(inazuma::rgba(0xffffff20))
                })
                .child(
                    div()
                        .flex()
                        .child(meta.name.clone())
                )
                .child(
                    div()
                        .flex()
                        .px(px(4.0))
                        .child(appearance_label)
                );

            items.push(row);
        }

        div()
            .id("theme-selector")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .w(px(400.0))
            .max_h(px(500.0))
            .overflow_y_scroll()
            .bg(inazuma::rgba(0x1e1e1eff))
            .border_1()
            .border_color(inazuma::rgba(0x333333ff))
            .rounded(px(8.0))
            .p(px(8.0))
            .children(items)
    }
}
