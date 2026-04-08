//! Stub crate for raijin-terminal-view.
//!
//! Provides minimal type definitions so that crates depending on terminal_view
//! (agent_ui, debugger_ui, repl, etc.) can compile.
//!
//! Phase 20 (Workspace Integration) will replace this with a real implementation
//! built on our raijin-terminal + raijin-term Block system.

pub mod terminal_panel;
pub mod terminal_element;

use std::rc::Rc;

use inazuma::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Pixels, Render,
    Window, prelude::*,
};
use raijin_project::Project;
use raijin_terminal::Terminal;
use raijin_workspace::{Item, ItemEvent, Workspace, WorkspaceId, pane::Pane};

/// Properties for a block rendered below the terminal cursor.
pub struct BlockProperties {
    pub height: u8,
    pub render: Box<dyn Send + Fn(&mut BlockContext) -> AnyElement>,
}

/// Context passed to block render functions.
pub struct BlockContext<'a, 'b> {
    pub window: &'a mut Window,
    pub context: &'b mut App,
}

/// A terminal view — stub that will be replaced in Phase 20.
pub struct TerminalView {
    terminal: Entity<Terminal>,
    focus_handle: FocusHandle,
}

impl TerminalView {
    pub fn new(
        terminal: Entity<Terminal>,
        _workspace: Entity<Workspace>,
        _custom_title: Option<String>,
        _project: inazuma::WeakEntity<Project>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            terminal,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn terminal(&self) -> &Entity<Terminal> {
        &self.terminal
    }

    pub fn deploy(
        _workspace: &mut Workspace,
        _action: &workspace::OpenTerminal,
        _window: &mut Window,
        _cx: &mut Context<Workspace>,
    ) {
    }

    pub fn set_block_below_cursor(
        &mut self,
        _block: Rc<BlockProperties>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<ItemEvent> for TerminalView {}

impl Render for TerminalView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        inazuma::div().size_full()
    }
}

/// Initialize the terminal view system — stub, no-op for now.
pub fn init(_cx: &mut App) {}

/// Workspace action for opening a terminal.
mod workspace {
    use inazuma::actions;
    actions!(terminal, [OpenTerminal]);
}
