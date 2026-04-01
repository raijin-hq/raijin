use inazuma::{
    App, ElementId, Entity, Focusable, InteractiveElement, IntoElement, Length,
    ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder,
};

use crate::{Icon, Sizable, Size};

use super::state::{SelectOptions, SelectState};
use super::traits::SelectDelegate;

const CONTEXT: &str = "Select";

/// A Select element.
#[derive(IntoElement)]
pub struct Select<D: SelectDelegate + 'static> {
    id: ElementId,
    state: Entity<SelectState<D>>,
    options: SelectOptions,
}

impl<D> Select<D>
where
    D: SelectDelegate + 'static,
{
    pub fn new(state: &Entity<SelectState<D>>) -> Self {
        Self {
            id: ("select", state.entity_id()).into(),
            state: state.clone(),
            options: SelectOptions::default(),
        }
    }

    /// Set the width of the dropdown menu, default: Length::Auto
    pub fn menu_width(mut self, width: impl Into<Length>) -> Self {
        self.options.menu_width = width.into();
        self
    }

    /// Set the max height of the dropdown menu, default: 20rem
    pub fn menu_max_h(mut self, max_h: impl Into<Length>) -> Self {
        self.options.menu_max_h = max_h.into();
        self
    }

    /// Set the placeholder for display when select value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }

    /// Set the right icon for the select input, instead of the default arrow icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.options.icon = Some(icon.into());
        self
    }

    /// Set title prefix for the select.
    ///
    /// e.g.: Country: United States
    ///
    /// You should set the label is `Country: `
    pub fn title_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.options.title_prefix = Some(prefix.into());
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.options.cleanable = cleanable;
        self
    }

    /// Sets the placeholder text for the search input.
    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.search_placeholder = Some(placeholder.into());
        self
    }

    /// Set the disable state for the select.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.options.disabled = disabled;
        self
    }

    /// Set the element to display when the select list is empty.
    pub fn empty(mut self, el: impl IntoElement) -> Self {
        self.options.empty = Some(el.into_any_element());
        self
    }

    /// Set the appearance of the select, if false the select input will no border, background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.options.appearance = appearance;
        self
    }
}

impl<D> Sizable for Select<D>
where
    D: SelectDelegate + 'static,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.options.size = size.into();
        self
    }
}

impl<D> Styled for Select<D>
where
    D: SelectDelegate,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.options.style
    }
}

impl<D> RenderOnce for Select<D>
where
    D: SelectDelegate + 'static,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.options.disabled;
        let focus_handle = self.state.focus_handle(cx);
        // If the size has change, set size to self.list, to change the QueryInput size.
        self.state.update(cx, |this, _| {
            this.options = self.options;
        });

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .when(!disabled, |this| {
                this.track_focus(&focus_handle.tab_stop(true))
            })
            .on_action(window.listener_for(&self.state, SelectState::up))
            .on_action(window.listener_for(&self.state, SelectState::down))
            .on_action(window.listener_for(&self.state, SelectState::enter))
            .on_action(window.listener_for(&self.state, SelectState::escape))
            .size_full()
            .child(self.state)
    }
}
