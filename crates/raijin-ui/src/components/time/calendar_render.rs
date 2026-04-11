use chrono::{Datelike, NaiveDate};
use inazuma::{
    App, ClickEvent, Div, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window, prelude::FluentBuilder as _, px,
    relative,
};
use raijin_i18n::t;

use crate::{
    ActiveTheme, Button, ButtonVariants as _, Disableable as _, IconName, Selectable, Sizable,
    Size, StyledExt as _, h_flex, v_flex,
};

use super::calendar::{Calendar, CalendarEvent, CalendarState, Date, ViewMode};

impl Calendar {
    pub(super) fn render_day(
        &self,
        d: &NaiveDate,
        offset_month: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let state = self.state.read(cx);
        let (_, month) = state.offset_year_month(offset_month);
        let day = d.day();
        let is_current_month = d.month() == month;
        let is_active = state.date.is_active(d);
        let is_in_range = state.date.is_in_range(d);

        let date = *d;
        let is_today = *d == state.today;
        let disabled = state
            .disabled_matcher
            .as_ref()
            .map_or(false, |disabled| disabled.matched(&date));

        let date_id: SharedString = format!("{}_{}", date.format("%Y-%m-%d"), offset_month).into();

        self.item_button(
            date_id.clone(),
            day.to_string(),
            is_active,
            is_in_range,
            !is_current_month || disabled,
            disabled,
            window,
            cx,
        )
        .when(is_today && !is_active, |this| {
            this.border_1().border_color(cx.theme().colors().border)
        }) // Add border for today
        .when(!disabled, |this| {
            this.on_click(window.listener_for(
                &self.state,
                move |view, _: &ClickEvent, window, cx| {
                    if view.date.is_single() {
                        view.set_date(date, window, cx);
                        cx.emit(CalendarEvent::Selected(view.date()));
                    } else {
                        let start = view.date.start();
                        let end = view.date.end();

                        if start.is_none() && end.is_none() {
                            view.set_date(Date::Range(Some(date), None), window, cx);
                        } else if start.is_some() && end.is_none() {
                            if date < start.unwrap() {
                                view.set_date(Date::Range(Some(date), None), window, cx);
                            } else {
                                view.set_date(
                                    Date::Range(Some(start.unwrap()), Some(date)),
                                    window,
                                    cx,
                                );
                            }
                        } else {
                            view.set_date(Date::Range(Some(date), None), window, cx);
                        }

                        if view.date.is_complete() {
                            cx.emit(CalendarEvent::Selected(view.date()));
                        }
                    }
                },
            ))
        })
    }

    pub(super) fn render_header(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let current_year = state.current_year;
        let view_mode = state.view_mode;
        let disabled = view_mode.is_month();
        let multiple_months = self.number_of_months > 1;
        let icon_size = match self.size {
            Size::Small => Size::Small,
            Size::Large => Size::Medium,
            _ => Size::Medium,
        };

        h_flex()
            .gap_0p5()
            .justify_between()
            .items_center()
            .child(
                Button::with_id("prev")
                    .icon(IconName::ArrowLeft)
                    .tab_stop(false)
                    .ghost()
                    .disabled(disabled)
                    .with_size(icon_size)
                    .when(view_mode.is_day(), |this| {
                        this.on_click(window.listener_for(&self.state, CalendarState::prev_month))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.when(!state.has_prev_year_page(), |this| this.disabled(true))
                            .on_click(
                                window.listener_for(&self.state, CalendarState::prev_year_page),
                            )
                    }),
            )
            .when(!multiple_months, |this| {
                this.child(
                    h_flex()
                        .justify_center()
                        .gap_3()
                        .child(
                            Button::with_id("month")
                                .ghost()
                                .label(state.month_name(0))
                                .compact()
                                .tab_stop(false)
                                .with_size(self.size)
                                .selected(view_mode.is_month())
                                .on_click(window.listener_for(
                                    &self.state,
                                    move |view, _, window, cx| {
                                        if view_mode.is_month() {
                                            view.set_view_mode(ViewMode::Day, window, cx);
                                        } else {
                                            view.set_view_mode(ViewMode::Month, window, cx);
                                        }
                                        cx.notify();
                                    },
                                )),
                        )
                        .child(
                            Button::with_id("year")
                                .ghost()
                                .label(current_year.to_string())
                                .compact()
                                .tab_stop(false)
                                .with_size(self.size)
                                .selected(view_mode.is_year())
                                .on_click(window.listener_for(
                                    &self.state,
                                    |view, _, window, cx| {
                                        if view.view_mode.is_year() {
                                            view.set_view_mode(ViewMode::Day, window, cx);
                                        } else {
                                            view.set_view_mode(ViewMode::Year, window, cx);
                                        }
                                        cx.notify();
                                    },
                                )),
                        ),
                )
            })
            .when(multiple_months, |this| {
                this.child(h_flex().flex_1().justify_around().children(
                    (0..self.number_of_months).map(|n| {
                        h_flex()
                            .justify_center()
                            .map(|this| match self.size {
                                Size::Small => this.gap_2(),
                                Size::Large => this.gap_4(),
                                _ => this.gap_3(),
                            })
                            .child(state.month_name(n))
                            .child(state.year_name(n))
                    }),
                ))
            })
            .child(
                Button::with_id("next")
                    .icon(IconName::ArrowRight)
                    .ghost()
                    .tab_stop(false)
                    .disabled(disabled)
                    .with_size(icon_size)
                    .when(view_mode.is_day(), |this| {
                        this.on_click(window.listener_for(&self.state, CalendarState::next_month))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.when(!state.has_next_year_page(), |this| this.disabled(true))
                            .on_click(
                                window.listener_for(&self.state, CalendarState::next_year_page),
                            )
                    }),
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn item_button(
        &self,
        id: impl Into<inazuma::ElementId>,
        label: impl Into<SharedString>,
        active: bool,
        secondary_active: bool,
        muted: bool,
        disabled: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        h_flex()
            .id(id.into())
            .map(|this| match self.size {
                Size::Small => this.size_7().rounded(cx.theme().colors().radius / 2.),
                Size::Large => this.size_10().rounded(cx.theme().colors().radius * 2.),
                _ => this.size_9().rounded(cx.theme().colors().radius),
            })
            .justify_center()
            .when(muted, |this| {
                this.text_color(if disabled {
                    cx.theme().colors().muted_foreground.opacity(0.3)
                } else {
                    cx.theme().colors().muted_foreground
                })
            })
            .when(secondary_active, |this| {
                this.bg(if muted {
                    cx.theme().colors().accent.opacity(0.5)
                } else {
                    cx.theme().colors().accent
                })
                .text_color(cx.theme().colors().accent_foreground)
            })
            .when(!active && !disabled, |this| {
                this.hover(|this| {
                    this.bg(cx.theme().colors().accent)
                        .text_color(cx.theme().colors().accent_foreground)
                })
            })
            .when(active, |this| {
                this.bg(cx.theme().colors().primary)
                    .text_color(cx.theme().colors().primary_foreground)
            })
            .child(label.into())
    }

    pub(super) fn render_days(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let weeks = [
            t!("Calendar.week.0"),
            t!("Calendar.week.1"),
            t!("Calendar.week.2"),
            t!("Calendar.week.3"),
            t!("Calendar.week.4"),
            t!("Calendar.week.5"),
            t!("Calendar.week.6"),
        ];

        h_flex()
            .map(|this| match self.size {
                Size::Small => this.gap_3().text_sm(),
                Size::Large => this.gap_5().text_base(),
                _ => this.gap_4().text_sm(),
            })
            .justify_between()
            .children(
                state
                    .days()
                    .chunks(5)
                    .enumerate()
                    .map(|(offset_month, days)| {
                        v_flex()
                            .gap_0p5()
                            .child(
                                h_flex().gap_0p5().justify_between().children(
                                    weeks
                                        .iter()
                                        .map(|week| self.render_week(week.clone(), window, cx)),
                                ),
                            )
                            .children(days.iter().map(|week| {
                                h_flex().gap_0p5().justify_between().children(
                                    week.iter()
                                        .map(|d| self.render_day(d, offset_month, window, cx)),
                                )
                            }))
                    }),
            )
    }

    pub(super) fn render_week(
        &self,
        week: impl Into<SharedString>,
        _: &mut Window,
        cx: &mut App,
    ) -> Div {
        h_flex()
            .map(|this| match self.size {
                Size::Small => this.size_7().rounded(cx.theme().colors().radius / 2.0),
                Size::Large => this.size_10().rounded(cx.theme().colors().radius),
                _ => this.size_9().rounded(cx.theme().colors().radius),
            })
            .justify_center()
            .text_color(cx.theme().colors().muted_foreground)
            .text_sm()
            .child(week.into())
    }

    pub(super) fn render_months(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let months = state.months();
        let current_month = state.current_month;

        h_flex()
            .mt_3()
            .gap_0p5()
            .gap_y_3()
            .map(|this| match self.size {
                Size::Small => this.mt_2().gap_y_2().w(px(208.)),
                Size::Large => this.mt_4().gap_y_4().w(px(292.)),
                _ => this.mt_3().gap_y_3().w(px(264.)),
            })
            .justify_between()
            .flex_wrap()
            .children(
                months
                    .iter()
                    .enumerate()
                    .map(|(ix, month)| {
                        let active = (ix + 1) as u8 == current_month;

                        self.item_button(
                            ix,
                            month.to_string(),
                            active,
                            false,
                            false,
                            false,
                            window,
                            cx,
                        )
                        .w(relative(0.3))
                        .text_sm()
                        .on_click(window.listener_for(
                            &self.state,
                            move |view, _, window, cx| {
                                view.current_month = (ix + 1) as u8;
                                view.set_view_mode(ViewMode::Day, window, cx);
                                cx.notify();
                            },
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
    }

    pub(super) fn render_years(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let current_year = state.current_year;
        let current_page_years = &self.state.read(cx).years[state.year_page as usize].clone();

        h_flex()
            .id("years")
            .gap_0p5()
            .map(|this| match self.size {
                Size::Small => this.mt_2().gap_y_2().w(px(208.)),
                Size::Large => this.mt_4().gap_y_4().w(px(292.)),
                _ => this.mt_3().gap_y_3().w(px(264.)),
            })
            .justify_between()
            .flex_wrap()
            .children(
                current_page_years
                    .iter()
                    .enumerate()
                    .map(|(ix, year)| {
                        let year = *year;
                        let active = year == current_year;

                        self.item_button(
                            ix,
                            year.to_string(),
                            active,
                            false,
                            false,
                            false,
                            window,
                            cx,
                        )
                        .w(relative(0.2))
                        .on_click(window.listener_for(
                            &self.state,
                            move |view, _, window, cx| {
                                view.current_year = year;
                                view.set_view_mode(ViewMode::Day, window, cx);
                                cx.notify();
                            },
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
    }
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view_mode = self.state.read(cx).view_mode;
        let number_of_months = self.number_of_months;
        self.state.update(cx, |state, _| {
            state.number_of_months = number_of_months;
        });

        v_flex()
            .id(self.id.clone())
            .track_focus(&self.state.read(cx).focus_handle)
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded(cx.theme().colors().radius_lg())
            .p_3()
            .gap_0p5()
            .refine_style(&self.style)
            .child(self.render_header(window, cx))
            .child(
                v_flex()
                    .when(view_mode.is_day(), |this| {
                        this.child(self.render_days(window, cx))
                    })
                    .when(view_mode.is_month(), |this| {
                        this.child(self.render_months(window, cx))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.child(self.render_years(window, cx))
                    }),
            )
    }
}
