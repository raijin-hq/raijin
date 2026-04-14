use std::ops::Range;

use crate::{LabelLike, prelude::*};
use inazuma::{HighlightStyle, StyleRefinement, StyledText};

/// A struct representing a label element in the UI.
///
/// The `Label` struct stores the label text and common properties for a label element.
/// It provides methods for modifying these properties.
///
/// # Examples
///
/// ```
/// use raijin_ui::prelude::*;
///
/// Label::new("Hello, World!");
/// ```
///
/// **A colored label**, for example labeling a dangerous action:
///
/// ```
/// use raijin_ui::prelude::*;
///
/// let my_label = Label::new("Delete").color(Color::Error);
/// ```
///
/// **A label with a strikethrough**, for example labeling something that has been deleted:
///
/// ```
/// use raijin_ui::prelude::*;
///
/// let my_label = Label::new("Deleted").strikethrough();
/// ```
const MASKED_CHAR: &str = "•";

/// Represents the type of match for highlighting text in a label.
#[derive(Clone)]
pub enum HighlightsMatch {
    /// Matches only at the beginning of the text.
    Prefix(SharedString),
    /// Matches all occurrences throughout the text.
    Full(SharedString),
}

impl HighlightsMatch {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Prefix(s) => s.as_str(),
            Self::Full(s) => s.as_str(),
        }
    }

    #[inline]
    pub fn is_prefix(&self) -> bool {
        matches!(self, Self::Prefix(_))
    }
}

impl From<&str> for HighlightsMatch {
    fn from(value: &str) -> Self {
        Self::Full(value.to_string().into())
    }
}

impl From<String> for HighlightsMatch {
    fn from(value: String) -> Self {
        Self::Full(value.into())
    }
}

impl From<SharedString> for HighlightsMatch {
    fn from(value: SharedString) -> Self {
        Self::Full(value)
    }
}

#[derive(IntoElement, RegisterComponent)]
pub struct Label {
    base: LabelLike,
    label: SharedString,
    secondary: Option<SharedString>,
    masked: bool,
    highlights_text: Option<HighlightsMatch>,
}

impl Label {
    /// Creates a new [`Label`] with the given text.
    ///
    /// # Examples
    ///
    /// ```
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!");
    /// ```
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            base: LabelLike::new(),
            label: label.into(),
            secondary: None,
            masked: false,
            highlights_text: None,
        }
    }

    /// Sets the text of the [`Label`].
    pub fn set_text(&mut self, text: impl Into<SharedString>) {
        self.label = text.into();
    }

    /// Sets secondary text displayed after the main label in muted color.
    pub fn secondary(mut self, secondary: impl Into<SharedString>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    /// Masks the label text with bullet characters (for passwords).
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Sets text to highlight within the label.
    pub fn highlights(mut self, text: impl Into<HighlightsMatch>) -> Self {
        self.highlights_text = Some(text.into());
        self
    }

    /// Truncates the label from the start, keeping the end visible.
    pub fn truncate_start(mut self) -> Self {
        self.base = self.base.truncate_start();
        self
    }

    fn full_text(&self) -> SharedString {
        match &self.secondary {
            Some(secondary) => format!("{} {}", self.label, secondary).into(),
            None => self.label.clone(),
        }
    }

    fn highlight_ranges(&self, total_length: usize) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let full_text = self.full_text();

        if self.secondary.is_some() {
            ranges.push(0..self.label.len());
            ranges.push(self.label.len()..total_length);
        }

        if let Some(matched) = &self.highlights_text {
            let matched_str = matched.as_str();
            if !matched_str.is_empty() {
                let search_lower = matched_str.to_lowercase();
                let full_text_lower = full_text.to_lowercase();

                if matched.is_prefix() {
                    if full_text_lower.starts_with(&search_lower) {
                        ranges.push(0..matched_str.len());
                    }
                } else {
                    let mut search_start = 0;
                    while let Some(pos) = full_text_lower[search_start..].find(&search_lower) {
                        let match_start = search_start + pos;
                        let match_end = match_start + matched_str.len();

                        if match_end <= full_text.len() {
                            ranges.push(match_start..match_end);
                        }

                        search_start = match_start + 1;
                        while !full_text.is_char_boundary(search_start)
                            && search_start < full_text.len()
                        {
                            search_start += 1;
                        }

                        if search_start >= full_text.len() {
                            break;
                        }
                    }
                }
            }
        }

        ranges
    }

    fn measure_highlights(
        &self,
        length: usize,
        cx: &App,
    ) -> Option<Vec<(Range<usize>, HighlightStyle)>> {
        let ranges = self.highlight_ranges(length);
        if ranges.is_empty() {
            return None;
        }

        let mut highlights = Vec::new();
        let mut highlight_ranges_added = 0;

        if self.secondary.is_some() {
            highlights.push((ranges[0].clone(), HighlightStyle::default()));
            highlights.push((
                ranges[1].clone(),
                HighlightStyle {
                    color: Some(cx.theme().colors().text_muted),
                    ..Default::default()
                },
            ));
            highlight_ranges_added = 2;
        }

        for range in ranges.iter().skip(highlight_ranges_added) {
            highlights.push((
                range.clone(),
                HighlightStyle {
                    color: Some(cx.theme().colors().text_accent),
                    ..Default::default()
                },
            ));
        }

        Some(inazuma::combine_highlights(vec![], highlights).collect())
    }
}

// Style methods.
impl Label {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.base.style()
    }

    inazuma::margin_style_methods!({
        visibility: pub
    });

    pub fn flex_1(mut self) -> Self {
        self.style().flex_grow = Some(1.);
        self.style().flex_shrink = Some(1.);
        self.style().flex_basis = Some(inazuma::relative(0.).into());
        self
    }

    pub fn flex_none(mut self) -> Self {
        self.style().flex_grow = Some(0.);
        self.style().flex_shrink = Some(0.);
        self
    }

    pub fn flex_grow(mut self) -> Self {
        self.style().flex_grow = Some(1.);
        self
    }

    pub fn flex_shrink(mut self) -> Self {
        self.style().flex_shrink = Some(1.);
        self
    }

    pub fn flex_shrink_0(mut self) -> Self {
        self.style().flex_shrink = Some(0.);
        self
    }
}

impl LabelCommon for Label {
    /// Sets the size of the label using a [`LabelSize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!").size(LabelSize::Small);
    /// ```
    fn size(mut self, size: LabelSize) -> Self {
        self.base = self.base.size(size);
        self
    }

    /// Sets the weight of the label using a [`FontWeight`].
    ///
    /// # Examples
    ///
    /// ```
    /// use inazuma::FontWeight;
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!").weight(FontWeight::BOLD);
    /// ```
    fn weight(mut self, weight: inazuma::FontWeight) -> Self {
        self.base = self.base.weight(weight);
        self
    }

    /// Sets the line height style of the label using a [`LineHeightStyle`].
    ///
    /// # Examples
    ///
    /// ```
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!").line_height_style(LineHeightStyle::UiLabel);
    /// ```
    fn line_height_style(mut self, line_height_style: LineHeightStyle) -> Self {
        self.base = self.base.line_height_style(line_height_style);
        self
    }

    /// Sets the color of the label using a [`Color`].
    ///
    /// # Examples
    ///
    /// ```
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!").color(Color::Accent);
    /// ```
    fn color(mut self, color: Color) -> Self {
        self.base = self.base.color(color);
        self
    }

    /// Sets the strikethrough property of the label.
    ///
    /// # Examples
    ///
    /// ```
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!").strikethrough();
    /// ```
    fn strikethrough(mut self) -> Self {
        self.base = self.base.strikethrough();
        self
    }

    /// Sets the italic property of the label.
    ///
    /// # Examples
    ///
    /// ```
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!").italic();
    /// ```
    fn italic(mut self) -> Self {
        self.base = self.base.italic();
        self
    }

    /// Sets the alpha property of the color of label.
    ///
    /// # Examples
    ///
    /// ```
    /// use raijin_ui::prelude::*;
    ///
    /// let my_label = Label::new("Hello, World!").alpha(0.5);
    /// ```
    fn alpha(mut self, alpha: f32) -> Self {
        self.base = self.base.alpha(alpha);
        self
    }

    fn underline(mut self) -> Self {
        self.base = self.base.underline();
        self
    }

    /// Truncates overflowing text with an ellipsis (`…`) if needed.
    fn truncate(mut self) -> Self {
        self.base = self.base.truncate();
        self
    }

    fn single_line(mut self) -> Self {
        self.label = SharedString::from(self.label.replace('\n', "⏎"));
        self.base = self.base.single_line();
        self
    }

    fn buffer_font(mut self, cx: &App) -> Self {
        self.base = self.base.buffer_font(cx);
        self
    }

    /// Styles the label to look like inline code.
    fn inline_code(mut self, cx: &App) -> Self {
        self.base = self.base.inline_code(cx);
        self
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_highlights = self.secondary.is_some() || self.highlights_text.is_some();

        if !has_highlights && !self.masked {
            return self.base.child(self.label).into_any_element();
        }

        let mut text = self.full_text();
        let chars_count = text.chars().count();

        if self.masked {
            text = SharedString::from(MASKED_CHAR.repeat(chars_count));
        }

        let highlights = self.measure_highlights(text.len(), cx);

        match highlights {
            Some(hl) => self
                .base
                .child(StyledText::new(text).with_highlights(hl))
                .into_any_element(),
            None => self.base.child(text).into_any_element(),
        }
    }
}

impl Component for Label {
    fn scope() -> ComponentScope {
        ComponentScope::Typography
    }

    fn description() -> Option<&'static str> {
        Some("A text label component that supports various styles, sizes, and formatting options.")
    }

    fn preview(_window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Sizes",
                        vec![
                            single_example("Default", Label::new("Project Explorer").into_any_element()),
                            single_example("Small", Label::new("File: main.rs").size(LabelSize::Small).into_any_element()),
                            single_example("Large", Label::new("Welcome to Raijin").size(LabelSize::Large).into_any_element()),
                        ],
                    ),
                    example_group_with_title(
                        "Colors",
                        vec![
                            single_example("Default", Label::new("Status: Ready").into_any_element()),
                            single_example("Accent", Label::new("New Update Available").color(Color::Accent).into_any_element()),
                            single_example("Error", Label::new("Build Failed").color(Color::Error).into_any_element()),
                        ],
                    ),
                    example_group_with_title(
                        "Styles",
                        vec![
                            single_example("Default", Label::new("Normal Text").into_any_element()),
                            single_example("Bold", Label::new("Important Notice").weight(inazuma::FontWeight::BOLD).into_any_element()),
                            single_example("Italic", Label::new("Code Comment").italic().into_any_element()),
                            single_example("Strikethrough", Label::new("Deprecated Feature").strikethrough().into_any_element()),
                            single_example("Underline", Label::new("Clickable Link").underline().into_any_element()),
                            single_example("Inline Code", Label::new("fn main() {}").inline_code(cx).into_any_element()),
                        ],
                    ),
                    example_group_with_title(
                        "Line Height Styles",
                        vec![
                            single_example("Default", Label::new("Multi-line\nText\nExample").into_any_element()),
                            single_example("UI Label", Label::new("Compact\nUI\nLabel").line_height_style(LineHeightStyle::UiLabel).into_any_element()),
                        ],
                    ),
                    example_group_with_title(
                        "Special Cases",
                        vec![
                            single_example("Single Line", Label::new("Line 1\nLine 2\nLine 3").single_line().into_any_element()),
                            single_example("Regular Truncation", div().max_w_24().child(Label::new("This is a very long file name that should be truncated: very_long_file_name_with_many_words.rs").truncate()).into_any_element()),
                            single_example("Start Truncation", div().max_w_24().child(Label::new("raijin/crates/ui/src/components/label/truncate/label/label.rs").truncate_start()).into_any_element()),
                        ],
                    ),
                ])
                .into_any_element()
        )
    }
}
