use inazuma::{
    AnyElement, App, AppContext, Bounds, ClickEvent, Context, DismissEvent, Edges, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, Length, Oklch,
    ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Subscription, Task, WeakEntity, Window, anchored, deferred, div,
    prelude::FluentBuilder, px, rems,
};
use raijin_i18n::t;

use crate::{
    ActiveTheme, Disableable, ElementExt as _, Icon, IconName, IconSize, IndexPath,
    Selectable, Sizable, Size, StyleSized, StyledExt,
    actions::{Cancel, Confirm, SelectDown, SelectUp},
    PopoverGlobalState,
    h_flex,
    clear_button, input_style,
    InteractiveList, ListDelegate, ListState,
    v_flex,
};

use super::traits::{SelectDelegate, SelectItem};

struct SelectListDelegate<D: SelectDelegate + 'static> {
    delegate: D,
    state: WeakEntity<SelectState<D>>,
    selected_index: Option<IndexPath>,
}

impl<D> ListDelegate for SelectListDelegate<D>
where
    D: SelectDelegate + 'static,
{
    type Item = super::list_item::SelectListItem;

    fn sections_count(&self, cx: &App) -> usize {
        self.delegate.sections_count(cx)
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.delegate.items_count(section)
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let state = self.state.upgrade()?.read(cx);
        let Some(item) = self.delegate.section(section) else {
            return None;
        };

        return Some(
            div()
                .py_0p5()
                .px_2()
                .list_size(state.options.size)
                .text_sm()
                .text_color(cx.theme().colors().muted_foreground)
                .child(item),
        );
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = self
            .selected_index
            .map_or(false, |selected_index| selected_index == ix);
        let size = self
            .state
            .upgrade()
            .map_or(Size::Medium, |state| state.read(cx).options.size);

        if let Some(item) = self.delegate.item(ix) {
            let list_item = super::list_item::SelectListItem::new(ix.row)
                .selected(selected)
                .with_size(size)
                .child(div().whitespace_nowrap().child(item.render(window, cx)));
            Some(list_item)
        } else {
            None
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let state = self.state.clone();
        let final_selected_index = state
            .read_with(cx, |this, _| this.final_selected_index)
            .ok()
            .flatten();

        // If the selected index is not the final selected index, we need to restore it.
        let need_restore = if final_selected_index != self.selected_index {
            self.selected_index = final_selected_index;
            true
        } else {
            false
        };

        cx.defer_in(window, move |this, window, cx| {
            if need_restore {
                this.set_selected_index(final_selected_index, window, cx);
            }

            _ = state.update(cx, |this, cx| {
                this.set_open(false, cx);
                this.focus(window, cx);
            });
        });
    }

    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let selected_index = self.selected_index;
        let selected_value = selected_index
            .and_then(|ix| self.delegate.item(ix))
            .map(|item| item.value().clone());
        let state = self.state.clone();

        cx.defer_in(window, move |_, window, cx| {
            _ = state.update(cx, |this, cx| {
                cx.emit(SelectEvent::Confirm(selected_value.clone()));
                this.final_selected_index = selected_index;
                this.selected_value = selected_value;
                this.set_open(false, cx);
                this.focus(window, cx);
            });
        });
    }

    fn perform_search(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.state.upgrade().map_or(Task::ready(()), |state| {
            state.update(cx, |_, cx| self.delegate.perform_search(query, window, cx))
        })
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn render_empty(
        &mut self,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        if let Some(empty) = self
            .state
            .upgrade()
            .and_then(|state| state.read(cx).empty.as_ref())
        {
            empty(window, cx).into_any_element()
        } else {
            h_flex()
                .justify_center()
                .py_6()
                .text_color(cx.theme().colors().muted_foreground.opacity(0.6))
                .child(Icon::new(IconName::Envelope).size(IconSize::XLarge))
                .into_any_element()
        }
    }
}

/// Events emitted by the [`SelectState`].
pub enum SelectEvent<D: SelectDelegate + 'static> {
    Confirm(Option<<D::Item as SelectItem>::Value>),
}

pub(super) struct SelectOptions {
    pub(super) style: StyleRefinement,
    pub(super) size: Size,
    pub(super) icon: Option<Icon>,
    pub(super) cleanable: bool,
    pub(super) placeholder: Option<SharedString>,
    pub(super) title_prefix: Option<SharedString>,
    pub(super) search_placeholder: Option<SharedString>,
    pub(super) empty: Option<AnyElement>,
    pub(super) menu_width: Length,
    pub(super) menu_max_h: Length,
    pub(super) disabled: bool,
    pub(super) appearance: bool,
}

impl Default for SelectOptions {
    fn default() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: Size::default(),
            icon: None,
            cleanable: false,
            placeholder: None,
            title_prefix: None,
            empty: None,
            menu_width: Length::Auto,
            menu_max_h: rems(20.).into(),
            disabled: false,
            appearance: true,
            search_placeholder: None,
        }
    }
}

/// State of the [`Select`](super::Select).
pub struct SelectState<D: SelectDelegate + 'static> {
    focus_handle: FocusHandle,
    pub(super) options: SelectOptions,
    searchable: bool,
    list: Entity<ListState<SelectListDelegate<D>>>,
    empty: Option<Box<dyn Fn(&Window, &App) -> AnyElement>>,
    /// Store the bounds of the input
    bounds: Bounds<Pixels>,
    open: bool,
    selected_value: Option<<D::Item as SelectItem>::Value>,
    final_selected_index: Option<IndexPath>,
    _subscriptions: Vec<Subscription>,
}

impl<D> SelectState<D>
where
    D: SelectDelegate + 'static,
{
    /// Create a new Select state.
    pub fn new(
        delegate: D,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let delegate = SelectListDelegate {
            delegate,
            state: cx.entity().downgrade(),
            selected_index,
        };

        let list = cx.new(|cx| ListState::new(delegate, window, cx).reset_on_cancel(false));
        let list_focus_handle = list.read(cx).focus_handle.clone();
        let list_search_focus_handle = list.read(cx).query_input.focus_handle(cx);

        let _subscriptions = vec![
            cx.on_blur(&list_focus_handle, window, Self::on_blur),
            cx.on_blur(&list_search_focus_handle, window, Self::on_blur),
            cx.on_blur(&focus_handle, window, Self::on_blur),
        ];

        let mut this = Self {
            focus_handle,
            options: SelectOptions::default(),
            searchable: false,
            list,
            selected_value: None,
            open: false,
            bounds: Bounds::default(),
            empty: None,
            final_selected_index: None,
            _subscriptions,
        };
        this.set_selected_index(selected_index, window, cx);
        this
    }

    /// Sets whether the dropdown menu is searchable, default is `false`.
    ///
    /// When `true`, there will be a search input at the top of the dropdown menu.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Set the selected index for the select.
    pub fn set_selected_index(
        &mut self,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            list._set_selected_index(selected_index, window, cx);
        });
        self.final_selected_index = selected_index;
        self.update_selected_value(window, cx);
    }

    /// Set selected value for the select.
    ///
    /// This method will to get position from delegate and set selected index.
    ///
    /// If the value is not found, the None will be sets.
    pub fn set_selected_value(
        &mut self,
        selected_value: &<D::Item as SelectItem>::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        <<D as SelectDelegate>::Item as SelectItem>::Value: PartialEq,
    {
        let delegate = self.list.read(cx).delegate();
        let selected_index = delegate.delegate.position(selected_value);
        self.set_selected_index(selected_index, window, cx);
    }

    /// Set the items for the select state.
    pub fn set_items(&mut self, items: D, _: &mut Window, cx: &mut Context<Self>)
    where
        D: SelectDelegate + 'static,
    {
        self.list.update(cx, |list, _| {
            list.delegate_mut().delegate = items;
        });
    }

    /// Get the selected index of the select.
    pub fn selected_index(&self, cx: &App) -> Option<IndexPath> {
        self.list.read(cx).selected_index()
    }

    /// Get the selected value of the select.
    pub fn selected_value(&self) -> Option<&<D::Item as SelectItem>::Value> {
        self.selected_value.as_ref()
    }

    /// Focus the select input.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    fn update_selected_value(&mut self, _: &Window, cx: &App) {
        self.selected_value = self
            .selected_index(cx)
            .and_then(|ix| self.list.read(cx).delegate().delegate.item(ix))
            .map(|item| item.value().clone());
    }

    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // When the select and dropdown menu are both not focused, close the dropdown menu.
        if self.list.read(cx).is_focused(window, cx) || self.focus_handle.is_focused(window) {
            return;
        }

        // If the selected index is not the final selected index, we need to restore it.
        let final_selected_index = self.final_selected_index;
        let selected_index = self.selected_index(cx);
        if final_selected_index != selected_index {
            self.list.update(cx, |list, cx| {
                list.set_selected_index(self.final_selected_index, window, cx);
            });
        }

        self.set_open(false, cx);
        cx.notify();
    }

    pub(super) fn up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.set_open(true, cx);
        }

        self.list.focus_handle(cx).focus(window, cx);
        cx.propagate();
    }

    pub(super) fn down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.set_open(true, cx);
        }

        self.list.focus_handle(cx).focus(window, cx);
        cx.propagate();
    }

    pub(super) fn enter(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        // Propagate the event to the parent view, for example to the Dialog to support ENTER to confirm.
        cx.propagate();

        if !self.open {
            self.set_open(true, cx);
            cx.notify();
        }

        self.list.focus_handle(cx).focus(window, cx);
    }

    fn toggle_menu(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();

        self.set_open(!self.open, cx);
        if self.open {
            self.list.focus_handle(cx).focus(window, cx);
        }
        cx.notify();
    }

    pub(super) fn escape(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
        }

        self.set_open(false, cx);
        cx.notify();
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.open = open;
        if self.open {
            PopoverGlobalState::global_mut(cx).register_deferred_popover(&self.focus_handle)
        } else {
            PopoverGlobalState::global_mut(cx).unregister_deferred_popover(&self.focus_handle)
        }
        cx.notify();
    }

    fn clean(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.set_selected_index(None, window, cx);
        cx.emit(SelectEvent::Confirm(None));
    }

    /// Returns the title element for the select input.
    fn display_title(&mut self, _: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let default_title = div().text_color(cx.theme().colors().muted_foreground).child(
            self.options
                .placeholder
                .clone()
                .unwrap_or_else(|| t!("Select.placeholder").into()),
        );

        let Some(selected_index) = &self.selected_index(cx) else {
            return default_title;
        };

        let Some(title) = self
            .list
            .read(cx)
            .delegate()
            .delegate
            .item(*selected_index)
            .map(|item| {
                if let Some(el) = item.display_title() {
                    el
                } else {
                    if let Some(prefix) = self.options.title_prefix.as_ref() {
                        format!("{}{}", prefix, item.title()).into_any_element()
                    } else {
                        item.title().into_any_element()
                    }
                }
            })
        else {
            return default_title;
        };

        div()
            .when(self.options.disabled, |this| {
                this.text_color(cx.theme().colors().muted_foreground)
            })
            .child(title)
    }
}

impl<D> Render for SelectState<D>
where
    D: SelectDelegate + 'static,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let searchable = self.searchable;
        let is_focused = self.focus_handle.is_focused(window);
        let show_clean = self.options.cleanable && self.selected_index(cx).is_some();
        let bounds = self.bounds;
        let allow_open = !(self.open || self.options.disabled);
        let outline_visible = self.open || is_focused && !self.options.disabled;
        let popup_radius = cx.theme().colors().radius.min(px(8.));

        let (bg, fg) = input_style(self.options.disabled, cx);

        self.list
            .update(cx, |list, cx| list.set_searchable(searchable, cx));

        div()
            .size_full()
            .relative()
            .child(
                div()
                    .id("input")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_1()
                    .border_color(Oklch::transparent_black())
                    .when(self.options.appearance, |this| {
                        this.bg(bg)
                            .text_color(fg)
                            .when(self.options.disabled, |this| this.opacity(0.5))
                            .border_color(cx.theme().colors().input)
                            .rounded(cx.theme().colors().radius)
                            .when(cx.theme().is_dark(), |this| this.shadow_xs())
                    })
                    .map(|this| {
                        if self.options.disabled {
                            this.shadow_none()
                        } else {
                            this
                        }
                    })
                    .overflow_hidden()
                    .input_size(self.options.size)
                    .input_text_size(self.options.size)
                    .refine_style(&self.options.style)
                    .when(outline_visible, |this| this.focused_border(cx))
                    .when(allow_open, |this| {
                        this.on_click(cx.listener(Self::toggle_menu))
                    })
                    .child(
                        h_flex()
                            .id("inner")
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_1()
                            .child(
                                div()
                                    .id("title")
                                    .w_full()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .truncate()
                                    .child(self.display_title(window, cx)),
                            )
                            .when(show_clean, |this| {
                                this.child(clear_button(cx).map(|this| {
                                    if self.options.disabled {
                                        this.disabled(true)
                                    } else {
                                        this.on_click(cx.listener(Self::clean))
                                    }
                                }))
                            })
                            .when(!show_clean, |this| {
                                let icon = match self.options.icon.clone() {
                                    Some(icon) => icon,
                                    None => Icon::new(IconName::ChevronDown),
                                };

                                this.child(icon.xsmall().text_color(cx.theme().colors().muted_foreground))
                            }),
                    )
                    .on_prepaint({
                        let state = cx.entity();
                        move |bounds, _, cx| state.update(cx, |r, _| r.bounds = bounds)
                    }),
            )
            .when(self.open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .occlude()
                                .map(|this| match self.options.menu_width {
                                    Length::Auto => this.w(bounds.size.width + px(2.)),
                                    Length::Definite(w) => this.w(w),
                                })
                                .child(
                                    v_flex()
                                        .occlude()
                                        .mt_1p5()
                                        .bg(cx.theme().colors().background)
                                        .border_1()
                                        .border_color(cx.theme().colors().border)
                                        .rounded(popup_radius)
                                        .shadow_md()
                                        .child(
                                            InteractiveList::new(&self.list)
                                                .when_some(
                                                    self.options.search_placeholder.clone(),
                                                    |this, placeholder| {
                                                        this.search_placeholder(placeholder)
                                                    },
                                                )
                                                .with_size(self.options.size)
                                                .max_h(self.options.menu_max_h)
                                                .paddings(Edges::all(px(4.))),
                                        ),
                                )
                                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                                    this.escape(&Cancel, window, cx);
                                })),
                        ),
                    )
                    .with_priority(1),
                )
            })
    }
}

impl<D> EventEmitter<SelectEvent<D>> for SelectState<D> where D: SelectDelegate + 'static {}
impl<D> EventEmitter<DismissEvent> for SelectState<D> where D: SelectDelegate + 'static {}
impl<D> Focusable for SelectState<D>
where
    D: SelectDelegate,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.open {
            self.list.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
