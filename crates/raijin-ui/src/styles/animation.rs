use crate::prelude::*;
use inazuma::{AnimationElement, AnimationExt, Styled};
use std::time::Duration;

use inazuma::ease_out_quint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationDuration {
    Instant = 50,
    Fast = 150,
    Slow = 300,
}

impl AnimationDuration {
    pub fn duration(&self) -> Duration {
        Duration::from_millis(*self as u64)
    }
}

impl Into<std::time::Duration> for AnimationDuration {
    fn into(self) -> Duration {
        self.duration()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationDirection {
    FromBottom,
    FromLeft,
    FromRight,
    FromTop,
}

pub trait DefaultAnimations: Styled + Sized + Element {
    fn animate_in(
        self,
        animation_type: AnimationDirection,
        fade_in: bool,
    ) -> AnimationElement<Self> {
        let animation_name = match animation_type {
            AnimationDirection::FromBottom => "animate_from_bottom",
            AnimationDirection::FromLeft => "animate_from_left",
            AnimationDirection::FromRight => "animate_from_right",
            AnimationDirection::FromTop => "animate_from_top",
        };

        let animation_id = self.id().map_or_else(
            || ElementId::from(animation_name),
            |id| (id, animation_name).into(),
        );

        self.with_animation(
            animation_id,
            inazuma::Animation::new(AnimationDuration::Fast.into()).with_easing(ease_out_quint()),
            move |mut this, delta| {
                let start_opacity = 0.4;
                let start_pos = 0.0;
                let end_pos = 40.0;

                if fade_in {
                    this = this.opacity(start_opacity + delta * (1.0 - start_opacity));
                }

                match animation_type {
                    AnimationDirection::FromBottom => {
                        this.bottom(px(start_pos + delta * (end_pos - start_pos)))
                    }
                    AnimationDirection::FromLeft => {
                        this.left(px(start_pos + delta * (end_pos - start_pos)))
                    }
                    AnimationDirection::FromRight => {
                        this.right(px(start_pos + delta * (end_pos - start_pos)))
                    }
                    AnimationDirection::FromTop => {
                        this.top(px(start_pos + delta * (end_pos - start_pos)))
                    }
                }
            },
        )
    }

    fn animate_in_from_bottom(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromBottom, fade)
    }

    fn animate_in_from_left(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromLeft, fade)
    }

    fn animate_in_from_right(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromRight, fade)
    }

    fn animate_in_from_top(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromTop, fade)
    }
}

impl<E: Styled + Element> DefaultAnimations for E {}

/// A cubic bezier easing function, equivalent to CSS `cubic-bezier(x1, y1, x2, y2)`.
///
/// Visualize at: https://cubic-bezier.com
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    move |t: f32| {
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let t2 = t * t;
        let t3 = t2 * t;

        let _x = 3.0 * x1 * one_t2 * t + 3.0 * x2 * one_t * t2 + t3;
        let y = 3.0 * y1 * one_t2 * t + 3.0 * y2 * one_t * t2 + t3;

        y
    }
}

// Don't use this directly, it only exists to show animation previews
#[derive(RegisterComponent)]
struct Animation {}

impl Component for Animation {
    fn scope() -> ComponentScope {
        ComponentScope::Utilities
    }

    fn description() -> Option<&'static str> {
        Some("Demonstrates various animation patterns and transitions available in the UI system.")
    }

    fn preview(_window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let container_size = 128.0;
        let element_size = 32.0;
        let offset = container_size / 2.0 - element_size / 2.0;

        let container = || {
            h_flex()
                .relative()
                .justify_center()
                .bg(cx.theme().colors().text.opacity(0.05))
                .border_1()
                .border_color(cx.theme().colors().border)
                .rounded_sm()
        };

        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Animate In",
                        vec![
                            single_example(
                                "From Bottom",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("animate-in-from-bottom")
                                            .absolute()
                                            .size(px(element_size))
                                            .left(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::red())
                                            .animate_in_from_bottom(false),
                                    )
                                    .into_any_element(),
                            ),
                            single_example(
                                "From Top",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("animate-in-from-top")
                                            .absolute()
                                            .size(px(element_size))
                                            .left(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::blue())
                                            .animate_in_from_top(false),
                                    )
                                    .into_any_element(),
                            ),
                            single_example(
                                "From Left",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("animate-in-from-left")
                                            .absolute()
                                            .size(px(element_size))
                                            .top(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::green())
                                            .animate_in_from_left(false),
                                    )
                                    .into_any_element(),
                            ),
                            single_example(
                                "From Right",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("animate-in-from-right")
                                            .absolute()
                                            .size(px(element_size))
                                            .top(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::yellow())
                                            .animate_in_from_right(false),
                                    )
                                    .into_any_element(),
                            ),
                        ],
                    )
                    .grow(),
                    example_group_with_title(
                        "Fade and Animate In",
                        vec![
                            single_example(
                                "From Bottom",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("fade-animate-in-from-bottom")
                                            .absolute()
                                            .size(px(element_size))
                                            .left(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::red())
                                            .animate_in_from_bottom(true),
                                    )
                                    .into_any_element(),
                            ),
                            single_example(
                                "From Top",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("fade-animate-in-from-top")
                                            .absolute()
                                            .size(px(element_size))
                                            .left(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::blue())
                                            .animate_in_from_top(true),
                                    )
                                    .into_any_element(),
                            ),
                            single_example(
                                "From Left",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("fade-animate-in-from-left")
                                            .absolute()
                                            .size(px(element_size))
                                            .top(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::green())
                                            .animate_in_from_left(true),
                                    )
                                    .into_any_element(),
                            ),
                            single_example(
                                "From Right",
                                container()
                                    .size(px(container_size))
                                    .child(
                                        div()
                                            .id("fade-animate-in-from-right")
                                            .absolute()
                                            .size(px(element_size))
                                            .top(px(offset))
                                            .rounded_md()
                                            .bg(inazuma::yellow())
                                            .animate_in_from_right(true),
                                    )
                                    .into_any_element(),
                            ),
                        ],
                    )
                    .grow(),
                ])
                .into_any_element(),
        )
    }
}
