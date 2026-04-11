use std::collections::HashSet;

use inazuma::{App, ElementId, FocusHandle, Global};

/// Tracks which popovers are currently open with deferred rendering.
///
/// When this set is not empty, at least one deferred popover context is active.
/// This prevents double-deferred elements which would cause the framework to panic.
pub struct PopoverGlobalState {
    open_deferred_popovers: HashSet<ElementId>,
}

impl Global for PopoverGlobalState {}

impl PopoverGlobalState {
    pub fn init(cx: &mut App) {
        cx.set_global(Self {
            open_deferred_popovers: HashSet::new(),
        });
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn is_in_deferred_context(&self) -> bool {
        !self.open_deferred_popovers.is_empty()
    }

    pub fn register_deferred_popover(&mut self, focus_handle: &FocusHandle) {
        self.open_deferred_popovers
            .insert(format!("{focus_handle:?}").into());
    }

    pub fn unregister_deferred_popover(&mut self, focus_handle: &FocusHandle) {
        let element_id: ElementId = format!("{focus_handle:?}").into();
        self.open_deferred_popovers.remove(&element_id);
    }
}
