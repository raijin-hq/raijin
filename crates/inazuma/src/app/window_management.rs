use std::any::TypeId;

use anyhow::{Context as _, Result};
use inazuma_collections::FxHashSet;
use futures::FutureExt;

use super::{Effect, QuitMode, SHUTDOWN_TIMEOUT};
use crate::{
    AnyView, AnyWindowHandle, App, AsyncApp, BackgroundExecutor, CursorStyle, EntityId,
    FocusHandle, ForegroundExecutor, Priority, PromptBuilder, PromptButton, PromptHandle,
    PromptLevel, Render, RenderablePromptHandle, Task, TextRenderingMode, Window,
    WindowHandle, WindowId,
};

impl App {
    #[doc(hidden)]
    pub fn ref_counts_drop_handle(&self) -> impl Sized + use<> {
        self.entities.ref_counts_drop_handle()
    }

    /// Captures a snapshot of all entities that currently have alive handles.
    ///
    /// The returned [`LeakDetectorSnapshot`] can later be passed to
    /// [`assert_no_new_leaks`](Self::assert_no_new_leaks) to verify that no
    /// entities created after the snapshot are still alive.
    #[cfg(any(test, feature = "leak-detection"))]
    pub fn leak_detector_snapshot(&self) -> crate::LeakDetectorSnapshot {
        self.entities.leak_detector_snapshot()
    }

    /// Asserts that no entities created after `snapshot` still have alive handles.
    ///
    /// Entities that were already tracked at the time of the snapshot are ignored,
    /// even if they still have handles. Only *new* entities (those whose
    /// `EntityId` was not present in the snapshot) are considered leaks.
    ///
    /// # Panics
    ///
    /// Panics if any new entity handles exist. The panic message lists every
    /// leaked entity with its type name, and includes allocation-site backtraces
    /// when `LEAK_BACKTRACE` is set.
    #[cfg(any(test, feature = "leak-detection"))]
    pub fn assert_no_new_leaks(&self, snapshot: &crate::LeakDetectorSnapshot) {
        self.entities.assert_no_new_leaks(snapshot)
    }

    /// Quit the application gracefully. Handlers registered with [`Context::on_app_quit`]
    /// will be given 100ms to complete before exiting.
    pub fn shutdown(&mut self) {
        let mut futures = Vec::new();

        for observer in self.quit_observers.remove(&()) {
            futures.push(observer(self));
        }

        self.windows.clear();
        self.window_handles.clear();
        self.flush_effects();
        self.quitting = true;

        let futures = futures::future::join_all(futures);
        if self
            .foreground_executor
            .block_with_timeout(SHUTDOWN_TIMEOUT, futures)
            .is_err()
        {
            log::error!("timed out waiting on app_will_quit");
        }

        self.quitting = false;
    }

    /// Schedules all windows in the application to be redrawn. This can be called
    /// multiple times in an update cycle and still result in a single redraw.
    pub fn refresh_windows(&mut self) {
        self.pending_effects.push_back(Effect::RefreshWindows);
    }

    pub(crate) fn update<R>(&mut self, update: impl FnOnce(&mut Self) -> R) -> R {
        self.start_update();
        let result = update(self);
        self.finish_update();
        result
    }

    pub(crate) fn start_update(&mut self) {
        self.pending_updates += 1;
    }

    pub(crate) fn finish_update(&mut self) {
        if !self.flushing_effects && self.pending_updates == 1 {
            self.flushing_effects = true;
            self.flush_effects();
            self.flushing_effects = false;
        }
        self.pending_updates -= 1;
    }

    /// Returns handles to all open windows in the application.
    /// Each handle could be downcast to a handle typed for the root view of that window.
    /// To find all windows of a given type, you could filter on
    pub fn windows(&self) -> Vec<AnyWindowHandle> {
        self.windows
            .keys()
            .flat_map(|window_id| self.window_handles.get(&window_id).copied())
            .collect()
    }

    /// Returns the window handles ordered by their appearance on screen, front to back.
    ///
    /// The first window in the returned list is the active/topmost window of the application.
    ///
    /// This method returns None if the platform doesn't implement the method yet.
    pub fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        self.platform.window_stack()
    }

    /// Returns a handle to the window that is currently focused at the platform level, if one exists.
    pub fn active_window(&self) -> Option<AnyWindowHandle> {
        self.platform.active_window()
    }

    /// Opens a new window with the given option and the root view returned by the given function.
    /// The function is invoked with a `Window`, which can be used to interact with window-specific
    /// functionality.
    pub fn open_window<V: 'static + Render>(
        &mut self,
        options: crate::WindowOptions,
        build_root_view: impl FnOnce(&mut Window, &mut App) -> crate::Entity<V>,
    ) -> anyhow::Result<WindowHandle<V>> {
        self.update(|cx| {
            let id = cx.windows.insert(None);
            let handle = WindowHandle::new(id);
            match Window::new(handle.into(), options, cx) {
                Ok(mut window) => {
                    cx.window_update_stack.push(id);
                    let root_view = build_root_view(&mut window, cx);
                    cx.window_update_stack.pop();
                    window.root.replace(root_view.into());
                    window.defer(cx, |window: &mut Window, cx| window.appearance_changed(cx));

                    // allow a window to draw at least once before returning
                    // this didn't cause any issues on non windows platforms as it seems we always won the race to on_request_frame
                    // on windows we quite frequently lose the race and return a window that has never rendered, which leads to a crash
                    // where DispatchTree::root_node_id asserts on empty nodes
                    let clear = window.draw(cx);
                    clear.clear();

                    cx.window_handles.insert(id, window.handle);
                    cx.windows.get_mut(id).unwrap().replace(Box::new(window));
                    Ok(handle)
                }
                Err(e) => {
                    cx.windows.remove(id);
                    Err(e)
                }
            }
        })
    }

    pub(crate) fn detect_accessed_entities<R>(
        &mut self,
        callback: impl FnOnce(&mut App) -> R,
    ) -> (R, FxHashSet<EntityId>) {
        let accessed_entities_start = self.entities.accessed_entities.get_mut().clone();
        let result = callback(self);
        let entities_accessed_in_callback = self
            .entities
            .accessed_entities
            .get_mut()
            .difference(&accessed_entities_start)
            .copied()
            .collect::<FxHashSet<EntityId>>();
        (result, entities_accessed_in_callback)
    }

    pub(crate) fn record_entities_accessed(
        &mut self,
        window_handle: AnyWindowHandle,
        invalidator: crate::WindowInvalidator,
        entities: &FxHashSet<EntityId>,
    ) {
        let mut tracked_entities =
            std::mem::take(self.tracked_entities.entry(window_handle.id).or_default());
        for entity in tracked_entities.iter() {
            self.window_invalidators_by_entity
                .entry(*entity)
                .and_modify(|windows| {
                    windows.remove(&window_handle.id);
                });
        }
        for entity in entities.iter() {
            self.window_invalidators_by_entity
                .entry(*entity)
                .or_default()
                .insert(window_handle.id, invalidator.clone());
        }
        tracked_entities.clear();
        tracked_entities.extend(entities.iter().copied());
        self.tracked_entities
            .insert(window_handle.id, tracked_entities);
    }

    pub(super) fn update_window_id<T, F>(&mut self, id: WindowId, update: F) -> Result<T>
    where
        F: FnOnce(AnyView, &mut Window, &mut App) -> T,
    {
        self.update(|cx| {
            let mut window = cx.windows.get_mut(id)?.take()?;

            let root_view = window.root.clone().unwrap();

            cx.window_update_stack.push(window.handle.id);
            let result = update(root_view, &mut window, cx);
            fn trail(id: WindowId, window: Box<Window>, cx: &mut App) -> Option<()> {
                cx.window_update_stack.pop();

                if window.removed {
                    cx.window_handles.remove(&id);
                    cx.windows.remove(id);

                    cx.window_closed_observers.clone().retain(&(), |callback| {
                        callback(cx);
                        true
                    });

                    let quit_on_empty = match cx.quit_mode {
                        QuitMode::Explicit => false,
                        QuitMode::LastWindowClosed => true,
                        QuitMode::Default => cfg!(not(target_os = "macos")),
                    };

                    if quit_on_empty && cx.windows.is_empty() {
                        cx.quit();
                    }
                } else {
                    cx.windows.get_mut(id)?.replace(window);
                }
                Some(())
            }
            trail(id, window, cx)?;

            Some(result)
        })
        .context("window not found")
    }

    /// Creates an `AsyncApp`, which can be cloned and has a static lifetime
    /// so it can be held across `await` points.
    pub fn to_async(&self) -> AsyncApp {
        AsyncApp {
            app: self.this.clone(),
            background_executor: self.background_executor.clone(),
            foreground_executor: self.foreground_executor.clone(),
        }
    }

    /// Obtains a reference to the executor, which can be used to spawn futures.
    pub fn background_executor(&self) -> &BackgroundExecutor {
        &self.background_executor
    }

    /// Obtains a reference to the executor, which can be used to spawn futures.
    pub fn foreground_executor(&self) -> &ForegroundExecutor {
        if self.quitting {
            panic!("Can't spawn on main thread after on_app_quit")
        };
        &self.foreground_executor
    }

    /// Spawns the future returned by the given function on the main thread. The closure will be invoked
    /// with [AsyncApp], which allows the application state to be accessed across await points.
    #[track_caller]
    pub fn spawn<AsyncFn, R>(&self, f: AsyncFn) -> Task<R>
    where
        AsyncFn: AsyncFnOnce(&mut AsyncApp) -> R + 'static,
        R: 'static,
    {
        if self.quitting {
            inazuma_util::debug_panic!("Can't spawn on main thread after on_app_quit")
        };

        let mut cx = self.to_async();

        self.foreground_executor
            .spawn(async move { f(&mut cx).await }.boxed_local())
    }

    /// Spawns the future returned by the given function on the main thread with
    /// the given priority. The closure will be invoked with [AsyncApp], which
    /// allows the application state to be accessed across await points.
    pub fn spawn_with_priority<AsyncFn, R>(&self, priority: Priority, f: AsyncFn) -> Task<R>
    where
        AsyncFn: AsyncFnOnce(&mut AsyncApp) -> R + 'static,
        R: 'static,
    {
        if self.quitting {
            inazuma_util::debug_panic!("Can't spawn on main thread after on_app_quit")
        };

        let mut cx = self.to_async();

        self.foreground_executor
            .spawn_with_priority(priority, async move { f(&mut cx).await }.boxed_local())
    }

    /// Schedules the given function to be run at the end of the current effect cycle, allowing entities
    /// that are currently on the stack to be returned to the app.
    pub fn defer(&mut self, f: impl FnOnce(&mut App) + 'static) {
        self.push_effect(Effect::Defer {
            callback: Box::new(f),
        });
    }

    /// Accessor for the application's asset source, which is provided when constructing the `App`.
    pub fn asset_source(&self) -> &std::sync::Arc<dyn crate::AssetSource> {
        &self.asset_source
    }

    /// Accessor for the text system.
    pub fn text_system(&self) -> &std::sync::Arc<crate::TextSystem> {
        &self.text_system
    }

    /// Sets the text rendering mode for the application.
    pub fn set_text_rendering_mode(&mut self, mode: TextRenderingMode) {
        self.text_rendering_mode.set(mode);
    }

    /// Returns the current text rendering mode for the application.
    pub fn text_rendering_mode(&self) -> TextRenderingMode {
        self.text_rendering_mode.get()
    }

    /// Configures when the application should automatically quit.
    /// By default, [`QuitMode::Default`] is used.
    pub fn set_quit_mode(&mut self, mode: QuitMode) {
        self.quit_mode = mode;
    }

    /// Returns the SVG renderer used by the application.
    pub fn svg_renderer(&self) -> crate::SvgRenderer {
        self.svg_renderer.clone()
    }

    /// Is there currently something being dragged?
    pub fn has_active_drag(&self) -> bool {
        self.active_drag.is_some()
    }

    /// Gets the cursor style of the currently active drag operation.
    pub fn active_drag_cursor_style(&self) -> Option<CursorStyle> {
        self.active_drag.as_ref().and_then(|drag| drag.cursor_style)
    }

    /// Stops active drag and clears any related effects.
    pub fn stop_active_drag(&mut self, window: &mut Window) -> bool {
        if self.active_drag.is_some() {
            self.active_drag = None;
            window.refresh();
            true
        } else {
            false
        }
    }

    /// Sets the cursor style for the currently active drag operation.
    pub fn set_active_drag_cursor_style(
        &mut self,
        cursor_style: CursorStyle,
        window: &mut Window,
    ) -> bool {
        if let Some(ref mut drag) = self.active_drag {
            drag.cursor_style = Some(cursor_style);
            window.refresh();
            true
        } else {
            false
        }
    }

    /// Set the prompt renderer for GPUI. This will replace the default or platform specific
    /// prompts with this custom implementation.
    pub fn set_prompt_builder(
        &mut self,
        renderer: impl Fn(
            PromptLevel,
            &str,
            Option<&str>,
            &[PromptButton],
            PromptHandle,
            &mut Window,
            &mut App,
        ) -> RenderablePromptHandle
        + 'static,
    ) {
        self.prompt_builder = Some(PromptBuilder::Custom(Box::new(renderer)));
    }

    /// Reset the prompt builder to the default implementation.
    pub fn reset_prompt_builder(&mut self) {
        self.prompt_builder = Some(PromptBuilder::Default);
    }

    /// Remove an asset from GPUI's cache
    pub fn remove_asset<A: crate::Asset>(&mut self, source: &A::Source) {
        let asset_id = (TypeId::of::<A>(), crate::hash(source));
        self.loading_assets.remove(&asset_id);
    }

    /// Asynchronously load an asset, if the asset hasn't finished loading this will return None.
    ///
    /// Note that the multiple calls to this method will only result in one `Asset::load` call at a
    /// time, and the results of this call will be cached
    pub fn fetch_asset<A: crate::Asset>(
        &mut self,
        source: &A::Source,
    ) -> (futures::future::Shared<Task<A::Output>>, bool) {
        use futures::future::Shared;
        let asset_id = (TypeId::of::<A>(), crate::hash(source));
        let mut is_first = false;
        let task = self
            .loading_assets
            .remove(&asset_id)
            .map(|boxed_task| *boxed_task.downcast::<Shared<Task<A::Output>>>().unwrap())
            .unwrap_or_else(|| {
                is_first = true;
                let future = A::load(source.clone(), self);

                self.background_executor().spawn(future).shared()
            });

        self.loading_assets.insert(asset_id, Box::new(task.clone()));

        (task, is_first)
    }

    /// Obtain a new [`FocusHandle`], which allows you to track and manipulate the keyboard focus
    /// for elements rendered within this window.
    #[track_caller]
    pub fn focus_handle(&self) -> FocusHandle {
        FocusHandle::new(&self.focus_handles)
    }
}
