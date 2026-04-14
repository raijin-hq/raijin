use inazuma::{Context, Window};
use std::any::TypeId;

use raijin_ui::Notification;
use super::shell::AppShell;

impl AppShell {
    pub fn push_notification(
        &mut self,
        note: impl Into<Notification>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.notification
            .update(cx, |view, cx| view.push(note, window, cx));
        cx.notify();
    }

    pub fn remove_notification<T: Sized + 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.notification.update(cx, |view, cx| {
            let id = TypeId::of::<T>();
            view.close(id, window, cx);
        });
        cx.notify();
    }

    pub fn clear_notifications(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.notification
            .update(cx, |view, cx| view.clear(window, cx));
        cx.notify();
    }
}
