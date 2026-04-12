use inazuma::{
    AnyElement, App, Corner, Div, Edges, ElementId, InteractiveElement, IntoElement, Oklch,
    ParentElement, RenderOnce, ScrollHandle, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use smallvec::SmallVec;
use std::rc::Rc;

use super::{Tab, TabVariant};
use crate::{
    ActiveTheme, Button, ButtonVariants as _, Component, ComponentScope, PopupMenuExt as _, IconName, PopupMenuItem,
    RegisterComponent, Selectable, Sizable, Size, StyledExt, example_group_with_title, h_flex,
    single_example,
};

/// A TabBar element that contains multiple [`Tab`] items.
///
/// Supports multiple visual variants (Tab, Pill, Outline, Segmented, Underline),
/// optional scroll tracking, prefix/suffix elements, overflow menu, and size variants.
#[derive(IntoElement, RegisterComponent)]
pub struct TabBar {
    base: Stateful<Div>,
    style: StyleRefinement,
    scroll_handle: Option<ScrollHandle>,
    start_children: SmallVec<[AnyElement; 2]>,
    end_children: SmallVec<[AnyElement; 2]>,
    /// Typed Tab children — used when variant/size/selected propagation is needed.
    tab_children: SmallVec<[Tab; 2]>,
    /// Generic AnyElement children — for arbitrary content (divs, drop targets, etc.).
    children: SmallVec<[AnyElement; 2]>,
    last_empty_space: AnyElement,
    selected_index: Option<usize>,
    variant: TabVariant,
    size: Size,
    menu: bool,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl TabBar {
    /// Create a new TabBar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).px(px(-1.)),
            style: StyleRefinement::default(),
            tab_children: SmallVec::new(),
            children: SmallVec::new(),
            scroll_handle: None,
            start_children: SmallVec::new(),
            end_children: SmallVec::new(),
            variant: TabVariant::default(),
            size: Size::default(),
            last_empty_space: div().w_3().into_any_element(),
            selected_index: None,
            on_click: None,
            menu: false,
        }
    }

    /// Set the Tab variant, all children will inherit the variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the Tab variant to Pill, all children will inherit the variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Set the Tab variant to Outline, all children will inherit the variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Set the Tab variant to Segmented, all children will inherit the variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Set the Tab variant to Underline, all children will inherit the variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set whether to show the menu button when tabs overflow, default is false.
    pub fn menu(mut self, menu: bool) -> Self {
        self.menu = menu;
        self
    }

    /// Track the scroll of the TabBar.
    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    /// Set a single prefix element before the tabs.
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.start_children.push(prefix.into_any_element());
        self
    }

    /// Set a single suffix element after the tabs.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.end_children.push(suffix.into_any_element());
        self
    }

    /// Add a single element before the tabs.
    pub fn start_child(mut self, child: impl IntoElement) -> Self {
        self.start_children.push(child.into_any_element());
        self
    }

    /// Add multiple elements before the tabs.
    pub fn start_children(
        mut self,
        children: impl IntoIterator<Item = impl IntoElement>,
    ) -> Self {
        self.start_children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }

    /// Mutable access to start children.
    pub fn start_children_mut(&mut self) -> &mut SmallVec<[AnyElement; 2]> {
        &mut self.start_children
    }

    /// Add a single element after the tabs.
    pub fn end_child(mut self, child: impl IntoElement) -> Self {
        self.end_children.push(child.into_any_element());
        self
    }

    /// Add multiple elements after the tabs.
    pub fn end_children(
        mut self,
        children: impl IntoIterator<Item = impl IntoElement>,
    ) -> Self {
        self.end_children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }

    /// Mutable access to end children.
    pub fn end_children_mut(&mut self) -> &mut SmallVec<[AnyElement; 2]> {
        &mut self.end_children
    }

    /// Add typed Tab children that inherit the bar's variant, size, and selection.
    pub fn tab_children(mut self, children: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.tab_children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Add a single typed Tab child that inherits the bar's variant, size, and selection.
    pub fn tab_child(mut self, child: impl Into<Tab>) -> Self {
        self.tab_children.push(child.into());
        self
    }

    /// Set the selected index of the TabBar.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Set the last empty space element of the TabBar.
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.last_empty_space = last_empty_space.into_any_element();
        self
    }

    /// Set the on_click callback of the TabBar, the first parameter is the index of the clicked tab.
    ///
    /// When this is set, the children's on_click will be ignored.
    pub fn on_click<F>(mut self, on_click: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(on_click));
        self
    }
}

impl ParentElement for TabBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl Styled for TabBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TabBar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_gap = match self.size {
            Size::Small | Size::XSmall => px(8.),
            Size::Large => px(16.),
            _ => px(12.),
        };
        let (bg, paddings, gap) = match self.variant {
            TabVariant::Tab => {
                let padding = Edges::all(px(0.));
                (cx.theme().colors().tab.bar_background, padding, px(0.))
            }
            TabVariant::Outline => {
                let padding = Edges::all(px(0.));
                (Oklch::transparent_black(), padding, default_gap)
            }
            TabVariant::Pill => {
                let padding = Edges::all(px(0.));
                (Oklch::transparent_black(), padding, px(4.))
            }
            TabVariant::Segmented => {
                let padding_x = match self.size {
                    Size::XSmall => px(2.),
                    Size::Small => px(3.),
                    _ => px(4.),
                };
                let padding = Edges {
                    left: padding_x,
                    right: padding_x,
                    ..Default::default()
                };

                (cx.theme().colors().element_background, padding, px(2.))
            }
            TabVariant::Underline => {
                let gap = match self.size {
                    Size::XSmall => px(10.),
                    Size::Small => px(12.),
                    Size::Large => px(20.),
                    _ => px(16.),
                };

                (Oklch::transparent_black(), Edges::all(px(0.)), gap)
            }
            TabVariant::Workspace => {
                let padding = Edges::all(px(0.));
                (cx.theme().colors().tab.bar_background, padding, px(0.))
            }
        };

        let mut item_labels = Vec::new();
        let selected_index = self.selected_index;
        let on_click = self.on_click.clone();
        let has_end = !self.end_children.is_empty();

        self.base
            .group("tab-bar")
            .relative()
            .flex()
            .items_center()
            .bg(bg)
            .text_color(cx.theme().colors().tab.inactive_foreground)
            .when(
                self.variant == TabVariant::Underline || self.variant == TabVariant::Tab,
                |this| {
                    this.child(
                        div()
                            .id("border-b")
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .size_full()
                            .border_b_1()
                            .border_color(cx.theme().colors().border),
                    )
                },
            )
            .rounded(self.variant.tab_bar_radius(self.size, cx))
            .paddings(paddings)
            .refine_style(&self.style)
            .when(!self.start_children.is_empty(), |this| {
                this.children(self.start_children)
            })
            .child(
                h_flex()
                    .id("tabs")
                    .flex_1()
                    .overflow_x_scroll()
                    .when_some(self.scroll_handle, |this, scroll_handle| {
                        this.track_scroll(&scroll_handle)
                    })
                    .gap(gap)
                    // Typed Tab children with variant/size/selection propagation
                    .children(self.tab_children.into_iter().enumerate().map(|(ix, child)| {
                        item_labels.push((child.label.clone(), child.disabled));
                        let tab_bar_prefix = child.tab_bar_prefix.unwrap_or(true);
                        child
                            .ix(ix)
                            .tab_bar_prefix(tab_bar_prefix)
                            .with_variant(self.variant)
                            .with_size(self.size)
                            .when_some(self.selected_index, |this, selected_ix| {
                                this.selected(selected_ix == ix)
                            })
                            .when_some(self.on_click.clone(), move |this, on_click| {
                                this.on_click(move |_, window, cx| on_click(&ix, window, cx))
                            })
                    }))
                    // Generic AnyElement children (divs, drop targets, etc.)
                    .children(self.children)
                    .when(has_end || self.menu, |this| {
                        this.child(self.last_empty_space)
                    }),
            )
            .when(self.menu, |this| {
                this.child(
                    Button::with_id("more")
                        .xsmall()
                        .ghost()
                        .icon(IconName::ChevronDown)
                        .popup_menu(move |mut this, _, _| {
                            this = this.scrollable(true);
                            for (ix, (label, disabled)) in item_labels.iter().enumerate() {
                                this = this.item(
                                    PopupMenuItem::new(label.clone().unwrap_or_default())
                                        .checked(selected_index == Some(ix))
                                        .disabled(*disabled)
                                        .when_some(on_click.clone(), |this, on_click| {
                                            this.on_click(move |_, window, cx| {
                                                on_click(&ix, window, cx)
                                            })
                                        }),
                                )
                            }

                            this
                        })
                        .anchor(Corner::TopRight),
                )
            })
            .when(!self.end_children.is_empty(), |this| {
                this.children(self.end_children)
            })
    }
}

impl Component for TabBar {
    fn scope() -> ComponentScope {
        ComponentScope::Navigation
    }

    fn name() -> &'static str {
        "TabBar"
    }

    fn description() -> Option<&'static str> {
        Some("A horizontal bar containing tabs for navigation between different views or sections.")
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            crate::v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Variants",
                        vec![
                            single_example(
                                "Tab (default)",
                                TabBar::new("tab_variant")
                                    .tab_child(Tab::new("tab1"))
                                    .tab_child(Tab::new("tab2"))
                                    .tab_child(Tab::new("tab3"))
                                    .selected_index(0)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Pill",
                                TabBar::new("pill_variant")
                                    .pill()
                                    .tab_child(Tab::new("tab1"))
                                    .tab_child(Tab::new("tab2"))
                                    .tab_child(Tab::new("tab3"))
                                    .selected_index(0)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Segmented",
                                TabBar::new("segmented_variant")
                                    .segmented()
                                    .tab_child(Tab::new("tab1"))
                                    .tab_child(Tab::new("tab2"))
                                    .tab_child(Tab::new("tab3"))
                                    .selected_index(0)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Underline",
                                TabBar::new("underline_variant")
                                    .underline()
                                    .tab_child(Tab::new("tab1"))
                                    .tab_child(Tab::new("tab2"))
                                    .tab_child(Tab::new("tab3"))
                                    .selected_index(0)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "With Start and End Children",
                        vec![single_example(
                            "Full TabBar",
                            TabBar::new("full_tab_bar")
                                .start_child(Button::new("start_button", "Start"))
                                .child(Tab::new("tab1"))
                                .child(Tab::new("tab2"))
                                .child(Tab::new("tab3"))
                                .end_child(Button::new("end_button", "End"))
                                .selected_index(0)
                                .into_any_element(),
                        )],
                    ),
                ])
                .into_any_element(),
        )
    }
}
