//! Stub terminal panel — will be replaced in Phase 20.

use inazuma::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, Pixels,
    Render, Window, prelude::*, px,
};
use raijin_workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
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

    /// Whether the assistant integration is enabled for this terminal panel.
    pub fn assistant_enabled(&self) -> bool {
        false
    }

    /// Enable or disable assistant integration for this terminal panel.
    pub fn set_assistant_enabled(&mut self, _enabled: bool, _cx: &mut Context<Self>) {
    }

    /// Spawn a task in a new terminal tab, returning a handle to the terminal.
    pub fn spawn_task(
        &mut self,
        _task: &raijin_task::ResolvedTask,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> inazuma::Task<anyhow::Result<Entity<raijin_terminal::Terminal>>> {
        cx.spawn(async move |_this, _cx| {
            anyhow::bail!("TerminalPanel task spawning — not yet implemented")
        })
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

    fn panel_key() -> &'static str {
        "TerminalPanel"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }

    fn position_is_valid(&self, _position: DockPosition) -> bool {
        true
    }

    fn set_position(&mut self, _position: DockPosition, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(300.0)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<raijin_ui::IconName> {
        None
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Terminal")
    }

    fn toggle_action(&self) -> Box<dyn inazuma::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        5
    }
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
