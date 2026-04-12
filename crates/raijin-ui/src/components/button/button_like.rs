use documented::Documented;
use inazuma::{
    AnyElement, AnyView, ClickEvent, CursorStyle, DefiniteLength, FocusHandle, MouseButton,
    MouseClickEvent, MouseDownEvent, MouseUpEvent, Oklch, StyleRefinement, relative,
};
use smallvec::SmallVec;

use super::button_variant::{
    ButtonCommon, ButtonLikeRounding, ButtonSize, ButtonStyle, SelectableButton, TintColor,
};
use crate::{DynamicSpacing, ElevationIndex, prelude::*};

/// A button-like element that can be used to create a custom button when
/// prebuilt buttons are not sufficient. Use this sparingly, as it is
/// unconstrained and may make the UI feel less consistent.
///
/// This is also used to build the prebuilt buttons.
#[derive(IntoElement, Documented, RegisterComponent)]
pub struct ButtonLike {
    pub(super) base: Div,
    id: ElementId,
    pub(super) style: ButtonStyle,
    pub(super) disabled: bool,
    pub(super) selected: bool,
    pub(super) selected_style: Option<ButtonStyle>,
    pub(super) width: Option<DefiniteLength>,
    pub(super) height: Option<DefiniteLength>,
    pub(super) layer: Option<ElevationIndex>,
    tab_index: Option<isize>,
    size: ButtonSize,
    rounding: Option<ButtonLikeRounding>,
    tooltip: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyView>>,
    hoverable_tooltip: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyView>>,
    cursor_style: CursorStyle,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_right_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    children: SmallVec<[AnyElement; 2]>,
    focus_handle: Option<FocusHandle>,
    border_color_override: Option<Oklch>,
}

impl ButtonLike {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            style: ButtonStyle::default(),
            disabled: false,
            selected: false,
            selected_style: None,
            width: None,
            height: None,
            size: ButtonSize::Default,
            rounding: Some(ButtonLikeRounding::ALL),
            tooltip: None,
            hoverable_tooltip: None,
            children: SmallVec::new(),
            cursor_style: CursorStyle::PointingHand,
            on_click: None,
            on_right_click: None,
            layer: None,
            tab_index: None,
            focus_handle: None,
            border_color_override: None,
        }
    }

    pub fn new_rounded_left(id: impl Into<ElementId>) -> Self {
        Self::new(id).rounding(ButtonLikeRounding::LEFT)
    }

    pub fn new_rounded_right(id: impl Into<ElementId>) -> Self {
        Self::new(id).rounding(ButtonLikeRounding::RIGHT)
    }

    pub fn new_rounded_all(id: impl Into<ElementId>) -> Self {
        Self::new(id).rounding(ButtonLikeRounding::ALL)
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.base = self.base.opacity(opacity);
        self
    }

    pub fn height(mut self, height: DefiniteLength) -> Self {
        self.height = Some(height);
        self
    }

    /// Overrides the border color from the button style.
    pub fn border_color(mut self, color: impl Into<Oklch>) -> Self {
        self.border_color_override = Some(color.into());
        self
    }

    pub(crate) fn rounding(mut self, rounding: impl Into<Option<ButtonLikeRounding>>) -> Self {
        self.rounding = rounding.into();
        self
    }

    pub fn on_right_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_right_click = Some(Box::new(handler));
        self
    }

    /// Set the tooltip shown on hover.
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Box::new(tooltip));
        self
    }

    pub fn hoverable_tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.hoverable_tooltip = Some(Box::new(tooltip));
        self
    }

    /// Set the tab index for keyboard navigation.
    pub fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.tab_index = Some(tab_index.into());
        self
    }
}

impl Disableable for ButtonLike {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for ButtonLike {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl SelectableButton for ButtonLike {
    fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.selected_style = Some(style);
        self
    }
}

impl Clickable for ButtonLike {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.cursor_style = cursor_style;
        self
    }
}

impl FixedWidth for ButtonLike {
    fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    fn full_width(mut self) -> Self {
        self.width = Some(relative(1.));
        self
    }
}

impl ButtonCommon for ButtonLike {
    fn id(&self) -> &ElementId {
        &self.id
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    fn button_tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Box::new(tooltip));
        self
    }

    fn layer(mut self, elevation: ElevationIndex) -> Self {
        self.layer = Some(elevation);
        self
    }

    fn track_focus(mut self, focus_handle: &inazuma::FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }
}

impl VisibleOnHover for ButtonLike {
    fn visible_on_hover(mut self, group_name: impl Into<SharedString>) -> Self {
        self.base = self.base.visible_on_hover(group_name);
        self
    }
}

impl ParentElement for ButtonLike {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for ButtonLike {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = self
            .selected_style
            .filter(|_| self.selected)
            .unwrap_or(self.style);

        let is_outlined = style.has_border();

        self.base
            .h_flex()
            .id(self.id.clone())
            .when_some(self.tab_index, |this, tab_index| this.tab_index(tab_index))
            .when_some(self.focus_handle, |this, focus_handle| {
                this.track_focus(&focus_handle)
            })
            .font_ui(cx)
            .group("")
            .flex_none()
            .h(self.height.unwrap_or(self.size.rems().into()))
            .when_some(self.width, |this, width| {
                this.w(width).justify_center().text_center()
            })
            .when(is_outlined, |this| this.border_1())
            .when_some(self.rounding, |this, rounding| {
                this.when(rounding.top_left, |this| this.rounded_tl_sm())
                    .when(rounding.top_right, |this| this.rounded_tr_sm())
                    .when(rounding.bottom_right, |this| this.rounded_br_sm())
                    .when(rounding.bottom_left, |this| this.rounded_bl_sm())
            })
            .gap(DynamicSpacing::Base04.rems(cx))
            .map(|this| match self.size {
                ButtonSize::Large | ButtonSize::Medium => this.px(DynamicSpacing::Base08.rems(cx)),
                ButtonSize::Default | ButtonSize::Compact => {
                    this.px(DynamicSpacing::Base04.rems(cx))
                }
                ButtonSize::None => this.px_px(),
            })
            .border_color(
                self.border_color_override
                    .unwrap_or(style.enabled(self.layer, cx).border_color),
            )
            .bg(style.enabled(self.layer, cx).background)
            .when(self.disabled, |this| {
                if self.cursor_style == CursorStyle::PointingHand {
                    this.cursor_not_allowed()
                } else {
                    this.cursor(self.cursor_style)
                }
            })
            .when(!self.disabled, |this| {
                let hovered_style = style.hovered_style(self.layer, cx);
                let focus_color =
                    |refinement: StyleRefinement| refinement.bg(hovered_style.background);

                this.cursor(self.cursor_style)
                    .hover(focus_color)
                    .map(|this| {
                        if is_outlined {
                            this.focus_visible(|s| {
                                s.border_color(cx.theme().colors().border_focused)
                            })
                        } else {
                            this.focus_visible(focus_color)
                        }
                    })
                    .active(|active| active.bg(style.active_style(cx).background))
            })
            .when_some(
                self.on_right_click.filter(|_| !self.disabled),
                |this, on_right_click| {
                    this.on_mouse_down(MouseButton::Right, |_event, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_mouse_up(
                        MouseButton::Right,
                        move |event, window, cx| {
                            cx.stop_propagation();
                            let click_event = ClickEvent::Mouse(MouseClickEvent {
                                down: MouseDownEvent {
                                    button: MouseButton::Right,
                                    position: event.position,
                                    modifiers: event.modifiers,
                                    click_count: 1,
                                    first_mouse: false,
                                },
                                up: MouseUpEvent {
                                    button: MouseButton::Right,
                                    position: event.position,
                                    modifiers: event.modifiers,
                                    click_count: 1,
                                },
                            });
                            (on_right_click)(&click_event, window, cx)
                        },
                    )
                },
            )
            .when_some(
                self.on_click.filter(|_| !self.disabled),
                |this, on_click| {
                    this.on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            (on_click)(event, window, cx)
                        })
                },
            )
            .when_some(self.tooltip, |this, tooltip| {
                this.tooltip(move |window, cx| tooltip(window, cx))
            })
            .when_some(self.hoverable_tooltip, |this, tooltip| {
                this.hoverable_tooltip(move |window, cx| tooltip(window, cx))
            })
            .children(self.children)
    }
}

impl Component for ButtonLike {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn sort_name() -> &'static str {
        // ButtonLike should be at the bottom of the button list
        "ButtonZ"
    }

    fn description() -> Option<&'static str> {
        Some(ButtonLike::DOCS)
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group(vec![
                        single_example(
                            "Default",
                            ButtonLike::new("default")
                                .child(Label::new("Default"))
                                .into_any_element(),
                        ),
                        single_example(
                            "Primary",
                            ButtonLike::new("filled")
                                .style(ButtonStyle::FILLED)
                                .child(Label::new("Primary"))
                                .into_any_element(),
                        ),
                        single_example(
                            "Default",
                            ButtonLike::new("outline")
                                .style(ButtonStyle::SUBTLE)
                                .child(Label::new("Default"))
                                .into_any_element(),
                        ),
                        single_example(
                            "Info",
                            ButtonLike::new("tinted_accent_style")
                                .style(ButtonStyle::tinted(TintColor::Accent))
                                .child(Label::new("Info"))
                                .into_any_element(),
                        ),
                        single_example(
                            "Ghost",
                            ButtonLike::new("transparent")
                                .style(ButtonStyle::TRANSPARENT)
                                .child(Label::new("Ghost"))
                                .into_any_element(),
                        ),
                    ]),
                    example_group_with_title(
                        "Button Group Constructors",
                        vec![
                            single_example(
                                "Left Rounded",
                                ButtonLike::new_rounded_left("left_rounded")
                                    .child(Label::new("Left Rounded"))
                                    .style(ButtonStyle::FILLED)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Right Rounded",
                                ButtonLike::new_rounded_right("right_rounded")
                                    .child(Label::new("Right Rounded"))
                                    .style(ButtonStyle::FILLED)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Button Group",
                                h_flex()
                                    .gap_px()
                                    .child(
                                        ButtonLike::new_rounded_left("bg_left")
                                            .child(Label::new("Left"))
                                            .style(ButtonStyle::FILLED),
                                    )
                                    .child(
                                        ButtonLike::new_rounded_right("bg_right")
                                            .child(Label::new("Right"))
                                            .style(ButtonStyle::FILLED),
                                    )
                                    .into_any_element(),
                            ),
                        ],
                    ),
                ])
                .into_any_element(),
        )
    }
}
