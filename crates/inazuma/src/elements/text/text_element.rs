use super::*;

impl Element for &'static str {
    type RequestLayoutState = TextLayout;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut state = TextLayout::default();
        let layout_id = state.layout(SharedString::from(*self), None, window, cx);
        (layout_id, state)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        text_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
        text_layout.prepaint(bounds, self)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        text_layout: &mut TextLayout,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        text_layout.paint(self, window, cx)
    }
}

impl IntoElement for &'static str {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl IntoElement for String {
    type Element = SharedString;

    fn into_element(self) -> Self::Element {
        self.into()
    }
}

impl IntoElement for Cow<'static, str> {
    type Element = SharedString;

    fn into_element(self) -> Self::Element {
        self.into()
    }
}

impl Element for SharedString {
    type RequestLayoutState = TextLayout;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut state = TextLayout::default();
        let layout_id = state.layout(self.clone(), None, window, cx);
        (layout_id, state)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        text_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
        text_layout.prepaint(bounds, self.as_ref())
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        text_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        text_layout.paint(self.as_ref(), window, cx)
    }
}

impl IntoElement for SharedString {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Renders text with runs of different styles.
///
/// Callers are responsible for setting the correct style for each run.
/// For text with a uniform style, you can usually avoid calling this constructor
/// and just pass text directly.
pub struct StyledText {
    pub(super) text: SharedString,
    pub(super) runs: Option<Vec<TextRun>>,
    pub(super) delayed_highlights: Option<Vec<(Range<usize>, HighlightStyle)>>,
    pub(super) layout: TextLayout,
}

impl StyledText {
    /// Construct a new styled text element from the given string.
    pub fn new(text: impl Into<SharedString>) -> Self {
        StyledText {
            text: text.into(),
            runs: None,
            delayed_highlights: None,
            layout: TextLayout::default(),
        }
    }

    /// Get the layout for this element. This can be used to map indices to pixels and vice versa.
    pub fn layout(&self) -> &TextLayout {
        &self.layout
    }

    /// Set the styling attributes for the given text, as well as
    /// as any ranges of text that have had their style customized.
    pub fn with_default_highlights(
        mut self,
        default_style: &TextStyle,
        highlights: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>,
    ) -> Self {
        debug_assert!(
            self.delayed_highlights.is_none(),
            "Can't use `with_default_highlights` and `with_highlights`"
        );
        let runs = Self::compute_runs(&self.text, default_style, highlights);
        self.with_runs(runs)
    }

    /// Set the styling attributes for the given text, as well as
    /// as any ranges of text that have had their style customized.
    pub fn with_highlights(
        mut self,
        highlights: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>,
    ) -> Self {
        debug_assert!(
            self.runs.is_none(),
            "Can't use `with_highlights` and `with_default_highlights`"
        );
        self.delayed_highlights = Some(
            highlights
                .into_iter()
                .inspect(|(run, _)| {
                    debug_assert!(self.text.is_char_boundary(run.start));
                    debug_assert!(self.text.is_char_boundary(run.end));
                })
                .collect::<Vec<_>>(),
        );
        self
    }

    pub(super) fn compute_runs(
        text: &str,
        default_style: &TextStyle,
        highlights: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>,
    ) -> Vec<TextRun> {
        let mut runs = Vec::new();
        let mut ix = 0;
        for (range, highlight) in highlights {
            if ix < range.start {
                debug_assert!(text.is_char_boundary(range.start));
                runs.push(default_style.clone().to_run(range.start - ix));
            }
            debug_assert!(text.is_char_boundary(range.end));
            runs.push(
                default_style
                    .clone()
                    .highlight(highlight)
                    .to_run(range.len()),
            );
            ix = range.end;
        }
        if ix < text.len() {
            runs.push(default_style.to_run(text.len() - ix));
        }
        runs
    }

    /// Set the text runs for this piece of text.
    pub fn with_runs(mut self, runs: Vec<TextRun>) -> Self {
        let mut text = &**self.text;
        for run in &runs {
            text = text.get(run.len..).unwrap_or_else(|| {
                #[cfg(debug_assertions)]
                panic!("invalid text run. Text: '{text}', run: {run:?}");
                #[cfg(not(debug_assertions))]
                panic!("invalid text run");
            });
        }
        assert!(text.is_empty(), "invalid text run");
        self.runs = Some(runs);
        self
    }
}

impl Element for StyledText {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let runs = self.runs.take().or_else(|| {
            self.delayed_highlights.take().map(|delayed_highlights| {
                Self::compute_runs(&self.text, &window.text_style(), delayed_highlights)
            })
        });

        let layout_id = self.layout.layout(self.text.clone(), runs, window, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
        self.layout.prepaint(bounds, &self.text)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.layout.paint(&self.text, window, cx)
    }
}

impl IntoElement for StyledText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// The Layout for TextElement. This can be used to map indices to pixels and vice versa.
#[derive(Default, Clone)]
pub struct TextLayout(Rc<RefCell<Option<TextLayoutInner>>>);

pub(super) struct TextLayoutInner {
    pub(super) len: usize,
    pub(super) lines: SmallVec<[WrappedLine; 1]>,
    pub(super) line_height: Pixels,
    pub(super) wrap_width: Option<Pixels>,
    pub(super) size: Option<Size<Pixels>>,
    pub(super) bounds: Option<Bounds<Pixels>>,
}

impl TextLayout {
    pub(super) fn layout(
        &self,
        text: SharedString,
        runs: Option<Vec<TextRun>>,
        window: &mut Window,
        _: &mut App,
    ) -> LayoutId {
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = text_style
            .line_height
            .to_pixels(font_size.into(), window.rem_size());

        let runs = if let Some(runs) = runs {
            runs
        } else {
            vec![text_style.to_run(text.len())]
        };
        window.request_measured_layout(Default::default(), {
            let element_state = self.clone();

            move |known_dimensions, available_space, window, cx| {
                let wrap_width = if text_style.white_space == WhiteSpace::Normal {
                    known_dimensions.width.or(match available_space.width {
                        crate::AvailableSpace::Definite(x) => Some(x),
                        _ => None,
                    })
                } else {
                    None
                };

                let (truncate_width, truncation_affix, truncate_from) =
                    if let Some(text_overflow) = text_style.text_overflow.clone() {
                        let width = known_dimensions.width.or(match available_space.width {
                            crate::AvailableSpace::Definite(x) => match text_style.line_clamp {
                                Some(max_lines) => Some(x * max_lines),
                                None => Some(x),
                            },
                            _ => None,
                        });

                        match text_overflow {
                            TextOverflow::Truncate(s) => (width, s, TruncateFrom::End),
                            TextOverflow::TruncateStart(s) => (width, s, TruncateFrom::Start),
                        }
                    } else {
                        (None, "".into(), TruncateFrom::End)
                    };

                // Only use cached layout if:
                // 1. We have a cached size
                // 2. wrap_width matches (or both are None)
                // 3. truncate_width is None (if truncate_width is Some, we need to re-layout
                //    because the previous layout may have been computed without truncation)
                if let Some(text_layout) = element_state.0.borrow().as_ref()
                    && let Some(size) = text_layout.size
                    && (wrap_width.is_none() || wrap_width == text_layout.wrap_width)
                    && truncate_width.is_none()
                {
                    return size;
                }

                let mut line_wrapper = cx.text_system().line_wrapper(text_style.font(), font_size);
                let (text, runs) = if let Some(truncate_width) = truncate_width {
                    line_wrapper.truncate_line(
                        text.clone(),
                        truncate_width,
                        &truncation_affix,
                        &runs,
                        truncate_from,
                    )
                } else {
                    (text.clone(), Cow::Borrowed(&*runs))
                };
                let len = text.len();

                let Some(lines) = window
                    .text_system()
                    .shape_text(
                        text,
                        font_size,
                        &runs,
                        wrap_width,            // Wrap if we know the width.
                        text_style.line_clamp, // Limit the number of lines if line_clamp is set.
                    )
                    .log_err()
                else {
                    element_state.0.borrow_mut().replace(TextLayoutInner {
                        lines: Default::default(),
                        len: 0,
                        line_height,
                        wrap_width,
                        size: Some(Size::default()),
                        bounds: None,
                    });
                    return Size::default();
                };

                let mut size: Size<Pixels> = Size::default();
                for line in &lines {
                    let line_size = line.size(line_height);
                    size.height += line_size.height;
                    size.width = size.width.max(line_size.width).ceil();
                }

                element_state.0.borrow_mut().replace(TextLayoutInner {
                    lines,
                    len,
                    line_height,
                    wrap_width,
                    size: Some(size),
                    bounds: None,
                });

                size
            }
        })
    }

    pub(super) fn prepaint(&self, bounds: Bounds<Pixels>, text: &str) {
        let mut element_state = self.0.borrow_mut();
        let element_state = element_state
            .as_mut()
            .with_context(|| format!("measurement has not been performed on {text}"))
            .unwrap();
        element_state.bounds = Some(bounds);
    }

    pub(super) fn paint(&self, text: &str, window: &mut Window, cx: &mut App) {
        let element_state = self.0.borrow();
        let element_state = element_state
            .as_ref()
            .with_context(|| format!("measurement has not been performed on {text}"))
            .unwrap();
        let bounds = element_state
            .bounds
            .with_context(|| format!("prepaint has not been performed on {text}"))
            .unwrap();

        let line_height = element_state.line_height;
        let mut line_origin = bounds.origin;
        let text_style = window.text_style();
        for line in &element_state.lines {
            line.paint_background(
                line_origin,
                line_height,
                text_style.text_align,
                Some(bounds),
                window,
                cx,
            )
            .log_err();
            line.paint(
                line_origin,
                line_height,
                text_style.text_align,
                Some(bounds),
                window,
                cx,
            )
            .log_err();
            line_origin.y += line.size(line_height).height;
        }
    }

    /// Get the byte index into the input of the pixel position.
    pub fn index_for_position(&self, mut position: Point<Pixels>) -> Result<usize, usize> {
        let element_state = self.0.borrow();
        let element_state = element_state
            .as_ref()
            .expect("measurement has not been performed");
        let bounds = element_state
            .bounds
            .expect("prepaint has not been performed");

        if position.y < bounds.top() {
            return Err(0);
        }

        let line_height = element_state.line_height;
        let mut line_origin = bounds.origin;
        let mut line_start_ix = 0;
        for line in &element_state.lines {
            let line_bottom = line_origin.y + line.size(line_height).height;
            if position.y > line_bottom {
                line_origin.y = line_bottom;
                line_start_ix += line.len() + 1;
            } else {
                let position_within_line = position - line_origin;
                match line.index_for_position(position_within_line, line_height) {
                    Ok(index_within_line) => return Ok(line_start_ix + index_within_line),
                    Err(index_within_line) => return Err(line_start_ix + index_within_line),
                }
            }
        }

        Err(line_start_ix.saturating_sub(1))
    }

    /// Get the pixel position for the given byte index.
    pub fn position_for_index(&self, index: usize) -> Option<Point<Pixels>> {
        let element_state = self.0.borrow();
        let element_state = element_state
            .as_ref()
            .expect("measurement has not been performed");
        let bounds = element_state
            .bounds
            .expect("prepaint has not been performed");
        let line_height = element_state.line_height;

        let mut line_origin = bounds.origin;
        let mut line_start_ix = 0;

        for line in &element_state.lines {
            let line_end_ix = line_start_ix + line.len();
            if index < line_start_ix {
                break;
            } else if index > line_end_ix {
                line_origin.y += line.size(line_height).height;
                line_start_ix = line_end_ix + 1;
                continue;
            } else {
                let ix_within_line = index - line_start_ix;
                return Some(line_origin + line.position_for_index(ix_within_line, line_height)?);
            }
        }

        None
    }

    /// Retrieve the layout for the line containing the given byte index.
    pub fn line_layout_for_index(&self, index: usize) -> Option<Arc<WrappedLineLayout>> {
        let element_state = self.0.borrow();
        let element_state = element_state
            .as_ref()
            .expect("measurement has not been performed");
        let bounds = element_state
            .bounds
            .expect("prepaint has not been performed");
        let line_height = element_state.line_height;

        let mut line_origin = bounds.origin;
        let mut line_start_ix = 0;

        for line in &element_state.lines {
            let line_end_ix = line_start_ix + line.len();
            if index < line_start_ix {
                break;
            } else if index > line_end_ix {
                line_origin.y += line.size(line_height).height;
                line_start_ix = line_end_ix + 1;
                continue;
            } else {
                return Some(line.layout.clone());
            }
        }

        None
    }

    /// The bounds of this layout.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.0.borrow().as_ref().unwrap().bounds.unwrap()
    }

    /// The line height for this layout.
    pub fn line_height(&self) -> Pixels {
        self.0.borrow().as_ref().unwrap().line_height
    }

    /// The UTF-8 length of the underlying text.
    pub fn len(&self) -> usize {
        self.0.borrow().as_ref().unwrap().len
    }

    /// The text for this layout.
    pub fn text(&self) -> String {
        self.0
            .borrow()
            .as_ref()
            .unwrap()
            .lines
            .iter()
            .map(|s| &s.text)
            .join("\n")
    }

    /// The text for this layout (with soft-wraps as newlines)
    pub fn wrapped_text(&self) -> String {
        let mut accumulator = String::new();

        for wrapped in self.0.borrow().as_ref().unwrap().lines.iter() {
            let mut seen = 0;
            for boundary in wrapped.layout.wrap_boundaries.iter() {
                let index = wrapped.layout.unwrapped_layout.runs[boundary.run_ix].glyphs
                    [boundary.glyph_ix]
                    .index;

                accumulator.push_str(&wrapped.text[seen..index]);
                accumulator.push('\n');
                seen = index;
            }
            accumulator.push_str(&wrapped.text[seen..]);
            accumulator.push('\n');
        }
        // Remove trailing newline
        accumulator.pop();
        accumulator
    }
}
