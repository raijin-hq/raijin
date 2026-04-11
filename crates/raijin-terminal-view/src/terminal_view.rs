//! Raijin Terminal View — Feature Crate for terminal UI.
//!
//! Contains terminal rendering (grid, blocks, colors, built-in font),
//! the TerminalPane (Workspace Item), shell install modal, and history panel.
//! Imports backend logic from raijin-terminal, raijin-completions, raijin-session, raijin-shell.

// Terminal rendering modules
pub mod block_element;
pub mod block_list;
pub mod builtin_font;
pub mod colors;
pub mod constants;
pub mod grid_element;
pub mod grid_snapshot;
pub mod live_block;

// Terminal UI modules
pub mod input;
pub mod shell_install_modal;
pub mod terminal_pane;
pub mod terminal_panel;

use std::rc::Rc;

use inazuma::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render,
    Window, prelude::*,
};
use raijin_project::Project;
use raijin_terminal::Terminal;
use raijin_workspace::{Workspace, item::ItemEvent};

/// Properties for a block rendered below the terminal cursor.
/// Used by raijin-agent-ui, raijin-debugger-ui, raijin-repl.
pub struct BlockProperties {
    pub height: u8,
    pub render: Box<dyn Send + Fn(&mut BlockContext) -> AnyElement>,
}

/// Context passed to block render functions.
pub struct BlockContext<'a, 'b> {
    pub window: &'a mut Window,
    pub context: &'b mut App,
}

/// A terminal view — backward-compatible API for external crates.
/// The real terminal UI lives in `terminal_pane::TerminalPane` which implements
/// the Workspace `Item` trait with our full Warp-style Block system.
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
        _action: &workspace_actions::OpenTerminal,
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

/// Initialize the terminal view system.
pub fn init(_cx: &mut App) {}

/// Workspace actions for terminal.
mod workspace_actions {
    use inazuma::actions;
    actions!(terminal, [OpenTerminal]);
}
