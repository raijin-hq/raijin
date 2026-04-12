use std::time::Duration;

use crate::{
    ActiveTheme, Color, Component, ComponentScope, Disableable, ElevationIndex, FocusableExt,
    IconName, LabelCommon as _, RegisterComponent, Selectable, Sizable, Size, StyledExt as _,
    ToggleState, example_group_with_title, single_example, Text, v_flex,
};
use inazuma::{
    Animation, AnimationExt, AnyElement, AnyView, App, ClickEvent, Div, ElementId,
    InteractiveElement, IntoElement, Oklch, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, oklcha,
    prelude::FluentBuilder as _, px, relative, rems, svg,
};

/// Creates a new checkbox.
pub fn checkbox(id: impl Into<ElementId>, toggle_state: ToggleState) -> Checkbox {
    Checkbox::new(id).toggle_state(toggle_state)
}

/// The visual style of a checkbox.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ToggleStyle {
    /// Checkbox has a transparent background.
    #[default]
    Ghost,
    /// Checkbox has a filled background based on the elevation index of the parent container.
    ElevationBased(ElevationIndex),
    /// A custom style using a color to tint the checkbox.
    Custom(Oklch),
}

/// A Checkbox element with animated check icon, three-state support, and multiple visual styles.
///
/// Supports `ToggleState` (Selected/Unselected/Indeterminate), animated check icon transitions,
/// multiple sizes, visual styles (Ghost/ElevationBased/Custom), focus ring, tab navigation,
/// placeholder and visualization-only modes.
#[derive(IntoElement, RegisterComponent)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    style_refinement: StyleRefinement,
    toggle_state: ToggleState,
    toggle_style: ToggleStyle,
    disabled: bool,
    placeholder: bool,
    filled: bool,
    visualization: bool,
    size: Size,
    label: Option<SharedString>,
    label_text: Option<Text>,
    label_size: crate::LabelSize,
    label_color: Color,
    children: Vec<AnyElement>,
    tab_stop: bool,
    tab_index: isize,
    tooltip: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyView>>,
    on_click: Option<Box<dyn Fn(&ToggleState, &ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
    /// Creates a new [`Checkbox`].
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style_refinement: StyleRefinement::default(),
            toggle_state: ToggleState::Unselected,
            toggle_style: ToggleStyle::default(),
            disabled: false,
            placeholder: false,
            filled: false,
            visualization: false,
            size: Size::Medium,
            label: None,
            label_text: None,
            label_size: crate::LabelSize::Default,
            label_color: Color::Muted,
            children: Vec::new(),
            tab_stop: true,
            tab_index: 0,
            tooltip: None,
            on_click: None,
        }
    }

    /// Set the toggle state (Selected, Unselected, or Indeterminate).
    pub fn toggle_state(mut self, state: ToggleState) -> Self {
        self.toggle_state = state;
        self
    }

    /// Set checked from a bool (convenience — maps to Selected/Unselected).
    pub fn checked(mut self, checked: bool) -> Self {
        self.toggle_state = if checked {
            ToggleState::Selected
        } else {
            ToggleState::Unselected
        };
        self
    }

    /// Sets the disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the placeholder state (shows a dot instead of checkmark when selected).
    pub fn placeholder(mut self, placeholder: bool) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Sets the `fill` setting, indicating whether the background should be filled.
    pub fn fill(mut self) -> Self {
        self.filled = true;
        self
    }

    /// Makes the checkbox look enabled but without pointer cursor and hover styles.
    /// Primarily used for uninteractive markdown previews.
    pub fn visualization_only(mut self, visualization: bool) -> Self {
        self.visualization = visualization;
        self
    }

    /// Sets the visual style using the specified [`ToggleStyle`].
    pub fn style(mut self, style: ToggleStyle) -> Self {
        self.toggle_style = style;
        self
    }

    /// Match the style to the current elevation using [`ToggleStyle::ElevationBased`].
    pub fn elevation(mut self, elevation: ElevationIndex) -> Self {
        self.toggle_style = ToggleStyle::ElevationBased(elevation);
        self
    }

    /// Set a simple text label (SharedString).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set a rich text label (Text element).
    pub fn label_text(mut self, label: impl Into<Text>) -> Self {
        self.label_text = Some(label.into());
        self
    }

    pub fn label_size(mut self, size: crate::LabelSize) -> Self {
        self.label_size = size;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    /// Sets the tooltip.
    pub fn tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.tooltip = Some(Box::new(tooltip));
        self
    }

    /// Binds a handler that will be called when clicked.
    /// Receives the **new** toggle state after the click.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(move |state, _, window, cx| {
            handler(state, window, cx)
        }));
        self
    }

    /// Binds a handler with access to the ClickEvent.
    pub fn on_click_ext(
        mut self,
        handler: impl Fn(&ToggleState, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Set the tab stop, default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Set the tab index, default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    fn is_checked(&self) -> bool {
        self.toggle_state == ToggleState::Selected
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut inazuma::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Checkbox {}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style_refinement
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Checkbox {
    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }

    fn is_selected(&self) -> bool {
        self.is_checked()
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// Renders an animated check/dash icon for the checkbox.
pub fn checkbox_check_icon(
    id: ElementId,
    size: Size,
    checked: bool,
    disabled: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let toggle_state = if checked { ToggleState::Selected } else { ToggleState::Unselected };
    render_check_icon(id, size, toggle_state, disabled, window, cx)
}

fn render_check_icon(
    id: ElementId,
    size: Size,
    toggle_state: ToggleState,
    disabled: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let is_checked = toggle_state == ToggleState::Selected;
    let is_indeterminate = toggle_state == ToggleState::Indeterminate;
    let show_icon = is_checked || is_indeterminate;

    let anim_state = window.use_keyed_state(id, cx, |_, _| show_icon);
    let color = if disabled {
        cx.theme().colors().primary_foreground.opacity(0.5)
    } else {
        cx.theme().colors().primary_foreground
    };

    let icon_path = if is_indeterminate {
        IconName::Dash.path()
    } else {
        IconName::Check.path()
    };

    svg()
        .absolute()
        .top_px()
        .left_px()
        .map(|this| match size {
            Size::XSmall => this.size_2(),
            Size::Small => this.size_2p5(),
            Size::Medium => this.size_3(),
            Size::Large => this.size_3p5(),
            _ => this.size_3(),
        })
        .text_color(color)
        .when(show_icon, |this| this.path(icon_path))
        .map(|this| {
            if !disabled && show_icon != *anim_state.read(cx) {
                let duration = Duration::from_secs_f64(0.25);
                cx.spawn({
                    let anim_state = anim_state.clone();
                    async move |cx| {
                        cx.background_executor().timer(duration).await;
                        _ = anim_state.update(cx, |this, _| *this = show_icon);
                    }
                })
                .detach();

                this.with_animation(
                    ElementId::NamedInteger("toggle".into(), show_icon as u64),
                    Animation::new(duration),
                    move |this, delta| {
                        this.opacity(if show_icon { delta } else { 1.0 - delta })
                    },
                )
                .into_any_element()
            } else {
                this.into_any_element()
            }
        })
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_checked = self.is_checked();
        let is_indeterminate = self.toggle_state == ToggleState::Indeterminate;

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let border_color = if is_checked || is_indeterminate {
            cx.theme().colors().primary
        } else {
            cx.theme().colors().input
        };
        let color = if self.disabled {
            border_color.opacity(0.5)
        } else {
            border_color
        };
        let radius = cx.theme().colors().radius.min(px(4.));

        let has_label = self.label.is_some() || self.label_text.is_some() || !self.children.is_empty();

        div().child(
            self.base
                .id(self.id.clone())
                .when(!self.disabled && !self.visualization, |this| {
                    this.track_focus(
                        &focus_handle
                            .tab_stop(self.tab_stop)
                            .tab_index(self.tab_index),
                    )
                })
                .h_flex()
                .gap_2()
                .items_start()
                .line_height(relative(1.))
                .text_color(cx.theme().colors().foreground)
                .map(|this| match self.size {
                    Size::XSmall => this.text_xs(),
                    Size::Small => this.text_sm(),
                    Size::Medium => this.text_base(),
                    Size::Large => this.text_lg(),
                    _ => this,
                })
                .map(|this| {
                    if self.disabled {
                        this.cursor_not_allowed()
                            .text_color(cx.theme().colors().muted_foreground)
                    } else if self.visualization {
                        this.cursor_default()
                    } else {
                        this.cursor_pointer()
                    }
                })
                .rounded(cx.theme().colors().radius * 0.5)
                .focus_ring(is_focused, px(2.), window, cx)
                .refine_style(&self.style_refinement)
                .child(
                    div()
                        .relative()
                        .map(|this| match self.size {
                            Size::XSmall => this.size_3(),
                            Size::Small => this.size_3p5(),
                            Size::Medium => this.size_4(),
                            Size::Large => this.size(rems(1.125)),
                            _ => this.size_4(),
                        })
                        .flex_shrink_0()
                        .border_1()
                        .border_color(color)
                        .rounded(radius)
                        .when(cx.theme().is_dark() && !self.disabled, |this| this.shadow_xs())
                        .map(|this| {
                            if is_checked || is_indeterminate {
                                this.bg(color)
                            } else {
                                this.bg(cx.theme().colors().input)
                            }
                        })
                        .when(self.placeholder && is_checked, |this| {
                            this.child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .flex_none()
                                            .rounded_full()
                                            .bg(cx.theme().colors().primary_foreground.opacity(0.5))
                                            .size(px(4.)),
                                    ),
                            )
                        })
                        .when(!self.placeholder, |this| {
                            this.child(render_check_icon(
                                self.id.clone(),
                                self.size,
                                self.toggle_state,
                                self.disabled,
                                window,
                                cx,
                            ))
                        }),
                )
                .when(has_label, |this| {
                    this.child(
                        v_flex()
                            .w_full()
                            .line_height(relative(1.2))
                            .gap_1()
                            .map(|this| {
                                if let Some(label) = self.label {
                                    this.child(
                                        div()
                                            .size_full()
                                            .text_color(cx.theme().colors().foreground)
                                            .when(self.disabled, |this| {
                                                this.text_color(cx.theme().colors().muted_foreground)
                                            })
                                            .line_height(relative(1.))
                                            .child(crate::Label::new(label)
                                                .color(self.label_color)
                                                .size(self.label_size)),
                                    )
                                } else if let Some(label_text) = self.label_text {
                                    this.child(
                                        div()
                                            .size_full()
                                            .text_color(cx.theme().colors().foreground)
                                            .when(self.disabled, |this| {
                                                this.text_color(cx.theme().colors().muted_foreground)
                                            })
                                            .line_height(relative(1.))
                                            .child(label_text),
                                    )
                                } else {
                                    this
                                }
                            })
                            .children(self.children),
                    )
                })
                .when_some(self.tooltip, |this, tooltip| {
                    this.tooltip(move |window, cx| tooltip(window, cx))
                })
                .on_mouse_down(inazuma::MouseButton::Left, |_, window, _| {
                    window.prevent_default();
                })
                .when(!self.disabled && !self.visualization, |this| {
                    this.when_some(self.on_click, |this, on_click| {
                        let toggle_state = self.toggle_state;
                        this.on_click(move |click, window, cx| {
                            window.prevent_default();
                            on_click(&toggle_state.inverse(), click, window, cx);
                        })
                    })
                }),
        )
    }
}

impl Component for Checkbox {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn name() -> &'static str {
        "Checkbox"
    }

    fn description() -> Option<&'static str> {
        Some("A checkbox with animated check icon, three-state support, and multiple visual styles.")
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
                                "Unselected",
                                Checkbox::new("cb_unselected").into_any_element(),
                            ),
                            single_example(
                                "Selected",
                                Checkbox::new("cb_selected")
                                    .toggle_state(ToggleState::Selected)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Indeterminate",
                                Checkbox::new("cb_indeterminate")
                                    .toggle_state(ToggleState::Indeterminate)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Placeholder",
                                Checkbox::new("cb_placeholder")
                                    .toggle_state(ToggleState::Selected)
                                    .placeholder(true)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "Sizes",
                        vec![
                            single_example(
                                "XSmall",
                                Checkbox::new("cb_xs")
                                    .toggle_state(ToggleState::Selected)
                                    .with_size(Size::XSmall)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Small",
                                Checkbox::new("cb_sm")
                                    .toggle_state(ToggleState::Selected)
                                    .with_size(Size::Small)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Medium",
                                Checkbox::new("cb_md")
                                    .toggle_state(ToggleState::Selected)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Large",
                                Checkbox::new("cb_lg")
                                    .toggle_state(ToggleState::Selected)
                                    .with_size(Size::Large)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "Disabled",
                        vec![
                            single_example(
                                "Unselected",
                                Checkbox::new("cb_dis_off")
                                    .disabled(true)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Selected",
                                Checkbox::new("cb_dis_on")
                                    .toggle_state(ToggleState::Selected)
                                    .disabled(true)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "With Label",
                        vec![
                            single_example(
                                "Default",
                                Checkbox::new("cb_label")
                                    .toggle_state(ToggleState::Selected)
                                    .label("Always save on quit")
                                    .into_any_element(),
                            ),
                            single_example(
                                "Visualization-Only",
                                Checkbox::new("cb_viz")
                                    .toggle_state(ToggleState::Selected)
                                    .visualization_only(true)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "Styles",
                        vec![
                            single_example(
                                "Ghost (default)",
                                Checkbox::new("cb_ghost")
                                    .toggle_state(ToggleState::Selected)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Custom Color",
                                Checkbox::new("cb_custom")
                                    .toggle_state(ToggleState::Selected)
                                    .style(ToggleStyle::Custom(oklcha(0.712, 0.1864, 149.8088, 0.7)))
                                    .into_any_element(),
                            ),
                        ],
                    ),
                ])
                .into_any_element(),
        )
    }
}
