//! Stub terminal panel — will be replaced in Phase 20.

use inazuma::{
    AnyElement, AnyView, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render,
    Window, prelude::*,
};
use raijin_workspace::{
    Item, ItemEvent, Workspace,
    dock::{DockPosition, Panel, PanelEvent},
    pane::Pane,
};

/// Stub terminal panel.
pub struct TerminalPanel {
    focus_handle: FocusHandle,
}

impl TerminalPanel {
    pub fn new(_workspace: &Workspace, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn new_terminal(
        _workspace: &mut Workspace,
        _action: &NewTerminal,
        _window: &mut Window,
        _cx: &mut Context<Workspace>,
    ) {
    }

    pub fn open_terminal(
        _workspace: &mut Workspace,
        _action: &OpenTerminal,
        _window: &mut Window,
        _cx: &mut Context<Workspace>,
    ) {
    }

    pub fn terminal_selections(&self, _cx: &App) -> Vec<String> {
        Vec::new()
    }

    pub async fn load(
        _workspace: inazuma::WeakEntity<Workspace>,
        _cx: inazuma::AsyncWindowContext,
    ) -> anyhow::Result<Entity<Self>> {
        anyhow::bail!("TerminalPanel stub — Phase 20 not yet implemented")
    }
}

impl Focusable for TerminalPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for TerminalPanel {}

impl Panel for TerminalPanel {
    fn persistent_name() -> &'static str {
        "TerminalPanel"
    }

    fn position(&self, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }

    fn position_is_valid(&self, _position: DockPosition) -> bool {
        true
    }

    fn set_position(&mut self, _position: DockPosition, _cx: &mut Context<Self>) {}
}

impl Render for TerminalPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        inazuma::div().size_full()
    }
}

/// Initialize the terminal panel — stub, no-op.
pub fn init(_cx: &mut App) {}

use inazuma::actions;
actions!(terminal, [NewTerminal, OpenTerminal, ToggleFocus, Toggle]);
