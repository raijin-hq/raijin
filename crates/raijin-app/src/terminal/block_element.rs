//! A single terminal block rendered as an Inazuma div element.
//!
//! Structure:
//!   div (Block Container — transparent background, border, padding)
//!   ├── div (Header — metadata line + command text)
//!   └── TerminalGridElement (Grid output — renders from pre-extracted snapshot)

use inazuma::{
    div, hsla, px, Font, IntoElement, ParentElement, Styled,
    prelude::FluentBuilder,
};

use super::constants::*;
use super::grid_element::TerminalGridElement;
use super::grid_snapshot::BlockSnapshot;

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

    // --- Header metadata (Warp-style: "nyxb Mac.fritz.box ~ git:(main) 23:32 (0.028s)") ---
    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(ref user) = header.username {
        meta_parts.push(user.clone());
    }
    if let Some(ref host) = header.hostname {
        meta_parts.push(host.clone());
    }
    if let Some(ref cwd) = header.cwd {
        // Abbreviate home directory to ~
        let display_cwd = if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy();
            if cwd.starts_with(home_str.as_ref()) {
                format!("~{}", &cwd[home_str.len()..])
            } else {
                cwd.clone()
            }
        } else {
            cwd.clone()
        };
        meta_parts.push(display_cwd);
    }
    if let Some(ref branch) = header.git_branch {
        meta_parts.push(format!("git:({})", branch));
    }

    let elapsed = header.started_at.elapsed();
    let now = time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let at_start = now - elapsed;
    let time_display = at_start
        .format(time::macros::format_description!("[hour]:[minute]"))
        .unwrap_or_else(|_| "--:--".to_string());
    meta_parts.push(time_display);

    let dur_text = if header.is_running {
        "running...".to_string()
    } else if let Some(ms) = header.duration_ms {
        // Shell-measured duration (preexec → precmd), includes shell overhead
        format!("({:.3}s)", ms as f64 / 1000.0)
    } else {
        // Fallback to terminal-side measurement (OSC 133;C → D)
        let duration = header.finished_at
            .map(|f| f.duration_since(header.started_at))
            .unwrap_or_default();
        format!("({:.3}s)", duration.as_secs_f64())
    };
    meta_parts.push(dur_text);
    let meta_text = meta_parts.join("  ");

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
