//! A single terminal block rendered as an Inazuma div element.
//!
//! Each block contains: header (metadata + command) + grid output.
//! Blocks are clickable and selectable with accent-colored highlight.

use inazuma::{
    div, px, Bounds, Font, Hsla, IntoElement, ParentElement, Pixels, Point,
    ShapedLine, Styled, Window,
};
use raijin_term::block_grid::BlockGrid;
use raijin_term::grid::Dimensions;
use raijin_term::index::Line;
use raijin_term::term::color::Colors;

use super::constants::*;
use super::text_rendering::shape_grid_row;

/// Render a single block as an Inazuma element (div-based).
pub fn render_block(
    block: &BlockGrid,
    colors: &Colors,
    font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
    cell_height: Pixels,
    selected: bool,
    window: &mut Window,
) -> impl IntoElement {
    let is_error = block.is_error();
    let is_running = !block.is_finished();
    let bg = if selected {
        block_selected_bg()
    } else {
        block_body_bg()
    };

    let text_x = px(BLOCK_HEADER_PAD_X);

    // --- Header ---
    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(ref user) = block.metadata.username {
        meta_parts.push(user.clone());
    }
    if let Some(ref host) = block.metadata.hostname {
        meta_parts.push(host.clone());
    }
    if let Some(ref cwd) = block.metadata.cwd {
        meta_parts.push(cwd.clone());
    }
    if let Some(ref branch) = block.metadata.git_branch {
        meta_parts.push(format!(" {}", branch));
    }

    let elapsed = block.started_at.elapsed();
    let now = time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let at_start = now - elapsed;
    let time_display = at_start
        .format(time::macros::format_description!("[hour]:[minute]"))
        .unwrap_or_else(|_| "--:--".to_string());
    meta_parts.push(time_display);

    let dur_text = if is_running {
        "running...".to_string()
    } else {
        let duration = block.finished_at
            .map(|f| f.duration_since(block.started_at))
            .unwrap_or_default();
        format!("({:.3}s)", duration.as_secs_f64())
    };
    meta_parts.push(dur_text);

    let meta_text = meta_parts.join("  ");

    // --- Command ---
    let cmd_text = if block.command.is_empty() {
        "(empty)".to_string()
    } else {
        block.command.clone()
    };

    // Flatten multi-line commands for the header display
    let cmd_display: String = cmd_text.lines().collect::<Vec<_>>().join(" ");
    let cmd_truncated = if cmd_display.len() > 120 {
        format!("{}...", &cmd_display[..117])
    } else {
        cmd_display
    };

    // --- Grid output rows ---
    let grid = &block.grid;
    let history_size = grid.history_size();
    let cursor_line = grid.cursor.point.line.0.max(0) as usize;
    let content_rows = history_size + cursor_line + 1;
    let screen_lines = grid.screen_lines() as i32;

    let mut output_lines: Vec<(Point<Pixels>, ShapedLine)> = Vec::new();
    let mut output_bgs: Vec<(Bounds<Pixels>, Hsla)> = Vec::new();
    let bg_color = terminal_bg();

    let mut row_y = px(0.0);
    for row_offset in 0..content_rows {
        let line_idx = row_offset as i32 - history_size as i32;
        let line = Line(line_idx);

        if line.0 >= screen_lines || line.0 < -(history_size as i32) {
            row_y += cell_height;
            continue;
        }

        let (shaped, bgs) = shape_grid_row(
            grid,
            line,
            colors,
            text_x,
            row_y,
            font,
            font_size,
            cell_width,
            cell_height,
            bg_color,
            window,
        );

        if let Some((origin, shaped_line)) = shaped {
            output_lines.push((origin, shaped_line));
        }
        output_bgs.extend(bgs);

        row_y += cell_height;
    }

    let output_height = cell_height * content_rows as f32;
    let header_height = px(BLOCK_HEADER_HEIGHT);
    let total_height = header_height + output_height + px(BLOCK_BODY_PAD_BOTTOM);

    // Build the div
    let mut block_div = div()
        .w_full()
        .h(total_height)
        .bg(bg)
        .px(px(0.0));

    // Error left border
    if is_error {
        block_div = block_div
            .border_l(px(BLOCK_LEFT_BORDER))
            .border_color(error_color());
    }

    block_div
}
