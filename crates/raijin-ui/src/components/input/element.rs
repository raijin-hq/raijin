use std::rc::Rc;

use inazuma::{
    App, Bounds, Element, ElementId, ElementInputHandler, Entity, GlobalElementId, Half,
    Hitbox, Oklch, IntoElement, LayoutId, MouseButton, MouseMoveEvent, Path, Pixels, Point,
    ShapedLine, SharedString, Size, Style, TextAlign, TextRun, UnderlineStyle, Window, fill,
    px, relative, size,
};
use ropey::Rope;
use smallvec::SmallVec;

use crate::{
    ActiveTheme as _, AppShell,
    input::RopeExt as _,
};

use super::{InputState, LastLayout};

pub(super) const BOTTOM_MARGIN_ROWS: usize = 3;
pub(super) const RIGHT_MARGIN: Pixels = px(10.);
pub(super) const LINE_NUMBER_RIGHT_MARGIN: Pixels = px(10.);
pub(super) const FOLD_ICON_WIDTH: Pixels = px(14.);
pub(super) const FOLD_ICON_HITBOX_WIDTH: Pixels = px(18.);
pub(super) const MAX_HIGHLIGHT_LINE_LENGTH: usize = 10_000;

/// Layout information for fold icons.
pub(super) struct FoldIconLayout {
    /// Hitbox for the line number area (used for hover detection)
    pub(super) line_number_hitbox: Hitbox,
    /// List of (display_row, is_folded, icon_element) pairs for each fold candidate
    pub(super) icons: Vec<(usize, bool, inazuma::AnyElement)>,
}

pub(super) struct TextElement {
    pub(crate) state: Entity<InputState>,
    placeholder: SharedString,
}

impl TextElement {
    pub(super) fn new(state: Entity<InputState>) -> Self {
        Self {
            state,
            placeholder: SharedString::default(),
        }
    }

    /// Set the placeholder text of the input field.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    fn paint_mouse_listeners(&mut self, window: &mut Window, _: &mut App) {
        window.on_mouse_event({
            let state = self.state.clone();

            move |event: &MouseMoveEvent, _, window, cx| {
                if event.pressed_button == Some(MouseButton::Left) {
                    state.update(cx, |state, cx| {
                        state.on_drag_move(event, window, cx);
                    });
                }
            }
        });
    }
}

pub(super) struct PrepaintState {
    /// The lines of entire lines.
    pub(super) last_layout: LastLayout,
    /// The lines only contains the visible lines in the viewport, based on `visible_range`.
    ///
    /// The child is the soft lines.
    pub(super) line_numbers: Option<Vec<SmallVec<[ShapedLine; 1]>>>,
    /// Size of the scrollable area by entire lines.
    pub(super) scroll_size: Size<Pixels>,
    pub(super) cursor_bounds: Option<Bounds<Pixels>>,
    pub(super) cursor_scroll_offset: Point<Pixels>,
    /// row index (zero based), no wrap, same line as the cursor.
    pub(super) current_row: Option<usize>,
    pub(super) selection_path: Option<Path<Pixels>>,
    pub(super) hover_highlight_path: Option<Path<Pixels>>,
    pub(super) search_match_paths: Vec<(Path<Pixels>, bool)>,
    pub(super) document_color_paths: Vec<(Path<Pixels>, Oklch)>,
    pub(super) hover_definition_hitbox: Option<Hitbox>,
    pub(super) indent_guides_path: Option<Path<Pixels>>,
    pub(super) bounds: Bounds<Pixels>,
    /// Fold icon layout data
    pub(super) fold_icon_layout: FoldIconLayout,
    // Inline completion rendering data
    /// Shaped ghost lines to paint after cursor row (completion lines 2+)
    pub(super) ghost_lines: Vec<ShapedLine>,
    /// First line of inline completion (painted after cursor on same line)
    pub(super) ghost_first_line: Option<ShapedLine>,
    pub(super) ghost_lines_height: Pixels,
}

impl PrepaintState {
    /// Returns cursor bounds adjusted for scroll offset, if available.
    pub(super) fn cursor_bounds_with_scroll(&self) -> Option<Bounds<Pixels>> {
        self.cursor_bounds.map(|mut bounds| {
            bounds.origin.y += self.cursor_scroll_offset.y;
            bounds
        })
    }
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// A debug function to print points as SVG path.
#[allow(unused)]
fn print_points_as_svg_path(
    line_corners: &Vec<inazuma::Corners<Point<Pixels>>>,
    points: &Vec<Point<Pixels>>,
) {
    for corners in line_corners {
        println!(
            "tl: ({}, {}), tr: ({}, {}), bl: ({}, {}), br: ({}, {})",
            corners.top_left.x.as_f32() as i32,
            corners.top_left.y.as_f32() as i32,
            corners.top_right.x.as_f32() as i32,
            corners.top_right.y.as_f32() as i32,
            corners.bottom_left.x.as_f32() as i32,
            corners.bottom_left.y.as_f32() as i32,
            corners.bottom_right.x.as_f32() as i32,
            corners.bottom_right.y.as_f32() as i32,
        );
    }

    if points.len() > 0 {
        println!(
            "M{},{}",
            points[0].x.as_f32() as i32,
            points[0].y.as_f32() as i32
        );
        for p in points.iter().skip(1) {
            println!("L{},{}", p.x.as_f32() as i32, p.y.as_f32() as i32);
        }
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _: Option<&inazuma::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = self.state.read(cx);
        let line_height = window.line_height();

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        if state.mode.is_multi_line() {
            style.flex_grow = 1.0;
            style.size.height = relative(1.).into();
            if state.mode.is_auto_grow() {
                // Auto grow to let height match to rows, but not exceed max rows.
                let rows = state.mode.max_rows().min(state.mode.rows());
                style.min_size.height = (rows * line_height).into();
            } else {
                style.min_size.height = line_height.into();
            }
        } else {
            // For single-line inputs, the minimum height should be the line height
            style.size.height = line_height.into();
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _: Option<&inazuma::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let style = window.text_style();
        let font = style.font();
        let text_size = style.font_size.to_pixels(window.rem_size());

        self.state.update(cx, |state, cx| {
            state.display_map.set_font(font, text_size, cx);
            state.display_map.ensure_text_prepared(&state.text, cx);
        });

        let state = self.state.read(cx);
        let line_height = window.line_height();

        let (visible_range, visible_buffer_lines, visible_top) =
            self.calculate_visible_range(&state, line_height, bounds.size.height);
        let visible_start_offset = state.text.line_start_offset(visible_range.start);
        let visible_end_offset = state
            .text
            .line_end_offset(visible_range.end.saturating_sub(1));

        let highlight_styles = self.highlight_lines(
            &visible_buffer_lines,
            visible_top,
            visible_start_offset..visible_end_offset,
            cx,
        );

        let state = self.state.read(cx);

        // Merge overlay highlights (completion preview, command validation) on top
        let highlight_styles = if !state.overlay_highlights.is_empty() {
            let overlay = state.overlay_highlights.clone();
            match highlight_styles {
                Some(base) => Some(inazuma::combine_highlights(base, overlay).collect()),
                None => {
                    // Create base highlights covering the full text
                    let base = vec![(0..state.text.len(), inazuma::HighlightStyle::default())];
                    Some(inazuma::combine_highlights(base, overlay).collect())
                }
            }
        } else {
            highlight_styles
        };
        let multi_line = state.mode.is_multi_line();
        let text = state.text.clone();
        let is_empty = text.len() == 0;
        let placeholder = self.placeholder.clone();

        let mut bounds = bounds;

        let (display_text, text_color): (_, inazuma::Oklch) = if is_empty {
            (
                &Rope::from(placeholder.as_str()),
                cx.theme().colors().muted_foreground.into(),
            )
        } else if state.masked {
            (
                &Rope::from("*".repeat(text.chars().count())),
                cx.theme().colors().foreground.into(),
            )
        } else {
            (&text, cx.theme().colors().foreground.into())
        };

        let text_style = window.text_style();

        // Calculate the width of the line numbers
        let (line_number_width, line_number_len) =
            Self::layout_line_numbers(&state, &text, text_size, &text_style, window);

        let wrap_width = if multi_line && state.soft_wrap {
            Some(bounds.size.width - line_number_width - RIGHT_MARGIN)
        } else {
            None
        };

        let visible_line_byte_offsets: Vec<usize> = visible_buffer_lines
            .iter()
            .map(|&bl| state.text.line_start_offset(bl))
            .collect();

        let mut last_layout = LastLayout {
            visible_range,
            visible_buffer_lines,
            visible_line_byte_offsets,
            visible_top,
            visible_range_offset: visible_start_offset..visible_end_offset,
            line_height,
            wrap_width,
            line_number_width,
            lines: Rc::new(vec![]),
            cursor_bounds: None,
            text_align: state.text_align,
            content_width: bounds.size.width,
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let marked_run = TextRun {
            len: 0,
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(text_color),
                wavy: false,
            }),
            strikethrough: None,
        };

        let runs = if !is_empty {
            if let Some(highlight_styles) = highlight_styles {
                let mut runs = Vec::with_capacity(highlight_styles.len());

                runs.extend(highlight_styles.iter().map(|(range, style)| {
                    let mut run = text_style.clone().highlight(*style).to_run(range.len());
                    if let Some(ime_marked_range) = &state.ime_marked_range {
                        if range.start >= ime_marked_range.start
                            && range.end <= ime_marked_range.end
                        {
                            run.color = marked_run.color;
                            run.strikethrough = marked_run.strikethrough;
                            run.underline = marked_run.underline;
                        }
                    }

                    run
                }));

                runs.into_iter().filter(|run| run.len > 0).collect()
            } else {
                vec![run]
            }
        } else if let Some(ime_marked_range) = &state.ime_marked_range {
            // IME marked text
            vec![
                TextRun {
                    len: ime_marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: ime_marked_range.end - ime_marked_range.start,
                    underline: marked_run.underline,
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - ime_marked_range.end,
                    ..run.clone()
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let document_colors = state
            .lsp
            .document_colors_for_range(&text, &last_layout.visible_range);

        // Create shaped lines for whitespace indicators before layout
        let whitespace_indicators =
            Self::layout_whitespace_indicators(&state, text_size, &text_style, window, cx);

        let lines = Self::layout_lines(
            &state,
            &display_text,
            &last_layout,
            text_size,
            &runs,
            &document_colors,
            whitespace_indicators,
            window,
        );

        let mut longest_line_width = wrap_width.unwrap_or(px(0.));
        // 1. Single line
        // 2. Multi-line with soft wrap disabled.
        if state.mode.is_single_line() || !state.soft_wrap {
            let longest_row = state.display_map.longest_row();
            let longest_line: SharedString = state.text.slice_line(longest_row).to_string().into();
            longest_line_width = window
                .text_system()
                .shape_line(
                    longest_line.clone(),
                    text_size,
                    &[TextRun {
                        len: longest_line.len(),
                        font: style.font(),
                        color: inazuma::Oklch::black(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    wrap_width,
                )
                .width;
        }
        last_layout.lines = Rc::new(lines);

        let (ghost_first_line, ghost_lines) = Self::layout_inline_completion(
            state,
            &last_layout.visible_range,
            text_size,
            window,
            cx,
        );
        let ghost_line_count = ghost_lines.len();
        let ghost_lines_height = ghost_line_count as f32 * line_height;

        let total_wrapped_lines = state.display_map.wrap_row_count();
        let empty_bottom_height = if state.mode.is_code_editor() {
            bounds
                .size
                .height
                .half()
                .max(BOTTOM_MARGIN_ROWS * line_height)
        } else {
            px(0.)
        };

        let mut scroll_size = size(
            if longest_line_width + line_number_width + RIGHT_MARGIN > bounds.size.width {
                longest_line_width + line_number_width + RIGHT_MARGIN
            } else {
                longest_line_width
            },
            (total_wrapped_lines as f32 * line_height + empty_bottom_height + ghost_lines_height)
                .max(bounds.size.height),
        );

        if last_layout.text_align == TextAlign::Right || last_layout.text_align == TextAlign::Center
        {
            scroll_size.width = longest_line_width + line_number_width;
        }

        let (cursor_bounds, cursor_scroll_offset, current_row) =
            self.layout_cursor(&last_layout, &mut bounds, window, cx);
        last_layout.cursor_bounds = cursor_bounds;

        let search_match_paths = self.layout_search_matches(&last_layout, &mut bounds, cx);
        let selection_path = self.layout_selections(&last_layout, &mut bounds, window, cx);
        let hover_highlight_path = self.layout_hover_highlight(&last_layout, &mut bounds, cx);
        let document_color_paths =
            self.layout_document_colors(&document_colors, &last_layout, &bounds, cx);

        let state = self.state.read(cx);
        let line_numbers = if state.mode.line_number() {
            let mut line_numbers = Vec::with_capacity(last_layout.visible_buffer_lines.len());
            let other_line_runs = vec![TextRun {
                len: line_number_len,
                font: style.font(),
                color: cx.theme().colors().muted_foreground.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }];
            let current_line_runs = vec![TextRun {
                len: line_number_len,
                font: style.font(),
                color: cx.theme().colors().foreground.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }];

            // build line numbers
            for (line, &buffer_line) in last_layout
                .lines
                .iter()
                .zip(last_layout.visible_buffer_lines.iter())
            {
                let line_no: SharedString =
                    format!("{:>width$}", buffer_line + 1, width = line_number_len).into();

                let runs = if current_row == Some(buffer_line) {
                    &current_line_runs
                } else {
                    &other_line_runs
                };

                let mut sub_lines: SmallVec<[ShapedLine; 1]> = SmallVec::new();
                sub_lines.push(
                    window
                        .text_system()
                        .shape_line(line_no, text_size, &runs, None),
                );
                for _ in 0..line.wrapped_lines.len().saturating_sub(1) {
                    sub_lines.push(ShapedLine::default());
                }
                line_numbers.push(sub_lines);
            }
            Some(line_numbers)
        } else {
            None
        };

        let hover_definition_hitbox = self.layout_hover_definition_hitbox(state, window, cx);
        let indent_guides_path =
            self.layout_indent_guides(state, &bounds, &last_layout, &text_style, window);
        let fold_icon_layout = self.layout_fold_icons(&bounds, &last_layout, window, cx);

        PrepaintState {
            bounds,
            last_layout,
            scroll_size,
            line_numbers,
            cursor_bounds,
            cursor_scroll_offset,
            current_row,
            selection_path,
            search_match_paths,
            hover_highlight_path,
            hover_definition_hitbox,
            document_color_paths,
            indent_guides_path,
            fold_icon_layout,
            ghost_first_line,
            ghost_lines,
            ghost_lines_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _: Option<&inazuma::InspectorElementId>,
        input_bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let show_cursor = self.state.read(cx).show_cursor(window, cx);
        let focused = focus_handle.is_focused(window);
        let bounds = prepaint.bounds;
        let selected_range = self.state.read(cx).selected_range;
        let text_align = prepaint.last_layout.text_align;

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );

        // Set AppShell focused_input when self is focused
        if focused {
            let state = self.state.clone();
            if AppShell::read(window, cx).focused_input() != Some(&state) {
                AppShell::update(window, cx, |root, _, cx| {
                    root.set_focused_input(Some(state));
                    cx.notify();
                });
            }
        }

        // And reset focused_input when next_frame start
        window.on_next_frame({
            let state = self.state.clone();
            move |window, cx| {
                if !focused && AppShell::read(window, cx).focused_input() == Some(&state) {
                    AppShell::update(window, cx, |root, _, cx| {
                        root.set_focused_input(None);
                        cx.notify();
                    });
                }
            }
        });

        let line_height = window.line_height();
        let origin = bounds.origin;
        let active_line_color: Option<inazuma::Oklch> = None;

        let mut mask_offset_y = px(0.);
        let state = self.state.read(cx);
        if state.masked && state.text.len() > 0 {
            if cfg!(target_os = "macos") {
                mask_offset_y = px(3.);
            } else {
                mask_offset_y = px(2.5);
            }
        }

        // Paint active line backgrounds
        Self::paint_active_lines(prepaint, &input_bounds, origin, line_height, active_line_color, window);

        // Paint indent guides
        if let Some(path) = prepaint.indent_guides_path.take() {
            window.paint_path(path, cx.theme().colors().border.opacity(0.85));
        }

        // Paint selections and highlights
        Self::paint_selections_and_highlights(prepaint, window, cx);

        // Paint document colors
        Self::paint_document_colors(prepaint, window);

        // Paint text lines and get cursor row y position
        let cursor_row_y = Self::paint_text_lines(
            prepaint,
            origin,
            mask_offset_y,
            &prepaint.scroll_size,
            text_align,
            line_height,
            window,
            cx,
        );

        // Paint blinking cursor
        if focused && show_cursor {
            if let Some(cursor_bounds) = prepaint.cursor_bounds_with_scroll() {
                window.paint_quad(fill(cursor_bounds, cx.theme().colors().accent));
            }
        }

        // Paint line numbers
        Self::paint_line_numbers(prepaint, &input_bounds, origin, line_height, active_line_color, window, cx);

        // Paint fold icons
        self.paint_fold_icons(
            &mut prepaint.fold_icon_layout,
            prepaint.current_row,
            window,
            cx,
        );

        // Update state
        self.paint_state_update(prepaint, input_bounds, selected_range.start..selected_range.end, cx);

        if let Some(hitbox) = prepaint.hover_definition_hitbox.as_ref() {
            window.set_cursor_style(inazuma::CursorStyle::PointingHand, &hitbox);
        }

        // Paint inline completion first line suffix
        if focused {
            Self::paint_inline_completion_suffix(prepaint, cursor_row_y, text_align, line_height, window, cx);
        }

        self.paint_mouse_listeners(window, cx);
    }
}
