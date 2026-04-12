use std::cmp::Ordering;
use std::rc::Rc;

use inazuma::prelude::FluentBuilder as _;
use inazuma::{
    AnyElement, App, ClickEvent, Div, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, Stateful, StatefulInteractiveElement, Styled,
    Window, div, px, relative,
};
use raijin_theme::ActiveTheme;

use super::variant::TabVariant;
use crate::prelude::*;
use crate::traits::selectable::Selectable;
use crate::traits::size::{Sizable, Size};

/// The position of a [`Tab`] within a workspace tab bar.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TabPosition {
    First,
    Middle(Ordering),
    Last,
}

/// Which side the close button appears on for workspace tabs.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TabCloseSide {
    Start,
    End,
}

const START_TAB_SLOT_SIZE: Pixels = px(12.);
const END_TAB_SLOT_SIZE: Pixels = px(14.);

/// A unified Tab element supporting multiple visual variants.
///
/// Combines workspace tabs (position-awareness, close-side) with
/// visual variants (Outline, Pill, Segmented, Underline).
#[derive(IntoElement)]
pub struct Tab {
    ix: usize,
    base: inazuma::Stateful<Div>,
    pub(crate) label: Option<SharedString>,
    icon: Option<Icon>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    /// Start/end slots for Workspace variant
    start_slot: Option<AnyElement>,
    end_slot: Option<AnyElement>,
    children: Vec<AnyElement>,
    variant: TabVariant,
    size: Size,
    pub(crate) disabled: bool,
    selected: bool,
    /// Whether this tab should show a prefix in the tab bar.
    pub tab_bar_prefix: Option<bool>,
    /// Workspace-specific: position in tab bar
    position: TabPosition,
    /// Workspace-specific: close button side
    close_side: TabCloseSide,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl Default for Tab {
    fn default() -> Self {
        Self {
            ix: 0,
            base: div().id("tab-default"),
            label: None,
            icon: None,
            children: Vec::new(),
            disabled: false,
            selected: false,
            tab_bar_prefix: None,
            prefix: None,
            suffix: None,
            start_slot: None,
            end_slot: None,
            variant: TabVariant::default(),
            size: Size::default(),
            position: TabPosition::First,
            close_side: TabCloseSide::End,
            on_click: None,
        }
    }
}

impl Tab {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).debug_selector(|| format!("TAB-{}", id)),
            ..Default::default()
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    pub fn workspace(mut self) -> Self {
        self.variant = TabVariant::Workspace;
        self
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Start slot (Workspace variant — maps to left side content).
    pub fn start_slot<E: IntoElement>(mut self, element: impl Into<Option<E>>) -> Self {
        self.start_slot = element.into().map(IntoElement::into_any_element);
        self
    }

    /// End slot (Workspace variant — maps to right side content).
    pub fn end_slot<E: IntoElement>(mut self, element: impl Into<Option<E>>) -> Self {
        self.end_slot = element.into().map(IntoElement::into_any_element);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn position(mut self, position: TabPosition) -> Self {
        self.position = position;
        self
    }

    pub fn close_side(mut self, close_side: TabCloseSide) -> Self {
        self.close_side = close_side;
        self
    }

    /// Sets whether this tab should show a prefix in the tab bar.
    pub fn tab_bar_prefix(mut self, prefix: bool) -> Self {
        self.tab_bar_prefix = Some(prefix);
        self
    }

    /// Returns the label text, if any.
    pub fn get_label(&self) -> Option<&SharedString> {
        self.label.as_ref()
    }

    /// Returns whether the tab is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    pub(crate) fn ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }

    pub fn content_height(cx: &App) -> Pixels {
        DynamicSpacing::Base32.px(cx) - px(1.)
    }

    pub fn container_height(cx: &App) -> Pixels {
        DynamicSpacing::Base32.px(cx)
    }
}

impl From<&'static str> for Tab {
    fn from(label: &'static str) -> Self {
        Self::default().label(label)
    }
}

impl From<String> for Tab {
    fn from(label: String) -> Self {
        Self::default().label(label)
    }
}

impl From<SharedString> for Tab {
    fn from(label: SharedString) -> Self {
        Self::default().label(label)
    }
}

impl Selectable for Tab {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Toggleable for Tab {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Sizable for Tab {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut inazuma::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl Styled for Tab {
    fn style(&mut self) -> &mut inazuma::StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Tab {
    #[allow(refining_impl_trait)]
    fn render(self, _: &mut Window, cx: &mut App) -> Stateful<Div> {
        // Workspace variant uses the position-based rendering
        if self.variant == TabVariant::Workspace {
            return self.render_workspace(cx);
        }

        // Visual variants use the style system
        self.render_visual(cx)
    }
}

impl Tab {
    fn render_workspace(self, cx: &App) -> Stateful<Div> {
        let (text_color, tab_bg) = match self.selected {
            false => (
                cx.theme().colors().text_muted,
                cx.theme().colors().tab.inactive_background,
            ),
            true => (
                cx.theme().colors().text,
                cx.theme().colors().tab.active_background,
            ),
        };

        let (start_slot, end_slot) = {
            let start = h_flex()
                .size(START_TAB_SLOT_SIZE)
                .justify_center()
                .children(self.start_slot);
            let end = h_flex()
                .size(END_TAB_SLOT_SIZE)
                .justify_center()
                .children(self.end_slot);
            match self.close_side {
                TabCloseSide::End => (start, end),
                TabCloseSide::Start => (end, start),
            }
        };

        self.base
            .h(Tab::container_height(cx))
            .bg(tab_bg)
            .border_color(cx.theme().colors().border)
            .map(|this| match self.position {
                TabPosition::First => {
                    if self.selected {
                        this.pl_px().border_r_1().pb_px()
                    } else {
                        this.pl_px().pr_px().border_b_1()
                    }
                }
                TabPosition::Last => {
                    if self.selected {
                        this.border_l_1().border_r_1().pb_px()
                    } else {
                        this.pl_px().border_b_1().border_r_1()
                    }
                }
                TabPosition::Middle(Ordering::Equal) => this.border_l_1().border_r_1().pb_px(),
                TabPosition::Middle(Ordering::Less) => this.border_l_1().pr_px().border_b_1(),
                TabPosition::Middle(Ordering::Greater) => this.border_r_1().pl_px().border_b_1(),
            })
            .cursor_pointer()
            .child(
                h_flex()
                    .group("")
                    .relative()
                    .h(Tab::content_height(cx))
                    .px(DynamicSpacing::Base04.px(cx))
                    .gap(DynamicSpacing::Base04.rems(cx))
                    .text_color(text_color)
                    .child(start_slot)
                    .children(self.children)
                    .child(end_slot),
            )
    }

    fn render_visual(self, cx: &App) -> Stateful<Div> {
        let mut tab_style = if self.selected {
            self.variant.selected(cx)
        } else {
            self.variant.normal(cx)
        };
        let mut hover_style = self.variant.hovered(self.selected, cx);
        if self.disabled {
            tab_style = self.variant.disabled(self.selected, cx);
            hover_style = self.variant.disabled(self.selected, cx);
        }
        let radius = self.variant.radius(self.size, cx);
        let inner_radius = self.variant.inner_radius(self.size, cx);
        let inner_paddings = self.variant.inner_paddings(self.size);
        let inner_margins = self.variant.inner_margins(self.size);
        let inner_height = self.variant.inner_height(self.size);
        let height = self.variant.height(self.size);

        self.base
            .flex()
            .flex_wrap()
            .gap_1()
            .items_center()
            .flex_shrink_0()
            .h(height)
            .overflow_hidden()
            .text_color(tab_style.fg)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Large => this.text_base(),
                _ => this.text_sm(),
            })
            .bg(tab_style.bg)
            .border_l(tab_style.borders.left)
            .border_r(tab_style.borders.right)
            .border_t(tab_style.borders.top)
            .border_b(tab_style.borders.bottom)
            .border_color(tab_style.border_color)
            .rounded(radius)
            .when(!self.selected && !self.disabled, |this| {
                this.hover(|this| {
                    this.text_color(hover_style.fg)
                        .bg(hover_style.bg)
                        .border_l(hover_style.borders.left)
                        .border_r(hover_style.borders.right)
                        .border_t(hover_style.borders.top)
                        .border_b(hover_style.borders.bottom)
                        .border_color(hover_style.border_color)
                        .rounded(radius)
                })
            })
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                h_flex()
                    .flex_1()
                    .h(inner_height)
                    .line_height(relative(1.))
                    .whitespace_nowrap()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .margins(inner_margins)
                    .flex_shrink_0()
                    .map(|this| match self.icon {
                        Some(icon) => this.w(inner_height * 1.25).child(
                            icon.map(|this| match self.size {
                                Size::XSmall => this.size_2p5(),
                                Size::Small => this.size_3p5(),
                                _ => this.size_4(),
                            }),
                        ),
                        None => this
                            .paddings(inner_paddings)
                            .map(|this| match self.label {
                                Some(label) => this.child(label),
                                None => this,
                            })
                            .children(self.children),
                    })
                    .bg(tab_style.inner_bg)
                    .rounded(inner_radius)
                    .when(tab_style.shadow, |this| this.shadow_xs())
                    .hover(|this| this.bg(hover_style.inner_bg).rounded(inner_radius)),
            )
            .when_some(self.suffix, |this, suffix| this.child(suffix))
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .when(!self.disabled, |this| {
                this.when_some(self.on_click.clone(), |this, on_click| {
                    this.on_click(move |event, window, cx| on_click(event, window, cx))
                })
            })
    }
}
