use inazuma::{Oklch, IntoElement, ParentElement, PathBuilder, canvas, point};

use crate::prelude::*;

pub fn divider() -> Divider {
    Divider::horizontal()
}

pub fn vertical_divider() -> Divider {
    Divider::vertical()
}

#[derive(Clone, Copy, PartialEq)]
enum DividerStyle {
    Solid,
    Dashed,
}

#[derive(Clone, Copy, PartialEq)]
enum DividerDirection {
    Horizontal,
    Vertical,
}

/// The color of a [`Divider`].
#[derive(Default)]
pub enum DividerColor {
    Border,
    BorderFaded,
    #[default]
    BorderVariant,
    Custom(Oklch),
}

impl DividerColor {
    pub fn hsla(self, cx: &mut App) -> Oklch {
        match self {
            DividerColor::Border => cx.theme().colors().border,
            DividerColor::BorderFaded => cx.theme().colors().border.opacity(0.6),
            DividerColor::BorderVariant => cx.theme().colors().border_variant,
            DividerColor::Custom(color) => color,
        }
    }
}

impl From<Oklch> for DividerColor {
    fn from(color: Oklch) -> Self {
        DividerColor::Custom(color)
    }
}

#[derive(IntoElement, RegisterComponent)]
pub struct Divider {
    style: DividerStyle,
    direction: DividerDirection,
    color: DividerColor,
    inset: bool,
    label: Option<SharedString>,
}

impl Divider {
    pub fn horizontal() -> Self {
        Self {
            style: DividerStyle::Solid,
            direction: DividerDirection::Horizontal,
            color: DividerColor::default(),
            inset: false,
            label: None,
        }
    }

    pub fn vertical() -> Self {
        Self {
            style: DividerStyle::Solid,
            direction: DividerDirection::Vertical,
            color: DividerColor::default(),
            inset: false,
            label: None,
        }
    }

    pub fn horizontal_dashed() -> Self {
        Self {
            style: DividerStyle::Dashed,
            direction: DividerDirection::Horizontal,
            color: DividerColor::default(),
            inset: false,
            label: None,
        }
    }

    pub fn vertical_dashed() -> Self {
        Self {
            style: DividerStyle::Dashed,
            direction: DividerDirection::Vertical,
            color: DividerColor::default(),
            inset: false,
            label: None,
        }
    }

    pub fn inset(mut self) -> Self {
        self.inset = true;
        self
    }

    pub fn color(mut self, color: DividerColor) -> Self {
        self.color = color;
        self
    }

    /// Sets a custom Oklch color for the divider line.
    pub fn custom_color(mut self, color: impl Into<Oklch>) -> Self {
        self.color = DividerColor::Custom(color.into());
        self
    }

    /// Sets a label displayed centered on the divider (horizontal only).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn render_solid_line(direction: DividerDirection, color: Oklch) -> Div {
        div()
            .absolute()
            .map(|this| match direction {
                DividerDirection::Horizontal => this.h(px(1.)).w_full(),
                DividerDirection::Vertical => this.w(px(1.)).h_full(),
            })
            .bg(color)
    }

    fn render_dashed_line(direction: DividerDirection, color: Oklch) -> impl IntoElement {
        div()
            .absolute()
            .map(|this| match direction {
                DividerDirection::Horizontal => this.h(px(1.)).w_full(),
                DividerDirection::Vertical => this.w(px(1.)).h_full(),
            })
            .child(
                canvas(
                    move |_, _, _| {},
                    move |bounds, _, window, _| {
                        let mut builder =
                            PathBuilder::stroke(px(1.)).dash_array(&[px(4.), px(2.)]);
                        let (start, end) = match direction {
                            DividerDirection::Horizontal => {
                                let x = bounds.origin.x;
                                let y = bounds.origin.y + px(0.5);
                                (point(x, y), point(x + bounds.size.width, y))
                            }
                            DividerDirection::Vertical => {
                                let x = bounds.origin.x + px(0.5);
                                let y = bounds.origin.y;
                                (point(x, y), point(x, y + bounds.size.height))
                            }
                        };
                        builder.move_to(start);
                        builder.line_to(end);
                        if let Ok(line) = builder.build() {
                            window.paint_path(line, color);
                        }
                    },
                )
                .size_full(),
            )
    }
}

impl RenderOnce for Divider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.hsla(cx);
        let direction = self.direction;
        let style = self.style;

        let base = match direction {
            DividerDirection::Horizontal => div()
                .min_w_0()
                .h_px()
                .w_full()
                .when(self.inset, |this| this.mx_1p5()),
            DividerDirection::Vertical => div()
                .min_w_0()
                .w_px()
                .h_full()
                .when(self.inset, |this| this.my_1p5()),
        };

        base.flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .child(match style {
                DividerStyle::Solid => Self::render_solid_line(direction, color).into_any_element(),
                DividerStyle::Dashed => {
                    Self::render_dashed_line(direction, color).into_any_element()
                }
            })
            .when_some(self.label, |this, label| {
                this.child(
                    div()
                        .px_2()
                        .py_1()
                        .mx_auto()
                        .text_xs()
                        .bg(cx.theme().colors().background)
                        .text_color(cx.theme().colors().text_muted)
                        .child(label),
                )
            })
    }
}

impl Component for Divider {
    fn scope() -> ComponentScope {
        ComponentScope::Layout
    }

    fn description() -> Option<&'static str> {
        Some(
            "Visual separator used to create divisions between groups of content or sections in a layout.",
        )
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Horizontal Dividers",
                        vec![
                            single_example("Default", Divider::horizontal().into_any_element()),
                            single_example(
                                "Border Color",
                                Divider::horizontal()
                                    .color(DividerColor::Border)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Inset",
                                Divider::horizontal().inset().into_any_element(),
                            ),
                            single_example(
                                "Dashed",
                                Divider::horizontal_dashed().into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "Vertical Dividers",
                        vec![
                            single_example(
                                "Default",
                                div().h_16().child(Divider::vertical()).into_any_element(),
                            ),
                            single_example(
                                "Border Color",
                                div()
                                    .h_16()
                                    .child(Divider::vertical().color(DividerColor::Border))
                                    .into_any_element(),
                            ),
                            single_example(
                                "Inset",
                                div()
                                    .h_16()
                                    .child(Divider::vertical().inset())
                                    .into_any_element(),
                            ),
                            single_example(
                                "Dashed",
                                div()
                                    .h_16()
                                    .child(Divider::vertical_dashed())
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "Example Usage",
                        vec![single_example(
                            "Between Content",
                            v_flex()
                                .w_full()
                                .gap_4()
                                .px_4()
                                .child(Label::new("Section One"))
                                .child(Divider::horizontal())
                                .child(Label::new("Section Two"))
                                .child(Divider::horizontal_dashed())
                                .child(Label::new("Section Three"))
                                .into_any_element(),
                        )],
                    ),
                ])
                .into_any_element(),
        )
    }
}
