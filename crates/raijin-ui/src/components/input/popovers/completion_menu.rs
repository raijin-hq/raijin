use std::rc::Rc;

use inazuma::{
    Action, AnyElement, App, AppContext, Context, DismissEvent, Empty, Entity, EventEmitter,
    HighlightStyle, InteractiveElement as _, IntoElement, ParentElement, Pixels, Point,
    Render, RenderOnce, SharedString, Styled, StyledText, Subscription, Window, deferred, div,
    prelude::FluentBuilder, px, relative,
};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit};

const MAX_MENU_WIDTH: Pixels = px(300.);
const MAX_MENU_HEIGHT: Pixels = px(260.);
const POPOVER_GAP: Pixels = px(6.);
/// Dark gray popover background matching Warp's completion menu.
const POPOVER_BG: inazuma::Oklch = inazuma::Oklch { l: 0.248, c: 0.0, h: 0.0, a: 1.0 };
#[allow(clippy::approx_constant)]
const POPOVER_BORDER: inazuma::Oklch = inazuma::Oklch { l: 0.318, c: 0.0, h: 0.0, a: 1.0 };

use crate::{
    ActiveTheme, Icon, IconName, IndexPath, InteractiveList, ListDelegate, ListEvent, ListState, Selectable,
    actions, h_flex,
    input::{
        self, InputState, RopeExt,
    },
};

/// Map CompletionItemKind to an appropriate icon.
fn icon_for_kind(kind: Option<CompletionItemKind>) -> IconName {
    match kind {
        Some(CompletionItemKind::FOLDER) => IconName::Folder,
        Some(CompletionItemKind::FILE) => IconName::File,
        Some(CompletionItemKind::FUNCTION) => IconName::Terminal,
        Some(CompletionItemKind::VARIABLE) => IconName::Terminal,
        Some(CompletionItemKind::KEYWORD) => IconName::Terminal,
        Some(CompletionItemKind::REFERENCE) => IconName::GitBranch,
        _ => IconName::Terminal,
    }
}

struct ContextMenuDelegate {
    query: SharedString,
    menu: Entity<CompletionMenu>,
    items: Vec<Rc<CompletionItem>>,
    selected_ix: usize,
}

impl ContextMenuDelegate {
    fn set_items(&mut self, items: Vec<CompletionItem>) {
        self.items = items.into_iter().map(Rc::new).collect();
        self.selected_ix = 0;
    }

    fn selected_item(&self) -> Option<&Rc<CompletionItem>> {
        self.items.get(self.selected_ix)
    }
}

#[derive(IntoElement)]
struct CompletionMenuItem {
    ix: usize,
    item: Rc<CompletionItem>,
    children: Vec<AnyElement>,
    selected: bool,
    highlight_prefix: SharedString,
}

impl CompletionMenuItem {
    fn new(ix: usize, item: Rc<CompletionItem>) -> Self {
        Self {
            ix,
            item,
            children: vec![],
            selected: false,
            highlight_prefix: "".into(),
        }
    }

    fn highlight_prefix(mut self, s: impl Into<SharedString>) -> Self {
        self.highlight_prefix = s.into();
        self
    }
}
impl Selectable for CompletionMenuItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl ParentElement for CompletionMenuItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl RenderOnce for CompletionMenuItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let item = self.item;

        let deprecated = item.deprecated.unwrap_or(false);
        let matched_len = item
            .filter_text
            .as_ref()
            .map(|s| s.len())
            .unwrap_or(self.highlight_prefix.len())
            .min(item.label.len());

        let highlights = vec![(
            0..matched_len,
            HighlightStyle {
                color: Some(cx.theme().colors().terminal.ansi.blue),
                ..Default::default()
            },
        )];

        let icon = icon_for_kind(item.kind);

        h_flex()
            .id(self.ix)
            .w_full()
            .gap_1p5()
            .px_1p5()
            .py_0p5()
            .text_xs()
            .line_height(relative(1.6))
            .rounded_md()
            .when(deprecated, |this| this.line_through())
            .hover(|this| this.bg(inazuma::Oklch::white().opacity(0.06)))
            .when(self.selected, |this| {
                // Brand color #7FE6EF with 25% opacity
                this.bg(inazuma::hsla(195. / 360., 1.0, 0.5, 0.25))
            })
            .child(
                Icon::new(icon)
                    .text_color(cx.theme().colors().muted_foreground),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .child(StyledText::new(item.label.clone()).with_highlights(highlights)),
            )
            .when(item.detail.is_some(), |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .text_color(cx.theme().colors().muted_foreground)
                        .child(item.detail.as_deref().unwrap_or("").to_string()),
                )
            })
            .children(self.children)
    }
}

impl EventEmitter<DismissEvent> for ContextMenuDelegate {}

impl ListDelegate for ContextMenuDelegate {
    type Item = CompletionMenuItem;

    fn items_count(&self, _: usize, _: &inazuma::App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        ix: crate::IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let item = self.items.get(ix.row)?;
        Some(CompletionMenuItem::new(ix.row, item.clone()).highlight_prefix(self.query.clone()))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<crate::IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_ix = ix.map(|i| i.row).unwrap_or(0);
        cx.notify();
    }

    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let Some(item) = self.selected_item() else {
            return;
        };

        self.menu.update(cx, |this, cx| {
            this.select_item(&item, window, cx);
        });
    }
}

/// A context menu for code completions and code actions.
pub struct CompletionMenu {
    offset: usize,
    editor: Entity<InputState>,
    list: Entity<ListState<ContextMenuDelegate>>,
    open: bool,

    /// The offset of the first character that triggered the completion.
    pub(crate) trigger_start_offset: Option<usize>,
    query: SharedString,
    /// Frozen origin from when the menu was first shown. Stays stable during navigation.
    frozen_origin: Option<Point<Pixels>>,
    _subscriptions: Vec<Subscription>,
}

impl CompletionMenu {
    /// Creates a new `CompletionMenu` with the given offset and completion items.
    ///
    /// NOTE: This element should not call from InputState::new, unless that will stack overflow.
    pub(crate) fn new(
        editor: Entity<InputState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let view = cx.entity();
            let menu = ContextMenuDelegate {
                query: SharedString::default(),
                menu: view,
                items: vec![],
                selected_ix: 0,
            };

            let list = cx.new(|cx| ListState::new(menu, window, cx));

            let _subscriptions =
                vec![
                    cx.subscribe(&list, |this: &mut Self, _, ev: &ListEvent, cx| {
                        match ev {
                            ListEvent::Confirm(_) => {
                                this.hide(cx);
                            }
                            _ => {}
                        }
                        cx.notify();
                    }),
                ];

            Self {
                offset: 0,
                editor,
                list,
                open: false,
                trigger_start_offset: None,
                query: SharedString::default(),
                frozen_origin: None,
                _subscriptions,
            }
        })
    }

    fn select_item(&mut self, item: &CompletionItem, window: &mut Window, cx: &mut Context<Self>) {
        let item = item.clone();
        let mut range = self.trigger_start_offset.unwrap_or(self.offset)..self.offset;

        let editor = self.editor.clone();

        cx.spawn_in(window, async move |_, cx| {
            editor.update_in(cx, |editor, window, cx| {
                editor.completion_inserting = true;

                let mut new_text = item.label.clone();
                if let Some(text_edit) = item.text_edit.as_ref() {
                    match text_edit {
                        CompletionTextEdit::Edit(edit) => {
                            new_text = edit.new_text.clone();
                            range.start = editor.text.position_to_offset(&edit.range.start);
                            range.end = editor.text.position_to_offset(&edit.range.end);
                        }
                        CompletionTextEdit::InsertAndReplace(edit) => {
                            new_text = edit.new_text.clone();
                            range.start = editor.text.position_to_offset(&edit.replace.start);
                            range.end = editor.text.position_to_offset(&edit.replace.end);
                        }
                    }
                } else if let Some(insert_text) = item.insert_text.clone() {
                    new_text = insert_text;
                    // Replace only the typed portion (after trigger character),
                    // not the trigger character itself (e.g. the space).
                    // Find the start of the actual token by skipping whitespace
                    // after trigger_start_offset.
                    let text_str = editor.text.to_string();
                    let token_start = text_str[range.start..]
                        .find(|c: char| !c.is_whitespace())
                        .map(|i| range.start + i)
                        .unwrap_or(range.end);
                    range.start = token_start;
                }

                let insert_start = range.start;
                editor.replace_text_in_range_silent(
                    Some(editor.range_to_utf16(&range)),
                    &new_text,
                    window,
                    cx,
                );

                editor.completion_inserted_range = Some(insert_start..insert_start + new_text.len());
                editor.completion_inserting = false;

                // FIXME: Input not get the focus
                editor.focus(window, cx);
            })
        })
        .detach();

        self.hide(cx);
    }

    pub(crate) fn handle_action(
        &mut self,
        action: Box<dyn Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.open {
            return false;
        }

        cx.propagate();
        if action.partial_eq(&input::Enter { secondary: false }) {
            self.on_action_enter(window, cx);
        } else if action.partial_eq(&input::Escape) {
            self.on_action_escape(window, cx);
        } else if action.partial_eq(&input::MoveUp) {
            self.on_action_up(window, cx);
        } else if action.partial_eq(&input::MoveDown) {
            self.on_action_down(window, cx);
        } else {
            return false;
        }

        true
    }

    fn on_action_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm(window, cx);
    }

    /// Confirm the currently selected item (used by Enter and Tab).
    pub(crate) fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.list.read(cx).delegate().selected_item().cloned() else {
            return;
        };
        self.select_item(&item, window, cx);
    }

    fn on_action_escape(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.hide(cx);
    }

    fn on_action_up(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |this, cx| {
            this.on_action_select_prev(&actions::SelectUp, window, cx)
        });
        self.apply_selected_to_editor(window, cx);
    }

    fn on_action_down(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |this, cx| {
            this.on_action_select_next(&actions::SelectDown, window, cx)
        });
        self.apply_selected_to_editor(window, cx);
    }

    /// Write the currently selected completion item into the editor input.
    fn apply_selected_to_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.list.read(cx).delegate().selected_item().cloned() else {
            return;
        };
        let Some(start_offset) = self.trigger_start_offset else {
            return;
        };

        let new_text = item.insert_text.clone()
            .unwrap_or_else(|| item.label.clone());

        let editor = self.editor.clone();
        let offset = self.offset;
        let menu = cx.entity().clone();

        cx.spawn_in(window, async move |_, cx| {
            editor.update_in(cx, |editor, window, cx| {
                let text_str = editor.text.to_string();
                let token_start = text_str[start_offset..]
                    .find(|c: char| !c.is_whitespace())
                    .map(|i| start_offset + i)
                    .unwrap_or(offset);

                let range = editor.range_to_utf16(&(token_start..offset));
                editor.completion_inserting = true;
                editor.replace_text_in_range_silent(Some(range), &new_text, window, cx);

                editor.completion_inserted_range = Some(token_start..token_start + new_text.len());
                editor.completion_inserting = false;
                let new_offset = editor.cursor();

                menu.update(cx, |menu, _| {
                    menu.offset = new_offset;
                });
            })
        })
        .detach();
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Hide the completion menu and reset the trigger start offset.
    pub(crate) fn hide(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.trigger_start_offset = None;
        self.frozen_origin = None;
        cx.notify();
    }

    /// Sets the trigger start offset if it is not already set.
    pub(crate) fn update_query(&mut self, start_offset: usize, query: impl Into<SharedString>) {
        if self.trigger_start_offset.is_none() {
            self.trigger_start_offset = Some(start_offset);
        }
        self.query = query.into();
    }

    pub(crate) fn show(
        &mut self,
        offset: usize,
        items: impl Into<Vec<CompletionItem>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items = items.into();
        self.offset = offset;
        self.open = true;
        self.list.update(cx, |this, cx| {
            let longest_ix = items
                .iter()
                .enumerate()
                .max_by_key(|(_, item)| {
                    item.label.len() + item.detail.as_ref().map(|d| d.len()).unwrap_or(0)
                })
                .map(|(ix, _)| ix)
                .unwrap_or(0);

            this.delegate_mut().query = self.query.clone();
            this.delegate_mut().set_items(items);
            this.set_selected_index(None, window, cx);
            this.set_item_to_measure_index(IndexPath::new(longest_ix), window, cx);
        });

        cx.notify();
    }

    fn origin(&mut self, cx: &App) -> Option<Point<Pixels>> {
        // Return frozen origin if already set (stable during navigation)
        if let Some(origin) = self.frozen_origin {
            return Some(origin);
        }

        let editor = self.editor.read(cx);
        let Some(last_layout) = editor.last_layout.as_ref() else {
            return None;
        };
        let Some(cursor_origin) = last_layout.cursor_bounds.map(|b| b.origin) else {
            return None;
        };

        let scroll_origin = self.editor.read(cx).scroll_handle.offset();

        let origin = scroll_origin + cursor_origin - editor.input_bounds.origin
            + Point::new(px(4.), last_layout.line_height - px(2.));

        // Freeze origin on first call so menu stays stable
        self.frozen_origin = Some(origin);
        Some(origin)
    }
}

impl Render for CompletionMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return Empty.into_any_element();
        }

        if self.list.read(cx).delegate().items.is_empty() {
            self.open = false;
            return Empty.into_any_element();
        }

        let Some(pos) = self.origin(cx) else {
            return Empty.into_any_element();
        };

        let has_selection = self.list.read(cx).selected_index().is_some();
        let delegate = self.list.read(cx).delegate();
        let selected_ix = delegate.selected_ix;
        let selected_documentation = if has_selection {
            delegate
                .selected_item()
                .and_then(|item| item.documentation.clone())
        } else {
            None
        };

        let window_width = window.bounds().size.width;

        let menu_width = MAX_MENU_WIDTH.min(window_width - px(16.));
        let menu_x = pos.x.max(px(0.));

        // Menu opens above the cursor (terminal input is always at the bottom)
        let editor = self.editor.read(cx);
        let line_height = editor
            .last_layout
            .as_ref()
            .map(|l| l.line_height)
            .unwrap_or(px(20.));
        let menu_y = pos.y - line_height - MAX_MENU_HEIGHT - POPOVER_GAP;

        // Use exact measured item height from the list's layout cache
        let item_height = self.list.read(cx).measured_item_height();
        let doc_panel = selected_documentation.map(|documentation| {
            let doc = match documentation {
                lsp_types::Documentation::String(s) => s.clone(),
                lsp_types::Documentation::MarkupContent(mc) => mc.value.clone(),
            };
            let lines: Vec<&str> = doc.split('\n').collect();
            let title = lines.first().copied().unwrap_or("");
            let subtitle = lines.get(1).copied().unwrap_or("");

            // Visual position = absolute position minus scroll offset
            let list = self.list.read(cx);
            let scroll_y = list.scroll_handle().offset().y;
            let content_height = list.content_height();
            let visual_top = (item_height * (selected_ix as f32)) + scroll_y;
            let menu_height = content_height.min(MAX_MENU_HEIGHT);
            // If doc panel would overflow menu bottom, anchor to bottom instead
            let use_bottom = visual_top + px(60.) > menu_height;

            div()
                .absolute()
                .left(menu_width + POPOVER_GAP)
                .when(!use_bottom, |this| this.top(visual_top.max(px(0.))))
                .when(use_bottom, |this| this.bottom(px(0.)))
                .w(px(200.))
                .bg(POPOVER_BG)
                .text_color(cx.theme().colors().popover_foreground)
                .border_1()
                .border_color(POPOVER_BORDER)
                .shadow_lg()
                .rounded_lg()
                .p_2()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(inazuma::FontWeight::MEDIUM)
                        .child(title.to_string()),
                )
                .when(!subtitle.is_empty(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().colors().muted_foreground)
                            .child(subtitle.to_string()),
                    )
                })
        });

        deferred(
            div()
                .absolute()
                .left(menu_x)
                .top(menu_y)
                .h(MAX_MENU_HEIGHT + POPOVER_GAP)
                .flex()
                .flex_col()
                .justify_end()
                .child(
                    div()
                        .relative()
                        .child(
                            div()
                                .id("completion-menu")
                                .flex_none()
                                .occlude()
                                .w(menu_width)
                                .bg(POPOVER_BG)
                                .text_color(cx.theme().colors().popover_foreground)
                                .border_1()
                                .border_color(POPOVER_BORDER)
                                .shadow_lg()
                                .rounded_lg()
                                .p_1()
                                .child(div().max_h(MAX_MENU_HEIGHT).child(InteractiveList::new(&self.list))),
                        )
                        .children(doc_panel)
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.hide(cx);
                        })),
                ),
        )
        .into_any_element()
    }
}
