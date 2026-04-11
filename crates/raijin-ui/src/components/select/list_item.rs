use inazuma::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::{ActiveTheme, Disableable, Selectable, Sizable, Size, StyleSized, StyledExt, h_flex};

#[derive(IntoElement)]
pub(super) struct SelectListItem {
    id: ElementId,
    size: Size,
    style: StyleRefinement,
    selected: bool,
    disabled: bool,
    children: Vec<AnyElement>,
}

impl SelectListItem {
    pub fn new(ix: usize) -> Self {
        Self {
            id: ("select-item", ix).into(),
            size: Size::default(),
            style: StyleRefinement::default(),
            selected: false,
            disabled: false,
            children: Vec::new(),
        }
    }
}

impl ParentElement for SelectListItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for SelectListItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for SelectListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for SelectListItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for SelectListItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for SelectListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .id(self.id)
            .relative()
            .gap_x_1()
            .py_1()
            .px_2()
            .rounded(cx.theme().colors().radius)
            .text_base()
            .text_color(cx.theme().colors().foreground)
            .relative()
            .items_center()
            .justify_between()
            .input_text_size(self.size)
            .list_size(self.size)
            .refine_style(&self.style)
            .when(!self.disabled, |this| {
                this.when(!self.selected, |this| {
                    this.hover(|this| this.bg(cx.theme().colors().accent.alpha(0.7)))
                })
            })
            .when(self.selected, |this| this.bg(cx.theme().colors().accent))
            .when(self.disabled, |this| {
                this.text_color(cx.theme().colors().muted_foreground)
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_x_1()
                    .child(div().w_full().children(self.children)),
            )
    }
}
