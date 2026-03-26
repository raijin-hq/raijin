use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;
use inazuma::{
    div, hsla, px, rgb, App, Context, Entity, Focusable, FocusHandle, KeyDownEvent,
    ParentElement, Render, Styled, Window, prelude::*,
};
use inazuma_component::{
    IconName, TitleBar,
    chip::{Chip, GitBranchChip, GitStatsChip},
    h_flex,
    input::{AutoPairConfig, Input, InputEvent, InputState},
};
use raijin_shell::ShellContext;
use raijin_terminal::{BlockManager, Terminal, TerminalEvent};

use std::rc::Rc;
use std::sync::{Arc, RwLock};

use crate::command_correction;
use crate::command_history::CommandHistory;
use crate::history_panel::HistoryPanel;
use crate::shell_completion::ShellCompletionProvider;
use crate::settings_view;
use crate::terminal_element::{BlockRenderInfo, TerminalElement};

/// Which view is currently active.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Terminal,
    Settings,
}

/// Top-level workspace view that composes the Warp-style layout:
/// TabBar (top) → Terminal Output (middle) → Context Chips + Input (bottom)
pub struct Workspace {
    terminal: Terminal,
    terminal_title: String,
    input_state: Entity<InputState>,
    focus_handle: FocusHandle,
    shell_context: ShellContext,
    block_manager: BlockManager,
    command_history: Arc<RwLock<CommandHistory>>,
    history_panel: HistoryPanel,
    correction_suggestion: Option<command_correction::CorrectionResult>,
    shell_completion: Rc<ShellCompletionProvider>,
    shell_name: String,
    interactive_mode: bool,
    show_terminal: bool,
    view_mode: ViewMode,
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

        let focus_handle = cx.focus_handle();

        // Detect shell language for syntax highlighting
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let shell_name = shell.rsplit('/').next().unwrap_or("zsh");
        let shell_lang = match shell_name {
            "nu" => "nu",
            "fish" => "bash", // Fallback: fish → bash highlighting (close enough)
            _ => "bash",      // zsh, bash, sh → bash highlighting
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
            block_manager: BlockManager::new(),
            command_history,
            history_panel: HistoryPanel::new(),
            correction_suggestion: None,
            shell_completion,
            shell_name: shell_name.to_string(),
            interactive_mode: false,
            show_terminal: false,
            view_mode: ViewMode::Terminal,
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
        // Get absolute cursor row: history_size + cursor_line gives a monotonic position
        let cursor_row = {
            let handle = self.terminal.handle();
            let term = handle.lock();
            let history = term.grid().history_size();
            let cursor_line = term.grid().cursor.point.line.0 as usize;
            history + cursor_line
        };

        // Handle metadata — update context chips before block processing
        if let raijin_terminal::ShellMarker::Metadata(ref json) = marker {
            match serde_json::from_str::<raijin_shell::ShellMetadataPayload>(json) {
                Ok(payload) => {
                    self.shell_context.update_from_metadata(&payload);
                    // Update completion provider CWD for file path completions
                    self.shell_completion.update_cwd(std::path::PathBuf::from(&payload.cwd));
                    // Transfer shell-measured duration to the last finished block
                    if let Some(ms) = payload.last_duration_ms {
                        self.block_manager.set_last_block_duration(ms);
                    }
                    cx.notify();
                }
                Err(e) => {
                    log::warn!("Failed to parse shell metadata JSON: {}", e);
                }
            }
        }

        // Feed marker to BlockManager for block tracking
        self.block_manager.process_marker(marker.clone(), cursor_row);

        match marker {
            raijin_terminal::ShellMarker::PromptStart => {
                // In Raijin mode, don't show terminal until first command runs
                cx.notify();
            }
            raijin_terminal::ShellMarker::InputStart => {}
            raijin_terminal::ShellMarker::CommandStart => {
                self.show_terminal = true;
                cx.notify();
            }
            raijin_terminal::ShellMarker::CommandEnd { exit_code } => {
                log::info!("Command finished with exit code: {}", exit_code);

                // Check for typo correction on "command not found"
                if exit_code == 127 {
                    if let Some(last_cmd) = self.block_manager.last_command() {
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
                    self.block_manager
                        .set_pending_command(value.to_string());
                    let mut bytes = value.as_bytes().to_vec();
                    bytes.push(b'\r');
                    self.terminal.write(&bytes);
                    self.show_terminal = true;
                    self.input_state.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                    });
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
            }
            _ => {}
        }
    }

    fn on_key_down_interactive(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Handle Escape to dismiss history panel
        if self.history_panel.is_visible() && event.keystroke.key.as_str() == "escape" {
            let saved = self.history_panel.close();
            self.input_state.update(cx, |state, cx| {
                state.set_value(&saved, window, cx);
            });
            cx.notify();
            return;
        }

        if !self.interactive_mode {
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
            .child(Chip::new(&self.shell_context.username, rgb(0x14F195).into()))
            .child(Chip::new(&self.shell_context.hostname, rgb(0xc8c8c8).into()))
            .child(
                Chip::new(&self.shell_context.cwd_short, rgb(0x6ee7b7).into())
                    .icon(IconName::Folder),
            )
            .child(
                Chip::new(&time_str, rgb(0xff5f5f).into())
                    .icon(IconName::Clock),
            )
            .child(Chip::new(&self.shell_name, rgb(0xa78bfa).into()));

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
                    .text_color(rgb(0x14F195))
                    .child(format!("{} ({}%)", suggestion, confidence_pct)),
            )
            .child(
                div()
                    .text_color(rgb(0x666666))
                    .child("? Press ↵ to run"),
            )
    }

    fn build_block_render_info(&self) -> Vec<BlockRenderInfo> {
        self.block_manager
            .blocks()
            .iter()
            .map(|block| {
                let payload = block
                    .metadata_json
                    .as_ref()
                    .and_then(|json| {
                        serde_json::from_str::<raijin_shell::ShellMetadataPayload>(json).ok()
                    });

                BlockRenderInfo {
                    command: block.command.clone(),
                    duration_display: block.duration_display(),
                    exit_code: block.exit_code,
                    abs_start_row: block.start_row,
                    abs_end_row: block.end_row,
                    cwd_short: payload.as_ref().map(|p| raijin_shell::shorten_path(&p.cwd)),
                    git_branch: payload.as_ref().and_then(|p| p.git_branch.clone()),
                    username: payload.as_ref().and_then(|p| p.username.clone()),
                    hostname: payload.as_ref().and_then(|p| p.hostname.clone()),
                    time_display: {
                        let t = block.started_at;
                        let elapsed = t.elapsed();
                        // Approximate wall-clock time at block start
                        let now = time::OffsetDateTime::now_local()
                            .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                        let at_start = now - elapsed;
                        at_start
                            .format(time::macros::format_description!("[hour]:[minute]"))
                            .unwrap_or_else(|_| "--:--".to_string())
                    },
                }
            })
            .collect()
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                let blocks = self.build_block_render_info();
                let hide_before = self.block_manager.prompt_start_row();
                let hidden_regions = self.block_manager.hidden_prompt_regions().to_vec();
                let config = cx.global::<raijin_settings::RaijinConfig>();
                let font_family = config.appearance.font_family.clone();
                let font_size = config.appearance.font_size as f32;
                let cursor_beam = config.terminal.cursor_style
                    == raijin_settings::CursorStyle::Beam;

                container = container.child({
                    let output_area = div()
                        .flex()
                        .flex_col()
                        .justify_end()
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden();

                    if self.show_terminal || self.interactive_mode {
                        output_area.child(
                            TerminalElement::new(handle)
                                .with_font(&font_family, font_size)
                                .with_cursor_beam(cursor_beam)
                                .with_blocks(blocks)
                                .with_hide_before_row(hide_before)
                                .with_hidden_prompt_regions(hidden_regions),
                        )
                    } else {
                        output_area
                    }
                });

                if !self.interactive_mode {
                    // Correction banner (e.g. "Did you mean `git status`?")
                    if let Some(ref correction) = self.correction_suggestion {
                        container = container.child(self.render_correction_banner(correction));
                    }

                    // History panel overlay (between terminal output and input area)
                    if self.history_panel.is_visible() {
                        container = container.child(self.history_panel.render());
                    }
                    container = container.child(self.render_input_area());
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
