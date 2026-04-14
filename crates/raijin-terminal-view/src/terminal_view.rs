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
pub mod branch_picker;
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

/// The content mode of a terminal view, describing how content is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalContentMode {
    /// Normal terminal output.
    Normal,
    /// Terminal has scrollable content (e.g. alternate screen buffer).
    Scrollable,
}

impl TerminalContentMode {
    /// Returns whether this content mode allows scrolling.
    pub fn is_scrollable(&self) -> bool {
        matches!(self, Self::Scrollable)
    }
}

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
        _window: &mut Window,
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

    pub fn set_block_below_cursor(
        &mut self,
        _block: Rc<BlockProperties>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    /// Set embedded mode with an optional max line height.
    /// In embedded mode, the terminal restricts its output display.
    pub fn set_embedded_mode(
        &mut self,
        _max_lines: Option<u32>,
        _cx: &mut Context<Self>,
    ) {
    }

    /// Returns the content mode of this terminal view.
    pub fn content_mode(&self, _window: &Window, _cx: &App) -> TerminalContentMode {
        TerminalContentMode::Normal
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
///
/// Registers action handlers on every new Workspace:
/// - `workspace::NewTerminal` → creates a new TerminalPane tab
pub fn init(cx: &mut App) {
    raijin_ui::input::erased_editor_impl::register_input_editor_factory();
    cx.observe_new(
        |workspace: &mut Workspace, _window, _cx: &mut inazuma::Context<Workspace>| {
            workspace.register_action(|workspace, _action: &raijin_workspace::NewTerminal, window, cx| {
                let terminal = cx.new(|cx| {
                    crate::terminal_pane::TerminalPane::new(window, cx)
                });
                workspace.add_item_to_active_pane(
                    Box::new(terminal),
                    None,
                    true,
                    window,
                    cx,
                );
            });
        },
    )
    .detach();
}
