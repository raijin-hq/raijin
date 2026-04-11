use raijin_term::term::TermMode;
use inazuma::{
    div, Oklch, oklcha, px, rgb, App, Context, Entity, Focusable, FocusHandle, KeyDownEvent,
    ParentElement, Render, SharedString, Styled, Window, prelude::*,
};
use raijin_ui::{
    Anchor, Chip, GitBranchChip, GitStatsChip, IconName, Popover,
    h_flex, v_flex,
    input::{AutoPairConfig, Input, InputEvent, InputState},
};
use raijin_shell::ShellContext;
use raijin_terminal::{Terminal, TerminalEvent};
use raijin_workspace::{Item, item::ItemEvent, Workspace, WorkspaceId};

use std::rc::Rc;
use std::sync::{Arc, RwLock};

use raijin_session::command_history::CommandHistory;
use raijin_completions::command_correction;
use raijin_completions::shell_completion::ShellCompletionProvider;
use crate::input::history_panel::HistoryPanel;
use raijin_shell::shell_install;

/// A detected shell on the system.
#[derive(Clone)]
pub struct ShellOption {
    pub name: String,
    pub path: Option<String>,
    pub installed: bool,
}

/// Global state for pending shell switch requests from UI click handlers.
pub struct PendingShellSwitch(pub Option<ShellOption>);

impl inazuma::Global for PendingShellSwitch {}

/// Global state for pending shell install — set when install command is sent to PTY.
/// Cleared on CommandEnd (success or failure) or manual shell switch.
pub struct PendingShellInstallName(pub Option<String>);

impl inazuma::Global for PendingShellInstallName {}

/// Detect all supported shells and their install status.
fn detect_available_shells() -> Vec<ShellOption> {
    let candidates: &[(&str, &[&str])] = &[
        ("zsh", &["/bin/zsh", "/usr/bin/zsh"]),
        ("bash", &["/bin/bash", "/usr/bin/bash"]),
        ("fish", &["/usr/local/bin/fish", "/opt/homebrew/bin/fish"]),
        ("nu", &["/usr/local/bin/nu", "/opt/homebrew/bin/nu"]),
    ];

    let mut shells = Vec::new();
    for (name, paths) in candidates {
        let mut found_path = None;
        for path in *paths {
            if std::path::Path::new(path).exists() {
                found_path = Some(path.to_string());
                break;
            }
        }
        if found_path.is_none() {
            found_path = shell_install::resolve_shell_path(name);
        }
        shells.push(ShellOption {
            name: name.to_string(),
            installed: found_path.is_some(),
            path: found_path,
        });
    }
    shells
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum TerminalPaneEvent {
    TitleChanged,
    CloseRequested,
    BellRang,
}

impl inazuma::EventEmitter<TerminalPaneEvent> for TerminalPane {}

// ---------------------------------------------------------------------------
// TerminalPane — our Warp-style terminal as a Workspace Item
// ---------------------------------------------------------------------------

/// Raijin's terminal view — a Warp-style terminal with block system, context
/// chips, shell integration, input bar, history panel, and command correction.
/// Implements `Item` so it can live inside a `raijin_workspace::Workspace` pane.
pub struct TerminalPane {
    terminal: Terminal,
    terminal_title: String,
    focus_handle: FocusHandle,

    // Input system
    input_state: Entity<InputState>,
    shell_completion: Rc<ShellCompletionProvider>,
    command_history: Arc<RwLock<CommandHistory>>,
    history_panel: HistoryPanel,
    correction_suggestion: Option<command_correction::CorrectionResult>,

    // Shell context
    shell_context: ShellContext,
    shell_name: String,
    available_shells: Vec<ShellOption>,
    pending_shell_install: Option<&'static shell_install::ShellInstallInfo>,

    // Rendering
    block_list: Entity<crate::block_list::BlockListView>,
    cached_bg_image: Option<(std::path::PathBuf, Arc<inazuma::RenderImage>)>,

    // UI state
    interactive_mode: bool,
    show_terminal: bool,
    last_terminal_rows: u16,
    last_terminal_cols: u16,

    // Workspace reference (for modal access via action dispatch)
    workspace: Option<inazuma::WeakEntity<Workspace>>,
}

impl TerminalPane {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = cx.global::<raijin_settings::RaijinSettings>().clone();
        let cwd = config.resolve_working_directory();
        let input_mode = match config.general.input_mode {
            raijin_settings::InputMode::Raijin => raijin_terminal::InputMode::Raijin,
            raijin_settings::InputMode::ShellPs1 => raijin_terminal::InputMode::ShellPs1,
        };
        let scrollback = config.terminal.scrollback_history as usize;
        let terminal = Terminal::new(24, 80, &cwd, input_mode, scrollback)
            .expect("failed to create terminal");

        let block_list_view = {
            let handle = terminal.handle();
            cx.new(|_cx| crate::block_list::BlockListView::new(handle))
        };

        let focus_handle = cx.focus_handle();

        // Detect shell language for syntax highlighting
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let shell_name = shell.rsplit('/').next().unwrap_or("zsh");
        let shell_lang = match shell_name {
            "nu" => "nu",
            "fish" => "bash",
            _ => "bash",
        };

        // Check if the shell binary is actually available
        let pending_shell_install = if !shell_install::check_shell_available(shell_name) {
            shell_install::shell_install_info(shell_name)
        } else {
            None
        };

        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .shell_editor(shell_lang, 1, 10)
                .auto_pairs(AutoPairConfig::shell_defaults())
        });

        cx.subscribe_in(&input_state, window, Self::on_input_event)
            .detach();

        let events_rx = terminal.event_receiver().clone();
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(event) = events_rx.recv_async().await {
                this.update_in(cx, |view, window, cx| {
                    view.handle_terminal_event(event, window, cx);
                })
                .ok();
            }
        })
        .detach();

        // Focus the input so the user can type immediately
        input_state.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        let shell_context = ShellContext::gather_for(&cwd);

        // Load command history from shell's histfile
        let command_history = Arc::new(RwLock::new(CommandHistory::detect_and_load(shell_name)));

        // Set up shell completion provider
        let shell_completion = Rc::new(ShellCompletionProvider::new(
            shell_name,
            cwd.clone(),
            command_history.clone(),
        ));
        input_state.update(cx, |state, _cx| {
            state.lsp.completion_provider = Some(shell_completion.clone());
        });

        Self {
            terminal,
            terminal_title: "~ zsh".to_string(),
            input_state,
            focus_handle,
            shell_context,
            command_history,
            history_panel: HistoryPanel::new(),
            correction_suggestion: None,
            shell_completion,
            shell_name: shell_name.to_string(),
            available_shells: detect_available_shells(),
            pending_shell_install,
            block_list: block_list_view,
            cached_bg_image: None,
            interactive_mode: false,
            show_terminal: false,
            last_terminal_rows: 0,
            last_terminal_cols: 0,
            workspace: None,
        }
    }

    // -----------------------------------------------------------------------
    // Background image helpers
    // -----------------------------------------------------------------------

    fn bg_render_image(&mut self, path: &std::path::Path) -> Option<Arc<inazuma::RenderImage>> {
        if let Some((cached_path, cached_img)) = &self.cached_bg_image {
            if cached_path == path {
                return Some(Arc::clone(cached_img));
            }
        }
        let render = inazuma::preload_image(path)?;
        self.cached_bg_image = Some((path.to_path_buf(), Arc::clone(&render)));
        Some(render)
    }

    fn bg_render_image_from_bytes(&mut self, bytes: &[u8]) -> Option<Arc<inazuma::RenderImage>> {
        let sentinel = std::path::PathBuf::from("__bundled_bg__");
        if let Some((cached_path, cached_img)) = &self.cached_bg_image {
            if cached_path == &sentinel {
                return Some(Arc::clone(cached_img));
            }
        }
        let render = inazuma::preload_image_from_bytes(bytes)?;
        self.cached_bg_image = Some((sentinel, Arc::clone(&render)));
        Some(render)
    }

    // -----------------------------------------------------------------------
    // Terminal event handling
    // -----------------------------------------------------------------------

    fn handle_terminal_event(&mut self, event: TerminalEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            TerminalEvent::Wakeup => {
                self.update_interactive_mode();
                cx.notify();
            }
            TerminalEvent::Title(title) => {
                self.terminal_title = title;
                cx.emit(TerminalPaneEvent::TitleChanged);
                cx.notify();
            }
            TerminalEvent::Bell => {
                cx.emit(TerminalPaneEvent::BellRang);
            }
            TerminalEvent::Exit => {
                cx.emit(TerminalPaneEvent::CloseRequested);
                cx.notify();
            }
            TerminalEvent::ShellMarker(marker) => {
                self.handle_shell_marker(marker, window, cx);
            }
        }
    }

    fn handle_shell_marker(
        &mut self,
        marker: raijin_terminal::ShellMarker,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Handle metadata — update context chips before block processing
        if let raijin_terminal::ShellMarker::Metadata(ref json) = marker {
            match serde_json::from_str::<raijin_shell::ShellMetadataPayload>(json) {
                Ok(payload) => {
                    self.shell_context.update_from_metadata(&payload);
                    self.shell_completion.update_cwd(std::path::PathBuf::from(&payload.cwd));
                    if let Some(duration_ms) = payload.last_duration_ms {
                        let handle = self.terminal.handle();
                        let mut term = handle.lock();
                        if let Some(block) = term.block_router_mut().blocks_mut().last_mut() {
                            if block.is_finished() {
                                block.metadata.duration_ms = Some(duration_ms);
                            }
                        }
                    }
                    // Title changed because CWD changed
                    cx.emit(TerminalPaneEvent::TitleChanged);
                    cx.notify();
                }
                Err(e) => {
                    log::warn!("Failed to parse shell metadata JSON: {}", e);
                }
            }
        }

        match marker {
            raijin_terminal::ShellMarker::PromptStart => {
                cx.notify();
            }
            raijin_terminal::ShellMarker::InputStart => {}
            raijin_terminal::ShellMarker::CommandStart => {
                {
                    let handle = self.terminal.handle();
                    let mut term = handle.lock();
                    term.block_router_mut().set_block_metadata(
                        raijin_term::block_grid::BlockMetadata {
                            cwd: Some(self.shell_context.cwd.clone()),
                            username: Some(self.shell_context.username.clone()),
                            hostname: Some(self.shell_context.hostname.clone()),
                            git_branch: self.shell_context.git_branch.clone(),
                            shell: Some(self.shell_name.clone()),
                            duration_ms: None,
                        },
                    );
                }
                self.show_terminal = true;
                cx.notify();
            }
            raijin_terminal::ShellMarker::CommandEnd { exit_code } => {
                log::debug!("Command finished with exit code: {}", exit_code);

                self.input_state.update(cx, |state, cx| {
                    state.focus(window, cx);
                });

                // Auto-switch to newly installed shell if install succeeded
                if let Some(shell_name) = cx.global_mut::<PendingShellInstallName>().0.take() {
                    log::debug!("Install completed for {} (exit={})", shell_name, exit_code);
                    if exit_code == 0 && shell_install::check_shell_available(&shell_name) {
                        log::debug!("Shell {} is now available — queuing auto-switch", shell_name);
                        let path = shell_install::resolve_shell_path(&shell_name);
                        cx.global_mut::<PendingShellSwitch>().0 = Some(ShellOption {
                            name: shell_name,
                            path,
                            installed: true,
                        });
                        cx.notify();
                    } else {
                        log::debug!("Shell {} not found after install (exit={})", shell_name, exit_code);
                    }
                }

                // Check for typo correction on "command not found"
                if exit_code == 127 {
                    let last_cmd = {
                        let handle = self.terminal.handle();
                        let term = handle.lock();
                        term.block_router().blocks().last().map(|b| b.command.clone())
                    };
                    if let Some(last_cmd) = last_cmd {
                        let known: Vec<String> = std::env::var("PATH")
                            .unwrap_or_default()
                            .split(':')
                            .filter_map(|dir| std::fs::read_dir(dir).ok())
                            .flat_map(|entries| entries.flatten())
                            .filter_map(|e| e.file_name().into_string().ok())
                            .collect();
                        self.correction_suggestion =
                            command_correction::suggest_correction(&last_cmd, exit_code, &known);
                    }
                } else {
                    self.correction_suggestion = None;
                }

                cx.notify();
            }
            raijin_terminal::ShellMarker::PromptKind { kind } => {
                log::debug!("Prompt kind: {:?}", kind);
            }
            raijin_terminal::ShellMarker::Metadata(_) => {
                // Already handled above
            }
        }
    }

    fn update_interactive_mode(&mut self) {
        let handle = self.terminal.handle();
        let term = handle.lock();
        self.interactive_mode = term.mode().contains(TermMode::ALT_SCREEN);
    }

    // -----------------------------------------------------------------------
    // Input handling
    // -----------------------------------------------------------------------

    fn on_input_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { secondary: false } => {
                if self.history_panel.is_visible() {
                    self.history_panel.close();
                }

                let value = self.input_state.read(cx).value();
                if !value.is_empty() {
                    if let Ok(mut hist) = self.command_history.write() {
                        hist.push(value.to_string());
                    }
                    {
                        let handle = self.terminal.handle();
                        let mut term = handle.lock();
                        term.set_pending_block_command(value.to_string());
                    }
                    let mut bytes = value.as_bytes().to_vec();
                    bytes.push(b'\r');
                    self.terminal.write(&bytes);
                    self.show_terminal = true;
                    self.input_state.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                    });
                    self.focus_handle.focus(window, cx);
                    cx.notify();
                }
            }
            InputEvent::HistoryUp => {
                if !self.history_panel.is_visible() {
                    let current = self.input_state.read(cx).value().to_string();
                    if let Ok(hist) = self.command_history.read() {
                        self.history_panel.open(&hist, &current);
                    }
                } else {
                    self.history_panel.select_previous();
                }
                if let Some(cmd) = self.history_panel.selected_command() {
                    let cmd = cmd.to_string();
                    self.input_state.update(cx, |state, cx| {
                        state.set_value(&cmd, window, cx);
                    });
                }
                cx.notify();
            }
            InputEvent::HistoryDown => {
                if self.history_panel.is_visible() {
                    if self.history_panel.is_at_bottom() {
                        let saved = self.history_panel.close();
                        self.input_state.update(cx, |state, cx| {
                            state.set_value(&saved, window, cx);
                        });
                    } else {
                        self.history_panel.select_next();
                        if let Some(cmd) = self.history_panel.selected_command() {
                            let cmd = cmd.to_string();
                            self.input_state.update(cx, |state, cx| {
                                state.set_value(&cmd, window, cx);
                            });
                        }
                    }
                    cx.notify();
                }
            }
            InputEvent::Change => {
                if self.history_panel.is_visible() {
                    let query = self.input_state.read(cx).value().to_string();
                    if let Ok(hist) = self.command_history.read() {
                        self.history_panel.filter(&query, &hist);
                    }
                    cx.notify();
                }
                self.update_input_highlights(cx);
            }
            _ => {}
        }
    }

    fn update_input_highlights(&self, cx: &mut Context<Self>) {
        let text = self.input_state.read(cx).value().to_string();
        let completion_range = self.input_state.read(cx).completion_inserted_range.clone();

        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            self.input_state.update(cx, |state, _| {
                state.overlay_highlights.clear();
            });
            return;
        }

        let cmd_end = trimmed.find(|c: char| c.is_whitespace()).unwrap_or(trimmed.len());
        let cmd = &trimmed[..cmd_end];
        let cmd_start = text.len() - trimmed.len();

        let is_valid = if let Ok(executables) = self.shell_completion.path_executables.read() {
            executables.iter().any(|e| e == cmd)
        } else {
            false
        } || matches!(
            cmd,
            "cd" | "echo" | "export" | "source" | "alias" | "unalias" | "type"
                | "which" | "eval" | "exec" | "set" | "unset" | "pwd" | "pushd"
                | "popd" | "dirs" | "bg" | "fg" | "jobs" | "kill" | "wait"
                | "trap" | "umask" | "test" | "true" | "false" | "readonly" | "shift"
        );

        self.input_state.update(cx, |state, _| {
            state.overlay_highlights.clear();

            if is_valid {
                state.overlay_highlights.push((
                    cmd_start..cmd_start + cmd_end,
                    inazuma::HighlightStyle {
                        color: Some(inazuma::oklcha(0.75, 0.12, 220.0, 1.0).into()),
                        ..Default::default()
                    },
                ));
            }

            if let Some(range) = completion_range {
                let clamped_end = range.end.min(text.len());
                if range.start < clamped_end {
                    state.overlay_highlights.push((
                        range.start..clamped_end,
                        inazuma::HighlightStyle {
                            color: Some(inazuma::oklcha(0.75, 0.12, 220.0, 0.6).into()),
                            ..Default::default()
                        },
                    ));
                }
            }
        });
    }

    fn on_key_down_interactive(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Cmd+K: Clear all blocks
        if event.keystroke.key.as_str() == "k" && event.keystroke.modifiers.platform {
            let handle = self.terminal.handle();
            let mut term = handle.lock();
            term.block_router_mut().blocks_mut().clear();
            drop(term);
            self.block_list.update(cx, |view, _cx| view.clear());
            cx.notify();
            return;
        }

        // Cmd+C → copy text selection or selected block command
        if event.keystroke.key.as_str() == "c" && event.keystroke.modifiers.platform {
            let text = self.block_list.read(cx).copy_selection_text();
            if let Some(text) = text {
                cx.write_to_clipboard(inazuma::ClipboardItem::new_string(text));
                cx.notify();
                return;
            }
        }

        if event.keystroke.key.as_str() == "escape" {
            if self.history_panel.is_visible() {
                let saved = self.history_panel.close();
                self.input_state.update(cx, |state, cx| {
                    state.set_value(&saved, window, cx);
                });
                cx.notify();
                return;
            }
            if self.block_list.read(cx).selected_block().is_some() {
                self.block_list.update(cx, |view, _cx| view.set_selected_block(None));
                cx.notify();
                return;
            }
        }

        let command_running = {
            let handle = self.terminal.handle();
            let term = handle.lock();
            term.block_router().has_active_block()
        };

        if !self.interactive_mode && !command_running {
            return;
        }

        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform {
            return;
        }

        let bytes = keystroke_to_bytes(keystroke, &self.terminal);
        if !bytes.is_empty() {
            self.terminal.write(&bytes);
            cx.notify();
        }
    }

    // -----------------------------------------------------------------------
    // Shell management
    // -----------------------------------------------------------------------

    fn request_shell_change(
        &mut self,
        shell: ShellOption,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if shell.installed {
            self.switch_shell(&shell, window, cx);
        } else if let Some(info) = shell_install::shell_install_info(&shell.name) {
            // Use workspace modal layer via action dispatch
            if let Some(ws) = self.workspace.as_ref().and_then(|w| w.upgrade()) {
                let terminal_handle = self.terminal.handle();
                ws.update(cx, |workspace, cx| {
                    workspace.toggle_modal(window, cx, |window, cx| {
                        crate::shell_install_modal::ShellInstallModal::new(
                            info,
                            terminal_handle,
                            window,
                            cx,
                        )
                    });
                });
            }
        }
    }

    fn switch_shell(&mut self, shell: &ShellOption, window: &mut Window, cx: &mut Context<Self>) {
        cx.global_mut::<PendingShellInstallName>().0 = None;

        let shell_path = match &shell.path {
            Some(p) => p.clone(),
            None => return,
        };
        let shell_name = &shell.name;
        let config = cx.global::<raijin_settings::RaijinSettings>().clone();
        let cwd = std::path::PathBuf::from(&self.shell_context.cwd);
        let input_mode = match config.general.input_mode {
            raijin_settings::InputMode::Raijin => raijin_terminal::InputMode::Raijin,
            raijin_settings::InputMode::ShellPs1 => raijin_terminal::InputMode::ShellPs1,
        };
        let scrollback = config.terminal.scrollback_history as usize;

        let new_terminal = Terminal::with_shell(
            self.last_terminal_rows.max(24),
            self.last_terminal_cols.max(80),
            &cwd,
            input_mode,
            scrollback,
            Some(&shell_path),
        );
        let new_terminal = match new_terminal {
            Ok(t) => t,
            Err(e) => {
                log::error!("Failed to spawn shell {}: {}", shell_path, e);
                use raijin_ui::WindowExt as _;
                window.push_notification(
                    raijin_ui::Notification::error(
                        format!("Failed to start {}: {}", shell_name, e),
                    ),
                    &mut *cx,
                );
                return;
            }
        };

        let events_rx = new_terminal.event_receiver().clone();
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(event) = events_rx.recv_async().await {
                this.update_in(cx, |view, window, cx| {
                    view.handle_terminal_event(event, window, cx);
                })
                .ok();
            }
        })
        .detach();

        self.terminal = new_terminal;
        self.shell_name = shell_name.to_string();
        self.terminal_title = format!("~ {}", shell_name);
        self.show_terminal = false;

        if self.history_panel.is_visible() {
            self.history_panel.close();
        }

        self.available_shells = detect_available_shells();

        let shell_lang = match shell_name.as_str() {
            "nu" => "nu",
            "fish" => "bash",
            _ => "bash",
        };
        self.input_state.update(cx, |state, cx| {
            state.set_shell_language(shell_lang, window, cx);
        });

        self.command_history = Arc::new(RwLock::new(CommandHistory::detect_and_load(shell_name)));

        self.shell_completion = Rc::new(ShellCompletionProvider::new(
            shell_name,
            cwd,
            self.command_history.clone(),
        ));
        self.input_state.update(cx, |state, _cx| {
            state.lsp.completion_provider = Some(self.shell_completion.clone());
        });

        self.block_list.update(cx, |view, _cx| view.clear());

        cx.emit(TerminalPaneEvent::TitleChanged);
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // Render helpers
    // -----------------------------------------------------------------------

    fn render_shell_selector_chip(&self) -> impl IntoElement {
        let current = self.shell_name.clone();
        let shells = detect_available_shells();

        Popover::new("shell-selector")
            .anchor(Anchor::BottomLeft)
            .bg(oklcha(0.23, 0.0, 0.0, 1.0))
            .border_color(oklcha(0.30, 0.0, 0.0, 1.0))
            .rounded_lg()
            .shadow_lg()
            .trigger(Chip::new(&current, rgb(0xa78bfa).into()).interactive())
            .content(move |_state, _window, cx| {
                let popover_entity = cx.entity();
                let mut list = v_flex().min_w(px(180.0));

                for shell in &shells {
                    let is_current = shell.name == current;
                    let name = shell.name.clone();
                    let installed = shell.installed;
                    let detail = shell
                        .path
                        .clone()
                        .unwrap_or_else(|| "Not installed".to_string());

                    let shell_name_for_click = shell.name.clone();
                    let shell_path_for_click = shell.path.clone();
                    let popover = popover_entity.clone();
                    let row = div()
                        .id(inazuma::ElementId::Name(format!("shell-{}", shell.name).into()))
                        .px(px(8.0))
                        .py(px(5.0))
                        .text_sm()
                        .rounded(px(4.0))
                        .when(!is_current, |s| {
                            s.cursor_pointer()
                                .hover(|s| s.bg(Oklch::white().opacity(0.06)))
                        })
                        .on_mouse_down(
                            inazuma::MouseButton::Left,
                            move |_, window, cx| {
                                if is_current {
                                    return;
                                }
                                popover.update(cx, |state, cx| {
                                    state.dismiss(window, cx);
                                });
                                cx.global_mut::<PendingShellSwitch>().0 = Some(ShellOption {
                                    name: shell_name_for_click.clone(),
                                    path: shell_path_for_click.clone(),
                                    installed,
                                });
                                window.refresh();
                            },
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .gap(px(12.0))
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .w(px(14.0))
                                                .text_center()
                                                .when(is_current, |s| {
                                                    s.text_color(rgb(0x14F195)).child("✓")
                                                }),
                                        )
                                        .child(
                                            div()
                                                .when(is_current, |s| {
                                                    s.text_color(rgb(0x14F195))
                                                })
                                                .when(!is_current && installed, |s| {
                                                    s.text_color(rgb(0xf1f1f1))
                                                })
                                                .when(!installed, |s| {
                                                    s.text_color(Oklch::white().opacity(0.4))
                                                })
                                                .child(name),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .when(installed, |s| {
                                            s.text_color(Oklch::white().opacity(0.3))
                                                .child(detail)
                                        })
                                        .when(!installed, |s| {
                                            s.text_color(rgb(0xa78bfa)).child("Install")
                                        }),
                                ),
                        );

                    list = list.child(row);
                }
                list
            })
    }

    fn render_input_area(&self) -> impl IntoElement {
        let time_str = time::OffsetDateTime::now_local()
            .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
            .format(time::macros::format_description!("[hour]:[minute]"))
            .unwrap_or_else(|_| "--:--".to_string());

        let mut chips = h_flex()
            .gap(px(6.0))
            .px_4()
            .pt_2()
            .flex_wrap()
            .child(Chip::new(&self.shell_context.username, rgb(0x00BFFF).into()))
            .child(Chip::new(&self.shell_context.hostname, rgb(0xc8c8c8).into()))
            .child(
                Chip::new(&self.shell_context.cwd_short, rgb(0x6ee7b7).into())
                    .icon(IconName::Folder),
            )
            .child(
                Chip::new(&time_str, rgb(0xff5f5f).into())
                    .icon(IconName::Clock),
            )
            .child(self.render_shell_selector_chip());

        if let Some(branch) = &self.shell_context.git_branch {
            chips = chips.child(GitBranchChip::new(branch));
        }

        if let Some(stats) = &self.shell_context.git_stats {
            chips = chips.child(GitStatsChip::new(
                stats.files_changed,
                stats.insertions,
                stats.deletions,
            ));
        }

        div()
            .flex_shrink_0()
            .w_full()
            .border_color(Oklch::white().opacity(0.08))
            .border_t_1()
            .child(chips)
            .child(
                div()
                    .px_1()
                    .pt_1()
                    .pb_3()
                    .child(
                        Input::new(&self.input_state)
                            .appearance(false)
                            .cleanable(false),
                    ),
            )
    }

    fn render_correction_banner(
        &self,
        correction: &command_correction::CorrectionResult,
        theme: &raijin_theme::Theme,
    ) -> impl IntoElement {
        let original = correction.original.clone();
        let suggestion = correction.suggestion.clone();
        let confidence_pct = (correction.confidence * 100.0) as u32;
        div()
            .flex()
            .items_center()
            .w_full()
            .h(px(32.0))
            .px_4()
            .gap_2()
            .bg(Oklch::white().opacity(0.04))
            .border_t_1()
            .border_b_1()
            .border_color(Oklch::white().opacity(0.08))
            .text_xs()
            .child(
                div()
                    .text_color(rgb(0x666666))
                    .child(original),
            )
            .child(
                div()
                    .text_color(rgb(0xffaa00))
                    .child("→"),
            )
            .child(
                div()
                    .px_2()
                    .py(px(2.0))
                    .bg(Oklch::white().opacity(0.08))
                    .rounded(px(4.0))
                    .text_color(crate::constants::accent_color(theme))
                    .child(format!("{} ({}%)", suggestion, confidence_pct)),
            )
            .child(
                div()
                    .text_color(rgb(0x666666))
                    .child("? Press ↵ to run"),
            )
    }
}

// ---------------------------------------------------------------------------
// Render — the Warp-style terminal layout (NO title bar, NO modal layer)
// ---------------------------------------------------------------------------

impl Render for TerminalPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Show shell install modal on first render if shell is missing
        if let Some(shell_info) = self.pending_shell_install.take() {
            if let Some(ws) = self.workspace.as_ref().and_then(|w| w.upgrade()) {
                let terminal_handle = self.terminal.handle();
                ws.update(cx, |workspace, cx| {
                    workspace.toggle_modal(window, cx, |window, cx| {
                        crate::shell_install_modal::ShellInstallModal::new(
                            shell_info,
                            terminal_handle,
                            window,
                            cx,
                        )
                    });
                });
            }
        }

        // Process pending shell switch from the shell selector popover
        let pending = cx.global_mut::<PendingShellSwitch>().0.take();
        if let Some(shell) = pending {
            cx.defer_in(window, move |pane, window, cx| {
                pane.request_shell_change(shell, window, cx);
            });
        }

        let theme = raijin_theme::GlobalTheme::theme(cx).clone();
        let bg_color = theme.styles.colors.background;

        let mut container = div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .bg(bg_color)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down_interactive));

        // Background image layer from theme
        if let Some(bg_config) = &theme.styles.background_image {
            let opacity = (bg_config.opacity as f32 / 100.0).clamp(0.0, 1.0);

            let render_image = if self.cached_bg_image.is_some() {
                self.cached_bg_image.as_ref().map(|(_, img)| Arc::clone(img))
            } else if let Some(base_dir) = &theme.base_dir {
                let image_path = base_dir.join(&bg_config.path);
                self.bg_render_image(&image_path)
            } else {
                let asset_path = format!("themes/{}/{}", theme.id, bg_config.path);
                cx.asset_source()
                    .load(&asset_path)
                    .ok()
                    .flatten()
                    .and_then(|bytes| self.bg_render_image_from_bytes(&bytes))
            };

            if let Some(render_image) = render_image {
                container = container.child(
                    inazuma::img(inazuma::ImageSource::Render(render_image))
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .object_fit(inazuma::ObjectFit::Cover)
                        .opacity(opacity)
                );
            }
        }

        // NO title bar here — the Workspace renders it above us

        // Terminal output + resize logic
        let handle = self.terminal.handle();
        let config = cx.global::<raijin_settings::RaijinSettings>();
        let font_family = config.appearance.font_family.clone();
        let font_size = config.appearance.font_size as f32;
        let line_height_multiplier = config.appearance.line_height as f32;

        {
            let font = inazuma::Font {
                family: font_family.clone().into(),
                weight: inazuma::FontWeight::NORMAL,
                ..inazuma::Font::default()
            };
            let font_id = window.text_system().resolve_font(&font);
            let font_px = px(font_size);
            let cell_width = window
                .text_system()
                .advance(font_id, font_px, 'm')
                .expect("glyph not found for 'm'")
                .width;
            let ascent = window.text_system().ascent(font_id, font_px);
            let descent = window.text_system().descent(font_id, font_px);
            let base_height = ascent + descent.abs();
            let cell_height = base_height * line_height_multiplier;

            let viewport = window.viewport_size();
            let horizontal_padding = px(crate::constants::BLOCK_HEADER_PAD_X) * 2.0;
            let cols = ((viewport.width - horizontal_padding) / cell_width)
                .max(2.0) as u16;
            let rows = (viewport.height / cell_height).max(1.0) as u16;
            if rows != self.last_terminal_rows || cols != self.last_terminal_cols {
                handle.set_size(rows, cols);
                self.last_terminal_rows = rows;
                self.last_terminal_cols = cols;
            }
        }

        if self.show_terminal || self.interactive_mode {
            container = container.child(self.block_list.clone());
        } else {
            container = container.child(
                div().flex_1().min_h_0(),
            );
        }

        if !self.interactive_mode {
            let command_running = {
                let handle = self.terminal.handle();
                let term = handle.lock();
                term.block_router().has_active_block()
            };

            if let Some(ref correction) = self.correction_suggestion {
                container = container.child(self.render_correction_banner(correction, &theme));
            }

            if !command_running {
                if self.history_panel.is_visible() {
                    container = container.child(self.history_panel.render());
                }
                container = container.child(self.render_input_area());
            }
        }

        // NO modal layer here — the Workspace renders it on top

        container
    }
}

// ---------------------------------------------------------------------------
// Focusable
// ---------------------------------------------------------------------------

impl Focusable for TerminalPane {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ---------------------------------------------------------------------------
// Item trait — makes TerminalPane a Workspace item
// ---------------------------------------------------------------------------

impl Item for TerminalPane {
    type Event = TerminalPaneEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        format!("{} — {}", self.shell_name, self.shell_context.cwd_short).into()
    }

    fn tab_tooltip_text(&self, _cx: &App) -> Option<SharedString> {
        Some(self.shell_context.cwd.clone().into())
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        match event {
            TerminalPaneEvent::TitleChanged => f(ItemEvent::UpdateTab),
            TerminalPaneEvent::CloseRequested => f(ItemEvent::CloseItem),
            TerminalPaneEvent::BellRang => {}
        }
    }

    fn can_split(&self) -> bool {
        true
    }

    fn clone_on_split(
        &self,
        _workspace_id: Option<WorkspaceId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> inazuma::Task<Option<Entity<Self>>> {
        let shell_name = self.shell_name.clone();
        let cwd = std::path::PathBuf::from(&self.shell_context.cwd);
        let shell_path = self.available_shells
            .iter()
            .find(|s| s.name == shell_name)
            .and_then(|s| s.path.clone());
        let config = cx.global::<raijin_settings::RaijinSettings>().clone();
        let input_mode = match config.general.input_mode {
            raijin_settings::InputMode::Raijin => raijin_terminal::InputMode::Raijin,
            raijin_settings::InputMode::ShellPs1 => raijin_terminal::InputMode::ShellPs1,
        };
        let scrollback = config.terminal.scrollback_history as usize;

        // Try to spawn the terminal BEFORE creating the entity — if it fails,
        // we return None without creating anything.
        let new_terminal = if let Some(ref path) = shell_path {
            Terminal::with_shell(24, 80, &cwd, input_mode, scrollback, Some(path))
        } else {
            Terminal::new(24, 80, &cwd, input_mode, scrollback)
        };

        let terminal = match new_terminal {
            Ok(t) => t,
            Err(e) => {
                log::error!("Failed to clone terminal on split: {}", e);
                return inazuma::Task::ready(None);
            }
        };

        let ws = self.workspace.clone();

        // Terminal spawned successfully — now create the entity.
        // We use cx.new() which creates a new Entity<Self> with its own Context.
        // Event subscriptions and spawn happen inside the Context<Self> closure.
        let block_list_view = {
            let handle = terminal.handle();
            cx.new(|_cx| crate::block_list::BlockListView::new(handle))
        };

        let shell_lang = match shell_name.as_str() {
            "nu" => "nu",
            "fish" => "bash",
            _ => "bash",
        };

        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .shell_editor(shell_lang, 1, 10)
                .auto_pairs(AutoPairConfig::shell_defaults())
        });

        let command_history = Arc::new(RwLock::new(
            CommandHistory::detect_and_load(&shell_name),
        ));
        let shell_completion = Rc::new(ShellCompletionProvider::new(
            &shell_name,
            cwd.clone(),
            command_history.clone(),
        ));
        input_state.update(cx, |state, _cx| {
            state.lsp.completion_provider = Some(shell_completion.clone());
        });

        // Wire up event subscriptions — these will be on the NEW entity once
        // clone_on_split returns, because the Workspace re-wires them.
        // For now, subscribe on the current context — the pane system handles
        // routing events to the correct item.
        let events_rx = terminal.event_receiver().clone();
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(event) = events_rx.recv_async().await {
                this.update_in(cx, |view, window, cx| {
                    view.handle_terminal_event(event, window, cx);
                })
                .ok();
            }
        })
        .detach();

        cx.subscribe_in(&input_state, window, Self::on_input_event)
            .detach();

        input_state.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        let last_rows = self.last_terminal_rows;
        let last_cols = self.last_terminal_cols;

        // NOTE: cx.new() in a Context<Self> creates a new Entity<Self>
        let new_entity = cx.new(|cx| Self {
            terminal,
            terminal_title: format!("~ {}", shell_name),
            input_state,
            focus_handle: cx.focus_handle(),
            shell_context: ShellContext::gather_for(&cwd),
            command_history,
            history_panel: HistoryPanel::new(),
            correction_suggestion: None,
            shell_completion,
            shell_name,
            available_shells: detect_available_shells(),
            pending_shell_install: None,
            block_list: block_list_view,
            cached_bg_image: None,
            interactive_mode: false,
            show_terminal: false,
            last_terminal_rows: last_rows,
            last_terminal_cols: last_cols,
            workspace: ws,
        });

        inazuma::Task::ready(Some(new_entity))
    }

    fn is_dirty(&self, _cx: &App) -> bool {
        let handle = self.terminal.handle();
        let term = handle.lock();
        term.block_router().has_active_block()
    }

    fn can_save(&self, _cx: &App) -> bool {
        false
    }

    fn added_to_workspace(
        &mut self,
        workspace: &mut Workspace,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace = Some(workspace.weak_handle());
    }
}

// ---------------------------------------------------------------------------
// Keystroke → VT bytes conversion
// ---------------------------------------------------------------------------

fn keystroke_to_bytes(keystroke: &inazuma::Keystroke, terminal: &Terminal) -> Vec<u8> {
    let modifiers = &keystroke.modifiers;
    let key = keystroke.key.as_str();

    let handle = terminal.handle();
    let term = handle.lock();
    let app_cursor = term.mode().contains(TermMode::APP_CURSOR);
    drop(term);

    let prefix = if app_cursor { "\x1bO" } else { "\x1b[" };

    if modifiers.control {
        if let Some(ch) = key.chars().next() {
            if ch.is_ascii_alphabetic() {
                let ctrl_byte = (ch.to_ascii_lowercase() as u8) - b'a' + 1;
                return vec![ctrl_byte];
            }
        }
        if key == "space" {
            return vec![0x00];
        }
    }

    match key {
        "enter" | "return" => return b"\r".to_vec(),
        "backspace" => return vec![0x7f],
        "tab" => return b"\t".to_vec(),
        "escape" => return vec![0x1b],
        "up" => return format!("{}A", prefix).into_bytes(),
        "down" => return format!("{}B", prefix).into_bytes(),
        "right" => return format!("{}C", prefix).into_bytes(),
        "left" => return format!("{}D", prefix).into_bytes(),
        "home" => return b"\x1b[H".to_vec(),
        "end" => return b"\x1b[F".to_vec(),
        "delete" => return b"\x1b[3~".to_vec(),
        "pageup" => return b"\x1b[5~".to_vec(),
        "pagedown" => return b"\x1b[6~".to_vec(),
        "space" => {
            if modifiers.alt {
                return b"\x1b ".to_vec();
            }
            return b" ".to_vec();
        }
        _ => {}
    }

    if modifiers.alt {
        if let Some(ref key_char) = keystroke.key_char {
            let mut bytes = vec![0x1b];
            bytes.extend_from_slice(key_char.as_bytes());
            return bytes;
        }
    }

    if let Some(ref key_char) = keystroke.key_char {
        return key_char.as_bytes().to_vec();
    }

    Vec::new()
}
