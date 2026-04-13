/// Modal layer system — manages the lifecycle of modal views.
///
/// Ported from the reference workspace/modal_layer.rs. Provides:
/// - `ModalView` trait for modal implementations
/// - `ModalLayer` entity for managing active modals (toggle, hide, focus)
/// - Overlay background with click-to-dismiss
/// - Focus save/restore on modal open/close
use inazuma::{
    AnyView, App, AppContext as _, Context, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable as _, InteractiveElement as _, IntoElement, ManagedView, MouseButton,
    Oklch, ParentElement as _, Render, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::v_flex;

/// Decision returned by `ModalView::on_before_dismiss` to control dismissal.
#[derive(Debug)]
pub enum DismissDecision {
    /// Allow or prevent dismissal.
    Dismiss(bool),
    /// Dismissal is pending (e.g., async validation in progress).
    Pending,
}

/// Trait for views that can be shown as modals.
///
/// Requires `ManagedView` (= `Focusable + EventEmitter<DismissEvent> + Render`).
/// Emit `DismissEvent` to close the modal.
pub trait ModalView: ManagedView {
    /// Called before the modal is dismissed. Return `Dismiss(false)` to prevent closing.
    fn on_before_dismiss(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> DismissDecision {
        DismissDecision::Dismiss(true)
    }

    /// Whether to dim the background behind the modal.
    fn fade_out_background(&self) -> bool {
        false
    }
}

/// Type-erased handle to a modal view entity.
trait ModalViewHandle {
    fn on_before_dismiss(&mut self, window: &mut Window, cx: &mut App) -> DismissDecision;
    fn view(&self) -> AnyView;
    fn fade_out_background(&self, cx: &App) -> bool;
}

impl<V: ModalView> ModalViewHandle for Entity<V> {
    fn on_before_dismiss(&mut self, window: &mut Window, cx: &mut App) -> DismissDecision {
        self.update(cx, |this, cx| this.on_before_dismiss(window, cx))
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn fade_out_background(&self, cx: &App) -> bool {
        self.read(cx).fade_out_background()
    }
}

struct ActiveModal {
    modal: Box<dyn ModalViewHandle>,
    _subscriptions: [Subscription; 2],
    previous_focus_handle: Option<FocusHandle>,
    focus_handle: FocusHandle,
}

/// Manages the active modal view. Renders it centered with an overlay background.
///
/// Add as an `Entity<ModalLayer>` to your workspace and render it as a child.
/// Use `toggle_modal` or `show_modal` to open modals.
pub struct ModalLayer {
    active_modal: Option<ActiveModal>,
}

impl EventEmitter<DismissEvent> for ModalLayer {}

impl ModalLayer {
    pub fn new() -> Self {
        Self {
            active_modal: None,
        }
    }

    /// Toggle a modal of type `V`. If the same type is active, hide it.
    /// If a different modal is active, replace it (if the old one allows dismiss).
    pub fn toggle_modal<V, B>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        build_view: B,
    ) where
        V: ModalView,
        B: FnOnce(&mut Window, &mut Context<V>) -> V,
    {
        if let Some(active_modal) = &self.active_modal {
            let is_same_type = active_modal.modal.view().downcast::<V>().is_ok();
            let did_close = self.hide_modal(window, cx);
            if is_same_type || !did_close {
                return;
            }
        }
        let new_modal = cx.new(|cx| build_view(window, cx));
        self.show_modal(new_modal, window, cx);
    }

    /// Show a specific modal entity.
    pub fn show_modal<V>(
        &mut self,
        new_modal: Entity<V>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        V: ModalView,
    {
        let focus_handle = cx.focus_handle();
        self.active_modal = Some(ActiveModal {
            modal: Box::new(new_modal.clone()),
            _subscriptions: [
                cx.subscribe_in(
                    &new_modal,
                    window,
                    |this, _, _: &DismissEvent, window, cx| {
                        this.hide_modal(window, cx);
                    },
                ),
                cx.on_focus_out(&focus_handle, window, |this, _event, window, cx| {
                    this.hide_modal(window, cx);
                }),
            ],
            previous_focus_handle: window.focused(cx),
            focus_handle,
        });
        cx.defer_in(window, move |_, window, cx| {
            window.focus(&new_modal.focus_handle(cx), cx);
        });
        cx.notify();
    }

    /// Hide the active modal. Returns true if successfully hidden.
    pub fn hide_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(active_modal) = self.active_modal.as_mut() else {
            return false;
        };

        match active_modal.modal.on_before_dismiss(window, cx) {
            DismissDecision::Dismiss(should_dismiss) => {
                if !should_dismiss {
                    return false;
                }
            }
            DismissDecision::Pending => {
                return false;
            }
        }

        if let Some(active_modal) = self.active_modal.take() {
            if let Some(previous_focus) = active_modal.previous_focus_handle {
                if active_modal.focus_handle.contains_focused(window, cx) {
                    previous_focus.focus(window, cx);
                }
            }
            cx.notify();
        }
        true
    }

    /// Returns the active modal if it's of type `V`.
    pub fn active_modal<V: 'static>(&self) -> Option<Entity<V>> {
        let active_modal = self.active_modal.as_ref()?;
        active_modal.modal.view().downcast::<V>().ok()
    }

    pub fn has_active_modal(&self) -> bool {
        self.active_modal.is_some()
    }
}

impl Render for ModalLayer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(active_modal) = &self.active_modal else {
            return div().into_any_element();
        };

        let fade_bg = active_modal.modal.fade_out_background(cx);

        div()
            .absolute()
            .size_full()
            .inset_0()
            .occlude()
            .when(fade_bg, |this| {
                this.bg(Oklch::black().opacity(0.5))
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.hide_modal(window, cx);
                }),
            )
            .child(
                v_flex()
                    .h(px(0.0))
                    .top_20()
                    .items_center()
                    .track_focus(&active_modal.focus_handle)
                    .child(
                        div()
                            .occlude()
                            .child(active_modal.modal.view())
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            }),
                    ),
            )
            .into_any_element()
    }
}
