use std::{
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

use anyhow::Result;
use inazuma_collections::FxHashMap;
use itertools::Itertools;

use super::{AppCell, NullHttpClient, QuitMode};
use crate::{
    AnyWindowHandle, App, AssetSource, BackgroundExecutor, ForegroundExecutor, Global,
    Platform, SharedString, SvgRenderer, TextSystem, WindowId,
};
use crate::scheduler::Instant;

/// A reference to a GPUI application, typically constructed in the `main` function of your app.
/// You won't interact with this type much outside of initial configuration and startup.
pub struct Application(pub(super) Rc<AppCell>);

/// Represents an application before it is fully launched. Once your app is
/// configured, you'll start the app with `App::run`.
impl Application {
    /// Builds an app with the default platform for the current OS.
    pub fn new() -> Self {
        Self::with_platform(crate::current_platform(false))
    }

    /// Builds an app with a caller-provided platform implementation.
    pub fn with_platform(platform: Rc<dyn Platform>) -> Self {
        Self(App::new_app(
            platform,
            Arc::new(()),
            Arc::new(NullHttpClient),
        ))
    }

    /// Assigns the source of assets for the application.
    pub fn with_assets(self, asset_source: impl AssetSource) -> Self {
        let mut context_lock = self.0.borrow_mut();
        let asset_source = Arc::new(asset_source);
        context_lock.asset_source = asset_source.clone();
        context_lock.svg_renderer = SvgRenderer::new(asset_source);
        drop(context_lock);
        self
    }

    /// Sets the HTTP client for the application.
    pub fn with_http_client(self, http_client: Arc<dyn raijin_http_client::HttpClient>) -> Self {
        let mut context_lock = self.0.borrow_mut();
        context_lock.http_client = http_client;
        drop(context_lock);
        self
    }

    /// Configures when the application should automatically quit.
    /// By default, [`QuitMode::Default`] is used.
    pub fn with_quit_mode(self, mode: QuitMode) -> Self {
        self.0.borrow_mut().quit_mode = mode;
        self
    }

    /// Start the application. The provided callback will be called once the
    /// app is fully launched.
    pub fn run<F>(self, on_finish_launching: F)
    where
        F: 'static + FnOnce(&mut App),
    {
        let this = self.0.clone();
        let platform = self.0.borrow().platform.clone();
        platform.run(Box::new(move || {
            let cx = &mut *this.borrow_mut();
            on_finish_launching(cx);
        }));
    }

    /// Register a handler to be invoked when the platform instructs the application
    /// to open one or more URLs.
    pub fn on_open_urls<F>(&self, mut callback: F) -> &Self
    where
        F: 'static + FnMut(Vec<String>),
    {
        self.0.borrow().platform.on_open_urls(Box::new(callback));
        self
    }

    /// Invokes a handler when an already-running application is launched.
    /// On macOS, this can occur when the application icon is double-clicked or the app is launched via the dock.
    pub fn on_reopen<F>(&self, mut callback: F) -> &Self
    where
        F: 'static + FnMut(&mut App),
    {
        let this = Rc::downgrade(&self.0);
        self.0.borrow_mut().platform.on_reopen(Box::new(move || {
            if let Some(app) = this.upgrade() {
                callback(&mut app.borrow_mut());
            }
        }));
        self
    }

    /// Returns a handle to the [`BackgroundExecutor`] associated with this app, which can be used to spawn futures in the background.
    pub fn background_executor(&self) -> BackgroundExecutor {
        self.0.borrow().background_executor.clone()
    }

    /// Returns a handle to the [`ForegroundExecutor`] associated with this app, which can be used to spawn futures in the foreground.
    pub fn foreground_executor(&self) -> ForegroundExecutor {
        self.0.borrow().foreground_executor.clone()
    }

    /// Returns a reference to the [`TextSystem`] associated with this app.
    pub fn text_system(&self) -> Arc<TextSystem> {
        self.0.borrow().text_system.clone()
    }

    /// Returns the file URL of the executable with the specified name in the application bundle
    pub fn path_for_auxiliary_executable(&self, name: &str) -> Result<PathBuf> {
        self.0.borrow().path_for_auxiliary_executable(name)
    }
}

#[doc(hidden)]
#[derive(Clone, PartialEq, Eq)]
pub struct SystemWindowTab {
    pub id: WindowId,
    pub title: SharedString,
    pub handle: AnyWindowHandle,
    pub last_active_at: Instant,
}

impl SystemWindowTab {
    /// Create a new instance of the window tab.
    pub fn new(title: SharedString, handle: AnyWindowHandle) -> Self {
        Self {
            id: handle.id,
            title,
            handle,
            last_active_at: Instant::now(),
        }
    }
}

/// A controller for managing window tabs.
#[derive(Default)]
pub struct SystemWindowTabController {
    visible: Option<bool>,
    tab_groups: FxHashMap<usize, Vec<SystemWindowTab>>,
}

impl Global for SystemWindowTabController {}

impl SystemWindowTabController {
    /// Create a new instance of the window tab controller.
    pub fn new() -> Self {
        Self {
            visible: None,
            tab_groups: FxHashMap::default(),
        }
    }

    /// Initialize the global window tab controller.
    pub fn init(cx: &mut App) {
        cx.set_global(SystemWindowTabController::new());
    }

    /// Get all tab groups.
    pub fn tab_groups(&self) -> &FxHashMap<usize, Vec<SystemWindowTab>> {
        &self.tab_groups
    }

    /// Get the next tab group window handle.
    pub fn get_next_tab_group_window(cx: &mut App, id: WindowId) -> Option<&AnyWindowHandle> {
        let controller = cx.global::<SystemWindowTabController>();
        let current_group = controller
            .tab_groups
            .iter()
            .find_map(|(group, tabs)| tabs.iter().find(|tab| tab.id == id).map(|_| group));

        let current_group = current_group?;
        let mut group_ids: Vec<_> = controller.tab_groups.keys().collect();
        let idx = group_ids.iter().position(|g| *g == current_group)?;
        let next_idx = (idx + 1) % group_ids.len();

        controller
            .tab_groups
            .get(group_ids[next_idx])
            .and_then(|tabs| {
                tabs.iter()
                    .max_by_key(|tab| tab.last_active_at)
                    .or_else(|| tabs.first())
                    .map(|tab| &tab.handle)
            })
    }

    /// Get the previous tab group window handle.
    pub fn get_prev_tab_group_window(cx: &mut App, id: WindowId) -> Option<&AnyWindowHandle> {
        let controller = cx.global::<SystemWindowTabController>();
        let current_group = controller
            .tab_groups
            .iter()
            .find_map(|(group, tabs)| tabs.iter().find(|tab| tab.id == id).map(|_| group));

        let current_group = current_group?;
        let mut group_ids: Vec<_> = controller.tab_groups.keys().collect();
        let idx = group_ids.iter().position(|g| *g == current_group)?;
        let prev_idx = if idx == 0 {
            group_ids.len() - 1
        } else {
            idx - 1
        };

        controller
            .tab_groups
            .get(group_ids[prev_idx])
            .and_then(|tabs| {
                tabs.iter()
                    .max_by_key(|tab| tab.last_active_at)
                    .or_else(|| tabs.first())
                    .map(|tab| &tab.handle)
            })
    }

    /// Get all tabs in the same window.
    pub fn tabs(&self, id: WindowId) -> Option<&Vec<SystemWindowTab>> {
        self.tab_groups
            .values()
            .find(|tabs| tabs.iter().any(|tab| tab.id == id))
    }

    /// Initialize the visibility of the system window tab controller.
    pub fn init_visible(cx: &mut App, visible: bool) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        if controller.visible.is_none() {
            controller.visible = Some(visible);
        }
    }

    /// Get the visibility of the system window tab controller.
    pub fn is_visible(&self) -> bool {
        self.visible.unwrap_or(false)
    }

    /// Set the visibility of the system window tab controller.
    pub fn set_visible(cx: &mut App, visible: bool) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        controller.visible = Some(visible);
    }

    /// Update the last active of a window.
    pub fn update_last_active(cx: &mut App, id: WindowId) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        for windows in controller.tab_groups.values_mut() {
            for tab in windows.iter_mut() {
                if tab.id == id {
                    tab.last_active_at = Instant::now();
                }
            }
        }
    }

    /// Update the position of a tab within its group.
    pub fn update_tab_position(cx: &mut App, id: WindowId, ix: usize) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        for (_, windows) in controller.tab_groups.iter_mut() {
            if let Some(current_pos) = windows.iter().position(|tab| tab.id == id) {
                if ix < windows.len() && current_pos != ix {
                    let window_tab = windows.remove(current_pos);
                    windows.insert(ix, window_tab);
                }
                break;
            }
        }
    }

    /// Update the title of a tab.
    pub fn update_tab_title(cx: &mut App, id: WindowId, title: SharedString) {
        let controller = cx.global::<SystemWindowTabController>();
        let tab = controller
            .tab_groups
            .values()
            .flat_map(|windows| windows.iter())
            .find(|tab| tab.id == id);

        if tab.map_or(true, |t| t.title == title) {
            return;
        }

        let mut controller = cx.global_mut::<SystemWindowTabController>();
        for windows in controller.tab_groups.values_mut() {
            for tab in windows.iter_mut() {
                if tab.id == id {
                    tab.title = title;
                    return;
                }
            }
        }
    }

    /// Insert a tab into a tab group.
    pub fn add_tab(cx: &mut App, id: WindowId, tabs: Vec<SystemWindowTab>) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        let Some(tab) = tabs.iter().find(|tab| tab.id == id).cloned() else {
            return;
        };

        let mut expected_tab_ids: Vec<_> = tabs
            .iter()
            .filter(|tab| tab.id != id)
            .map(|tab| tab.id)
            .sorted()
            .collect();

        let mut tab_group_id = None;
        for (group_id, group_tabs) in &controller.tab_groups {
            let tab_ids: Vec<_> = group_tabs.iter().map(|tab| tab.id).sorted().collect();
            if tab_ids == expected_tab_ids {
                tab_group_id = Some(*group_id);
                break;
            }
        }

        if let Some(tab_group_id) = tab_group_id {
            if let Some(tabs) = controller.tab_groups.get_mut(&tab_group_id) {
                tabs.push(tab);
            }
        } else {
            let new_group_id = controller.tab_groups.len();
            controller.tab_groups.insert(new_group_id, tabs);
        }
    }

    /// Remove a tab from a tab group.
    pub fn remove_tab(cx: &mut App, id: WindowId) -> Option<SystemWindowTab> {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        let mut removed_tab = None;

        controller.tab_groups.retain(|_, tabs| {
            if let Some(pos) = tabs.iter().position(|tab| tab.id == id) {
                removed_tab = Some(tabs.remove(pos));
            }
            !tabs.is_empty()
        });

        removed_tab
    }

    /// Move a tab to a new tab group.
    pub fn move_tab_to_new_window(cx: &mut App, id: WindowId) {
        let mut removed_tab = Self::remove_tab(cx, id);
        let mut controller = cx.global_mut::<SystemWindowTabController>();

        if let Some(tab) = removed_tab {
            let new_group_id = controller.tab_groups.keys().max().map_or(0, |k| k + 1);
            controller.tab_groups.insert(new_group_id, vec![tab]);
        }
    }

    /// Merge all tab groups into a single group.
    pub fn merge_all_windows(cx: &mut App, id: WindowId) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        let Some(initial_tabs) = controller.tabs(id) else {
            return;
        };

        let initial_tabs_len = initial_tabs.len();
        let mut all_tabs = initial_tabs.clone();

        for (_, mut tabs) in controller.tab_groups.drain() {
            tabs.retain(|tab| !all_tabs[..initial_tabs_len].contains(tab));
            all_tabs.extend(tabs);
        }

        controller.tab_groups.insert(0, all_tabs);
    }

    /// Selects the next tab in the tab group in the trailing direction.
    pub fn select_next_tab(cx: &mut App, id: WindowId) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        let Some(tabs) = controller.tabs(id) else {
            return;
        };

        let current_index = tabs.iter().position(|tab| tab.id == id).unwrap();
        let next_index = (current_index + 1) % tabs.len();

        let _ = &tabs[next_index].handle.update(cx, |_, window, _| {
            window.activate_window();
        });
    }

    /// Selects the previous tab in the tab group in the leading direction.
    pub fn select_previous_tab(cx: &mut App, id: WindowId) {
        let mut controller = cx.global_mut::<SystemWindowTabController>();
        let Some(tabs) = controller.tabs(id) else {
            return;
        };

        let current_index = tabs.iter().position(|tab| tab.id == id).unwrap();
        let previous_index = if current_index == 0 {
            tabs.len() - 1
        } else {
            current_index - 1
        };

        let _ = &tabs[previous_index].handle.update(cx, |_, window, _| {
            window.activate_window();
        });
    }
}
