use std::any::Any;
use std::sync::Arc;

use inazuma::{AnyElement, App, AppContext, Entity, FocusHandle, IntoElement, Subscription, Window};

use crate::{ErasedEditor, ErasedEditorEvent, ERASED_EDITOR_FACTORY};
use crate::input::{Input, InputEvent, InputState};

/// Wrapper that implements `ErasedEditor` for `Entity<InputState>`.
#[derive(Clone)]
struct InputEditor(Entity<InputState>);

impl ErasedEditor for InputEditor {
    fn text(&self, cx: &App) -> String {
        self.0.read(cx).value().to_string()
    }

    fn set_text(&self, text: &str, window: &mut Window, cx: &mut App) {
        let text = text.to_string();
        self.0.update(cx, |state, cx| {
            state.set_value(text, window, cx);
        });
    }

    fn clear(&self, window: &mut Window, cx: &mut App) {
        self.0.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
    }

    fn set_placeholder_text(&self, text: &str, window: &mut Window, cx: &mut App) {
        let text = text.to_string();
        self.0.update(cx, |state, cx| {
            state.set_placeholder(text, window, cx);
        });
    }

    fn move_selection_to_end(&self, _window: &mut Window, _cx: &mut App) {
        // InputState handles cursor position internally
    }

    fn set_masked(&self, _masked: bool, _window: &mut Window, _cx: &mut App) {
        // InputState supports masking via Input::mask_toggle, not directly on state
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.0.read(cx).focus_handle.clone()
    }

    fn subscribe(
        &self,
        mut callback: Box<dyn FnMut(ErasedEditorEvent, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut App,
    ) -> Subscription {
        window.subscribe(&self.0, cx, move |_, event: &InputEvent, window, cx| {
            let event = match event {
                InputEvent::Change => ErasedEditorEvent::BufferEdited,
                InputEvent::Blur => ErasedEditorEvent::Blurred,
                _ => return,
            };
            (callback)(event, window, cx);
        })
    }

    fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement {
        Input::new(&self.0).appearance(false).cleanable(false).into_any_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The factory function that creates an InputState-based ErasedEditor.
pub fn input_editor_factory() -> fn(&mut Window, &mut App) -> Arc<dyn ErasedEditor> {
    |window: &mut Window, cx: &mut App| -> Arc<dyn ErasedEditor> {
        let state = cx.new(|cx| InputState::new(window, cx));
        Arc::new(InputEditor(state))
    }
}

/// Register the `InputState`-based editor factory on the raijin-ui ERASED_EDITOR_FACTORY.
pub fn register_input_editor_factory() {
    let _ = ERASED_EDITOR_FACTORY.set(input_editor_factory());
}
