use inazuma::{
    AnyElement, Div, InteractiveElement, IntoElement, ParentElement, RenderOnce, StyleRefinement,
    Styled, div, prelude::FluentBuilder as _,
};

use crate::{ActiveTheme as _, Collapsible, Selectable, StyledExt};
use super::super::menu::PopupMenuExt;

/// Header for the [`super::Sidebar`]
#[derive(IntoElement)]
pub struct SidebarHeader {
    base: Div,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    selected: bool,
    collapsed: bool,
}

impl SidebarHeader {
    /// Create a new [`SidebarHeader`].
    pub fn new() -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            selected: false,
            collapsed: false,
        }
    }
}

impl Default for SidebarHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl Selectable for SidebarHeader {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Collapsible for SidebarHeader {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl ParentElement for SidebarHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = inazuma::AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SidebarHeader {
    fn style(&mut self) -> &mut inazuma::StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for SidebarHeader {
    fn interactivity(&mut self) -> &mut inazuma::Interactivity {
        self.base.interactivity()
    }
}

impl PopupMenuExt for SidebarHeader {}

impl RenderOnce for SidebarHeader {
    fn render(self, _: &mut inazuma::Window, cx: &mut inazuma::App) -> impl inazuma::IntoElement {
        self.base
            .id("sidebar-header")
            .h_flex()
            .gap_2()
            .p_2()
            .w_full()
            .justify_between()
            .rounded(cx.theme().colors().radius)
            .refine_style(&self.style)
            .hover(|this| {
                this.bg(cx.theme().colors().accent)
                    .text_color(cx.theme().colors().accent_foreground)
            })
            .when(self.selected, |this| {
                this.bg(cx.theme().colors().accent)
                    .text_color(cx.theme().colors().accent_foreground)
            })
            .children(self.children)
    }
}
