use std::rc::Rc;

use inazuma::{App, Global, Window};

use crate::Dialog;

type DialogBuilderFn = Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static>;

/// Pending dialog queue — raijin-ui pushes, raijin-shell's AppShell drains.
///
/// This decouples dialog opening from AppShell: raijin-ui components can queue
/// dialogs without knowing about AppShell (which lives in raijin-shell).
/// AppShell observes this Global via `observe_global` and promotes queued
/// builders into active dialogs with proper focus management.
#[derive(Default)]
pub struct PendingDialogs {
    pub queue: Vec<DialogBuilderFn>,
}

impl Global for PendingDialogs {}

/// Queue a dialog to be opened by AppShell.
///
/// The builder receives a fresh `Dialog` and returns the configured dialog.
/// AppShell will drain the queue on the next observation cycle, create focus
/// handles, and render the dialog.
pub fn open_dialog(
    window: &mut Window,
    cx: &mut App,
    build: impl Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
) {
    // global_mut automatically triggers NotifyGlobalObservers
    cx.global_mut::<PendingDialogs>().queue.push(Rc::new(build));
    let _ = window;
}
