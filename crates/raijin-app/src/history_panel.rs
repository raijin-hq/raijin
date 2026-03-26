/// Warp-style History Panel — visual overlay between terminal output and input area.
///
/// When the user presses Up/Down, a scrollable panel appears showing previous commands
/// with relative timestamps. The selected command is displayed in the input editor
/// with full syntax highlighting.
use inazuma::{
    div, hsla, px, rgb, IntoElement, ParentElement, Styled,
    prelude::*,
};

use crate::command_history::{self, CommandHistory, HistoryEntry};

/// Maximum number of visible rows in the history panel.
const MAX_VISIBLE_ROWS: usize = 8;

/// State for the history panel overlay.
pub struct HistoryPanel {
    /// Whether the panel is currently visible.
    visible: bool,
    /// Filtered/displayed entries (newest first).
    entries: Vec<HistoryEntry>,
    /// Index of the currently highlighted entry.
    selected_index: usize,
    /// The input text saved when the panel opens.
    saved_input: String,
    /// Current filter query (user typing while panel is open).
    filter_query: String,
    /// Scroll offset for long history lists.
    scroll_offset: usize,
}

impl HistoryPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
            selected_index: 0,
            saved_input: String::new(),
            filter_query: String::new(),
            scroll_offset: 0,
        }
    }

    /// Open the panel with all history entries. Does nothing if history is empty.
    pub fn open(&mut self, history: &CommandHistory, current_input: &str) {
        if history.len() == 0 {
            return;
        }
        self.visible = true;
        self.saved_input = current_input.to_string();
        self.entries = history.entries().iter().rev().cloned().collect();
        self.selected_index = 0;
        self.filter_query.clear();
        self.scroll_offset = 0;
    }

    /// Close the panel and return the saved input to restore.
    pub fn close(&mut self) -> String {
        self.visible = false;
        self.filter_query.clear();
        std::mem::take(&mut self.saved_input)
    }

    /// Whether the panel is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Select the previous (older) entry.
    pub fn select_previous(&mut self) {
        if self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
            // Adjust scroll if selected goes below visible window
            if self.selected_index >= self.scroll_offset + MAX_VISIBLE_ROWS {
                self.scroll_offset = self.selected_index - MAX_VISIBLE_ROWS + 1;
            }
        }
    }

    /// Select the next (newer) entry.
    pub fn select_next(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            // Adjust scroll if selected goes above visible window
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    /// Whether the selection is at the bottom (newest entry, index 0).
    pub fn is_at_bottom(&self) -> bool {
        self.selected_index == 0
    }

    /// The currently selected command text, if any.
    pub fn selected_command(&self) -> Option<&str> {
        self.entries
            .get(self.selected_index)
            .map(|e| e.command.as_str())
    }

    /// Apply a fuzzy filter to the history entries.
    pub fn filter(&mut self, query: &str, history: &CommandHistory) {
        self.filter_query = query.to_string();
        if query.is_empty() {
            self.entries = history.entries().iter().rev().cloned().collect();
        } else {
            self.entries = history
                .fuzzy_filter(query)
                .into_iter()
                .cloned()
                .collect();
        }
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Render the history panel as an Inazuma element.
    pub fn render(&self) -> impl IntoElement {
        let bg = rgb(0x1a1a1a);
        let border_color = hsla(0.0, 0.0, 1.0, 0.08);

        let visible_start = self.scroll_offset;
        let visible_end = (self.scroll_offset + MAX_VISIBLE_ROWS).min(self.entries.len());
        let visible_entries = &self.entries[visible_start..visible_end];

        let mut list = div()
            .flex()
            .flex_col()
            .w_full()
            .bg(bg)
            .border_t_1()
            .border_b_1()
            .border_color(border_color)
            .py_1()
            .max_h(px(MAX_VISIBLE_ROWS as f32 * 28.0 + 32.0));

        for (i, entry) in visible_entries.iter().enumerate() {
            let abs_index = visible_start + i;
            let is_selected = abs_index == self.selected_index;
            let time_str = command_history::relative_time(entry.timestamp);

            // Flatten multi-line commands to single line for the panel row display
            let flat_cmd: String = entry.command.lines().collect::<Vec<_>>().join(" ");
            let cmd_display = if flat_cmd.len() > 80 {
                format!("{}...", &flat_cmd[..77])
            } else {
                flat_cmd
            };

            let row = div()
                .id(abs_index)
                .flex()
                .items_center()
                .w_full()
                .h(px(28.0))
                .px_3()
                .gap_2()
                .text_xs()
                .when(is_selected, |this| {
                    this.bg(hsla(
                        153.0 / 360.0,
                        0.93,
                        0.51,
                        0.10,
                    ))
                })
                // >_ icon
                .child(
                    div()
                        .text_color(rgb(0x666666))
                        .flex_shrink_0()
                        .child(">_"),
                )
                // Command text
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_color(if is_selected {
                            rgb(0xf1f1f1)
                        } else {
                            rgb(0xbbbbbb)
                        })
                        .child(cmd_display),
                )
                // Relative time
                .child(
                    div()
                        .flex_shrink_0()
                        .text_color(rgb(0x666666))
                        .child(time_str),
                );

            list = list.child(row);
        }

        // Navigation hints bar
        let hints = div()
            .flex()
            .items_center()
            .w_full()
            .h(px(24.0))
            .px_3()
            .gap_3()
            .text_xs()
            .text_color(rgb(0x555555))
            .border_t_1()
            .border_color(border_color)
            .child(format!("↑ ↓ to navigate ({} commands)", self.entries.len()))
            .child(
                div()
                    .px_1()
                    .border_1()
                    .border_color(rgb(0x444444))
                    .rounded(px(3.0))
                    .child("esc"),
            )
            .child("to dismiss");

        list = list.child(hints);

        list
    }
}
