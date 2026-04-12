use documented::Documented;
use inazuma::{
    Animation, AnimationExt as _, AnyElement, Bounds, Oklch, Pixels, StyleRefinement, canvas, div, px, relative,
};
use instant::Duration;
use std::f32::consts::TAU;

use crate::components::plot::shape::{Arc, ArcData};
use crate::prelude::*;

/// Tracks the previous value for smooth animation transitions.
struct ProgressState {
    value: f32,
}

/// A circular progress indicator that displays progress as an arc growing
/// clockwise from 12 o'clock, with smooth animation on value changes.
///
/// Supports the `Sizable` trait for consistent sizing across the UI, and can
/// contain children (e.g. a percentage label centered inside the circle).
///
/// # Examples
///
/// ```ignore
/// // Basic usage
/// CircularProgress::new("download", 75.0)
///
/// // With custom color and size
/// CircularProgress::new("upload", 42.0)
///     .color(cx.theme().status().success.color)
///     .large()
///
/// // With a centered label inside
/// CircularProgress::new("score", 88.0)
///     .child(Label::new("88%").size(LabelSize::XSmall))
/// ```
#[derive(IntoElement, RegisterComponent, Documented)]
pub struct CircularProgress {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Oklch>,
    value: f32,
    size: Size,
    children: Vec<AnyElement>,
}

impl CircularProgress {
    /// Creates a new circular progress indicator.
    ///
    /// `value` is a percentage between 0.0 and 100.0.
    pub fn new(id: impl Into<ElementId>, value: f32) -> Self {
        Self {
            id: id.into(),
            value: value.clamp(0.0, 100.0),
            color: None,
            style: StyleRefinement::default(),
            size: Size::default(),
            children: Vec::new(),
        }
    }

    /// Sets the progress color. Defaults to the theme's `progress_bar` color.
    pub fn color(mut self, color: impl Into<Oklch>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the progress value (0.0–100.0).
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 100.0);
        self
    }

    fn render_circle(current_value: f32, color: Oklch) -> impl IntoElement {
        struct PrepaintState {
            current_value: f32,
            inner_radius: f32,
            outer_radius: f32,
            bounds: Bounds<Pixels>,
        }

        canvas(
            {
                let display_value = current_value;
                move |bounds: Bounds<Pixels>, _window: &mut Window, _cx: &mut App| {
                    // Stroke width: 15% of diameter, capped at 5px
                    let stroke_width = (bounds.size.width * 0.15).min(px(5.));

                    let actual_size = bounds.size.width.min(bounds.size.height);
                    let radius = (actual_size.as_f32() - stroke_width.as_f32()) / 2.0;
                    let inner_radius = radius - stroke_width.as_f32() / 2.0;
                    let outer_radius = radius + stroke_width.as_f32() / 2.0;

                    PrepaintState {
                        current_value: display_value,
                        inner_radius,
                        outer_radius,
                        bounds,
                    }
                }
            },
            move |_bounds, prepaint, window: &mut Window, _cx: &mut App| {
                // Background circle (full ring at 20% opacity)
                let bg_arc_data = ArcData {
                    data: &(),
                    index: 0,
                    value: 100.0,
                    start_angle: 0.0,
                    end_angle: TAU,
                    pad_angle: 0.0,
                };

                let arc = Arc::new()
                    .inner_radius(prepaint.inner_radius)
                    .outer_radius(prepaint.outer_radius);

                arc.paint(
                    &bg_arc_data,
                    color.opacity(0.2),
                    None,
                    None,
                    &prepaint.bounds,
                    window,
                );

                // Progress arc
                if prepaint.current_value > 0.0 {
                    let progress_angle = (prepaint.current_value / 100.0) * TAU;
                    let progress_arc_data = ArcData {
                        data: &(),
                        index: 1,
                        value: prepaint.current_value,
                        start_angle: 0.0,
                        end_angle: progress_angle,
                        pad_angle: 0.0,
                    };

                    arc.paint(
                        &progress_arc_data,
                        color,
                        None,
                        None,
                        &prepaint.bounds,
                        window,
                    );
                }
            },
        )
        .absolute()
        .size_full()
    }
}

impl Styled for CircularProgress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for CircularProgress {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for CircularProgress {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for CircularProgress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let value = self.value;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| ProgressState { value });
        let prev_value = state.read(cx).value;

        let color = self.color.unwrap_or(cx.theme().colors().primary);
        let has_changed = prev_value != value;

        div()
            .id(self.id.clone())
            .flex()
            .items_center()
            .justify_center()
            .line_height(relative(1.))
            .map(|this| match self.size {
                Size::XSmall => this.size_2(),
                Size::Small => this.size_3(),
                Size::Medium => this.size_4(),
                Size::Large => this.size_5(),
                Size::Size(s) => this.size(s * 0.75),
            })
            .refine_style(&self.style)
            .children(self.children)
            .map(|this| {
                if has_changed {
                    this.with_animation(
                        format!("progress-circle-{}", prev_value),
                        Animation::new(Duration::from_secs_f64(0.15)),
                        move |this, delta| {
                            let animated_value = prev_value + (value - prev_value) * delta;
                            this.child(Self::render_circle(animated_value, color))
                        },
                    )
                    .into_any_element()
                } else {
                    this.child(Self::render_circle(value, color))
                        .into_any_element()
                }
            })
    }
}

impl Component for CircularProgress {
    fn scope() -> ComponentScope {
        ComponentScope::Status
    }

    fn description() -> Option<&'static str> {
        Some(CircularProgress::DOCS)
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        let container = || v_flex().items_center().gap_1();

        Some(
            example_group(vec![single_example(
                "Examples",
                h_flex()
                    .gap_6()
                    .child(
                        container()
                            .child(CircularProgress::new("p0", 0.0))
                            .child(Label::new("0%").size(LabelSize::Small)),
                    )
                    .child(
                        container()
                            .child(CircularProgress::new("p25", 25.0))
                            .child(Label::new("25%").size(LabelSize::Small)),
                    )
                    .child(
                        container()
                            .child(CircularProgress::new("p50", 50.0))
                            .child(Label::new("50%").size(LabelSize::Small)),
                    )
                    .child(
                        container()
                            .child(CircularProgress::new("p75", 75.0))
                            .child(Label::new("75%").size(LabelSize::Small)),
                    )
                    .child(
                        container()
                            .child(CircularProgress::new("p100", 100.0))
                            .child(Label::new("100%").size(LabelSize::Small)),
                    )
                    .into_any_element(),
            )])
            .into_any_element(),
        )
    }
}
