//! A single terminal block rendered as an Inazuma div element.
//!
//! Structure:
//!   div (Block Container — transparent background, border, padding)
//!   ├── div (Header — metadata line + command text)
//!   └── TerminalGridElement (Grid output — renders from pre-extracted snapshot)

use inazuma::{
    div, hsla, px, Font, InteractiveElement, IntoElement, ParentElement, SharedString, Styled,
    prelude::FluentBuilder,
};

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
    let now = time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
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
        let duration = header.finished_at
            .map(|f| f.duration_since(header.started_at))
            .unwrap_or_default();
        format!("({:.3}s)", duration.as_secs_f64())
    };
    parts.push(dur_text);
    parts.join("  ")
}

/// Render the sticky block header — pinned at viewport top when scrolling.
///
/// Absolutely positioned overlay with terminal background. On hover the
/// background switches to accent color (Warp-style). Shows metadata line
/// and command text in the terminal font.
pub fn render_sticky_header(
    header: &super::grid_snapshot::BlockHeaderSnapshot,
    command: &str,
    font: &Font,
    font_size: f32,
) -> impl IntoElement {
    let meta_text = build_metadata_text(header);
    let is_error = header.is_error;
    let command_text: SharedString = command.to_string().into();
    let font_family: SharedString = font.family.clone();

    div()
        .id("sticky-header")
        .w_full()
        .flex_shrink_0()
        .bg(block_body_bg())
        .hover(|s| s.bg(sticky_header_hover_bg()))
        .px(px(BLOCK_HEADER_PAD_X))
        .pt(px(4.0))
        .pb(px(4.0))
        .border_b_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.12))
        .when(is_error, |d| {
            d.border_l(px(BLOCK_LEFT_BORDER))
                .border_l_color(error_color())
        })
        // Metadata line
        .child(
            div()
                .text_xs()
                .text_color(header_metadata_fg())
                .child(meta_text),
        )
        // Command text (terminal font, full multi-line display like Warp)
        .child(
            div()
                .text_size(px(font_size))
                .font_family(font_family)
                .text_color(header_command_fg())
                .child(command_text),
        )
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
) -> impl IntoElement {
    let header = &snapshot.header;
    let meta_text = build_metadata_text(header);

    let is_error = header.is_error;

    // --- Grid element (renders from snapshot, no locking) ---
    // No cursor in output blocks — cursor belongs in the input field (Warp pattern)
    let grid_element = TerminalGridElement::new(snapshot.grid, snapshot.selection, font.clone(), font_size, line_height_multiplier);

    // --- Build div ---
    let bg = if selected {
        block_selected_bg()
    } else {
        block_body_bg()
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
                .border_l_color(error_color())
        })
        // Header
        .child(
            div()
                .px(px(BLOCK_HEADER_PAD_X))
                .pt(px(4.0))
                .pb(px(4.0))
                .child(
                    div()
                        .text_xs()
                        .text_color(header_metadata_fg())
                        .child(meta_text),
                )
                // Command text is rendered as the first line(s) of the grid element,
                // not as a separate div. This makes it selectable, uses the same
                // terminal font, and preserves multi-line formatting.
        )
        // Grid output
        .child(grid_element)
}
