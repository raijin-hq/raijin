use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::{Rc, Weak},
    sync::Arc,
    time::Duration,
};

use collections::{FxHashMap, FxHashSet, VecDeque};
use parking_lot::RwLock;
use slotmap::SlotMap;

use super::{
    AppCell, Effect, GpuiMode, Handler, KeystrokeObserver, Listener, NewEntityListener, QuitHandler,
    QuitMode, ReleaseListener, SystemWindowTabController, WindowClosedHandler,
};
use crate::{
    ActionRegistry, AnyWindowHandle, Arena, AssetSource, BackgroundExecutor, EntityMap, FocusMap,
    ForegroundExecutor, Keymap, LayoutId, Platform, PlatformKeyboardLayout,
    PlatformKeyboardMapper, PromptBuilder, SubscriberSet, SvgRenderer, TextRenderingMode,
    TextSystem, Window, WindowId, WindowInvalidator, init_app_menus,
};

#[cfg(any(feature = "inspector", debug_assertions))]
use crate::InspectorElementRegistry;

/// The duration for which futures returned from [Context::on_app_quit] can run before the application fully quits.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(100);

/// Contains the state of the full application, and passed as a reference to a variety of callbacks.
/// Other [Context] derefs to this type.
/// You need a reference to an `App` to access the state of a [Entity].
pub struct App {
    pub(crate) this: Weak<AppCell>,
    pub(crate) platform: Rc<dyn Platform>,
    pub(super) text_system: Arc<TextSystem>,

    pub(crate) actions: Rc<ActionRegistry>,
    pub(crate) active_drag: Option<super::AnyDrag>,
    pub(crate) background_executor: BackgroundExecutor,
    pub(crate) foreground_executor: ForegroundExecutor,
    pub(crate) entities: EntityMap,
    pub(crate) new_entity_observers: SubscriberSet<TypeId, NewEntityListener>,
    pub(crate) windows: SlotMap<WindowId, Option<Box<Window>>>,
    pub(crate) window_handles: FxHashMap<WindowId, AnyWindowHandle>,
    pub(crate) focus_handles: Arc<FocusMap>,
    pub(crate) keymap: Rc<RefCell<Keymap>>,
    pub(crate) keyboard_layout: Box<dyn PlatformKeyboardLayout>,
    pub(crate) keyboard_mapper: Rc<dyn PlatformKeyboardMapper>,
    pub(crate) global_action_listeners:
        FxHashMap<TypeId, Vec<Rc<dyn Fn(&dyn std::any::Any, crate::DispatchPhase, &mut Self)>>>,
    pub(super) pending_effects: VecDeque<Effect>,

    pub(crate) observers: SubscriberSet<crate::EntityId, Handler>,
    pub(crate) event_listeners: SubscriberSet<crate::EntityId, (TypeId, Listener)>,
    pub(crate) keystroke_observers: SubscriberSet<(), KeystrokeObserver>,
    pub(crate) keystroke_interceptors: SubscriberSet<(), KeystrokeObserver>,
    pub(crate) keyboard_layout_observers: SubscriberSet<(), Handler>,
    pub(crate) thermal_state_observers: SubscriberSet<(), Handler>,
    pub(crate) release_listeners: SubscriberSet<crate::EntityId, ReleaseListener>,
    pub(crate) global_observers: SubscriberSet<TypeId, Handler>,
    pub(crate) quit_observers: SubscriberSet<(), QuitHandler>,
    pub(crate) restart_observers: SubscriberSet<(), Handler>,
    pub(crate) window_closed_observers: SubscriberSet<(), WindowClosedHandler>,

    /// Per-App element arena. This isolates element allocations between different
    /// App instances (important for tests where multiple Apps run concurrently).
    pub(crate) element_arena: RefCell<Arena>,
    /// Per-App event arena.
    pub(crate) event_arena: Arena,

    // Drop globals last. We need to ensure all tasks owned by entities and
    // callbacks are marked cancelled at this point as this will also shutdown
    // the tokio runtime. As any task attempting to spawn a blocking tokio task,
    // might panic.
    pub(crate) globals_by_type: FxHashMap<TypeId, Box<dyn std::any::Any>>,

    // assets
    pub(crate) loading_assets: FxHashMap<(TypeId, u64), Box<dyn std::any::Any>>,
    pub(super) asset_source: Arc<dyn AssetSource>,
    pub(crate) svg_renderer: SvgRenderer,
    pub(super) http_client: Arc<dyn http_client::HttpClient>,

    // below is plain data, the drop order is insignificant here
    pub(crate) pending_notifications: FxHashSet<crate::EntityId>,
    pub(crate) pending_global_notifications: FxHashSet<TypeId>,
    pub(crate) restart_path: Option<PathBuf>,
    pub(crate) layout_id_buffer: Vec<LayoutId>, // We recycle this memory across layout requests.
    pub(crate) propagate_event: bool,
    pub(crate) prompt_builder: Option<PromptBuilder>,
    pub(crate) window_invalidators_by_entity:
        FxHashMap<crate::EntityId, FxHashMap<WindowId, WindowInvalidator>>,
    pub(crate) tracked_entities: FxHashMap<WindowId, FxHashSet<crate::EntityId>>,
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub(crate) inspector_renderer: Option<crate::InspectorRenderer>,
    #[cfg(any(feature = "inspector", debug_assertions))]
    pub(crate) inspector_element_registry: InspectorElementRegistry,
    #[cfg(any(test, feature = "test-support", debug_assertions))]
    pub(crate) name: Option<&'static str>,
    pub(crate) text_rendering_mode: Rc<Cell<TextRenderingMode>>,

    pub(crate) window_update_stack: Vec<WindowId>,
    pub(crate) mode: GpuiMode,
    pub(super) flushing_effects: bool,
    pub(super) pending_updates: usize,
    pub(super) quit_mode: QuitMode,
    pub(super) quitting: bool,

    // We need to ensure the leak detector drops last, after all tasks, callbacks and things have been dropped.
    // Otherwise it may report false positives.
    #[cfg(any(test, feature = "leak-detection"))]
    pub(crate) _ref_counts: Arc<RwLock<crate::EntityRefCounts>>,
}

impl App {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new_app(
        platform: Rc<dyn Platform>,
        asset_source: Arc<dyn AssetSource>,
        http_client: Arc<dyn http_client::HttpClient>,
    ) -> Rc<AppCell> {
        let background_executor = platform.background_executor();
        let foreground_executor = platform.foreground_executor();
        assert!(
            background_executor.is_main_thread(),
            "must construct App on main thread"
        );

        let text_system = Arc::new(TextSystem::new(platform.text_system()));
        let entities = EntityMap::new();
        let keyboard_layout = platform.keyboard_layout();
        let keyboard_mapper = platform.keyboard_mapper();

        #[cfg(any(test, feature = "leak-detection"))]
        let _ref_counts = entities.ref_counts_drop_handle();

        let app = Rc::new_cyclic(|this| AppCell {
            app: RefCell::new(App {
                this: this.clone(),
                platform: platform.clone(),
                text_system,
                text_rendering_mode: Rc::new(Cell::new(TextRenderingMode::default())),
                mode: GpuiMode::Production,
                actions: Rc::new(ActionRegistry::default()),
                flushing_effects: false,
                pending_updates: 0,
                active_drag: None,
                background_executor,
                foreground_executor,
                svg_renderer: SvgRenderer::new(asset_source.clone()),
                loading_assets: Default::default(),
                asset_source,
                http_client,
                globals_by_type: FxHashMap::default(),
                entities,
                new_entity_observers: SubscriberSet::new(),
                windows: SlotMap::with_key(),
                window_update_stack: Vec::new(),
                window_handles: FxHashMap::default(),
                focus_handles: Arc::new(RwLock::new(SlotMap::with_key())),
                keymap: Rc::new(RefCell::new(Keymap::default())),
                keyboard_layout,
                keyboard_mapper,
                global_action_listeners: FxHashMap::default(),
                pending_effects: VecDeque::new(),
                pending_notifications: FxHashSet::default(),
                pending_global_notifications: FxHashSet::default(),
                observers: SubscriberSet::new(),
                tracked_entities: FxHashMap::default(),
                window_invalidators_by_entity: FxHashMap::default(),
                event_listeners: SubscriberSet::new(),
                release_listeners: SubscriberSet::new(),
                keystroke_observers: SubscriberSet::new(),
                keystroke_interceptors: SubscriberSet::new(),
                keyboard_layout_observers: SubscriberSet::new(),
                thermal_state_observers: SubscriberSet::new(),
                global_observers: SubscriberSet::new(),
                quit_observers: SubscriberSet::new(),
                restart_observers: SubscriberSet::new(),
                restart_path: None,
                window_closed_observers: SubscriberSet::new(),
                layout_id_buffer: Default::default(),
                propagate_event: true,
                prompt_builder: Some(PromptBuilder::Default),
                #[cfg(any(feature = "inspector", debug_assertions))]
                inspector_renderer: None,
                #[cfg(any(feature = "inspector", debug_assertions))]
                inspector_element_registry: InspectorElementRegistry::default(),
                quit_mode: QuitMode::default(),
                quitting: false,

                #[cfg(any(test, feature = "test-support", debug_assertions))]
                name: None,
                element_arena: RefCell::new(Arena::new(1024 * 1024)),
                event_arena: Arena::new(1024 * 1024),

                #[cfg(any(test, feature = "leak-detection"))]
                _ref_counts,
            }),
        });

        init_app_menus(platform.as_ref(), &app.borrow());
        SystemWindowTabController::init(&mut app.borrow_mut());

        platform.on_keyboard_layout_change(Box::new({
            let app = Rc::downgrade(&app);
            move || {
                if let Some(app) = app.upgrade() {
                    let cx = &mut app.borrow_mut();
                    cx.keyboard_layout = cx.platform.keyboard_layout();
                    cx.keyboard_mapper = cx.platform.keyboard_mapper();
                    cx.keyboard_layout_observers
                        .clone()
                        .retain(&(), move |callback| (callback)(cx));
                }
            }
        }));

        platform.on_thermal_state_change(Box::new({
            let app = Rc::downgrade(&app);
            move || {
                if let Some(app) = app.upgrade() {
                    let cx = &mut app.borrow_mut();
                    cx.thermal_state_observers
                        .clone()
                        .retain(&(), move |callback| (callback)(cx));
                }
            }
        }));

        platform.on_quit(Box::new({
            let cx = Rc::downgrade(&app);
            move || {
                if let Some(cx) = cx.upgrade() {
                    cx.borrow_mut().shutdown();
                }
            }
        }));

        app
    }
}
