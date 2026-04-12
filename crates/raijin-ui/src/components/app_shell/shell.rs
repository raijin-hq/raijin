use inazuma::{
    AnyView, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, KeyBinding,
    ParentElement as _, Pixels, Render, StyleRefinement, Styled, Window, actions, div,
    prelude::FluentBuilder as _,
};
use raijin_theme::ActiveTheme;

use crate::StyledExt as _;
use crate::utils::window_border::{self, window_border};

use super::dialog_layer::ActiveDialog;
use super::sheet_layer::ActiveSheet;

actions!(app_shell, [Tab, TabPrev]);

const CONTEXT: &str = "AppShell";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
    ]);
}

/// The top-level window shell that orchestrates overlay layers
/// (dialogs, sheets, notifications) and focus navigation.
pub struct AppShell {
    pub(super) style: StyleRefinement,
    pub(super) view: AnyView,
    pub(super) active_sheet: Option<ActiveSheet>,
    pub(super) active_dialogs: Vec<ActiveDialog>,
    pub(super) focused_input: Option<Entity<crate::input::InputState>>,
    pub(super) notification: Entity<crate::components::notification::NotificationList>,
    pub(super) window_shadow_size: Pixels,
    pub(super) pending_focus_restore: Option<inazuma::WeakFocusHandle>,
}

impl AppShell {
    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            style: StyleRefinement::default(),
            view: view.into(),
            active_sheet: None,
            active_dialogs: Vec::new(),
            focused_input: None,
            notification: cx.new(|cx| crate::components::notification::NotificationList::new(window, cx)),
            window_shadow_size: window_border::SHADOW_SIZE,
            pending_focus_restore: None,
        }
    }

    pub fn window_shadow_size(mut self, size: impl Into<Pixels>) -> Self {
        self.window_shadow_size = size.into();
        self
    }

    pub fn update<F, R>(window: &mut Window, cx: &mut App, f: F) -> R
    where
        F: FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    {
        let root = window
            .root::<AppShell>()
            .flatten()
            .expect("BUG: window first layer should be an AppShell.");
        root.update(cx, |shell, cx| f(shell, window, cx))
    }

    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self {
        &window
            .root::<AppShell>()
            .expect("The window root view should be of type `AppShell`.")
            .unwrap()
            .read(cx)
    }

    pub fn focused_input(&self) -> Option<&Entity<crate::components::input::InputState>> {
        self.focused_input.as_ref()
    }

    pub fn set_focused_input(&mut self, input: Option<Entity<crate::components::input::InputState>>) {
        self.focused_input = input;
    }

    pub fn active_dialog_count(&self) -> usize {
        self.active_dialogs.len()
    }

    pub fn view(&self) -> &AnyView {
        &self.view
    }
}

impl Styled for AppShell {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use inazuma::Anchor;

        window.set_rem_size(raijin_theme::theme_settings(cx).ui_font_size(cx));

        let view = self.view.clone();
        let notification = self.notification.clone();
        let notification_placement = Anchor::TopRight;

        let sheet_el = Self::build_sheet_element(self, window, cx)
            .map(|el| el.into_any_element());
        let dialog_el = Self::build_dialog_element(self, window, cx)
            .map(|el| el.into_any_element());

        window_border()
            .shadow_size(self.window_shadow_size)
            .child(
                div()
                    .id("app-shell")
                    .key_context(CONTEXT)
                    .on_action(cx.listener(Self::on_action_tab))
                    .on_action(cx.listener(Self::on_action_tab_prev))
                    .relative()
                    .size_full()
                    .font_family(raijin_theme::theme_settings(cx).ui_font(cx).family.clone())
                    .bg(cx.theme().colors().background)
                    .text_color(cx.theme().colors().foreground)
                    .refine_style(&self.style)
                    .child(view)
                    .children(sheet_el)
                    .children(dialog_el)
                    .child(
                        div()
                            .absolute()
                            .when(matches!(notification_placement, Anchor::TopRight), |this| {
                                this.top_0().right_0()
                            })
                            .when(matches!(notification_placement, Anchor::TopLeft), |this| {
                                this.top_0().left_0()
                            })
                            .when(
                                matches!(notification_placement, Anchor::TopCenter),
                                |this| this.top_0().mx_auto(),
                            )
                            .when(
                                matches!(notification_placement, Anchor::BottomRight),
                                |this| this.bottom_0().right_0(),
                            )
                            .when(
                                matches!(notification_placement, Anchor::BottomLeft),
                                |this| this.bottom_0().left_0(),
                            )
                            .when(
                                matches!(notification_placement, Anchor::BottomCenter),
                                |this| this.bottom_0().mx_auto(),
                            )
                            .child(notification),
                    ),
            )
    }
}
