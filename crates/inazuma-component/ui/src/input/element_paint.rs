use inazuma::{
    App, Bounds, Hsla, Pixels, Point, Size, TextAlign, Window, fill, point, px, size,
};

use crate::{ActiveTheme as _, Colorize};

use super::element::{PrepaintState, TextElement, LINE_NUMBER_RIGHT_MARGIN};

impl TextElement {
    /// Paint active line backgrounds behind content.
    pub(super) fn paint_active_lines(
        prepaint: &PrepaintState,
        input_bounds: &Bounds<Pixels>,
        origin: Point<Pixels>,
        line_height: Pixels,
        active_line_color: Option<Hsla>,
        window: &mut Window,
    ) {
        let Some(line_numbers) = prepaint.line_numbers.as_ref() else {
            return;
        };

        let mut offset_y = prepaint.last_layout.visible_top;

        for (lines, &buffer_line) in line_numbers
            .iter()
            .zip(prepaint.last_layout.visible_buffer_lines.iter())
        {
            let is_active = prepaint.current_row == Some(buffer_line);
            let p = point(input_bounds.origin.x, origin.y + offset_y);
            let height = line_height * lines.len() as f32;
            if is_active {
                if let Some(bg_color) = active_line_color {
                    window.paint_quad(fill(
                        Bounds::new(p, size(prepaint.bounds.size.width, height)),
                        bg_color,
                    ));
                }
            }
            offset_y += height;
        }
    }

    /// Paint search matches, selection, hover highlight, and document color overlays.
    pub(super) fn paint_selections_and_highlights(
        prepaint: &mut PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !window.is_window_active() {
            return;
        }

        let secondary_selection = cx.theme().selection.saturation(0.1);
        for (path, is_active) in prepaint.search_match_paths.iter() {
            window.paint_path(path.clone(), secondary_selection);

            if *is_active {
                window.paint_path(path.clone(), cx.theme().selection);
            }
        }

        if let Some(path) = prepaint.selection_path.take() {
            window.paint_path(path, cx.theme().selection);
        }

        if let Some(path) = prepaint.hover_highlight_path.take() {
            window.paint_path(path, secondary_selection);
        }
    }

    /// Paint document color overlays.
    pub(super) fn paint_document_colors(prepaint: &PrepaintState, window: &mut Window) {
        for (path, color) in prepaint.document_color_paths.iter() {
            window.paint_path(path.clone(), *color);
        }
    }

    /// Paint text lines including inline completion ghost lines.
    ///
    /// Returns the y-position of the cursor row for positioning the first line suffix.
    pub(super) fn paint_text_lines(
        prepaint: &PrepaintState,
        origin: Point<Pixels>,
        mask_offset_y: Pixels,
        scroll_size: &Size<Pixels>,
        text_align: TextAlign,
        line_height: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Pixels> {
        let mut offset_y = mask_offset_y + prepaint.last_layout.visible_top;
        let ghost_lines = &prepaint.ghost_lines;
        let has_ghost_lines = !ghost_lines.is_empty();
        let bounds = &prepaint.bounds;

        // Keep scrollbar offset always be positive, start from the left position
        let scroll_offset = if text_align == TextAlign::Right {
            (scroll_size.width - bounds.size.width).max(px(0.))
        } else if text_align == TextAlign::Center {
            ((scroll_size.width - bounds.size.width) / 2.).max(px(0.))
        } else {
            px(0.)
        };

        let mut cursor_row_y = None;

        for (line, &buffer_line) in prepaint
            .last_layout
            .lines
            .iter()
            .zip(prepaint.last_layout.visible_buffer_lines.iter())
        {
            let row = buffer_line;
            let line_y = origin.y + offset_y;
            let p = point(
                origin.x + prepaint.last_layout.line_number_width + scroll_offset,
                line_y,
            );

            _ = line.paint(
                p,
                line_height,
                text_align,
                Some(prepaint.last_layout.content_width),
                window,
                cx,
            );
            offset_y += line.size(line_height).height;

            if Some(row) == prepaint.current_row {
                cursor_row_y = Some(line_y);
            }

            // After the cursor row, paint ghost lines (which shifts subsequent content down)
            if has_ghost_lines && Some(row) == prepaint.current_row {
                let ghost_x = origin.x + prepaint.last_layout.line_number_width;

                for ghost_line in ghost_lines {
                    let ghost_p = point(ghost_x, origin.y + offset_y);

                    let ghost_bounds = Bounds::new(
                        ghost_p,
                        size(
                            bounds.size.width - prepaint.last_layout.line_number_width,
                            line_height,
                        ),
                    );
                    window.paint_quad(fill(ghost_bounds, cx.theme().editor_background()));

                    _ = ghost_line.paint(
                        ghost_p,
                        line_height,
                        text_align,
                        Some(prepaint.last_layout.content_width),
                        window,
                        cx,
                    );
                    offset_y += line_height;
                }
            }
        }

        cursor_row_y
    }

    /// Paint line numbers and their backgrounds.
    pub(super) fn paint_line_numbers(
        prepaint: &PrepaintState,
        input_bounds: &Bounds<Pixels>,
        origin: Point<Pixels>,
        line_height: Pixels,
        active_line_color: Option<Hsla>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(line_numbers) = prepaint.line_numbers.as_ref() else {
            return;
        };

        let mut offset_y = prepaint.last_layout.visible_top;

        window.paint_quad(fill(
            Bounds {
                origin: input_bounds.origin,
                size: size(
                    prepaint.last_layout.line_number_width - LINE_NUMBER_RIGHT_MARGIN,
                    input_bounds.size.height + prepaint.ghost_lines_height,
                ),
            },
            cx.theme().editor_background(),
        ));

        for (lines, &buffer_line) in line_numbers
            .iter()
            .zip(prepaint.last_layout.visible_buffer_lines.iter())
        {
            let p = point(input_bounds.origin.x, origin.y + offset_y);
            let is_active = prepaint.current_row == Some(buffer_line);

            let height = line_height * lines.len() as f32;
            if is_active {
                if let Some(bg_color) = active_line_color {
                    window.paint_quad(fill(
                        Bounds::new(p, size(prepaint.last_layout.line_number_width, height)),
                        bg_color,
                    ));
                }
            }

            for line in lines {
                _ = line.paint(p, line_height, TextAlign::Left, None, window, cx);
                offset_y += line_height;
            }

            // Add ghost line height after cursor row for line numbers alignment
            if !prepaint.ghost_lines.is_empty() && prepaint.current_row == Some(buffer_line) {
                offset_y += prepaint.ghost_lines_height;
            }
        }
    }

    /// Paint the inline completion first line suffix after the cursor.
    pub(super) fn paint_inline_completion_suffix(
        prepaint: &PrepaintState,
        cursor_row_y: Option<Pixels>,
        text_align: TextAlign,
        line_height: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(first_line) = &prepaint.ghost_first_line else {
            return;
        };

        let Some(cursor_bounds) = prepaint.cursor_bounds_with_scroll() else {
            return;
        };

        let Some(cursor_row_y) = cursor_row_y else {
            return;
        };

        let first_line_x = cursor_bounds.origin.x + cursor_bounds.size.width;
        let p = point(first_line_x, cursor_row_y);

        // Paint background to cover any existing text
        let bg_bounds = Bounds::new(p, size(first_line.width + px(4.), line_height));
        window.paint_quad(fill(bg_bounds, cx.theme().editor_background()));

        // Paint first line completion text
        _ = first_line.paint(p, line_height, text_align, None, window, cx);
    }

    /// Update state after painting.
    pub(super) fn paint_state_update(
        &self,
        prepaint: &mut PrepaintState,
        input_bounds: Bounds<Pixels>,
        selected_range: std::ops::Range<usize>,
        cx: &mut App,
    ) {
        let bounds = prepaint.bounds;
        self.state.update(cx, |state, cx| {
            state.last_layout = Some(prepaint.last_layout.clone());
            state.last_bounds = Some(bounds);
            state.last_cursor = Some(state.cursor());
            state.set_input_bounds(input_bounds, cx);
            state.last_selected_range = Some(selected_range.into());
            state.scroll_size = prepaint.scroll_size;
            state.update_scroll_offset(Some(prepaint.cursor_scroll_offset), cx);
            state.deferred_scroll_offset = None;

            cx.notify();
        });
    }
}
