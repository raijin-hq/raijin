use std::any::Any;
use std::sync::{Arc, OnceLock};

use inazuma::{AnyElement, App, FocusHandle, Subscription, Window};

/// An abstraction over any text-editing widget (Input, Editor, etc.).
///
/// This is the bridge between the Picker and the input mechanism.
/// Currently implemented by `InputState`. When the full Editor is ported,
/// it will also implement this trait — and the Picker works with both
/// without any changes.
///
/// Ported from Zed's `ui_input::ErasedEditor`.
pub trait ErasedEditor: 'static {
    fn text(&self, cx: &App) -> String;
    fn set_text(&self, text: &str, window: &mut Window, cx: &mut App);
    fn clear(&self, window: &mut Window, cx: &mut App);
    fn set_placeholder_text(&self, text: &str, window: &mut Window, cx: &mut App);
    fn move_selection_to_end(&self, window: &mut Window, cx: &mut App);
    fn set_masked(&self, masked: bool, window: &mut Window, cx: &mut App);
    fn focus_handle(&self, cx: &App) -> FocusHandle;
    fn subscribe(
        &self,
        callback: Box<dyn FnMut(ErasedEditorEvent, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut App,
    ) -> Subscription;
    fn render(&self, window: &mut Window, cx: &App) -> AnyElement;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ErasedEditorEvent {
    BufferEdited,
    Blurred,
}

/// Global factory for creating erased editors.
/// Set this at app startup to configure which editor implementation the Picker uses.
pub static ERASED_EDITOR_FACTORY: OnceLock<fn(&mut Window, &mut App) -> Arc<dyn ErasedEditor>> =
    OnceLock::new();
