use inazuma::{
    Div, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled,
    prelude::FluentBuilder as _,
};

use crate::{ActiveTheme as _, Collapsible, Selectable, h_flex};
use super::super::menu::PopupMenuExt;

/// Footer for the [`super::Sidebar`].
#[derive(IntoElement)]
pub struct SidebarFooter {
    base: Div,
    selected: bool,
    collapsed: bool,
}

impl SidebarFooter {
    /// Create a new [`SidebarFooter`].
    pub fn new() -> Self {
        Self {
            base: h_flex().gap_2().w_full(),
            selected: false,
            collapsed: false,
        }
    }
}

impl Selectable for SidebarFooter {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Collapsible for SidebarFooter {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl ParentElement for SidebarFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = inazuma::AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for SidebarFooter {
    fn style(&mut self) -> &mut inazuma::StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SidebarFooter {
    fn interactivity(&mut self) -> &mut inazuma::Interactivity {
        self.base.interactivity()
    }
}

impl PopupMenuExt for SidebarFooter {}

impl RenderOnce for SidebarFooter {
    fn render(self, _: &mut inazuma::Window, cx: &mut inazuma::App) -> impl inazuma::IntoElement {
        h_flex()
            .id("sidebar-footer")
            .gap_2()
            .p_2()
            .w_full()
            .justify_between()
            .rounded(cx.theme().colors().radius)
            .hover(|this| {
                this.bg(cx.theme().colors().accent)
                    .text_color(cx.theme().colors().accent_foreground)
            })
            .when(self.selected, |this| {
                this.bg(cx.theme().colors().accent)
                    .text_color(cx.theme().colors().accent_foreground)
            })
            .child(self.base)
    }
}
