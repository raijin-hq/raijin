use inazuma::{
    App, EventEmitter, FocusHandle, Focusable, ParentElement as _, Render, SharedString,
    Styled as _, Window,
};

use raijin_theme::ActiveTheme as _;

use super::{Panel, PanelEvent, PanelState};

pub(crate) struct InvalidPanel {
    name: SharedString,
    focus_handle: FocusHandle,
    old_state: PanelState,
}

impl InvalidPanel {
    pub(crate) fn new(name: &str, state: PanelState, _: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            name: SharedString::from(name.to_owned()),
            old_state: state,
        }
    }
}
impl Panel for InvalidPanel {
    fn panel_name(&self) -> &'static str {
        "InvalidPanel"
    }

    fn dump(&self, _cx: &App) -> super::PanelState {
        self.old_state.clone()
    }
}
impl EventEmitter<PanelEvent> for InvalidPanel {}
impl Focusable for InvalidPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for InvalidPanel {
    fn render(
        &mut self,
        _: &mut inazuma::Window,
        cx: &mut inazuma::Context<Self>,
    ) -> impl inazuma::IntoElement {
        inazuma::div()
            .size_full()
            .my_6()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .text_color(cx.theme().colors().muted_foreground)
            .child(format!(
                "The `{}` panel type is not registered in PanelRegistry.",
                self.name.clone()
            ))
    }
}
