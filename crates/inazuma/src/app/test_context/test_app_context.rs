use super::*;

/// A TestAppContext is provided to tests created with `#[inazuma::test]`, it provides
/// an implementation of `Context` with additional methods that are useful in tests.
#[derive(Clone)]
pub struct TestAppContext {
    #[doc(hidden)]
    pub background_executor: BackgroundExecutor,
    #[doc(hidden)]
    pub foreground_executor: ForegroundExecutor,
    #[doc(hidden)]
    pub dispatcher: TestDispatcher,
    pub(super) test_platform: Rc<TestPlatform>,
    pub(super) text_system: Arc<TextSystem>,
    pub(super) fn_name: Option<&'static str>,
    pub(super) on_quit: Rc<RefCell<Vec<Box<dyn FnOnce() + 'static>>>>,
    #[doc(hidden)]
    pub app: Rc<AppCell>,
}

impl AppContext for TestAppContext {
    fn new<T: 'static>(&mut self, build_entity: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        let mut app = self.app.borrow_mut();
        app.new(build_entity)
    }

    fn reserve_entity<T: 'static>(&mut self) -> crate::Reservation<T> {
        let mut app = self.app.borrow_mut();
        app.reserve_entity()
    }

    fn insert_entity<T: 'static>(
        &mut self,
        reservation: crate::Reservation<T>,
        build_entity: impl FnOnce(&mut Context<T>) -> T,
    ) -> Entity<T> {
        let mut app = self.app.borrow_mut();
        app.insert_entity(reservation, build_entity)
    }

    fn update_entity<T: 'static, R>(
        &mut self,
        handle: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> R {
        let mut app = self.app.borrow_mut();
        app.update_entity(handle, update)
    }

    fn as_mut<'a, T>(&'a mut self, _: &Entity<T>) -> super::GpuiBorrow<'a, T>
    where
        T: 'static,
    {
        panic!("Cannot use as_mut with a test app context. Try calling update() first")
    }

    fn read_entity<T, R>(&self, handle: &Entity<T>, read: impl FnOnce(&T, &App) -> R) -> R
    where
        T: 'static,
    {
        let app = self.app.borrow();
        app.read_entity(handle, read)
    }

    fn update_window<T, F>(&mut self, window: AnyWindowHandle, f: F) -> Result<T>
    where
        F: FnOnce(AnyView, &mut Window, &mut App) -> T,
    {
        let mut lock = self.app.borrow_mut();
        lock.update_window(window, f)
    }

    fn read_window<T, R>(
        &self,
        window: &WindowHandle<T>,
        read: impl FnOnce(Entity<T>, &App) -> R,
    ) -> Result<R>
    where
        T: 'static,
    {
        let app = self.app.borrow();
        app.read_window(window, read)
    }

    fn background_spawn<R>(&self, future: impl Future<Output = R> + Send + 'static) -> Task<R>
    where
        R: Send + 'static,
    {
        self.background_executor.spawn(future)
    }

    fn read_global<G, R>(&self, callback: impl FnOnce(&G, &App) -> R) -> R
    where
        G: Global,
    {
        let app = self.app.borrow();
        app.read_global(callback)
    }
}

impl TestAppContext {
    /// Creates a new `TestAppContext`. Usually you can rely on `#[inazuma::test]` to do this for you.
    pub fn build(dispatcher: TestDispatcher, fn_name: Option<&'static str>) -> Self {
        let arc_dispatcher = Arc::new(dispatcher.clone());
        let background_executor = BackgroundExecutor::new(arc_dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(arc_dispatcher);
        let platform = TestPlatform::new(background_executor.clone(), foreground_executor.clone());
        let asset_source = Arc::new(());
        let http_client = http_client::FakeHttpClient::with_404_response();
        let text_system = Arc::new(TextSystem::new(platform.text_system()));

        let app = App::new_app(platform.clone(), asset_source, http_client);
        app.borrow_mut().mode = GpuiMode::test();

        Self {
            app,
            background_executor,
            foreground_executor,
            dispatcher,
            test_platform: platform,
            text_system,
            fn_name,
            on_quit: Rc::new(RefCell::new(Vec::default())),
        }
    }

    /// Skip all drawing operations for the duration of this test.
    pub fn skip_drawing(&mut self) {
        self.app.borrow_mut().mode = GpuiMode::Test { skip_drawing: true };
    }

    /// Create a single TestAppContext, for non-multi-client tests
    pub fn single() -> Self {
        let dispatcher = TestDispatcher::new(0);
        Self::build(dispatcher, None)
    }

    /// The name of the test function that created this `TestAppContext`
    pub fn test_function_name(&self) -> Option<&'static str> {
        self.fn_name
    }

    /// Checks whether there have been any new path prompts received by the platform.
    pub fn did_prompt_for_new_path(&self) -> bool {
        self.test_platform.did_prompt_for_new_path()
    }

    /// returns a new `TestAppContext` re-using the same executors to interleave tasks.
    pub fn new_app(&self) -> TestAppContext {
        Self::build(self.dispatcher.clone(), self.fn_name)
    }

    /// Called by the test helper to end the test.
    /// public so the macro can call it.
    pub fn quit(&self) {
        self.on_quit.borrow_mut().drain(..).for_each(|f| f());
        self.app.borrow_mut().shutdown();
    }

    /// Register cleanup to run when the test ends.
    pub fn on_quit(&mut self, f: impl FnOnce() + 'static) {
        self.on_quit.borrow_mut().push(Box::new(f));
    }

    /// Schedules all windows to be redrawn on the next effect cycle.
    pub fn refresh(&mut self) -> Result<()> {
        let mut app = self.app.borrow_mut();
        app.refresh_windows();
        Ok(())
    }

    /// Returns an executor (for running tasks in the background)
    pub fn executor(&self) -> BackgroundExecutor {
        self.background_executor.clone()
    }

    /// Returns an executor (for running tasks on the main thread)
    pub fn foreground_executor(&self) -> &ForegroundExecutor {
        &self.foreground_executor
    }

    #[expect(clippy::wrong_self_convention)]
    fn new<T: 'static>(&mut self, build_entity: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        let mut cx = self.app.borrow_mut();
        cx.new(build_entity)
    }

    /// Gives you an `&mut App` for the duration of the closure
    pub fn update<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        let mut cx = self.app.borrow_mut();
        cx.update(f)
    }

    /// Gives you an `&App` for the duration of the closure
    pub fn read<R>(&self, f: impl FnOnce(&App) -> R) -> R {
        let cx = self.app.borrow();
        f(&cx)
    }

    /// Adds a new window. The Window will always be backed by a `TestWindow` which
    /// can be retrieved with `self.test_window(handle)`
    pub fn add_window<F, V>(&mut self, build_window: F) -> WindowHandle<V>
    where
        F: FnOnce(&mut Window, &mut Context<V>) -> V,
        V: 'static + Render,
    {
        let mut cx = self.app.borrow_mut();

        // Some tests rely on the window size matching the bounds of the test display
        let bounds = Bounds::maximized(None, &cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| build_window(window, cx)),
        )
        .unwrap()
    }

    /// Opens a new window with a specific size.
    ///
    /// Unlike `add_window` which uses maximized bounds, this allows controlling
    /// the window dimensions, which is important for layout-sensitive tests.
    pub fn open_window<F, V>(
        &mut self,
        window_size: Size<Pixels>,
        build_window: F,
    ) -> WindowHandle<V>
    where
        F: FnOnce(&mut Window, &mut Context<V>) -> V,
        V: 'static + Render,
    {
        let mut cx = self.app.borrow_mut();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: window_size,
                })),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| build_window(window, cx)),
        )
        .unwrap()
    }

    /// Adds a new window with no content.
    pub fn add_empty_window(&mut self) -> &mut VisualTestContext {
        let mut cx = self.app.borrow_mut();
        let bounds = Bounds::maximized(None, &cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| Empty),
            )
            .unwrap();
        drop(cx);
        let cx = VisualTestContext::from_window(*window.deref(), self).into_mut();
        cx.run_until_parked();
        cx
    }

    /// Adds a new window, and returns its root view and a `VisualTestContext` which can be used
    /// as a `Window` and `App` for the rest of the test. Typically you would shadow this context with
    /// the returned one. `let (view, cx) = cx.add_window_view(...);`
    pub fn add_window_view<F, V>(
        &mut self,
        build_root_view: F,
    ) -> (Entity<V>, &mut VisualTestContext)
    where
        F: FnOnce(&mut Window, &mut Context<V>) -> V,
        V: 'static + Render,
    {
        let mut cx = self.app.borrow_mut();
        let bounds = Bounds::maximized(None, &cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| build_root_view(window, cx)),
            )
            .unwrap();
        drop(cx);
        let view = window.root(self).unwrap();
        let cx = VisualTestContext::from_window(*window.deref(), self).into_mut();
        cx.run_until_parked();

        // it might be nice to try and cleanup these at the end of each test.
        (view, cx)
    }

    /// returns the TextSystem
    pub fn text_system(&self) -> &Arc<TextSystem> {
        &self.text_system
    }

    /// Simulates writing to the platform clipboard
    pub fn write_to_clipboard(&self, item: ClipboardItem) {
        self.test_platform.write_to_clipboard(item)
    }

    /// Simulates reading from the platform clipboard.
    /// This will return the most recent value from `write_to_clipboard`.
    pub fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        self.test_platform.read_from_clipboard()
    }

    /// Simulates choosing a File in the platform's "Open" dialog.
    pub fn simulate_new_path_selection(
        &self,
        select_path: impl FnOnce(&std::path::Path) -> Option<std::path::PathBuf>,
    ) {
        self.test_platform.simulate_new_path_selection(select_path);
    }

    /// Simulates clicking a button in an platform-level alert dialog.
    #[track_caller]
    pub fn simulate_prompt_answer(&self, button: &str) {
        self.test_platform.simulate_prompt_answer(button);
    }

    /// Returns true if there's an alert dialog open.
    pub fn has_pending_prompt(&self) -> bool {
        self.test_platform.has_pending_prompt()
    }

    /// Returns true if there's an alert dialog open.
    pub fn pending_prompt(&self) -> Option<(String, String)> {
        self.test_platform.pending_prompt()
    }

    /// All the urls that have been opened with cx.open_url() during this test.
    pub fn opened_url(&self) -> Option<String> {
        self.test_platform.opened_url.borrow().clone()
    }

    /// Simulates the user resizing the window to the new size.
    pub fn simulate_window_resize(&self, window_handle: AnyWindowHandle, size: Size<Pixels>) {
        self.test_window(window_handle).simulate_resize(size);
    }

    /// Returns true if there's an alert dialog open.
    pub fn expect_restart(&self) -> oneshot::Receiver<Option<PathBuf>> {
        let (tx, rx) = futures::channel::oneshot::channel();
        self.test_platform.expect_restart.borrow_mut().replace(tx);
        rx
    }

    /// Causes the given sources to be returned if the application queries for screen
    /// capture sources.
    pub fn set_screen_capture_sources(&self, sources: Vec<TestScreenCaptureSource>) {
        self.test_platform.set_screen_capture_sources(sources);
    }

    /// Returns all windows open in the test.
    pub fn windows(&self) -> Vec<AnyWindowHandle> {
        self.app.borrow().windows()
    }

    /// Run the given task on the main thread.
    #[track_caller]
    pub fn spawn<Fut, R>(&self, f: impl FnOnce(AsyncApp) -> Fut) -> Task<R>
    where
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        self.foreground_executor.spawn(f(self.to_async()))
    }

    /// true if the given global is defined
    pub fn has_global<G: Global>(&self) -> bool {
        let app = self.app.borrow();
        app.has_global::<G>()
    }

    /// runs the given closure with a reference to the global
    /// panics if `has_global` would return false.
    pub fn read_global<G: Global, R>(&self, read: impl FnOnce(&G, &App) -> R) -> R {
        let app = self.app.borrow();
        read(app.global(), &app)
    }

    /// runs the given closure with a reference to the global (if set)
    pub fn try_read_global<G: Global, R>(&self, read: impl FnOnce(&G, &App) -> R) -> Option<R> {
        let lock = self.app.borrow();
        Some(read(lock.try_global()?, &lock))
    }

    /// sets the global in this context.
    pub fn set_global<G: Global>(&mut self, global: G) {
        let mut lock = self.app.borrow_mut();
        lock.update(|cx| cx.set_global(global))
    }

    /// updates the global in this context. (panics if `has_global` would return false)
    pub fn update_global<G: Global, R>(&mut self, update: impl FnOnce(&mut G, &mut App) -> R) -> R {
        let mut lock = self.app.borrow_mut();
        lock.update(|cx| cx.update_global(update))
    }

    /// Returns an `AsyncApp` which can be used to run tasks that expect to be on a background
    /// thread on the current thread in tests.
    pub fn to_async(&self) -> AsyncApp {
        AsyncApp {
            app: Rc::downgrade(&self.app),
            background_executor: self.background_executor.clone(),
            foreground_executor: self.foreground_executor.clone(),
        }
    }

    /// Wait until there are no more pending tasks.
    pub fn run_until_parked(&self) {
        self.dispatcher.run_until_parked();
    }

    /// Simulate dispatching an action to the currently focused node in the window.
    pub fn dispatch_action<A>(&mut self, window: AnyWindowHandle, action: A)
    where
        A: Action,
    {
        window
            .update(self, |_, window, cx| {
                window.dispatch_action(action.boxed_clone(), cx)
            })
            .unwrap();

        self.background_executor.run_until_parked()
    }

    /// simulate_keystrokes takes a space-separated list of keys to type.
    /// cx.simulate_keystrokes("cmd-shift-p b k s p enter")
    /// in Zed, this will run backspace on the current editor through the command palette.
    /// This will also run the background executor until it's parked.
    pub fn simulate_keystrokes(&mut self, window: AnyWindowHandle, keystrokes: &str) {
        for keystroke in keystrokes
            .split(' ')
            .map(Keystroke::parse)
            .map(Result::unwrap)
        {
            self.dispatch_keystroke(window, keystroke);
        }

        self.background_executor.run_until_parked()
    }

    /// simulate_input takes a string of text to type.
    /// cx.simulate_input("abc")
    /// will type abc into your current editor
    /// This will also run the background executor until it's parked.
    pub fn simulate_input(&mut self, window: AnyWindowHandle, input: &str) {
        for keystroke in input.split("").map(Keystroke::parse).map(Result::unwrap) {
            self.dispatch_keystroke(window, keystroke);
        }

        self.background_executor.run_until_parked()
    }

    /// dispatches a single Keystroke (see also `simulate_keystrokes` and `simulate_input`)
    pub fn dispatch_keystroke(&mut self, window: AnyWindowHandle, keystroke: Keystroke) {
        self.update_window(window, |_, window, cx| {
            window.dispatch_keystroke(keystroke, cx)
        })
        .unwrap();
    }

    /// Returns the `TestWindow` backing the given handle.
    pub(crate) fn test_window(&self, window: AnyWindowHandle) -> TestWindow {
        self.app
            .borrow_mut()
            .windows
            .get_mut(window.id)
            .unwrap()
            .as_deref_mut()
            .unwrap()
            .platform_window
            .as_test()
            .unwrap()
            .clone()
    }

    /// Returns a stream of notifications whenever the Entity is updated.
    pub fn notifications<T: 'static>(
        &mut self,
        entity: &Entity<T>,
    ) -> impl Stream<Item = ()> + use<T> {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        self.update(|cx| {
            cx.observe(entity, {
                let tx = tx.clone();
                move |_, _| {
                    let _ = tx.unbounded_send(());
                }
            })
            .detach();
            cx.observe_release(entity, move |_, _| tx.close_channel())
                .detach()
        });
        rx
    }

    /// Returns a stream of events emitted by the given Entity.
    pub fn events<Evt, T: 'static + EventEmitter<Evt>>(
        &mut self,
        entity: &Entity<T>,
    ) -> futures::channel::mpsc::UnboundedReceiver<Evt>
    where
        Evt: 'static + Clone,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        entity
            .update(self, |_, cx: &mut Context<T>| {
                cx.subscribe(entity, move |_entity, _handle, event, _cx| {
                    let _ = tx.unbounded_send(event.clone());
                })
            })
            .detach();
        rx
    }

    /// Runs until the given condition becomes true. (Prefer `run_until_parked` if you
    /// don't need to jump in at a specific time).
    pub async fn condition<T: 'static>(
        &mut self,
        entity: &Entity<T>,
        mut predicate: impl FnMut(&mut T, &mut Context<T>) -> bool,
    ) {
        let timer = self.executor().timer(Duration::from_secs(3));
        let mut notifications = self.notifications(entity);

        use futures::FutureExt as _;
        use futures_concurrency::future::Race as _;

        (
            async {
                loop {
                    if entity.update(self, &mut predicate) {
                        return Ok(());
                    }

                    if notifications.next().await.is_none() {
                        bail!("entity dropped")
                    }
                }
            },
            timer.map(|_| Err(anyhow!("condition timed out"))),
        )
            .race()
            .await
            .unwrap();
    }

    /// Set a name for this App.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_name(&mut self, name: &'static str) {
        self.update(|cx| cx.name = Some(name))
    }
}

impl<T: 'static> Entity<T> {
    /// Block until the next event is emitted by the entity, then return it.
    pub fn next_event<Event>(&self, cx: &mut TestAppContext) -> impl Future<Output = Event>
    where
        Event: Send + Clone + 'static,
        T: EventEmitter<Event>,
    {
        let (tx, mut rx) = oneshot::channel();
        let mut tx = Some(tx);
        let subscription = self.update(cx, |_, cx| {
            cx.subscribe(self, move |_, _, event, _| {
                if let Some(tx) = tx.take() {
                    _ = tx.send(event.clone());
                }
            })
        });

        async move {
            let event = rx.await.expect("no event emitted");
            drop(subscription);
            event
        }
    }
}

impl<V: 'static> Entity<V> {
    /// Returns a future that resolves when the view is next updated.
    pub fn next_notification(
        &self,
        advance_clock_by: Duration,
        cx: &TestAppContext,
    ) -> impl Future<Output = ()> {
        use postage::prelude::{Sink as _, Stream as _};

        let (mut tx, mut rx) = postage::mpsc::channel(1);
        let subscription = cx.app.borrow_mut().observe(self, move |_, _| {
            tx.try_send(()).ok();
        });

        cx.executor().advance_clock(advance_clock_by);

        async move {
            rx.recv()
                .await
                .expect("entity dropped while test was waiting for its next notification");
            drop(subscription);
        }
    }
}

impl<V> Entity<V> {
    /// Returns a future that resolves when the condition becomes true.
    pub fn condition<Evt>(
        &self,
        cx: &TestAppContext,
        mut predicate: impl FnMut(&V, &App) -> bool,
    ) -> impl Future<Output = ()>
    where
        Evt: 'static,
        V: EventEmitter<Evt>,
    {
        use postage::prelude::{Sink as _, Stream as _};

        let (tx, mut rx) = postage::mpsc::channel(1024);

        let mut cx = cx.app.borrow_mut();
        let subscriptions = (
            cx.observe(self, {
                let mut tx = tx.clone();
                move |_, _| {
                    tx.blocking_send(()).ok();
                }
            }),
            cx.subscribe(self, {
                let mut tx = tx;
                move |_, _: &Evt, _| {
                    tx.blocking_send(()).ok();
                }
            }),
        );

        let cx = cx.this.upgrade().unwrap();
        let handle = self.downgrade();

        async move {
            loop {
                {
                    let cx = cx.borrow();
                    let cx = &*cx;
                    if predicate(
                        handle
                            .upgrade()
                            .expect("view dropped with pending condition")
                            .read(cx),
                        cx,
                    ) {
                        break;
                    }
                }

                rx.recv()
                    .await
                    .expect("view dropped with pending condition");
            }
            drop(subscriptions);
        }
    }
}
