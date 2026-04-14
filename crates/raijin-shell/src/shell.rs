use inazuma::{
    AnyView, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, KeyBinding,
    ParentElement as _, Pixels, Render, StyleRefinement, Styled, Window, actions, div,
    prelude::FluentBuilder as _,
};
use raijin_theme::ActiveTheme;

use raijin_ui::StyledExt as _;
use raijin_ui::utils::window_border::{self, window_border};
use raijin_ui::input::InputState;
use raijin_ui::NotificationList;
use raijin_ui::{PendingDialogs, CloseDialog, DeferCloseDialog, CloseAllDialogs, CloseSheet, ClearNotifications};

use super::dialog_layer::ActiveDialog;
use super::sheet_layer::ActiveSheet;

actions!(app_shell, [Tab, TabPrev]);

const CONTEXT: &str = "AppShell";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
    ]);

    // Register the WorkspaceOpener so raijin-workspace can open windows via raijin-shell
    raijin_workspace::set_workspace_opener(std::sync::Arc::new(ShellWorkspaceOpener), cx);
}

/// WorkspaceOpener implementation that delegates to raijin-shell's open functions.
struct ShellWorkspaceOpener;

impl raijin_workspace::WorkspaceOpener for ShellWorkspaceOpener {
    fn open_paths(
        &self,
        abs_paths: &[std::path::PathBuf],
        app_state: std::sync::Arc<raijin_workspace::AppState>,
        env: Option<inazuma_collections::HashMap<String, String>>,
        cx: &mut App,
    ) -> inazuma::Task<anyhow::Result<Entity<raijin_workspace::Workspace>>> {
        let open_options = crate::open::OpenOptions {
            env,
            ..Default::default()
        };
        let paths = abs_paths.to_vec();
        let task = crate::open::open_paths(&paths, app_state, open_options, cx);
        cx.spawn(async move |_cx| {
            let result = task.await?;
            Ok(result.workspace)
        })
    }

    fn reload(&self, cx: &mut App) {
        crate::open::reload(cx);
    }

    fn join_in_room_project(
        &self,
        project_id: u64,
        follow_user_id: u64,
        app_state: std::sync::Arc<raijin_workspace::AppState>,
        cx: &mut App,
    ) -> inazuma::Task<anyhow::Result<()>> {
        crate::open::join_in_room_project(project_id, follow_user_id, app_state, cx)
    }

    fn local_workspace_windows(&self, cx: &App) -> Vec<inazuma::AnyWindowHandle> {
        crate::open::local_workspace_windows(cx)
            .into_iter()
            .map(|wh| wh.into())
            .collect()
    }
}

/// The top-level window shell that orchestrates overlay layers
/// (dialogs, sheets, notifications) and focus navigation.
pub struct AppShell {
    pub(crate) style: StyleRefinement,
    pub(crate) view: AnyView,
    pub(crate) active_sheet: Option<ActiveSheet>,
    pub(crate) active_dialogs: Vec<ActiveDialog>,
    pub(crate) focused_input: Option<Entity<InputState>>,
    pub(crate) notification: Entity<NotificationList>,
    pub(crate) window_shadow_size: Pixels,
    pub(crate) pending_focus_restore: Option<inazuma::WeakFocusHandle>,
    _subscriptions: Vec<inazuma::Subscription>,
}

impl AppShell {
    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();

        // Observe PendingDialogs queue — drain and promote to active dialogs
        subscriptions.push(
            cx.observe_global_in::<PendingDialogs>(window, |this: &mut Self, window, cx| {
                let pending: Vec<_> = cx.global_mut::<PendingDialogs>().queue.drain(..).collect();
                for builder in pending {
                    let mut previous_focused_handle = window.focused(cx).map(|h| h.downgrade());
                    if let Some(pending_handle) = this.pending_focus_restore.take() {
                        previous_focused_handle = Some(pending_handle);
                    }
                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window, cx);
                    this.active_dialogs.push(ActiveDialog::new(
                        focus_handle,
                        previous_focused_handle,
                        move |dialog, w, cx| (*builder)(dialog, w, cx),
                    ));
                }
                cx.notify();
            }),
        );

        let view: AnyView = view.into();

        // Register workspace in the global WorkspaceRegistry so Workspace::for_window() works
        if let Ok(workspace) = view.clone().downcast::<raijin_workspace::Workspace>() {
            let window_id = window.window_handle().window_id();
            raijin_workspace::register_workspace_for_window(
                window_id,
                workspace.downgrade(),
                cx,
            );
        }

        // Unregister on release so WorkspaceRegistry doesn't accumulate stale entries
        let release_window_id = window.window_handle().window_id();
        subscriptions.push(cx.on_release(move |_this: &mut Self, cx| {
            raijin_workspace::unregister_workspace_for_window(release_window_id, cx);
        }));

        Self {
            style: StyleRefinement::default(),
            view,
            active_sheet: None,
            active_dialogs: Vec::new(),
            focused_input: None,
            notification: cx.new(|cx| NotificationList::new(window, cx)),
            window_shadow_size: window_border::SHADOW_SIZE,
            pending_focus_restore: None,
            _subscriptions: subscriptions,
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

    pub fn focused_input(&self) -> Option<&Entity<InputState>> {
        self.focused_input.as_ref()
    }

    pub fn set_focused_input(&mut self, input: Option<Entity<InputState>>) {
        self.focused_input = input;
    }

    pub fn active_dialog_count(&self) -> usize {
        self.active_dialogs.len()
    }

    pub fn view(&self) -> &AnyView {
        &self.view
    }

    /// Get the Workspace entity from the current window's AppShell root.
    pub fn workspace(window: &Window, cx: &App) -> Option<Entity<raijin_workspace::Workspace>> {
        window.root::<AppShell>()
            .flatten()
            .and_then(|shell| shell.read(cx).view.clone().downcast::<raijin_workspace::Workspace>().ok())
    }
}

/// Access the active workspace from any App context.
///
/// Finds the active window, gets the AppShell root, extracts the Workspace,
/// and calls `f` with mutable access. Replaces `with_active_or_new_workspace`.
pub fn with_active_workspace(
    cx: &mut App,
    f: impl FnOnce(&mut raijin_workspace::Workspace, &mut Window, &mut Context<raijin_workspace::Workspace>) + Send + 'static,
) {
    if let Some(window) = cx.active_window() {
        cx.defer(move |cx| {
            window.update(cx, |_, window, cx| {
                if let Some(workspace) = AppShell::workspace(window, cx) {
                    workspace.update(cx, |ws, cx| f(ws, window, cx));
                }
            }).ok();
        });
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
                    // Action handlers for window_shell actions dispatched from raijin-ui
                    .on_action(cx.listener(|this, _: &CloseDialog, window, cx| {
                        this.close_dialog(window, cx);
                    }))
                    .on_action(cx.listener(|this, _: &DeferCloseDialog, window, cx| {
                        this.defer_close_dialog(window, cx);
                    }))
                    .on_action(cx.listener(|this, _: &CloseAllDialogs, window, cx| {
                        this.close_all_dialogs(window, cx);
                    }))
                    .on_action(cx.listener(|this, _: &CloseSheet, window, cx| {
                        this.close_sheet(window, cx);
                    }))
                    .on_action(cx.listener(|this, _: &ClearNotifications, window, cx| {
                        this.clear_notifications(window, cx);
                    }))
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
