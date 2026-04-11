use crate::{
    h_flex, v_flex, Text, Tooltip, ActiveTheme, ButtonCommon as _, ButtonStyle, Clickable as _,
    Color, Component, ComponentScope, Disableable, ElevationIndex, Icon, IconButton, IconName,
    IconSize, KeyBinding, Label, LabelCommon as _, LabelSize, RegisterComponent, Side, Sizable,
    Size, StyledExt, ToggleState, example_group_with_title, single_example,
};
use inazuma::{
    div, prelude::*, px, Animation, AnimationExt as _, AnyElement, AnyView, App, Edges, ElementId,
    InteractiveElement, IntoElement, Oklch, ParentElement as _, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window,
};
use std::{rc::Rc, sync::Arc, time::Duration};

/// Defines the color for a switch component.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum SwitchColor {
    #[default]
    Accent,
    Custom(Oklch),
}

/// Defines the label position for a switch component.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum SwitchLabelPosition {
    Start,
    #[default]
    End,
}

/// A Switch element with smooth toggle animation.
///
/// Supports multiple sizes, custom colors, labels, key bindings, and animated thumb movement.
#[derive(IntoElement, RegisterComponent)]
pub struct Switch {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    disabled: bool,
    size: Size,
    color: SwitchColor,
    label: Option<SharedString>,
    label_position: SwitchLabelPosition,
    label_size: LabelSize,
    full_width: bool,
    key_binding: Option<KeyBinding>,
    tab_index: Option<isize>,
    tooltip: Option<SharedString>,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Switch {
    /// Create a new Switch element.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            size: Size::Medium,
            color: SwitchColor::default(),
            label: None,
            label_position: SwitchLabelPosition::default(),
            label_size: LabelSize::Small,
            full_width: false,
            key_binding: None,
            tab_index: None,
            tooltip: None,
            on_click: None,
        }
    }

    /// Set the checked state of the switch.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set checked from a ToggleState (convenience for consumers using ToggleState).
    pub fn toggle_state(mut self, state: ToggleState) -> Self {
        self.checked = state == ToggleState::Selected;
        self
    }

    /// Set the switch color.
    pub fn color(mut self, color: SwitchColor) -> Self {
        self.color = color;
        self
    }

    /// Set the label of the switch.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the label position.
    pub fn label_position(mut self, position: SwitchLabelPosition) -> Self {
        self.label_position = position;
        self
    }

    /// Set the label size.
    pub fn label_size(mut self, size: LabelSize) -> Self {
        self.label_size = size;
        self
    }

    /// Set the switch to fill the entire width of its container.
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Display the keybinding that triggers the switch action.
    pub fn key_binding(mut self, key_binding: impl Into<Option<KeyBinding>>) -> Self {
        self.key_binding = key_binding.into();
        self
    }

    /// Set the tab index for keyboard navigation.
    pub fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.tab_index = Some(tab_index.into());
        self
    }

    /// Set tooltip for the switch.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Add a click handler for the switch. The `&bool` parameter is the new checked state.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Add a click handler using ToggleState (convenience for consumers using ToggleState).
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(move |checked, window, cx| {
            let state = if *checked {
                ToggleState::Selected
            } else {
                ToggleState::Unselected
            };
            handler(&state, window, cx);
        }));
        self
    }

    fn resolve_colors(&self, cx: &App) -> (Oklch, Oklch) {
        if !self.checked {
            return (cx.theme().colors().element_background, cx.theme().colors().icon);
        }

        match self.color {
            SwitchColor::Accent => (cx.theme().colors().primary, cx.theme().colors().icon),
            SwitchColor::Custom(color) => (color, cx.theme().colors().icon),
        }
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Switch {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let on_click = self.on_click.clone();
        let toggle_state = window.use_keyed_state(self.id.clone(), cx, |_, _| checked);

        let (bg, toggle_bg) = self.resolve_colors(cx);
        let (bg, toggle_bg) = if self.disabled {
            (
                if checked { bg.alpha(0.5) } else { bg },
                toggle_bg.alpha(0.35),
            )
        } else {
            (bg, toggle_bg)
        };

        let (bg_width, bg_height) = match self.size {
            Size::XSmall | Size::Small => (px(28.), px(16.)),
            _ => (px(36.), px(20.)),
        };
        let bar_width = match self.size {
            Size::XSmall | Size::Small => px(12.),
            _ => px(16.),
        };
        let inset = px(2.);
        let radius = if cx.theme().colors().radius >= px(4.) {
            bg_height
        } else {
            cx.theme().colors().radius
        };

        let switch_bar = div()
            .id((self.id.clone(), "bar"))
            .w(bg_width)
            .h(bg_height)
            .rounded(radius)
            .flex()
            .items_center()
            .border(inset)
            .border_color(Oklch::transparent_black())
            .bg(bg)
            .when_some(self.tab_index.filter(|_| !self.disabled), |this, tab_index| {
                this.tab_index(tab_index)
                    .focus_visible(|mut style| {
                        let c = cx.theme().colors().primary;
                        style.border_colors = Some(Edges {
                            top: Some(c),
                            right: Some(c),
                            bottom: Some(c),
                            left: Some(c),
                        });
                        style
                    })
            })
            .when_some(self.tooltip.clone(), |this, tooltip| {
                this.tooltip(move |window, cx| {
                    Tooltip::new(tooltip.clone()).build(window, cx)
                })
            })
            .child(
                div()
                    .rounded(radius)
                    .bg(toggle_bg)
                    .shadow_md()
                    .size(bar_width)
                    .map(|this| {
                        let prev_checked = toggle_state.read(cx);
                        if !self.disabled && *prev_checked != checked {
                            let duration = Duration::from_secs_f64(0.15);
                            cx.spawn({
                                let toggle_state = toggle_state.clone();
                                async move |cx| {
                                    cx.background_executor().timer(duration).await;
                                    _ = toggle_state.update(cx, |this, _| *this = checked);
                                }
                            })
                            .detach();

                            this.with_animation(
                                ElementId::NamedInteger("move".into(), checked as u64),
                                Animation::new(duration),
                                move |this, delta| {
                                    let max_x = bg_width - bar_width - inset * 2;
                                    let x = if checked {
                                        max_x * delta
                                    } else {
                                        max_x - max_x * delta
                                    };
                                    this.left(x)
                                },
                            )
                            .into_any_element()
                        } else {
                            let max_x = bg_width - bar_width - inset * 2;
                            let x = if checked { max_x } else { px(0.) };
                            this.left(x).into_any_element()
                        }
                    }),
            );

        div().refine_style(&self.style).child(
            h_flex()
                .id(self.id.clone())
                .gap_2()
                .items_center()
                .when(self.full_width, |this| this.w_full().justify_between())
                .when(
                    self.label_position == SwitchLabelPosition::Start,
                    |this| {
                        this.when_some(self.label.clone(), |this, label| {
                            this.child(Label::new(label).size(self.label_size))
                        })
                    },
                )
                .child(switch_bar)
                .when(
                    self.label_position == SwitchLabelPosition::End,
                    |this| {
                        this.when_some(self.label.clone(), |this, label| {
                            this.child(Label::new(label).size(self.label_size))
                        })
                    },
                )
                .children(self.key_binding)
                .when_some(
                    on_click
                        .as_ref()
                        .cloned()
                        .filter(|_| !self.disabled),
                    |this, on_click| {
                        let toggle_state = toggle_state.clone();
                        this.on_mouse_down(inazuma::MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            _ = toggle_state.update(cx, |this, _| *this = checked);
                            on_click(&!checked, window, cx);
                        })
                    },
                ),
        )
    }
}

/// A field component that combines a label, description, and switch into one reusable component.
#[derive(IntoElement, RegisterComponent)]
pub struct SwitchField {
    id: ElementId,
    label: Option<SharedString>,
    description: Option<SharedString>,
    checked: bool,
    on_click: Arc<dyn Fn(&bool, &mut Window, &mut App) + 'static>,
    disabled: bool,
    color: SwitchColor,
    tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView>>,
    tab_index: Option<isize>,
}

impl SwitchField {
    pub fn new(
        id: impl Into<ElementId>,
        label: Option<impl Into<SharedString>>,
        description: Option<SharedString>,
        checked: bool,
        on_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.map(Into::into),
            description,
            checked,
            on_click: Arc::new(on_click),
            disabled: false,
            color: SwitchColor::Accent,
            tooltip: None,
            tab_index: None,
        }
    }

    /// Create from ToggleState (convenience for consumers using ToggleState).
    pub fn from_toggle_state(
        id: impl Into<ElementId>,
        label: Option<impl Into<SharedString>>,
        description: Option<SharedString>,
        toggle_state: ToggleState,
        on_click: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::new(
            id,
            label,
            description,
            toggle_state == ToggleState::Selected,
            move |checked, window, cx| {
                let state = if *checked {
                    ToggleState::Selected
                } else {
                    ToggleState::Unselected
                };
                on_click(&state, window, cx);
            },
        )
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn color(mut self, color: SwitchColor) -> Self {
        self.color = color;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = Some(tab_index);
        self
    }
}

impl RenderOnce for SwitchField {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let tooltip = self
            .tooltip
            .zip(self.label.clone())
            .map(|(tooltip_fn, label)| {
                h_flex().gap_0p5().child(Label::new(label)).child(
                    IconButton::new("tooltip_button", IconName::Info)
                        .icon_size(IconSize::XSmall)
                        .icon_color(Color::Muted)
                        .shape(crate::IconButtonShape::Square)
                        .style(ButtonStyle::TRANSPARENT)
                        .tooltip({
                            let tooltip = tooltip_fn.clone();
                            move |window, cx| tooltip(window, cx)
                        })
                        .on_click(|_, _, _| {}),
                )
            });

        let checked = self.checked;

        h_flex()
            .id((self.id.clone(), "container"))
            .when(!self.disabled, |this| {
                this.hover(|this| this.cursor_pointer())
            })
            .w_full()
            .gap_4()
            .justify_between()
            .flex_wrap()
            .child(match (&self.description, tooltip) {
                (Some(description), Some(tooltip)) => v_flex()
                    .gap_0p5()
                    .max_w_5_6()
                    .child(tooltip)
                    .child(Label::new(description.clone()).color(Color::Muted))
                    .into_any_element(),
                (Some(description), None) => v_flex()
                    .gap_0p5()
                    .max_w_5_6()
                    .when_some(self.label.clone(), |this, label| this.child(Label::new(label)))
                    .child(Label::new(description.clone()).color(Color::Muted))
                    .into_any_element(),
                (None, Some(tooltip)) => tooltip.into_any_element(),
                (None, None) => {
                    if let Some(label) = self.label.clone() {
                        Label::new(label).into_any_element()
                    } else {
                        inazuma::Empty.into_any_element()
                    }
                }
            })
            .child(
                Switch::new((self.id.clone(), "switch"))
                    .checked(checked)
                    .color(self.color)
                    .disabled(self.disabled)
                    .when_some(
                        self.tab_index.filter(|_| !self.disabled),
                        |this, tab_index| this.tab_index(tab_index),
                    )
                    .on_click({
                        let on_click = self.on_click.clone();
                        move |new_checked, window, cx| {
                            (on_click)(new_checked, window, cx);
                        }
                    }),
            )
            .when(!self.disabled, |this| {
                this.on_click({
                    let on_click = self.on_click.clone();
                    move |_click, window, cx| {
                        (on_click)(&!checked, window, cx);
                    }
                })
            })
    }
}

impl Component for Switch {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn name() -> &'static str {
        "Switch"
    }

    fn description() -> Option<&'static str> {
        Some("A toggle switch with smooth animation for binary on/off states.")
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "States",
                        vec![
                            single_example(
                                "Off",
                                Switch::new("switch_off").into_any_element(),
                            ),
                            single_example(
                                "On",
                                Switch::new("switch_on")
                                    .checked(true)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Disabled Off",
                                Switch::new("switch_disabled_off")
                                    .disabled(true)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Disabled On",
                                Switch::new("switch_disabled_on")
                                    .checked(true)
                                    .disabled(true)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "Sizes",
                        vec![
                            single_example(
                                "Small",
                                Switch::new("switch_small")
                                    .checked(true)
                                    .with_size(Size::Small)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Medium",
                                Switch::new("switch_medium")
                                    .checked(true)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "With Label",
                        vec![
                            single_example(
                                "Label End",
                                Switch::new("switch_label_end")
                                    .checked(true)
                                    .label("Enable feature")
                                    .into_any_element(),
                            ),
                            single_example(
                                "Label Start",
                                Switch::new("switch_label_start")
                                    .checked(true)
                                    .label("Enable feature")
                                    .label_position(SwitchLabelPosition::Start)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                ])
                .into_any_element(),
        )
    }
}

impl Component for SwitchField {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn name() -> &'static str {
        "SwitchField"
    }

    fn description() -> Option<&'static str> {
        Some("A field component that combines a label, description, and switch.")
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "States",
                        vec![
                            single_example(
                                "Off",
                                SwitchField::new(
                                    "field_off",
                                    Some("Enable notifications"),
                                    Some("Receive notifications when new messages arrive.".into()),
                                    false,
                                    |_, _, _| {},
                                )
                                .into_any_element(),
                            ),
                            single_example(
                                "On",
                                SwitchField::new(
                                    "field_on",
                                    Some("Enable notifications"),
                                    Some("Receive notifications when new messages arrive.".into()),
                                    true,
                                    |_, _, _| {},
                                )
                                .into_any_element(),
                            ),
                            single_example(
                                "Disabled",
                                SwitchField::new(
                                    "field_disabled",
                                    Some("Disabled field"),
                                    Some("This field is disabled.".into()),
                                    true,
                                    |_, _, _| {},
                                )
                                .disabled(true)
                                .into_any_element(),
                            ),
                        ],
                    ),
                ])
                .into_any_element(),
        )
    }
}
