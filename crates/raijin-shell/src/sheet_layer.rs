use inazuma::{
    App, Context, FocusHandle, IntoElement, ParentElement as _, Styled as _, WeakFocusHandle, Window, div,
    Placement,
};
use std::rc::Rc;

use raijin_ui::Sheet;
use super::shell::AppShell;

#[derive(Clone)]
pub(crate) struct ActiveSheet {
    pub focus_handle: FocusHandle,
    pub previous_focused_handle: Option<WeakFocusHandle>,
    pub placement: Placement,
    pub builder: Rc<dyn Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static>,
}

impl AppShell {
    pub fn open_sheet_at<F>(
        &mut self,
        placement: Placement,
        build: F,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        let previous_focused_handle = self
            .active_sheet
            .take()
            .and_then(|s| s.previous_focused_handle)
            .or_else(|| window.focused(cx).map(|h| h.downgrade()));

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        self.active_sheet = Some(ActiveSheet {
            focus_handle,
            previous_focused_handle,
            placement,
            builder: Rc::new(build),
        });
        cx.notify();
    }

    pub fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focused_input = None;
        if let Some(previous_handle) = self
            .active_sheet
            .as_ref()
            .and_then(|s| s.previous_focused_handle.as_ref())
            .and_then(|h| h.upgrade())
        {
            window.focus(&previous_handle, cx);
        }
        self.active_sheet = None;
        cx.notify();
    }

    pub(super) fn build_sheet_element(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let active_sheet = self.active_sheet.clone()?;
        let mut sheet = Sheet::new(window, cx);
        sheet = (active_sheet.builder)(sheet, window, cx);
        sheet.focus_handle = active_sheet.focus_handle.clone();
        sheet.placement = active_sheet.placement;

        Some(div().absolute().size_full().top_0().left_0().child(sheet))
    }
}
