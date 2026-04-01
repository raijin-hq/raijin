use super::*;

use derive_more::{Deref, DerefMut};

#[derive(Deref, DerefMut, Clone)]
/// A VisualTestContext is the test-equivalent of a `Window` and `App`. It allows you to
/// run window-specific test code. It can be dereferenced to a `TextAppContext`.
pub struct VisualTestContext {
    #[deref]
    #[deref_mut]
    /// cx is the original TestAppContext (you can more easily access this using Deref)
    pub cx: TestAppContext,
    window: AnyWindowHandle,
}

impl VisualTestContext {
    /// Provides a `Window` and `App` for the duration of the closure.
    pub fn update<R>(&mut self, f: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        self.cx
            .update_window(self.window, |_, window, cx| f(window, cx))
            .unwrap()
    }

    /// Creates a new VisualTestContext. You would typically shadow the passed in
    /// TestAppContext with this, as this is typically more useful.
    /// `let cx = VisualTestContext::from_window(window, cx);`
    pub fn from_window(window: AnyWindowHandle, cx: &TestAppContext) -> Self {
        Self {
            cx: cx.clone(),
            window,
        }
    }

    /// Wait until there are no more pending tasks.
    pub fn run_until_parked(&self) {
        self.cx.background_executor.run_until_parked();
    }

    /// Dispatch the action to the currently focused node.
    pub fn dispatch_action<A>(&mut self, action: A)
    where
        A: Action,
    {
        self.cx.dispatch_action(self.window, action)
    }

    /// Read the title off the window (set by `Window#set_window_title`)
    pub fn window_title(&mut self) -> Option<String> {
        self.cx.test_window(self.window).0.lock().title.clone()
    }

    /// Simulate a sequence of keystrokes `cx.simulate_keystrokes("cmd-p escape")`
    /// Automatically runs until parked.
    pub fn simulate_keystrokes(&mut self, keystrokes: &str) {
        self.cx.simulate_keystrokes(self.window, keystrokes)
    }

    /// Simulate typing text `cx.simulate_input("hello")`
    /// Automatically runs until parked.
    pub fn simulate_input(&mut self, input: &str) {
        self.cx.simulate_input(self.window, input)
    }

    /// Simulate a mouse move event to the given point
    pub fn simulate_mouse_move(
        &mut self,
        position: Point<Pixels>,
        button: impl Into<Option<MouseButton>>,
        modifiers: Modifiers,
    ) {
        self.simulate_event(MouseMoveEvent {
            position,
            modifiers,
            pressed_button: button.into(),
        })
    }

    /// Simulate a mouse down event to the given point
    pub fn simulate_mouse_down(
        &mut self,
        position: Point<Pixels>,
        button: MouseButton,
        modifiers: Modifiers,
    ) {
        self.simulate_event(MouseDownEvent {
            position,
            modifiers,
            button,
            click_count: 1,
            first_mouse: false,
        })
    }

    /// Simulate a mouse up event to the given point
    pub fn simulate_mouse_up(
        &mut self,
        position: Point<Pixels>,
        button: MouseButton,
        modifiers: Modifiers,
    ) {
        self.simulate_event(MouseUpEvent {
            position,
            modifiers,
            button,
            click_count: 1,
        })
    }

    /// Simulate a primary mouse click at the given point
    pub fn simulate_click(&mut self, position: Point<Pixels>, modifiers: Modifiers) {
        self.simulate_event(MouseDownEvent {
            position,
            modifiers,
            button: MouseButton::Left,
            click_count: 1,
            first_mouse: false,
        });
        self.simulate_event(MouseUpEvent {
            position,
            modifiers,
            button: MouseButton::Left,
            click_count: 1,
        });
    }

    /// Simulate a modifiers changed event
    pub fn simulate_modifiers_change(&mut self, modifiers: Modifiers) {
        self.simulate_event(ModifiersChangedEvent {
            modifiers,
            capslock: Capslock { on: false },
        })
    }

    /// Simulate a capslock changed event
    pub fn simulate_capslock_change(&mut self, on: bool) {
        self.simulate_event(ModifiersChangedEvent {
            modifiers: Modifiers::none(),
            capslock: Capslock { on },
        })
    }

    /// Simulates the user resizing the window to the new size.
    pub fn simulate_resize(&self, size: Size<Pixels>) {
        self.simulate_window_resize(self.window, size)
    }

    /// debug_bounds returns the bounds of the element with the given selector.
    pub fn debug_bounds(&mut self, selector: &'static str) -> Option<Bounds<Pixels>> {
        self.update(|window, _| window.rendered_frame.debug_bounds.get(selector).copied())
    }

    /// Draw an element to the window. Useful for simulating events or actions
    pub fn draw<E>(
        &mut self,
        origin: Point<Pixels>,
        space: impl Into<Size<AvailableSpace>>,
        f: impl FnOnce(&mut Window, &mut App) -> E,
    ) -> (E::RequestLayoutState, E::PrepaintState)
    where
        E: Element,
    {
        self.update(|window, cx| {
            let _arena_scope = ElementArenaScope::enter(&cx.element_arena);

            window.invalidator.set_phase(DrawPhase::Prepaint);
            let mut element = Drawable::new(f(window, cx));
            element.layout_as_root(space.into(), window, cx);
            window.with_absolute_element_offset(origin, |window| element.prepaint(window, cx));

            window.invalidator.set_phase(DrawPhase::Paint);
            let (request_layout_state, prepaint_state) = element.paint(window, cx);

            window.invalidator.set_phase(DrawPhase::None);
            window.refresh();

            drop(element);
            cx.element_arena.borrow_mut().clear();

            (request_layout_state, prepaint_state)
        })
    }

    /// Simulate an event from the platform, e.g. a ScrollWheelEvent
    /// Make sure you've called [VisualTestContext::draw] first!
    pub fn simulate_event<E: InputEvent>(&mut self, event: E) {
        self.test_window(self.window)
            .simulate_input(event.to_platform_input());
        self.background_executor.run_until_parked();
    }

    /// Simulates the user blurring the window.
    pub fn deactivate_window(&mut self) {
        if Some(self.window) == self.test_platform.active_window() {
            self.test_platform.set_active_window(None)
        }
        self.background_executor.run_until_parked();
    }

    /// Simulates the user closing the window.
    /// Returns true if the window was closed.
    pub fn simulate_close(&mut self) -> bool {
        let handler = self
            .cx
            .update_window(self.window, |_, window, _| {
                window
                    .platform_window
                    .as_test()
                    .unwrap()
                    .0
                    .lock()
                    .should_close_handler
                    .take()
            })
            .unwrap();
        if let Some(mut handler) = handler {
            let should_close = handler();
            self.cx
                .update_window(self.window, |_, window, _| {
                    window.platform_window.on_should_close(handler);
                })
                .unwrap();
            should_close
        } else {
            false
        }
    }

    /// Get an &mut VisualTestContext (which is mostly what you need to pass to other methods).
    /// This method internally retains the VisualTestContext until the end of the test.
    pub fn into_mut(self) -> &'static mut Self {
        let ptr = Box::into_raw(Box::new(self));
        // safety: on_quit will be called after the test has finished.
        // the executor will ensure that all tasks related to the test have stopped.
        // so there is no way for cx to be accessed after on_quit is called.
        // todo: This is unsound under stacked borrows (also tree borrows probably?)
        // the mutable reference invalidates `ptr` which is later used in the closure
        let cx = unsafe { &mut *ptr };
        cx.on_quit(move || unsafe {
            drop(Box::from_raw(ptr));
        });
        cx
    }
}

impl AppContext for VisualTestContext {
    fn new<T: 'static>(&mut self, build_entity: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        self.cx.new(build_entity)
    }

    fn reserve_entity<T: 'static>(&mut self) -> crate::Reservation<T> {
        self.cx.reserve_entity()
    }

    fn insert_entity<T: 'static>(
        &mut self,
        reservation: crate::Reservation<T>,
        build_entity: impl FnOnce(&mut Context<T>) -> T,
    ) -> Entity<T> {
        self.cx.insert_entity(reservation, build_entity)
    }

    fn update_entity<T, R>(
        &mut self,
        handle: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> R
    where
        T: 'static,
    {
        self.cx.update_entity(handle, update)
    }

    fn as_mut<'a, T>(&'a mut self, handle: &Entity<T>) -> super::GpuiBorrow<'a, T>
    where
        T: 'static,
    {
        self.cx.as_mut(handle)
    }

    fn read_entity<T, R>(&self, handle: &Entity<T>, read: impl FnOnce(&T, &App) -> R) -> R
    where
        T: 'static,
    {
        self.cx.read_entity(handle, read)
    }

    fn update_window<T, F>(&mut self, window: AnyWindowHandle, f: F) -> Result<T>
    where
        F: FnOnce(AnyView, &mut Window, &mut App) -> T,
    {
        self.cx.update_window(window, f)
    }

    fn read_window<T, R>(
        &self,
        window: &WindowHandle<T>,
        read: impl FnOnce(Entity<T>, &App) -> R,
    ) -> Result<R>
    where
        T: 'static,
    {
        self.cx.read_window(window, read)
    }

    fn background_spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> Task<R>
    where
        R: Send + 'static,
    {
        self.cx.background_spawn(future)
    }

    fn read_global<G, R>(&self, callback: impl FnOnce(&G, &App) -> R) -> R
    where
        G: Global,
    {
        self.cx.read_global(callback)
    }
}

impl VisualContext for VisualTestContext {
    type Result<T> = T;

    /// Get the underlying window handle underlying this context.
    fn window_handle(&self) -> AnyWindowHandle {
        self.window
    }

    fn new_window_entity<T: 'static>(
        &mut self,
        build_entity: impl FnOnce(&mut Window, &mut Context<T>) -> T,
    ) -> Entity<T> {
        self.window
            .update(&mut self.cx, |_, window, cx| {
                cx.new(|cx| build_entity(window, cx))
            })
            .expect("window was unexpectedly closed")
    }

    fn update_window_entity<V: 'static, R>(
        &mut self,
        view: &Entity<V>,
        update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
    ) -> R {
        self.window
            .update(&mut self.cx, |_, window, cx| {
                view.update(cx, |v, cx| update(v, window, cx))
            })
            .expect("window was unexpectedly closed")
    }

    fn replace_root_view<V>(
        &mut self,
        build_view: impl FnOnce(&mut Window, &mut Context<V>) -> V,
    ) -> Entity<V>
    where
        V: 'static + Render,
    {
        self.window
            .update(&mut self.cx, |_, window, cx| {
                window.replace_root(cx, build_view)
            })
            .expect("window was unexpectedly closed")
    }

    fn focus<V: crate::Focusable>(&mut self, view: &Entity<V>) {
        self.window
            .update(&mut self.cx, |_, window, cx| {
                view.read(cx).focus_handle(cx).focus(window, cx)
            })
            .expect("window was unexpectedly closed")
    }
}

impl AnyWindowHandle {
    /// Creates the given view in this window.
    pub fn build_entity<V: Render + 'static>(
        &self,
        cx: &mut TestAppContext,
        build_view: impl FnOnce(&mut Window, &mut Context<V>) -> V,
    ) -> Entity<V> {
        self.update(cx, |_, window, cx| cx.new(|cx| build_view(window, cx)))
            .unwrap()
    }
}
