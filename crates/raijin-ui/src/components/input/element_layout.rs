use std::ops::Range;

use inazuma::{
    App, Bounds, Corners, Half, HighlightStyle, HitboxBehavior, Oklch, InteractiveElement,
    IntoElement, MouseButton, Path, Pixels, Point, ShapedLine, SharedString, Styled as _,
    TextAlign, TextRun, TextStyle, Window, point, px, size,
};
use ropey::Rope;
use smallvec::SmallVec;

use crate::{
    ActiveTheme as _, Button, ButtonVariants as _, IconName, Selectable, Sizable as _,
    input::{RopeExt as _, blink_cursor::CURSOR_WIDTH, display_map::LineLayout},
};

use super::{
    InputState, LastLayout, WhitespaceIndicators,
    element::{
        BOTTOM_MARGIN_ROWS, FOLD_ICON_HITBOX_WIDTH, FOLD_ICON_WIDTH, FoldIconLayout,
        LINE_NUMBER_RIGHT_MARGIN, MAX_HIGHLIGHT_LINE_LENGTH, RIGHT_MARGIN, TextElement,
    },
    mode::InputMode,
};

impl TextElement {
    /// Returns the:
    ///
    /// - cursor bounds
    /// - scroll offset
    /// - current row index (No only the visible lines, but all lines)
    ///
    /// This method also will update for track scroll to cursor.
    pub(super) fn layout_cursor(
        &self,
        last_layout: &LastLayout,
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        cx: &mut App,
    ) -> (Option<Bounds<Pixels>>, Point<Pixels>, Option<usize>) {
        let state = self.state.read(cx);

        let line_height = last_layout.line_height;
        let visible_range = &last_layout.visible_range;
        let lines = &last_layout.lines;
        let line_number_width = last_layout.line_number_width;

        let mut selected_range = state.selected_range;

        if let Some(ime_marked_range) = &state.ime_marked_range {
            selected_range = (ime_marked_range.end..ime_marked_range.end).into();
        }
        let is_selected_all = selected_range.len() == state.text.len();

        let mut cursor = state.cursor();
        if state.masked {
            // Because masked use `*`, 1 char with 1 byte.
            selected_range.start = state.text.offset_to_char_index(selected_range.start);
            selected_range.end = state.text.offset_to_char_index(selected_range.end);
            cursor = state.text.offset_to_char_index(cursor);
        }

        let mut current_row = None;
        let mut scroll_offset = state.scroll_handle.offset();
        let mut cursor_bounds = None;

        // If the input has a fixed height (Otherwise is auto-grow), we need to add a bottom margin to the input.
        let top_bottom_margin = if state.mode.is_auto_grow() {
            line_height
        } else if visible_range.len() < BOTTOM_MARGIN_ROWS * 8 {
            line_height
        } else {
            BOTTOM_MARGIN_ROWS * line_height
        };

        // The cursor corresponds to the current cursor position in the text no only the line.
        let mut cursor_pos = None;
        let mut cursor_start = None;
        let mut cursor_end = None;

        let mut prev_lines_offset = 0;
        let mut offset_y = px(0.);
        let buffer_lines = state.display_map.lines();
        let visible_buffer_lines = &last_layout.visible_buffer_lines;
        let mut vi = 0; // index into visible_buffer_lines / lines
        for (ix, wrap_line) in buffer_lines.iter().enumerate() {
            let row = ix;
            let line_origin = point(px(0.), offset_y);

            // break loop if all cursor positions are found
            if cursor_pos.is_some() && cursor_start.is_some() && cursor_end.is_some() {
                break;
            }

            // Check if this buffer line has a LineLayout in the compact lines vec
            let line_layout = if vi < visible_buffer_lines.len() && visible_buffer_lines[vi] == ix {
                let l = &lines[vi];
                vi += 1;
                Some(l)
            } else {
                None
            };

            if let Some(line) = line_layout {
                if cursor_pos.is_none() {
                    let offset = cursor.saturating_sub(prev_lines_offset);
                    if let Some(pos) = line.position_for_index(offset, last_layout) {
                        current_row = Some(row);
                        cursor_pos = Some(line_origin + pos);
                    }
                }
                if cursor_start.is_none() {
                    let offset = selected_range.start.saturating_sub(prev_lines_offset);
                    if let Some(pos) = line.position_for_index(offset, last_layout) {
                        cursor_start = Some(line_origin + pos);
                    }
                }
                if cursor_end.is_none() {
                    let offset = selected_range.end.saturating_sub(prev_lines_offset);
                    if let Some(pos) = line.position_for_index(offset, last_layout) {
                        cursor_end = Some(line_origin + pos);
                    }
                }

                offset_y += line.size(line_height).height;
                // +1 for the last `\n`
                prev_lines_offset += wrap_line.len() + 1;
            } else {
                // Not visible (before visible range or hidden/folded).
                // Just increase the offset_y and prev_lines_offset for scroll tracking.
                if prev_lines_offset >= cursor && cursor_pos.is_none() {
                    current_row = Some(row);
                    cursor_pos = Some(line_origin);
                }
                if prev_lines_offset >= selected_range.start && cursor_start.is_none() {
                    cursor_start = Some(line_origin);
                }
                if prev_lines_offset >= selected_range.end && cursor_end.is_none() {
                    cursor_end = Some(line_origin);
                }

                let visible_wrap_rows =
                    state.display_map.visible_wrap_row_count_for_buffer_line(ix);
                offset_y += line_height * visible_wrap_rows;
                // +1 for the last `\n`
                prev_lines_offset += wrap_line.len() + 1;
            }
        }

        if let (Some(cursor_pos), Some(cursor_start), Some(cursor_end)) =
            (cursor_pos, cursor_start, cursor_end)
        {
            let selection_changed = state.last_selected_range != Some(selected_range);
            if selection_changed && !is_selected_all {
                // Apart from left alignment, just leave enough space for the cursor size on the right side.
                let safety_margin = if last_layout.text_align == TextAlign::Left {
                    RIGHT_MARGIN
                } else {
                    CURSOR_WIDTH
                };

                scroll_offset.x = if scroll_offset.x + cursor_pos.x
                    > (bounds.size.width - line_number_width - safety_margin)
                {
                    // cursor is out of right
                    bounds.size.width - line_number_width - safety_margin - cursor_pos.x
                } else if scroll_offset.x + cursor_pos.x < px(0.) {
                    // cursor is out of left
                    scroll_offset.x - cursor_pos.x
                } else {
                    scroll_offset.x
                };

                // If we change the scroll_offset.y, GPUI will render and trigger the next run loop.
                // So, here we just adjust offset by `line_height` for move smooth.
                scroll_offset.y =
                    if scroll_offset.y + cursor_pos.y > bounds.size.height - top_bottom_margin {
                        // cursor is out of bottom
                        scroll_offset.y - line_height
                    } else if scroll_offset.y + cursor_pos.y < top_bottom_margin {
                        // cursor is out of top
                        (scroll_offset.y + line_height).min(px(0.))
                    } else {
                        scroll_offset.y
                    };

                // For selection to move scroll
                if state.selection_reversed {
                    if scroll_offset.x + cursor_start.x < px(0.) {
                        // selection start is out of left
                        scroll_offset.x = -cursor_start.x;
                    }
                    if scroll_offset.y + cursor_start.y < px(0.) {
                        // selection start is out of top
                        scroll_offset.y = -cursor_start.y;
                    }
                } else {
                    // TODO: Consider to remove this part,
                    // maybe is not necessary (But selection_reversed is needed).
                    if scroll_offset.x + cursor_end.x <= px(0.) {
                        // selection end is out of left
                        scroll_offset.x = -cursor_end.x;
                    }
                    if scroll_offset.y + cursor_end.y <= px(0.) {
                        // selection end is out of top
                        scroll_offset.y = -cursor_end.y;
                    }
                }
            }

            // cursor bounds
            let cursor_height = match state.size {
                crate::Size::Large => 1.,
                crate::Size::Small => 0.75,
                _ => 0.85,
            } * line_height;

            cursor_bounds = Some(Bounds::new(
                point(
                    bounds.left() + cursor_pos.x + line_number_width + scroll_offset.x,
                    bounds.top() + cursor_pos.y + ((line_height - cursor_height) / 2.),
                ),
                size(CURSOR_WIDTH, cursor_height),
            ));
        }

        if let Some(deferred_scroll_offset) = state.deferred_scroll_offset {
            scroll_offset = deferred_scroll_offset;
        }

        bounds.origin = bounds.origin + scroll_offset;

        (cursor_bounds, scroll_offset, current_row)
    }

    /// Layout the match range to a Path.
    pub(crate) fn layout_match_range(
        range: Range<usize>,
        last_layout: &LastLayout,
        bounds: &Bounds<Pixels>,
    ) -> Option<Path<Pixels>> {
        if range.is_empty() {
            return None;
        }

        if range.start < last_layout.visible_range_offset.start
            || range.end > last_layout.visible_range_offset.end
        {
            return None;
        }

        let line_height = last_layout.line_height;
        let visible_top = last_layout.visible_top;
        let lines = &last_layout.lines;
        let line_number_width = last_layout.line_number_width;

        let start_ix = range.start;
        let end_ix = range.end;

        // Start from visible_top (which already accounts for all lines before visible range)
        let mut offset_y = visible_top;
        let mut line_corners = vec![];

        // Iterate only over visible (non-hidden) buffer lines
        for (prev_lines_offset, line) in last_layout
            .visible_line_byte_offsets
            .iter()
            .zip(lines.iter())
        {
            let prev_lines_offset = *prev_lines_offset;
            let line_size = line.size(line_height);
            let line_wrap_width = line_size.width;

            let line_origin = point(px(0.), offset_y);

            let line_cursor_start =
                line.position_for_index(start_ix.saturating_sub(prev_lines_offset), last_layout);
            let line_cursor_end =
                line.position_for_index(end_ix.saturating_sub(prev_lines_offset), last_layout);

            if line_cursor_start.is_some() || line_cursor_end.is_some() {
                let start = line_cursor_start
                    .unwrap_or_else(|| line.position_for_index(0, last_layout).unwrap());

                let end = line_cursor_end
                    .unwrap_or_else(|| line.position_for_index(line.len(), last_layout).unwrap());

                // Split the selection into multiple items
                let wrapped_lines =
                    (end.y / line_height).ceil() as usize - (start.y / line_height).ceil() as usize;

                let mut end_x = end.x;
                if wrapped_lines > 0 {
                    end_x = line_wrap_width;
                }

                // Ensure at least 6px width for the selection for empty lines.
                end_x = end_x.max(start.x + px(6.));

                line_corners.push(Corners {
                    top_left: line_origin + point(start.x, start.y),
                    top_right: line_origin + point(end_x, start.y),
                    bottom_left: line_origin + point(start.x, start.y + line_height),
                    bottom_right: line_origin + point(end_x, start.y + line_height),
                });

                // wrapped lines
                for i in 1..=wrapped_lines {
                    let start = point(px(0.), start.y + i as f32 * line_height);
                    let mut end = point(end.x, end.y + i as f32 * line_height);
                    if i < wrapped_lines {
                        end.x = line_size.width;
                    }

                    line_corners.push(Corners {
                        top_left: line_origin + point(start.x, start.y),
                        top_right: line_origin + point(end.x, start.y),
                        bottom_left: line_origin + point(start.x, start.y + line_height),
                        bottom_right: line_origin + point(end.x, start.y + line_height),
                    });
                }
            }

            if line_cursor_start.is_some() && line_cursor_end.is_some() {
                break;
            }

            offset_y += line_size.height;
        }

        let mut points = vec![];
        if line_corners.is_empty() {
            return None;
        }

        // Fix corners to make sure the left to right direction
        for corners in &mut line_corners {
            if corners.top_left.x > corners.top_right.x {
                std::mem::swap(&mut corners.top_left, &mut corners.top_right);
                std::mem::swap(&mut corners.bottom_left, &mut corners.bottom_right);
            }
        }

        for corners in &line_corners {
            points.push(corners.top_right);
            points.push(corners.bottom_right);
            points.push(corners.bottom_left);
        }

        let mut rev_line_corners = line_corners.iter().rev().peekable();
        while let Some(corners) = rev_line_corners.next() {
            points.push(corners.top_left);
            if let Some(next) = rev_line_corners.peek() {
                if next.top_left.x > corners.top_left.x {
                    points.push(point(next.top_left.x, corners.top_left.y));
                }
            }
        }

        // print_points_as_svg_path(&line_corners, &points);

        let path_origin = bounds.origin + point(line_number_width, px(0.));
        let first_p = *points.get(0).unwrap();
        let mut builder = inazuma::PathBuilder::fill();
        builder.move_to(path_origin + first_p);
        for p in points.iter().skip(1) {
            builder.line_to(path_origin + *p);
        }

        builder.build().ok()
    }

    pub(super) fn layout_search_matches(
        &self,
        last_layout: &LastLayout,
        bounds: &Bounds<Pixels>,
        cx: &mut App,
    ) -> Vec<(Path<Pixels>, bool)> {
        let state = self.state.read(cx);
        let search_panel = state.search_panel.clone();

        let Some((ranges, current_match_ix)) = search_panel.and_then(|panel| {
            if let Some(matcher) = panel.read(cx).matcher() {
                Some((matcher.matched_ranges.clone(), matcher.current_match_ix))
            } else {
                None
            }
        }) else {
            return vec![];
        };

        let mut paths = Vec::with_capacity(ranges.as_ref().len());
        for (index, range) in ranges.as_ref().iter().enumerate() {
            if let Some(path) = Self::layout_match_range(range.clone(), last_layout, bounds) {
                paths.push((path, current_match_ix == index));
            }
        }

        paths
    }

    pub(super) fn layout_hover_highlight(
        &self,
        last_layout: &LastLayout,
        bounds: &Bounds<Pixels>,
        cx: &mut App,
    ) -> Option<Path<Pixels>> {
        let state = self.state.read(cx);
        let hover_popover = state.hover_popover.clone();

        let Some(symbol_range) = hover_popover.map(|popover| popover.read(cx).symbol_range.clone())
        else {
            return None;
        };

        Self::layout_match_range(symbol_range, last_layout, bounds)
    }

    pub(super) fn layout_document_colors(
        &self,
        document_colors: &[(Range<usize>, Oklch)],
        last_layout: &LastLayout,
        bounds: &Bounds<Pixels>,
        _cx: &mut App,
    ) -> Vec<(Path<Pixels>, Oklch)> {
        let mut paths = vec![];
        for (range, color) in document_colors.iter() {
            if let Some(path) = Self::layout_match_range(range.clone(), last_layout, bounds) {
                paths.push((path, *color));
            }
        }

        paths
    }

    pub(super) fn layout_selections(
        &self,
        last_layout: &LastLayout,
        bounds: &mut Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Path<Pixels>> {
        let state = self.state.read(cx);
        if !state.focus_handle.is_focused(window) {
            return None;
        }

        let mut selected_range = state.selected_range;
        if let Some(ime_marked_range) = &state.ime_marked_range {
            if !ime_marked_range.is_empty() {
                selected_range = (ime_marked_range.end..ime_marked_range.end).into();
            }
        }
        if selected_range.is_empty() {
            return None;
        }

        if state.masked {
            // Because masked use `*`, 1 char with 1 byte.
            selected_range.start = state.text.offset_to_char_index(selected_range.start);
            selected_range.end = state.text.offset_to_char_index(selected_range.end);
        }

        let (start_ix, end_ix) = if selected_range.start < selected_range.end {
            (selected_range.start, selected_range.end)
        } else {
            (selected_range.end, selected_range.start)
        };

        let range = start_ix.max(last_layout.visible_range_offset.start)
            ..end_ix.min(last_layout.visible_range_offset.end);

        Self::layout_match_range(range, &last_layout, bounds)
    }

    /// Calculate the visible range of lines in the viewport.
    ///
    /// Returns
    ///
    /// - visible_range: The visible range is based on unwrapped lines (Zero based).
    /// - visible_buffer_lines: Indices of non-hidden buffer lines within the visible range.
    /// - visible_top: The top position of the first visible line in the scroll viewport.
    pub(super) fn calculate_visible_range(
        &self,
        state: &InputState,
        line_height: Pixels,
        input_height: Pixels,
    ) -> (Range<usize>, Vec<usize>, Pixels) {
        // Add extra rows to avoid showing empty space when scroll to bottom.
        let extra_rows = 1;
        let mut visible_top = px(0.);
        if state.mode.is_single_line() {
            return (0..1, vec![0], visible_top);
        }

        let total_lines = state.display_map.wrap_row_count();
        let scroll_top = if let Some(deferred_scroll_offset) = state.deferred_scroll_offset {
            deferred_scroll_offset.y
        } else {
            state.scroll_handle.offset().y
        };

        let mut visible_range = 0..total_lines;
        let mut line_bottom = px(0.);
        for (ix, _line) in state.display_map.lines().iter().enumerate() {
            let visible_wrap_rows = state.display_map.visible_wrap_row_count_for_buffer_line(ix);

            if visible_wrap_rows == 0 {
                continue;
            }

            let wrapped_height = line_height * visible_wrap_rows;
            line_bottom += wrapped_height;

            if line_bottom < -scroll_top {
                visible_top = line_bottom - wrapped_height;
                visible_range.start = ix;
            }

            if line_bottom + scroll_top >= input_height {
                visible_range.end = (ix + extra_rows).min(total_lines);
                break;
            }
        }

        // Collect non-hidden buffer lines within the visible range
        let mut visible_buffer_lines = Vec::with_capacity(visible_range.len());
        for ix in visible_range.start..visible_range.end {
            let visible_wrap_rows = state.display_map.visible_wrap_row_count_for_buffer_line(ix);
            if visible_wrap_rows > 0 {
                visible_buffer_lines.push(ix);
            }
        }

        (visible_range, visible_buffer_lines, visible_top)
    }

    /// Return (line_number_width, line_number_len)
    pub(super) fn layout_line_numbers(
        state: &InputState,
        text: &Rope,
        font_size: Pixels,
        style: &TextStyle,
        window: &mut Window,
    ) -> (Pixels, usize) {
        let total_lines = text.lines_len();
        let line_number_len = match total_lines {
            0..=9999 => 5,
            10000..=99999 => 6,
            100000..=999999 => 7,
            _ => 8,
        };

        let mut line_number_width = if state.mode.line_number() {
            let empty_line_number = window.text_system().shape_line(
                "+".repeat(line_number_len).into(),
                font_size,
                &[TextRun {
                    len: line_number_len,
                    font: style.font(),
                    color: inazuma::Oklch::black(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            );

            empty_line_number.width + LINE_NUMBER_RIGHT_MARGIN
        } else if state.mode.is_code_editor() && state.mode.is_multi_line() {
            LINE_NUMBER_RIGHT_MARGIN
        } else {
            px(0.)
        };

        if state.mode.is_folding() {
            // Add extra space for fold icons
            line_number_width += FOLD_ICON_HITBOX_WIDTH
        }

        (line_number_width, line_number_len)
    }

    /// Layout shaped lines for whitespace indicators (space and tab).
    ///
    /// Returns `WhitespaceIndicators` with shaped lines for space and tab characters.
    pub(super) fn layout_whitespace_indicators(
        state: &InputState,
        text_size: Pixels,
        style: &TextStyle,
        window: &mut Window,
        cx: &App,
    ) -> Option<WhitespaceIndicators> {
        if !state.show_whitespaces {
            return None;
        }

        let invisible_color: inazuma::Oklch = cx.theme().colors().muted_foreground;

        let space_font_size = text_size.half();
        let tab_font_size = text_size;

        let space_text = SharedString::new_static("•");
        let space = window.text_system().shape_line(
            space_text.clone(),
            space_font_size,
            &[TextRun {
                len: space_text.len(),
                font: style.font(),
                color: invisible_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );

        let tab_text = SharedString::new_static("→");
        let tab = window.text_system().shape_line(
            tab_text.clone(),
            tab_font_size,
            &[TextRun {
                len: tab_text.len(),
                font: style.font(),
                color: invisible_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );

        Some(WhitespaceIndicators { space, tab })
    }

    /// Compute inline completion ghost lines for rendering.
    ///
    /// Returns (first_line, ghost_lines) where:
    /// - first_line: Shaped text for the first line (goes after cursor on same line)
    /// - ghost_lines: Shaped lines for subsequent lines (shift content down)
    pub(super) fn layout_inline_completion(
        state: &InputState,
        visible_range: &Range<usize>,
        font_size: Pixels,
        window: &mut Window,
        cx: &App,
    ) -> (Option<ShapedLine>, Vec<ShapedLine>) {
        // Must be focused to show inline completion
        if !state.focus_handle.is_focused(window) {
            return (None, vec![]);
        }

        let Some(completion_item) = state.inline_completion.item.as_ref() else {
            return (None, vec![]);
        };

        // Get cursor row from cursor position
        let cursor_row = state.cursor_position().line as usize;

        // Only show if cursor row is visible
        if cursor_row < visible_range.start || cursor_row >= visible_range.end {
            return (None, vec![]);
        }

        let completion_text = &completion_item.insert_text;
        let completion_color: inazuma::Oklch = inazuma::Oklch::from(cx.theme().colors().muted_foreground).opacity(0.5);

        let text_style = window.text_style();
        let font = text_style.font();

        let lines: Vec<&str> = completion_text.split('\n').collect();
        if lines.is_empty() {
            return (None, vec![]);
        }

        // Shape first line (goes after cursor)
        let first_text: SharedString = lines[0].to_string().into();
        let first_line = if !first_text.is_empty() {
            let first_run = TextRun {
                len: first_text.len(),
                font: font.clone(),
                color: completion_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            Some(
                window
                    .text_system()
                    .shape_line(first_text, font_size, &[first_run], None),
            )
        } else {
            None
        };

        // Shape ghost lines (lines 2+ that shift content down)
        let ghost_lines: Vec<ShapedLine> = lines[1..]
            .iter()
            .map(|line_text| {
                let text: SharedString = line_text.to_string().into();
                let len = text.len().max(1); // Ensure at least 1 for empty lines
                let run = TextRun {
                    len,
                    font: font.clone(),
                    color: completion_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                // Use space for empty lines so they take up height
                let shaped_text = if text.is_empty() { " ".into() } else { text };
                window
                    .text_system()
                    .shape_line(shaped_text, font_size, &[run], None)
            })
            .collect();

        (first_line, ghost_lines)
    }

    /// Layout fold icon hitboxes during prepaint phase.
    ///
    /// This creates hitboxes for the fold icon area, positioned to the right of line numbers.
    /// Icons are created and prepainted here to avoid panics.
    pub(super) fn layout_fold_icons(
        &self,
        bounds: &Bounds<Pixels>,
        last_layout: &LastLayout,
        window: &mut Window,
        cx: &mut App,
    ) -> FoldIconLayout {
        // First pass: collect fold information from state
        struct FoldInfo {
            buffer_line: usize,
            is_folded: bool,
            display_row: usize,
            offset_y: Pixels,
        }

        let line_number_hitbox = window.insert_hitbox(
            Bounds::new(
                bounds.origin + point(px(0.), last_layout.visible_top),
                size(last_layout.line_number_width, bounds.size.height),
            ),
            HitboxBehavior::Normal,
        );

        let mut icon_layout = FoldIconLayout {
            line_number_hitbox,
            icons: vec![],
        };

        let fold_infos: Vec<FoldInfo> = {
            let state = self.state.read(cx);
            if !state.mode.is_folding() {
                return icon_layout;
            }

            let mut infos = Vec::with_capacity(last_layout.visible_buffer_lines.len());
            let mut offset_y = last_layout.visible_top;

            for (line, &buffer_line) in last_layout
                .lines
                .iter()
                .zip(last_layout.visible_buffer_lines.iter())
            {
                if state.display_map.is_fold_candidate(buffer_line) {
                    let is_folded = state.display_map.is_folded_at(buffer_line);
                    infos.push(FoldInfo {
                        buffer_line,
                        is_folded,
                        display_row: buffer_line,
                        offset_y,
                    });
                }

                offset_y += line.wrapped_lines.len() * last_layout.line_height;
            }

            infos
        }; // state is dropped here

        // Second pass: create and prepaint icons
        let line_height = last_layout.line_height;
        let line_number_width = last_layout.line_number_width
            - LINE_NUMBER_RIGHT_MARGIN.half()
            - FOLD_ICON_HITBOX_WIDTH;
        let icon_relative_pos = point(
            (FOLD_ICON_HITBOX_WIDTH - FOLD_ICON_WIDTH).half(),
            (line_height - FOLD_ICON_WIDTH).half(),
        );

        for (ix, info) in fold_infos.iter().enumerate() {
            // Position fold icon to the right of line numbers
            let fold_icon_bounds = Bounds::new(
                bounds.origin + icon_relative_pos + point(line_number_width, info.offset_y),
                size(FOLD_ICON_HITBOX_WIDTH, line_height),
            );

            // Create and prepaint icon
            let mut icon = Button::with_id(("fold", ix))
                .ghost()
                .icon(if info.is_folded {
                    IconName::ChevronRight
                } else {
                    IconName::ChevronDown
                })
                .xsmall()
                .rounded_xs()
                .size(FOLD_ICON_WIDTH)
                .selected(info.is_folded)
                .on_mouse_down(MouseButton::Left, {
                    let state = self.state.clone();
                    let buffer_line = info.buffer_line;
                    move |_, _: &mut Window, cx: &mut App| {
                        cx.stop_propagation();

                        state.update(cx, |state, cx| {
                            state.display_map.toggle_fold(buffer_line);
                            cx.notify();
                        });
                    }
                })
                .into_any_element();

            icon.prepaint_as_root(
                fold_icon_bounds.origin,
                fold_icon_bounds.size.into(),
                window,
                cx,
            );

            icon_layout
                .icons
                .push((info.display_row, info.is_folded, icon));
        }

        icon_layout
    }

    /// Paint fold icons using prepaint hitboxes.
    ///
    /// This handles:
    /// - Rendering fold icons (chevron-right for folded, chevron-down for expanded)
    /// - Mouse click handling to toggle fold state
    /// - Cursor style changes on hover
    /// - Only show icon on hover or for current line
    pub(super) fn paint_fold_icons(
        &mut self,
        fold_icon_layout: &mut FoldIconLayout,
        current_row: Option<usize>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let is_hovered = fold_icon_layout.line_number_hitbox.is_hovered(window);
        for (display_row, is_folded, icon) in fold_icon_layout.icons.iter_mut() {
            let is_current_line = current_row == Some(*display_row);

            if !is_hovered && !is_current_line && !*is_folded {
                continue;
            }

            icon.paint(window, cx);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn layout_lines(
        state: &InputState,
        display_text: &Rope,
        last_layout: &LastLayout,
        font_size: Pixels,
        runs: &[TextRun],
        bg_segments: &[(Range<usize>, Oklch)],
        whitespace_indicators: Option<WhitespaceIndicators>,
        window: &mut Window,
    ) -> Vec<LineLayout> {
        let is_single_line = state.mode.is_single_line();
        let buffer_lines = state.display_map.lines();

        if is_single_line {
            let shaped_line = window.text_system().shape_line(
                display_text.to_string().into(),
                font_size,
                &runs,
                None,
            );

            let line_layout = LineLayout::new()
                .lines(smallvec::smallvec![shaped_line])
                .with_whitespaces(whitespace_indicators);
            return vec![line_layout];
        }

        // Empty to use placeholder, the placeholder is not in the wrapper map.
        if state.text.len() == 0 {
            return display_text
                .to_string()
                .split("\n")
                .map(|line| {
                    let shaped_line = window.text_system().shape_line(
                        line.to_string().into(),
                        font_size,
                        &runs,
                        None,
                    );
                    LineLayout::new()
                        .lines(smallvec::smallvec![shaped_line])
                        .with_whitespaces(whitespace_indicators.clone())
                })
                .collect();
        }

        let mut lines = Vec::with_capacity(last_layout.visible_buffer_lines.len());
        // run_offset tracks position in the runs vec coordinate space (only visible line bytes).
        // This is separate from the visible_text offset because runs from highlight_lines
        // only cover visible (non-folded) lines.
        let mut run_offset = 0;

        for (vi, &buffer_line) in last_layout.visible_buffer_lines.iter().enumerate() {
            let line_text: String = display_text.slice_line(buffer_line).into();
            let line_item = buffer_lines
                .get(buffer_line)
                .expect("line should exists in wrapper");

            debug_assert_eq!(line_item.len(), line_text.len());

            let mut wrapped_lines = SmallVec::with_capacity(1);

            for range in &line_item.wrapped_lines {
                let line_runs = runs_for_range(runs, run_offset, &range);
                let line_runs = if bg_segments.is_empty() {
                    line_runs
                } else {
                    split_runs_by_bg_segments(
                        last_layout.visible_line_byte_offsets[vi] + (range.start),
                        &line_runs,
                        bg_segments,
                    )
                };

                let sub_line: SharedString = line_text[range.clone()].to_string().into();
                let shaped_line = window
                    .text_system()
                    .shape_line(sub_line, font_size, &line_runs, None);

                wrapped_lines.push(shaped_line);
            }

            let line_layout = LineLayout::new()
                .lines(wrapped_lines)
                .with_whitespaces(whitespace_indicators.clone());
            lines.push(line_layout);

            // +1 for the `\n`
            run_offset += line_text.len() + 1;
        }

        lines
    }

    /// First usize is the offset of skipped.
    pub(super) fn highlight_lines(
        &mut self,
        visible_buffer_lines: &[usize],
        _visible_top: Pixels,
        visible_byte_range: Range<usize>,
        cx: &mut App,
    ) -> Option<Vec<(Range<usize>, HighlightStyle)>> {
        let state = self.state.read(cx);
        let text = &state.text;
        let is_multi_line = state.mode.is_multi_line();

        let (mut highlighter, diagnostics) = match &state.mode {
            InputMode::CodeEditor {
                highlighter,
                diagnostics,
                ..
            } => (highlighter.borrow_mut(), diagnostics),
            _ => return None,
        };
        let highlighter = highlighter.as_mut()?;

        let mut styles = Vec::with_capacity(visible_buffer_lines.len());

        // Helper to flush a contiguous range of lines
        let flush_range = |start_line: usize, end_line: usize, skip: bool, styles: &mut Vec<_>| {
            let byte_start = text.line_start_offset(start_line);
            let byte_end = if is_multi_line {
                // +1 for `\n`
                text.line_start_offset(end_line + 1)
            } else {
                text.line_end_offset(end_line)
            };
            let range_styles = if skip {
                vec![(byte_start..byte_end, HighlightStyle::default())]
            } else {
                {
                    let ht = if cx.theme().is_dark() {
                        raijin_syntax::HighlightTheme::default_dark()
                    } else {
                        raijin_syntax::HighlightTheme::default_light()
                    };
                    highlighter.styles(&(byte_start..byte_end), &ht)
                }
            };

            *styles = inazuma::combine_highlights(styles.clone(), range_styles).collect();
        };

        // Group contiguous visible lines into ranges and call styles() once per range
        let mut visible_iter = visible_buffer_lines.iter().peekable();
        let mut range_start: Option<usize> = None;

        while let Some(&line) = visible_iter.next() {
            // Check if this line is too long for highlighting
            let line_len = text.slice_line(line).len();
            if line_len > MAX_HIGHLIGHT_LINE_LENGTH {
                // Flush any accumulated range first
                if let Some(start) = range_start.take() {
                    flush_range(start, line - 1, false, &mut styles);
                }

                flush_range(line, line, true, &mut styles);
                continue;
            }

            range_start.get_or_insert(line);

            // Check if next line is contiguous, if so keep accumulating
            if visible_iter
                .peek()
                .map(|&&next| next == line + 1)
                .unwrap_or(false)
            {
                continue;
            }

            // Flush the contiguous range
            let start_line = range_start.take().unwrap();
            flush_range(start_line, line, false, &mut styles);
        }

        let diagnostic_styles = diagnostics.styles_for_range(&visible_byte_range, cx);

        // hover definition style
        if let Some(hover_style) = self.layout_hover_definition(cx) {
            styles.push(hover_style);
        }

        // Combine marker styles
        styles = inazuma::combine_highlights(diagnostic_styles, styles).collect();

        Some(styles)
    }
}

/// Get the runs for the given range.
///
/// The range is the byte range of the wrapped line.
pub(super) fn runs_for_range(
    runs: &[TextRun],
    line_offset: usize,
    range: &Range<usize>,
) -> Vec<TextRun> {
    let mut result = vec![];
    let range = (line_offset + range.start)..(line_offset + range.end);
    let mut cursor = 0;

    for run in runs {
        let run_start = cursor;
        let run_end = cursor + run.len;

        if run_end <= range.start {
            cursor = run_end;
            continue;
        }

        if run_start >= range.end {
            break;
        }

        let start = range.start.max(run_start) - run_start;
        let end = range.end.min(run_end) - run_start;
        let len = end - start;

        if len > 0 {
            result.push(TextRun { len, ..run.clone() });
        }

        cursor = run_end;
    }

    result
}

pub(super) fn split_runs_by_bg_segments(
    start_offset: usize,
    runs: &[TextRun],
    bg_segments: &[(Range<usize>, Oklch)],
) -> Vec<TextRun> {
    let mut result = vec![];

    let mut cursor = start_offset;
    for run in runs {
        let mut run_start = cursor;
        let run_end = cursor + run.len;

        for (bg_range, bg_color) in bg_segments {
            if run_end <= bg_range.start || run_start >= bg_range.end {
                continue;
            }

            // Overlap exists
            if run_start < bg_range.start {
                // Add the part before the background range
                result.push(TextRun {
                    len: bg_range.start - run_start,
                    ..run.clone()
                });
            }

            // Add the overlapping part with background color
            let overlap_start = run_start.max(bg_range.start);
            let overlap_end = run_end.min(bg_range.end);
            let text_color: inazuma::Oklch = if bg_color.l >= 0.5 {
                inazuma::Oklch::black()
            } else {
                inazuma::Oklch::white()
            };

            let run_len = overlap_end.saturating_sub(overlap_start);
            if run_len > 0 {
                result.push(TextRun {
                    len: run_len,
                    color: text_color,
                    ..run.clone()
                });

                cursor = bg_range.end;
                run_start = cursor;
            }
        }

        if run_end > cursor {
            // Add the part after the background range
            result.push(TextRun {
                len: run_end - cursor,
                ..run.clone()
            });
        }

        cursor = run_end;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runs_for_range() {
        let run = TextRun {
            len: 0,
            font: inazuma::font(".SystemUIFont"),
            color: inazuma::Oklch::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        // use hello this-is-test
        let runs = vec![
            // use
            TextRun {
                len: 3,
                ..run.clone()
            },
            // \s
            TextRun {
                len: 1,
                ..run.clone()
            },
            // hello
            TextRun {
                len: 5,
                ..run.clone()
            },
            // \s
            TextRun {
                len: 1,
                ..run.clone()
            },
            // this-is-test
            TextRun {
                len: 12,
                ..run.clone()
            },
        ];

        #[track_caller]
        fn assert_runs(actual: Vec<TextRun>, expected: &[usize]) {
            let left = actual.iter().map(|run| run.len).collect::<Vec<_>>();
            assert_eq!(left, expected);
        }

        assert_runs(runs_for_range(&runs, 0, &(0..0)), &[]);
        assert_runs(runs_for_range(&runs, 0, &(0..100)), &[3, 1, 5, 1, 12]);

        assert_runs(runs_for_range(&runs, 0, &(0..6)), &[3, 1, 2]);
        assert_runs(runs_for_range(&runs, 0, &(1..6)), &[2, 1, 2]);
        assert_runs(runs_for_range(&runs, 0, &(3..10)), &[1, 5, 1]);
        assert_runs(runs_for_range(&runs, 0, &(5..8)), &[3]);
        assert_runs(runs_for_range(&runs, 3, &(0..3)), &[1, 2]);
        assert_runs(runs_for_range(&runs, 3, &(2..10)), &[4, 1, 3]);
        assert_runs(runs_for_range(&runs, 9, &(0..8)), &[1, 7]);
    }

    #[test]
    fn test_split_runs_by_bg_segments() {
        let run = TextRun {
            len: 0,
            font: inazuma::font(".SystemUIFont"),
            color: Oklch::from(inazuma::blue()),
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = vec![
            TextRun {
                len: 5,
                ..run.clone()
            },
            TextRun {
                len: 7,
                ..run.clone()
            },
            TextRun {
                len: 24,
                ..run.clone()
            },
        ];

        let bg_segments = vec![(8..12, Oklch::from(inazuma::red())), (12..18, Oklch::from(inazuma::blue()))];
        let result = split_runs_by_bg_segments(5, &runs, &bg_segments);
        assert_eq!(
            result.iter().map(|run| run.len).collect::<Vec<_>>(),
            vec![3, 2, 2, 5, 1, 23]
        );
        // Segments outside bg_segments keep original color
        assert_eq!(result[0].color, Oklch::from(inazuma::blue()));
        assert_eq!(result[5].color, Oklch::from(inazuma::blue()));
        // Segments within bg_segments get contrasting text color (black or white)
        // based on bg lightness threshold of 0.5
        for idx in 1..5 {
            let c = result[idx].color;
            assert!(
                c == Oklch::black() || c == Oklch::white(),
                "segment {} should have black or white text, got {:?}", idx, c
            );
        }
    }
}
