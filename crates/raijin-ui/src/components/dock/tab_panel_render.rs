use inazuma::{
    App, AppContext, Context, Corner, DismissEvent, Empty, EventEmitter, Focusable,
    InteractiveElement as _, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px, relative, rems,
};
use raijin_i18n::t;

use crate::{
    ActiveTheme, IconName, Placement, Selectable, Sizable, Tooltip,
    Button, ButtonCommon as _, ButtonVariants as _,
    h_flex,
    Tab, TabBar,
    v_flex,
};

use super::{
    ClosePanel, DockPlacement, Panel, PanelEvent, PanelStyle, ToggleZoom,
};
use super::tab_panel::{DragPanel, TabPanel, TabState};

impl TabPanel {
    pub(super) fn render_toolbar(
        &mut self,
        state: &TabState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.collapsed {
            return div();
        }

        let zoomed = self.zoomed;
        let view = cx.entity().clone();
        let zoomable_toolbar_visible = state.zoomable.map_or(false, |v| v.toolbar_visible());

        h_flex()
            .gap_1()
            .occlude()
            .when_some(self.toolbar_buttons(window, cx), |this, buttons| {
                this.children(
                    buttons
                        .into_iter()
                        .map(|btn| btn.xsmall().ghost().tab_stop(false)),
                )
            })
            .map(|this| {
                let value = if zoomed {
                    Some(("zoom-out", IconName::Minimize, t!("Dock.Zoom Out")))
                } else if zoomable_toolbar_visible {
                    Some(("zoom-in", IconName::Maximize, t!("Dock.Zoom In")))
                } else {
                    None
                };

                if let Some((id, icon, tooltip)) = value {
                    this.child(
                        Button::with_id(id)
                            .icon(icon)
                            .xsmall()
                            .ghost()
                            .tab_stop(false)
                            .tooltip_with_action(tooltip, &ToggleZoom, None)
                            .when(zoomed, |this| this.selected(true))
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.on_action_toggle_zoom(&ToggleZoom, window, cx)
                            })),
                    )
                } else {
                    this
                }
            })
            .child(
                Button::with_id("menu")
                    .icon(IconName::Ellipsis)
                    .xsmall()
                    .ghost()
                    .tab_stop(false)
                    .dropdown_menu({
                        let zoomable = state.zoomable.map_or(false, |v| v.menu_visible());
                        let closable = state.closable;

                        move |menu, window, cx| {
                            view.update(cx, |this, cx| {
                                this.dropdown_menu(menu, window, cx)
                                    .separator()
                                    .menu_with_disabled(
                                        if zoomed {
                                            t!("Dock.Zoom Out")
                                        } else {
                                            t!("Dock.Zoom In")
                                        },
                                        Box::new(ToggleZoom),
                                        !zoomable,
                                    )
                                    .when(closable, |this| {
                                        this.separator()
                                            .menu(t!("Dock.Close"), Box::new(ClosePanel))
                                    })
                            })
                        }
                    })
                    .anchor(Corner::TopRight),
            )
    }

    pub(super) fn render_dock_toggle_button(
        &self,
        placement: DockPlacement,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Button> {
        if self.zoomed {
            return None;
        }

        let dock_area = self.dock_area.upgrade()?.read(cx);
        if !dock_area.toggle_button_visible {
            return None;
        }
        if !dock_area.is_dock_collapsible(placement, cx) {
            return None;
        }

        let view_entity_id = cx.entity().entity_id();
        let toggle_button_panels = dock_area.toggle_button_panels;

        // Check if current TabPanel's entity_id matches the one stored in DockArea for this placement
        if !match placement {
            DockPlacement::Left => {
                dock_area.left_dock.is_some() && toggle_button_panels.left == Some(view_entity_id)
            }
            DockPlacement::Right => {
                dock_area.right_dock.is_some() && toggle_button_panels.right == Some(view_entity_id)
            }
            DockPlacement::Bottom => {
                dock_area.bottom_dock.is_some()
                    && toggle_button_panels.bottom == Some(view_entity_id)
            }
            DockPlacement::Center => unreachable!(),
        } {
            return None;
        }

        let is_open = dock_area.is_dock_open(placement, cx);

        let icon = match placement {
            DockPlacement::Left => {
                if is_open {
                    IconName::ThreadsSidebarLeftClosed
                } else {
                    IconName::ThreadsSidebarLeftOpen
                }
            }
            DockPlacement::Right => {
                if is_open {
                    IconName::ThreadsSidebarRightClosed
                } else {
                    IconName::ThreadsSidebarRightOpen
                }
            }
            DockPlacement::Bottom => {
                if is_open {
                    IconName::ExpandDown
                } else {
                    IconName::ExpandUp
                }
            }
            DockPlacement::Center => unreachable!(),
        };

        Some(
            Button::with_id(inazuma::SharedString::from(format!("toggle-dock:{:?}", placement)))
                .icon(icon)
                .xsmall()
                .ghost()
                .tab_stop(false)
                .tooltip(Tooltip::text(
                    if is_open { t!("Dock.Collapse") } else { t!("Dock.Expand") }
                ))
                .on_click(cx.listener({
                    let dock_area = self.dock_area.clone();
                    move |_, _, window, cx| {
                        _ = dock_area.update(cx, |dock_area, cx| {
                            dock_area.toggle_dock(placement, window, cx);
                        });
                    }
                })),
        )
    }

    pub(super) fn render_title_bar(
        &mut self,
        state: &TabState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();

        let Some(dock_area) = self.dock_area.upgrade() else {
            return div().into_any_element();
        };

        let left_dock_button = self.render_dock_toggle_button(DockPlacement::Left, window, cx);
        let bottom_dock_button = self.render_dock_toggle_button(DockPlacement::Bottom, window, cx);
        let right_dock_button = self.render_dock_toggle_button(DockPlacement::Right, window, cx);
        let has_extend_dock_button = left_dock_button.is_some() || bottom_dock_button.is_some();

        let is_bottom_dock = bottom_dock_button.is_some();

        let panel_style = dock_area.read(cx).panel_style;
        let visible_panels = self.visible_panels(cx).collect::<Vec<_>>();

        if visible_panels.len() == 1 && panel_style == PanelStyle::default() {
            let panel = visible_panels.get(0).unwrap();

            if !panel.visible(cx) {
                return div().into_any_element();
            }

            let title_style = panel.title_style(cx);

            return h_flex()
                .justify_between()
                .line_height(rems(1.0))
                .h(px(30.))
                .py_2()
                .pl_3()
                .pr_2()
                .when(left_dock_button.is_some(), |this| this.pl_2())
                .when(right_dock_button.is_some(), |this| this.pr_2())
                .when_some(title_style, |this, theme| {
                    this.bg(theme.background).text_color(theme.foreground)
                })
                .when(has_extend_dock_button, |this| {
                    this.child(
                        h_flex()
                            .flex_shrink_0()
                            .mr_1()
                            .gap_1()
                            .children(left_dock_button)
                            .children(bottom_dock_button),
                    )
                })
                .child(
                    div()
                        .id("tab")
                        .flex_1()
                        .min_w_16()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(panel.title(window, cx))
                        .when(state.draggable, |this| {
                            this.on_drag(
                                DragPanel {
                                    panel: panel.clone(),
                                    tab_panel: view,
                                },
                                |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                },
                            )
                        }),
                )
                .children(panel.title_suffix(window, cx))
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .ml_1()
                        .gap_1()
                        .child(self.render_toolbar(&state, window, cx))
                        .children(right_dock_button),
                )
                .into_any_element();
        }

        let tabs_count = self.panels.len();

        TabBar::new("tab-bar")
            .track_scroll(&self.tab_bar_scroll_handle)
            .when(has_extend_dock_button, |this| {
                this.prefix(
                    h_flex()
                        .items_center()
                        .top_0()
                        // Right -1 for avoid border overlap with the first tab
                        .right(-px(1.))
                        .border_r_1()
                        .border_b_1()
                        .h_full()
                        .border_color(cx.theme().colors().border)
                        .bg(cx.theme().colors().tab.bar_background)
                        .px_2()
                        .children(left_dock_button)
                        .children(bottom_dock_button),
                )
            })
            .children(self.panels.iter().enumerate().filter_map(|(ix, panel)| {
                let mut active = state.active_panel.as_ref() == Some(panel);
                let droppable = self.collapsed;

                if !panel.visible(cx) {
                    return None;
                }

                // Always not show active tab style, if the panel is collapsed
                if self.collapsed {
                    active = false;
                }

                Some(
                    Tab::new(format!("tab-{}", ix))
                        .ix(ix)
                        .tab_bar_prefix(has_extend_dock_button)
                        .map(|this| {
                            if let Some(tab_name) = panel.tab_name(cx) {
                                this.child(tab_name)
                            } else {
                                this.child(panel.title(window, cx))
                            }
                        })
                        .selected(active)
                        .on_click(cx.listener({
                            let is_collapsed = self.collapsed;
                            let dock_area = self.dock_area.clone();
                            move |view, _, window, cx| {
                                view.set_active_ix(ix, window, cx);

                                // Open dock if clicked on the collapsed bottom dock
                                if is_bottom_dock && is_collapsed {
                                    _ = dock_area.update(cx, |dock_area, cx| {
                                        dock_area.toggle_dock(DockPlacement::Bottom, window, cx);
                                    });
                                }
                            }
                        }))
                        .when(!droppable, |this| {
                            this.when(state.draggable, |this| {
                                this.on_drag(
                                    DragPanel::new(panel.clone(), view.clone()),
                                    |drag, _, _, cx| {
                                        cx.stop_propagation();
                                        cx.new(|_| drag.clone())
                                    },
                                )
                            })
                            .when(state.droppable, |this| {
                                this.drag_over::<DragPanel>(|this, _, _, cx| {
                                    this.rounded_l_none()
                                        .border_l_2()
                                        .border_r_0()
                                        .border_color(cx.theme().colors().drop_target_border)
                                })
                                .on_drop(cx.listener(
                                    move |this, drag: &DragPanel, window, cx| {
                                        this.will_split_placement = None;
                                        this.on_drop(drag, Some(ix), true, window, cx)
                                    },
                                ))
                            })
                        }),
                )
            }))
            .last_empty_space(
                // empty space to allow move to last tab right
                div()
                    .id("tab-bar-empty-space")
                    .h_full()
                    .flex_grow()
                    .min_w_16()
                    .when(state.droppable, |this| {
                        this.drag_over::<DragPanel>(|this, _, _, cx| {
                            this.bg(cx.theme().colors().drop_target_background)
                        })
                        .on_drop(cx.listener(
                            move |this, drag: &DragPanel, window, cx| {
                                this.will_split_placement = None;

                                let ix = if drag.tab_panel == view {
                                    Some(tabs_count - 1)
                                } else {
                                    None
                                };

                                this.on_drop(drag, ix, false, window, cx)
                            },
                        ))
                    }),
            )
            .when(!self.collapsed, |this| {
                this.suffix(
                    h_flex()
                        .items_center()
                        .top_0()
                        .right_0()
                        .border_l_1()
                        .border_b_1()
                        .h_full()
                        .border_color(cx.theme().colors().border)
                        .bg(cx.theme().colors().tab.bar_background)
                        .px_2()
                        .gap_1()
                        .children(
                            self.active_panel(cx)
                                .and_then(|panel| panel.title_suffix(window, cx)),
                        )
                        .child(self.render_toolbar(state, window, cx))
                        .when_some(right_dock_button, |this, btn| this.child(btn)),
                )
            })
            .into_any_element()
    }

    pub(super) fn render_active_panel(
        &self,
        state: &TabState,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.collapsed {
            return Empty {}.into_any_element();
        }

        let Some(active_panel) = state.active_panel.as_ref() else {
            return Empty {}.into_any_element();
        };

        let is_render_in_tabs = self.panels.len() > 1 && self.inner_padding(cx);

        v_flex()
            .id("active-panel")
            .group("")
            .flex_1()
            .when(is_render_in_tabs, |this| this.pt_2())
            .child(
                div()
                    .id("tab-content")
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .flex_1()
                    .child(
                        active_panel
                            .view()
                            .cached(StyleRefinement::default().absolute().size_full()),
                    ),
            )
            .when(state.droppable, |this| {
                this.on_drag_move(cx.listener(Self::on_panel_drag_move))
                    .child(
                        div()
                            .invisible()
                            .absolute()
                            .bg(cx.theme().colors().drop_target_background)
                            .map(|this| match self.will_split_placement {
                                Some(placement) => {
                                    let size = relative(0.5);
                                    match placement {
                                        Placement::Left => this.left_0().top_0().bottom_0().w(size),
                                        Placement::Right => {
                                            this.right_0().top_0().bottom_0().w(size)
                                        }
                                        Placement::Top => this.top_0().left_0().right_0().h(size),
                                        Placement::Bottom => {
                                            this.bottom_0().left_0().right_0().h(size)
                                        }
                                    }
                                }
                                None => this.top_0().left_0().size_full(),
                            })
                            .group_drag_over::<DragPanel>("", |this| this.visible())
                            .on_drop(cx.listener(|this, drag: &DragPanel, window, cx| {
                                this.on_drop(drag, None, true, window, cx)
                            })),
                    )
            })
            .into_any_element()
    }
}

impl Focusable for TabPanel {
    fn focus_handle(&self, cx: &App) -> inazuma::FocusHandle {
        if let Some(active_panel) = self.active_panel(cx) {
            active_panel.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}
impl EventEmitter<DismissEvent> for TabPanel {}
impl EventEmitter<PanelEvent> for TabPanel {}
impl Render for TabPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl inazuma::IntoElement {
        let focus_handle = self.focus_handle(cx);
        let active_panel = self.active_panel(cx);
        let state = TabState {
            closable: self.closable(cx),
            draggable: self.draggable(cx),
            droppable: self.droppable(cx),
            zoomable: self.zoomable(cx),
            active_panel,
        };

        self.bind_actions(cx)
            .id("tab-panel")
            .track_focus(&focus_handle)
            .tab_group()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().colors().background)
            .child(self.render_title_bar(&state, window, cx))
            .child(self.render_active_panel(&state, window, cx))
    }
}
