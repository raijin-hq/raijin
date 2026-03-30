//! A single terminal block rendered as an Inazuma div element.
//!
//! Structure:
//!   div (Block Container — transparent background, border, padding)
//!   ├── div (Header — metadata line + command text)
//!   └── TerminalGridElement (Grid output — renders from pre-extracted snapshot)

use inazuma::{
    Font, InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled, Window, div, hsla, prelude::FluentBuilder, px,
};
use raijin_settings::ResolvedTheme;

use super::constants::*;
use super::grid_element::TerminalGridElement;
use super::grid_snapshot::BlockSnapshot;

/// Build the metadata text line for a block header.
///
/// Format: `"user  host  ~/cwd  git:(branch)  HH:MM  (duration)"`
pub fn build_metadata_text(header: &super::grid_snapshot::BlockHeaderSnapshot) -> String {
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

/// Render the sticky block header — pinned at viewport top when scrolling.
///
/// The header contains a chevron toggle button that straddles its bottom border
/// (like Warp): rounded top corners inside the header, flat bottom edge extending
/// below. The chevron is absolutely positioned so it peeks out from under the
/// header's bottom border.
pub fn render_sticky_header(
    header: &super::grid_snapshot::BlockHeaderSnapshot,
    command: &str,
    font: &Font,
    font_size: f32,
    collapsed: bool,
    theme: &ResolvedTheme,
    on_toggle: impl Fn(&inazuma::ClickEvent, &mut Window, &mut inazuma::App) + 'static,
) -> inazuma::AnyElement {
    let is_error = header.is_error;
    let chevron = if collapsed { "▼" } else { "▲" };

    // Chevron half-pill: rounded top, flat bottom — straddles the header border.
    // Collapsed header has ~0 height, so the pill needs to hang further below.
    let chevron_bottom = if collapsed { px(-18.0) } else { px(-3.0) };
    let chevron_pill = div()
        .absolute()
        .bottom(chevron_bottom)
        .left_0()
        .w_full()
        .flex()
        .justify_center()
        .child(
            div()
                .id("sticky-chevron")
                .px(px(14.0))
                .pt(px(3.0))
                .pb(px(1.0))
                .when(collapsed, |d| d.rounded_b(px(6.0)))
                .when(!collapsed, |d| d.rounded_t(px(6.0)))
                .bg(hsla(0.0, 0.0, 0.15, 0.9))
                .text_size(px(11.0))
                .text_color(hsla(0.0, 0.0, 1.0, 0.5))
                .cursor_pointer()
                .hover(|s| {
                    s.bg(hsla(0.0, 0.0, 0.25, 0.95))
                        .text_color(hsla(0.0, 0.0, 1.0, 0.8))
                })
                .on_click(on_toggle)
                .child(chevron),
        );

    if collapsed {
        // Collapsed: thin border line + chevron peeking below
        div()
            .id("sticky-header")
            .w_full()
            .flex_shrink_0()
            .relative()
            .border_b_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
            .when(is_error, |d| {
                d.border_l(px(BLOCK_LEFT_BORDER))
                    .border_l_color(error_color(theme))
            })
            .child(chevron_pill)
            .into_any_element()
    } else {
        // Expanded: metadata + command text + chevron at bottom border
        let meta_text = build_metadata_text(header);
        let command_text: SharedString = command.to_string().into();
        let font_family: SharedString = font.family.clone();
        let hover_bg = sticky_header_hover_bg(theme);

        div()
            .id("sticky-header")
            .w_full()
            .flex_shrink_0()
            .relative()
            .bg(block_body_bg(theme))
            .hover(move |s| s.bg(hover_bg))
            .border_b_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
            .when(is_error, |d| {
                d.border_l(px(BLOCK_LEFT_BORDER))
                    .border_l_color(error_color(theme))
            })
            .px(px(BLOCK_HEADER_PAD_X))
            .pt(px(4.0))
            .pb(px(4.0))
            .child(
                div()
                    .text_xs()
                    .text_color(header_metadata_fg(theme))
                    .child(meta_text),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .font_family(font_family)
                    .text_color(header_command_fg(theme))
                    .child(command_text),
            )
            .child(chevron_pill)
            .into_any_element()
    }
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
    theme: &ResolvedTheme,
) -> impl IntoElement {
    let header = &snapshot.header;
    let meta_text = build_metadata_text(header);

    let is_error = header.is_error;

    let grid_element = TerminalGridElement::new(
        snapshot.grid,
        snapshot.selection,
        font.clone(),
        font_size,
        line_height_multiplier,
        terminal_bg(theme),
    );

    let bg = if selected {
        block_selected_bg(theme)
    } else {
        block_body_bg(theme)
    };

    let error_bg = hsla(0.0, 0.5, 0.18, 0.15);

    div()
        .w_full()
        .bg(if is_error { error_bg } else { bg })
        .border_t_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.08))
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
