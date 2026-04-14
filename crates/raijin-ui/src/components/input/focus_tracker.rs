use inazuma::{Entity, Global};
use super::state::InputState;

/// Tracks which input currently has focus, as a window-global.
///
/// Replaces the previous pattern where this was stored on AppShell directly.
/// Components read/write this via `cx.global::<FocusedInputTracker>()`.
#[derive(Default)]
pub struct FocusedInputTracker {
    pub focused: Option<Entity<InputState>>,
}

impl Global for FocusedInputTracker {}
