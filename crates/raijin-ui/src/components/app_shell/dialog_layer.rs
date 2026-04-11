use inazuma::{
    App, Context, FocusHandle, IntoElement, ParentElement as _, Styled as _, WeakFocusHandle, Window, div,
};
use std::rc::Rc;

use crate::{Dialog, ANIMATION_DURATION};
use super::shell::AppShell;

#[derive(Clone)]
pub(crate) struct ActiveDialog {
    pub focus_handle: FocusHandle,
    pub previous_focused_handle: Option<WeakFocusHandle>,
    pub builder: Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static>,
}

impl ActiveDialog {
    pub fn new(
        focus_handle: FocusHandle,
        previous_focused_handle: Option<WeakFocusHandle>,
        builder: impl Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    ) -> Self {
        Self {
            focus_handle,
            previous_focused_handle,
            builder: Rc::new(builder),
        }
    }
}

impl AppShell {
    pub fn open_dialog<F>(&mut self, build: F, window: &mut Window, cx: &mut Context<Self>)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        let mut previous_focused_handle = window.focused(cx).map(|h| h.downgrade());

        if let Some(pending_handle) = self.pending_focus_restore.take() {
            previous_focused_handle = Some(pending_handle);
        }

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        self.active_dialogs.push(ActiveDialog::new(
            focus_handle,
            previous_focused_handle,
            build,
        ));
        cx.notify();
    }

    fn close_dialog_internal(&mut self) -> Option<FocusHandle> {
        self.focused_input = None;
        self.active_dialogs
            .pop()
            .and_then(|d| d.previous_focused_handle)
            .and_then(|h| h.upgrade())
    }

    pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(handle) = self.close_dialog_internal() {
            window.focus(&handle, cx);
        }
        cx.notify();
    }

    pub(crate) fn defer_close_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(handle) = self.close_dialog_internal() {
            let dialogs_count = self.active_dialogs.len();
            self.pending_focus_restore = Some(handle.downgrade());

            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(*ANIMATION_DURATION).await;
                let _ = this.update_in(cx, |this, window, cx| {
                    let current_dialogs_count = this.active_dialogs.len();
                    if current_dialogs_count == dialogs_count {
                        window.focus(&handle, cx);
                    }
                    this.pending_focus_restore = None;
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub fn close_all_dialogs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focused_input = None;
        let previous_focused_handle = self
            .active_dialogs
            .first()
            .and_then(|d| d.previous_focused_handle.clone());
        self.active_dialogs.clear();
        if let Some(handle) = previous_focused_handle.and_then(|h| h.upgrade()) {
            window.focus(&handle, cx);
        }
        cx.notify();
    }

    pub(super) fn build_dialog_element(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        if self.active_dialogs.is_empty() {
            return None;
        }

        let mut show_overlay_ix = None;
        let mut dialogs: Vec<Dialog> = self
            .active_dialogs
            .iter()
            .enumerate()
            .map(|(i, active_dialog)| {
                let mut dialog = Dialog::new(cx);
                dialog = (active_dialog.builder)(dialog, window, cx);
                dialog.focus_handle = active_dialog.focus_handle.clone();
                dialog.layer_ix = i;
                if dialog.has_overlay() {
                    show_overlay_ix = Some(i);
                }
                dialog
            })
            .collect();

        if let Some(overlay_ix) = show_overlay_ix {
            for (i, dialog) in dialogs.iter_mut().enumerate() {
                dialog.props.overlay_visible = i == overlay_ix;
            }
        }

        Some(
            div()
                .absolute()
                .size_full()
                .top_0()
                .left_0()
                .children(dialogs),
        )
    }
}
