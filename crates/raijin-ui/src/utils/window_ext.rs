use inazuma::{App, Entity, Placement, Window};
use std::rc::Rc;

use crate::{AppShell, Dialog, InputState, Notification, Sheet};

/// Extension trait for [`Window`] to add dialog, sheet, and notification functionality.
pub trait WindowExt: Sized {
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    fn close_sheet(&mut self, cx: &mut App);

    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    fn close_dialog(&mut self, cx: &mut App);
    fn close_all_dialogs(&mut self, cx: &mut App);

    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App);
    fn clear_notifications(&mut self, cx: &mut App);
}

impl WindowExt for Window {
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        self.open_sheet_at(Placement::Right, cx, build);
    }

    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.open_sheet_at(placement, build, window, cx);
        });
    }

    fn close_sheet(&mut self, cx: &mut App) {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.close_sheet(window, cx);
        });
    }

    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.open_dialog(build, window, cx);
        });
    }

    fn close_dialog(&mut self, cx: &mut App) {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.close_dialog(window, cx);
        });
    }

    fn close_all_dialogs(&mut self, cx: &mut App) {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.close_all_dialogs(window, cx);
        });
    }

    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.push_notification(note, window, cx);
        });
    }

    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App) {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.remove_notification::<T>(window, cx);
        });
    }

    fn clear_notifications(&mut self, cx: &mut App) {
        AppShell::update(self, cx, |shell, window, cx| {
            shell.clear_notifications(window, cx);
        });
    }
}
