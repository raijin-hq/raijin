use std::any::Any;
use std::sync::Arc;

use inazuma::{
    AnyElement, App, Entity, EventEmitter, FocusHandle, IntoElement, Subscription, Window,
};

use crate::erased_editor::{ErasedEditor, ErasedEditorEvent, ERASED_EDITOR_FACTORY};
use crate::input::{Input, InputEvent, InputState};

/// Wrapper that implements `ErasedEditor` for `Entity<InputState>`.
struct InputEditor(Entity<InputState>);

impl ErasedEditor for InputEditor {
    fn text(&self, cx: &App) -> String {
        self.0.read(cx).value().to_string()
    }

    fn set_text(&self, text: &str, _window: &mut Window, cx: &mut App) {
        self.0.update(cx, |state, cx| {
            state.set_value(text, cx);
        });
    }

    fn clear(&self, _window: &mut Window, cx: &mut App) {
        self.0.update(cx, |state, cx| {
            state.set_value("", cx);
        });
    }

    fn set_placeholder_text(&self, text: &str, _window: &mut Window, cx: &mut App) {
        self.0.update(cx, |state, cx| {
            state.set_placeholder(text, cx);
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
        _window: &mut Window,
        cx: &mut App,
    ) -> Subscription {
        cx.subscribe(&self.0, move |event: &InputEvent, cx| {
            match event {
                InputEvent::Change => callback(ErasedEditorEvent::BufferEdited, /* need window */ todo!()),
                InputEvent::Blur => callback(ErasedEditorEvent::Blurred, todo!()),
                _ => {}
            }
        })
    }

    fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement {
        Input::new(&self.0).appearance(false).cleanable(false).into_any_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Register the `InputState`-based editor factory as the default `ErasedEditor` implementation.
pub fn register_input_editor_factory() {
    let _ = ERASED_EDITOR_FACTORY.set(|window: &mut Window, cx: &mut App| -> Arc<dyn ErasedEditor> {
        let state = cx.new(|cx| InputState::new(window, cx));
        Arc::new(InputEditor(state))
    });
}
