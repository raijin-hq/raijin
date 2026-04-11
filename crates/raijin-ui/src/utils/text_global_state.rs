use inazuma::{App, Entity, Global};

use crate::text::TextViewState;

/// Tracks the text view state stack for nested text views.
///
/// When a `TextView` element is painted, it pushes its state onto this stack
/// so that child `Inline` elements can access it for selection tracking.
pub struct TextGlobalState {
    pub text_view_state_stack: Vec<Entity<TextViewState>>,
}

impl Global for TextGlobalState {}

impl TextGlobalState {
    pub fn init(cx: &mut App) {
        cx.set_global(Self {
            text_view_state_stack: Vec::new(),
        });
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Returns the current (topmost) text view state, if any.
    pub fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }
}
