use raijin_term::term::TermMode;
use inazuma::{
    div, hsla, px, rgb, App, Context, Entity, Focusable, FocusHandle, KeyDownEvent,
    ParentElement, Render, Styled, Window, prelude::*,
};
use inazuma_component::{
    Anchor, IconName, TitleBar,
    chip::{Chip, GitBranchChip, GitStatsChip},
    h_flex,
    input::{AutoPairConfig, Input, InputEvent, InputState},
    modal_layer::ModalLayer,
    popover::Popover,
    v_flex,
};
use raijin_shell::ShellContext;
use raijin_terminal::{Terminal, TerminalEvent};

use std::rc::Rc;
use std::sync::{Arc, RwLock};

use crate::command_history::CommandHistory;
use crate::completions::command_correction;
use crate::completions::shell_completion::ShellCompletionProvider;
use crate::input::history_panel::HistoryPanel;
use crate::settings_view;
use crate::shell_install;
// Block rendering now uses terminal::block_list::render_block_list

/// Which view is currently active.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Terminal,
    Settings,
}

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
        // Check standard paths first
        let mut found_path = None;
        for path in *paths {
            if std::path::Path::new(path).exists() {
                found_path = Some(path.to_string());
                break;
            }
        }
        // Also check via command -v for non-standard paths (e.g. ~/.cargo/bin/nu)
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

/// Top-level workspace view that composes the Warp-style layout:
/// TabBar (top) → Terminal Output (middle) → Context Chips + Input (bottom)
pub struct Workspace {
    terminal: Terminal,
    terminal_title: String,
    input_state: Entity<InputState>,
    focus_handle: FocusHandle,
    shell_context: ShellContext,
    command_history: Arc<RwLock<CommandHistory>>,
    history_panel: HistoryPanel,
    correction_suggestion: Option<command_correction::CorrectionResult>,
    shell_completion: Rc<ShellCompletionProvider>,
    modal_layer: Entity<ModalLayer>,
    shell_name: String,
    /// Available shells detected on the system.
    available_shells: Vec<ShellOption>,
    /// If set, the shell was not found at startup and the install modal should be shown.
    pending_shell_install: Option<&'static shell_install::ShellInstallInfo>,
    block_list: Entity<crate::terminal::block_list::BlockListView>,
    interactive_mode: bool,
    show_terminal: bool,
    view_mode: ViewMode,
    last_terminal_rows: u16,
    last_terminal_cols: u16,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = cx.global::<raijin_settings::RaijinConfig>().clone();
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
            cx.new(|_cx| crate::terminal::block_list::BlockListView::new(handle))
        };

        let focus_handle = cx.focus_handle();

        // Detect shell language for syntax highlighting
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let shell_name = shell.rsplit('/').next().unwrap_or("zsh");
        let shell_lang = match shell_name {
            "nu" => "nu",
            "fish" => "bash", // Fallback: fish → bash highlighting (close enough)
            _ => "bash",      // zsh, bash, sh → bash highlighting
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
                this.update_in(cx, |view, _window, cx| {
                    view.handle_terminal_event(event, cx);
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
            modal_layer: cx.new(|_cx| ModalLayer::new()),
            shell_name: shell_name.to_string(),
            available_shells: detect_available_shells(),
            pending_shell_install,
            block_list: block_list_view,
            interactive_mode: false,
            show_terminal: false,
            view_mode: ViewMode::Terminal,
            last_terminal_rows: 0,
            last_terminal_cols: 0,
        }
    }

    fn handle_terminal_event(&mut self, event: TerminalEvent, cx: &mut Context<Self>) {
        match event {
            TerminalEvent::Wakeup => {
                self.update_interactive_mode();
                cx.notify();
            }
            TerminalEvent::Title(title) => {
                self.terminal_title = title;
                cx.notify();
            }
            TerminalEvent::Bell => {}
            TerminalEvent::Exit => {
                cx.notify();
            }
            TerminalEvent::ShellMarker(marker) => {
                self.handle_shell_marker(marker, cx);
            }
        }
    }

    fn handle_shell_marker(
        &mut self,
        marker: raijin_terminal::ShellMarker,
        cx: &mut Context<Self>,
    ) {
        // Handle metadata — update context chips before block processing
        if let raijin_terminal::ShellMarker::Metadata(ref json) = marker {
            match serde_json::from_str::<raijin_shell::ShellMetadataPayload>(json) {
                Ok(payload) => {
                    self.shell_context.update_from_metadata(&payload);
                    // Update completion provider CWD for file path completions
                    self.shell_completion.update_cwd(std::path::PathBuf::from(&payload.cwd));
                    // Apply shell-measured duration to the last finalized block
                    if let Some(duration_ms) = payload.last_duration_ms {
                        let handle = self.terminal.handle();
                        let mut term = handle.lock();
                        if let Some(block) = term.block_router_mut().blocks_mut().last_mut() {
                            if block.is_finished() {
                                block.metadata.duration_ms = Some(duration_ms);
                            }
                        }
                    }
                    cx.notify();
                }
                Err(e) => {
                    log::warn!("Failed to parse shell metadata JSON: {}", e);
                }
            }
        }

        match marker {
            raijin_terminal::ShellMarker::PromptStart => {
                // In Raijin mode, don't show terminal until first command runs
                cx.notify();
            }
            raijin_terminal::ShellMarker::InputStart => {}
            raijin_terminal::ShellMarker::CommandStart => {
                // Pass current shell context metadata to the new block
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
                // Nushell-specific: used for multi-line prompt detection
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

    fn on_input_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { secondary: false } => {
                // Close history panel if open
                if self.history_panel.is_visible() {
                    self.history_panel.close();
                }

                let value = self.input_state.read(cx).value();
                if !value.is_empty() {
                    if let Ok(mut hist) = self.command_history.write() {
                        hist.push(value.to_string());
                    }
                    // Set pending command on the block router for when CommandStart fires
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
                    // Move focus to workspace container so key events
                    // (Ctrl+C etc.) reach on_key_down_interactive while
                    // the command runs and the input bar is hidden.
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
                        // At the newest entry — close panel, restore saved input
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
                // Filter history panel when user types while it's open
                if self.history_panel.is_visible() {
                    let query = self.input_state.read(cx).value().to_string();
                    if let Ok(hist) = self.command_history.read() {
                        self.history_panel.filter(&query, &hist);
                    }
                    cx.notify();
                }

                // Command validation: highlight first token green if valid command
                self.update_input_highlights(cx);
            }
            _ => {}
        }
    }

    /// Compute all input highlights from scratch: command validation + completion coloring.
    /// Called on every text change. Reads `completion_inserted_range` for completion coloring.
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

        // Extract the first token (command name)
        let cmd_end = trimmed.find(|c: char| c.is_whitespace()).unwrap_or(trimmed.len());
        let cmd = &trimmed[..cmd_end];
        let cmd_start = text.len() - trimmed.len();

        // Check if it's a valid command (in $PATH or builtin)
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

            // 1. Command highlight (full brand color)
            if is_valid {
                state.overlay_highlights.push((
                    cmd_start..cmd_start + cmd_end,
                    inazuma::HighlightStyle {
                        color: Some(inazuma::hsla(195. / 360., 1.0, 0.5, 1.0)),
                        ..Default::default()
                    },
                ));
            }

            // 2. Completion-inserted text highlight (dimmed brand color)
            if let Some(range) = completion_range {
                let clamped_end = range.end.min(text.len());
                if range.start < clamped_end {
                    state.overlay_highlights.push((
                        range.start..clamped_end,
                        inazuma::HighlightStyle {
                            color: Some(inazuma::hsla(195. / 360., 1.0, 0.5, 0.6)),
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
        // --- Platform shortcuts (Cmd on macOS) ---

        // Cmd+K: Clear all blocks (terminal clear)
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
            // Fallback: copy selected block's command
            if let Some(idx) = self.block_list.read(cx).selected_block() {
                let cmd = {
                    let handle = self.terminal.handle();
                    let term = handle.lock();
                    term.block_router().blocks().get(idx).map(|b| b.command.clone())
                };
                if let Some(text) = cmd {
                    cx.write_to_clipboard(inazuma::ClipboardItem::new_string(text));
                    self.block_list.update(cx, |view, _cx| view.set_selected_block(None));
                    cx.notify();
                    return;
                }
            }
        }

        if event.keystroke.key.as_str() == "escape" {
            // Dismiss history panel
            if self.history_panel.is_visible() {
                let saved = self.history_panel.close();
                self.input_state.update(cx, |state, cx| {
                    state.set_value(&saved, window, cx);
                });
                cx.notify();
                return;
            }
            // Deselect block
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

        // Forward keys to PTY when:
        // 1. ALT_SCREEN is active (interactive TUI: vim, less, htop)
        // 2. A command is running (Ctrl+C to interrupt, etc.)
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

    /// Tab label: shortened CWD (e.g. `~`, `~/Projects/raijin`).
    fn tab_label(&self) -> String {
        self.shell_context.cwd_short.clone()
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_terminal = self.view_mode == ViewMode::Terminal;
        let is_settings = self.view_mode == ViewMode::Settings;
        let border_color = hsla(0.0, 0.0, 1.0, 0.08);
        let active_bg = rgb(0x222222);

        TitleBar::new().child(
            div()
                .flex()
                .items_center()
                .h_full()
                // Terminal tab
                .child(
                    div()
                        .id("tab-terminal")
                        .flex()
                        .items_center()
                        .justify_center()
                        .h_full()
                        .w(px(160.0))
                        .flex_shrink_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .px_3()
                        .text_xs()
                        .cursor_pointer()
                        .border_r_1()
                        .border_l_1()
                        .border_color(border_color)
                        .when(is_terminal, |this| {
                            this.text_color(rgb(0xf1f1f1)).bg(active_bg)
                        })
                        .when(!is_terminal, |this| {
                            this.text_color(rgb(0x888888))
                        })
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.view_mode = ViewMode::Terminal;
                            cx.notify();
                        }))
                        .child(self.tab_label()),
                )
                // Settings tab
                .child(
                    div()
                        .id("tab-settings")
                        .flex()
                        .items_center()
                        .justify_center()
                        .h_full()
                        .w(px(120.0))
                        .flex_shrink_0()
                        .px_3()
                        .text_xs()
                        .cursor_pointer()
                        .border_r_1()
                        .border_color(border_color)
                        .when(is_settings, |this| {
                            this.text_color(rgb(0xf1f1f1)).bg(active_bg)
                        })
                        .when(!is_settings, |this| {
                            this.text_color(rgb(0x888888))
                        })
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.view_mode = ViewMode::Settings;
                            cx.notify();
                        }))
                        .child("Settings"),
                ),
        )
    }

    fn render_shell_selector_chip(&self) -> impl IntoElement {
        let current = self.shell_name.clone();

        // Re-detect shells each time popover opens (catches newly installed shells)
        let shells = detect_available_shells();

        Popover::new("shell-selector")
            .anchor(Anchor::BottomLeft)
            // Override popover chrome to match completion menu design
            .bg(hsla(0.0, 0.0, 0.16, 1.0))
            .border_color(hsla(0.0, 0.0, 0.22, 1.0))
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
                                .hover(|s| s.bg(hsla(0.0, 0.0, 1.0, 0.06)))
                        })
                        .on_mouse_down(
                            inazuma::MouseButton::Left,
                            move |_, window, cx| {
                                if is_current {
                                    return;
                                }
                                // Dismiss popover first
                                popover.update(cx, |state, cx| {
                                    state.dismiss(window, cx);
                                });
                                // Queue shell change for next frame
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
                                                    s.text_color(hsla(0.0, 0.0, 1.0, 0.4))
                                                })
                                                .child(name),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .when(installed, |s| {
                                            s.text_color(hsla(0.0, 0.0, 1.0, 0.3))
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

    /// Central entry point for shell changes — handles both installed (switch) and
    /// not-installed (install dialog) shells. Callable from any trigger (chip click,
    /// keyboard shortcut, settings, etc.).
    fn request_shell_change(
        &mut self,
        shell: ShellOption,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if shell.installed {
            self.switch_shell(&shell, window, cx);
        } else if let Some(info) = shell_install::shell_install_info(&shell.name) {
            let terminal_handle = self.terminal.handle();
            self.modal_layer.update(cx, |layer, cx| {
                layer.toggle_modal(window, cx, |window, cx| {
                    shell_install::ShellInstallModal::new(
                        info,
                        terminal_handle,
                        window,
                        cx,
                    )
                });
            });
        }
    }

    /// Switch to a different shell: terminate old PTY, spawn new one, update completions + history.
    fn switch_shell(&mut self, shell: &ShellOption, window: &mut Window, cx: &mut Context<Self>) {
        // Clear any pending install (we're switching now, don't auto-switch again)
        cx.global_mut::<PendingShellInstallName>().0 = None;

        let shell_path = match &shell.path {
            Some(p) => p.clone(),
            None => return, // Not installed — should show install modal instead
        };
        let shell_name = &shell.name;
        let config = cx.global::<raijin_settings::RaijinConfig>().clone();
        let cwd = std::path::PathBuf::from(&self.shell_context.cwd);
        let input_mode = match config.general.input_mode {
            raijin_settings::InputMode::Raijin => raijin_terminal::InputMode::Raijin,
            raijin_settings::InputMode::ShellPs1 => raijin_terminal::InputMode::ShellPs1,
        };
        let scrollback = config.terminal.scrollback_history as usize;

        // Spawn new terminal with the selected shell (old one drops and PTY closes)
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
                use inazuma_component::WindowExt as _;
                window.push_notification(
                    inazuma_component::notification::Notification::error(
                        format!("Failed to start {}: {}", shell_name, e),
                    ),
                    &mut *cx,
                );
                return;
            }
        };

        // Wire up terminal events
        let events_rx = new_terminal.event_receiver().clone();
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(event) = events_rx.recv_async().await {
                this.update_in(cx, |view, _window, cx| {
                    view.handle_terminal_event(event, cx);
                })
                .ok();
            }
        })
        .detach();

        self.terminal = new_terminal;
        self.shell_name = shell_name.to_string();
        self.terminal_title = format!("~ {}", shell_name);
        self.show_terminal = false;

        // Close history panel (history is about to be reloaded for new shell)
        if self.history_panel.is_visible() {
            self.history_panel.close();
        }

        // Update available shells list (in case a shell was just installed)
        self.available_shells = detect_available_shells();

        // Update syntax highlighting language
        let shell_lang = match shell_name.as_str() {
            "nu" => "nu",
            "fish" => "bash",
            _ => "bash",
        };
        self.input_state.update(cx, |state, cx| {
            state.set_shell_language(shell_lang, window, cx);
        });

        // Reload command history for the new shell
        self.command_history = Arc::new(RwLock::new(CommandHistory::detect_and_load(shell_name)));

        // Update completion provider
        self.shell_completion = Rc::new(ShellCompletionProvider::new(
            shell_name,
            cwd,
            self.command_history.clone(),
        ));
        self.input_state.update(cx, |state, _cx| {
            state.lsp.completion_provider = Some(self.shell_completion.clone());
        });

        // Clear block list state (old terminal's blocks are invalid)
        self.block_list.update(cx, |view, _cx| view.clear());

        cx.notify();
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
            .border_color(hsla(0.0, 0.0, 1.0, 0.08))
            .border_t_1()
            .child(chips)
            .child(
                // Input row — px_1 so cursor aligns with chip left edge
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
            .bg(hsla(0.0, 0.0, 1.0, 0.04))
            .border_t_1()
            .border_b_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.08))
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
                    .bg(hsla(0.0, 0.0, 1.0, 0.08))
                    .rounded(px(4.0))
                    .text_color(rgb(0x00BFFF))
                    .child(format!("{} ({}%)", suggestion, confidence_pct)),
            )
            .child(
                div()
                    .text_color(rgb(0x666666))
                    .child("? Press ↵ to run"),
            )
    }

}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Show shell install modal on first render if shell is missing
        if let Some(shell_info) = self.pending_shell_install.take() {
            let terminal_handle = self.terminal.handle();
            self.modal_layer.update(cx, |layer, cx| {
                layer.toggle_modal(window, cx, |window, cx| {
                    shell_install::ShellInstallModal::new(
                        shell_info,
                        terminal_handle,
                        window,
                        cx,
                    )
                });
            });
        }

        // Process pending shell switch from the shell selector popover.
        // Uses defer_in to run AFTER this render completes — ensures the popover
        // has closed before any dialog opens (avoids z-order conflicts).
        let pending = cx.global_mut::<PendingShellSwitch>().0.take();
        if let Some(shell) = pending {
            cx.defer_in(window, move |workspace, window, cx| {
                workspace.request_shell_change(shell, window, cx);
            });
        }

        let mut container = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x121212))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down_interactive))
            .child(self.render_title_bar(cx));

        match self.view_mode {
            ViewMode::Terminal => {
                let handle = self.terminal.handle();
                let config = cx.global::<raijin_settings::RaijinConfig>();
                let font_family = config.appearance.font_family.clone();
                let font_size = config.appearance.font_size as f32;
                let line_height_multiplier = config.appearance.line_height as f32;

                // Resize PTY based on viewport dimensions (once per frame)
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
                    let cols = ((viewport.width - px(crate::terminal::constants::BLOCK_HEADER_PAD_X)) / cell_width)
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

                    // Correction banner (e.g. "Did you mean `git status`?")
                    if let Some(ref correction) = self.correction_suggestion {
                        container = container.child(self.render_correction_banner(correction));
                    }

                    // Input area only visible when no command is running
                    if !command_running {
                        // History panel overlay (between terminal output and input area)
                        if self.history_panel.is_visible() {
                            container = container.child(self.history_panel.render());
                        }
                        container = container.child(self.render_input_area());
                    }
                }
            }
            ViewMode::Settings => {
                container = container.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .child(settings_view::build_settings()),
                );
            }
        }

        // Modal layer renders on top of everything
        container = container.child(self.modal_layer.clone());

        container
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

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
