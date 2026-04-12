use std::rc::Rc;

use inazuma::{
    Context, Corner, DismissEvent, ElementId, Entity, Focusable, InteractiveElement, IntoElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window,
};

use crate::{Button, InteractivePopover, Selectable};
use super::PopupMenu;

/// A trait for attaching a popup menu to any interactive element.
///
/// Use `.popup_menu(|menu, window, cx| menu.item(...))` on a Button or other
/// interactive element to attach an ad-hoc popup menu.
///
/// For a standalone form-style dropdown (label + chevron + menu), use
/// [`crate::DropdownMenu`] instead.
pub trait PopupMenuExt: Styled + Selectable + InteractiveElement + IntoElement + 'static {
    /// Attach a popup menu anchored to the TopLeft corner.
    fn popup_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> PopupMenuPopover<Self> {
        self.popup_menu_with_anchor(Corner::TopLeft, f)
    }

    /// Attach a popup menu anchored to the given corner.
    fn popup_menu_with_anchor(
        mut self,
        anchor: impl Into<Corner>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> PopupMenuPopover<Self> {
        let style = self.style().clone();
        let id = self.interactivity().element_id.clone();

        PopupMenuPopover::new(id.unwrap_or(0.into()), anchor, self, f).trigger_style(style)
    }
}

impl PopupMenuExt for Button {}

#[derive(IntoElement)]
pub struct PopupMenuPopover<T: Selectable + IntoElement + 'static> {
    id: ElementId,
    style: StyleRefinement,
    anchor: Corner,
    trigger: T,
    builder: Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>,
}

impl<T> PopupMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn new(
        id: ElementId,
        anchor: impl Into<Corner>,
        trigger: T,
        builder: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        Self {
            id: SharedString::from(format!("popup-menu:{:?}", id)).into(),
            style: StyleRefinement::default(),
            anchor: anchor.into(),
            trigger,
            builder: Rc::new(builder),
        }
    }

    /// Set the anchor corner for the popup menu popover.
    pub fn anchor(mut self, anchor: impl Into<Corner>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the style refinement for the popup menu trigger.
    fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.style = style;
        self
    }
}

#[derive(Default)]
struct PopupMenuState {
    menu: Option<Entity<PopupMenu>>,
}

impl<T> RenderOnce for PopupMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn render(self, window: &mut Window, cx: &mut inazuma::App) -> impl IntoElement {
        let builder = self.builder.clone();
        let menu_state =
            window.use_keyed_state(self.id.clone(), cx, |_, _| PopupMenuState::default());

        InteractivePopover::new(SharedString::from(format!("popover:{}", self.id)))
            .appearance(false)
            .overlay_closable(false)
            .trigger(self.trigger)
            .trigger_style(self.style)
            .anchor(self.anchor)
            .content(move |_, window, cx| {
                let menu = match menu_state.read(cx).menu.clone() {
                    Some(menu) => menu,
                    None => {
                        let builder = builder.clone();
                        let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                            builder(menu, window, cx)
                        });
                        menu_state.update(cx, |state, _| {
                            state.menu = Some(menu.clone());
                        });
                        menu.focus_handle(cx).focus(window, cx);

                        let popover_state = cx.entity();
                        window
                            .subscribe(&menu, cx, {
                                let menu_state = menu_state.clone();
                                move |_, _: &DismissEvent, window, cx| {
                                    popover_state.update(cx, |state, cx| {
                                        state.dismiss(window, cx);
                                    });
                                    menu_state.update(cx, |state, _| {
                                        state.menu = None;
                                    });
                                }
                            })
                            .detach();

                        menu.clone()
                    }
                };

                menu.clone()
            })
    }
}
