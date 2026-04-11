//! A single terminal block rendered as an Inazuma div element.
//!
//! Structure:
//!   div (Block Container — transparent background, border, padding)
//!   ├── div (Header — metadata line + command text)
//!   └── TerminalGridElement (Grid output — renders from pre-extracted snapshot)

use inazuma::{
    Font, InteractiveElement, IntoElement, Oklch, ParentElement, SharedString, StatefulInteractiveElement,
    Styled, Window, div, oklcha, prelude::FluentBuilder, px,
};
use raijin_theme::Theme;

use crate::constants::*;
use crate::grid_element::{GridOriginStore, TerminalGridElement};
use crate::grid_snapshot::BlockSnapshot;

/// Build the metadata text line for a block header.
///
/// Format: `"user  host  ~/cwd  git:(branch)  HH:MM  (duration)"`
pub fn build_metadata_text(header: &crate::grid_snapshot::BlockHeaderSnapshot) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(ref user) = header.username {
        parts.push(user.clone());
    }
    if let Some(ref host) = header.hostname {
        parts.push(host.clone());
    }
    if let Some(ref cwd) = header.cwd {
        let display = if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy();
            if cwd.starts_with(home_str.as_ref()) {
                format!("~{}", &cwd[home_str.len()..])
            } else {
                cwd.clone()
            }
        } else {
            cwd.clone()
        };
        parts.push(display);
    }
    if let Some(ref branch) = header.git_branch {
        parts.push(format!("git:({})", branch));
    }

    let elapsed = header.started_at.elapsed();
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let at_start = now - elapsed;
    let time_display = at_start
        .format(time::macros::format_description!("[hour]:[minute]"))
        .unwrap_or_else(|_| "--:--".to_string());
    parts.push(time_display);

    let dur_text = if header.is_running {
        "running...".to_string()
    } else if let Some(ms) = header.duration_ms {
        format!("({:.3}s)", ms as f64 / 1000.0)
    } else {
        let duration = header
            .finished_at
            .map(|f| f.duration_since(header.started_at))
            .unwrap_or_default();
        format!("({:.3}s)", duration.as_secs_f64())
    };
    parts.push(dur_text);
    parts.join("  ")
}

/// Format duration for fold-line display.
fn format_fold_duration(header: &crate::grid_snapshot::BlockHeaderSnapshot) -> String {
    if header.is_running {
        "running...".to_string()
    } else if let Some(ms) = header.duration_ms {
        format!("{:.3}s", ms as f64 / 1000.0)
    } else {
        let duration = header
            .finished_at
            .map(|f| f.duration_since(header.started_at))
            .unwrap_or_default();
        format!("{:.3}s", duration.as_secs_f64())
    }
}

/// Render a single fold-line for a block that scrolled above the viewport.
///
/// Layout: `[badge] command-text ...                  (duration)`
pub fn render_fold_line(
    header: &crate::grid_snapshot::BlockHeaderSnapshot,
    command: &str,
    index: usize,
    theme: &Theme,
    on_click: impl Fn(&inazuma::ClickEvent, &mut Window, &mut inazuma::App) + 'static,
) -> impl IntoElement {
    let (badge, badge_color) = if header.is_running {
        ("●", fold_badge_running(theme))
    } else if header.is_error {
        ("✗", fold_badge_error(theme))
    } else {
        ("✓", fold_badge_success(theme))
    };

    let hover_bg = fold_line_hover_bg(theme);
    let error_bg = fold_line_error_bg(theme);
    let is_error = header.is_error;
    let duration_text: SharedString = format_fold_duration(header).into();
    let command_text: SharedString = command.to_string().into();
    let id = SharedString::from(format!("fold-line-{}", index));

    div()
        .id(id)
        .h(px(FOLD_LINE_HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .px(px(BLOCK_HEADER_PAD_X))
        .gap(px(8.0))
        .when(is_error, |d| d.bg(error_bg))
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_click(on_click)
        .child(
            div()
                .text_size(px(11.0))
                .text_color(badge_color)
                .flex_shrink_0()
                .child(badge),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_x_hidden()
                .text_size(px(12.0))
                .text_color(header_command_fg(theme))
                .child(command_text),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(11.0))
                .text_color(header_metadata_fg(theme))
                .child(duration_text),
        )
}

/// Render the fold counter line — clickable to expand/collapse all fold-lines.
///
/// - `hidden_count > 0`: shows "⌃ N more commands above" (click → show all)
/// - `hidden_count == 0`: shows "▾ show less" (click → collapse back to 3)
pub fn render_fold_counter(
    hidden_count: usize,
    theme: &Theme,
    on_click: impl Fn(&inazuma::ClickEvent, &mut Window, &mut inazuma::App) + 'static,
) -> impl IntoElement {
    let text: SharedString = if hidden_count > 0 {
        format!("⌃ {} more commands above", hidden_count).into()
    } else {
        "▾ show less".into()
    };
    let hover_bg = fold_line_hover_bg(theme);

    div()
        .id("fold-counter")
        .h(px(FOLD_COUNTER_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .px(px(BLOCK_HEADER_PAD_X))
        .text_size(px(11.0))
        .text_color(header_metadata_fg(theme))
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_click(on_click)
        .child(text)
}


/// Render a single block from a pre-extracted snapshot.
///
/// No mutex locking happens here — all data comes from the snapshot.
pub fn render_block(
    snapshot: BlockSnapshot,
    font: &Font,
    font_size: f32,
    line_height_multiplier: f32,
    selected: bool,
    theme: &Theme,
    grid_origin_store: Option<GridOriginStore>,
) -> impl IntoElement {
    let header = &snapshot.header;
    let meta_text = build_metadata_text(header);

    let is_error = header.is_error;

    let selection_highlight = accent_color(theme).opacity(0.45);
    let mut grid_element = TerminalGridElement::new(
        snapshot.grid,
        snapshot.selection,
        font.clone(),
        font_size,
        line_height_multiplier,
        terminal_bg(theme),
        selection_highlight,
    );
    if let Some(store) = grid_origin_store {
        grid_element = grid_element.with_origin_store(store);
    }

    // No permanent background — transparent like Warp, so the background
    // image shows through uniformly. Only selected/error states get a tint.
    let error_bg = oklcha(0.25, 0.08, 25.0, 0.15);

    div()
        .w_full()
        .when(selected, |d| d.bg(block_selected_bg(theme)))
        .when(is_error && !selected, |d| d.bg(error_bg))
        .border_t_1()
        .border_color(Oklch::white().opacity(0.08))
        .pb(px(BLOCK_BODY_PAD_BOTTOM))
        .when(is_error, |d| {
            d.border_l(px(BLOCK_LEFT_BORDER))
                .border_l_color(error_color(theme))
        })
        .child(
            div()
                .px(px(BLOCK_HEADER_PAD_X))
                .pt(px(4.0))
                .pb(px(4.0))
                .child(
                    div()
                        .text_xs()
                        .text_color(header_metadata_fg(theme))
                        .child(meta_text),
                ),
        )
        .child(grid_element)
}
