use super::*;

pub(super) const MAC_PLATFORM_IVAR: &str = "platform";
pub(super) static mut APP_CLASS: *const Class = ptr::null();
pub(super) static mut APP_DELEGATE_CLASS: *const Class = ptr::null();

/// The macOS implementation of the GPUI platform.
pub struct MacPlatform(pub(super) Mutex<MacPlatformState>);

pub(super) struct MacPlatformState {
    pub(super) background_executor: BackgroundExecutor,
    pub(super) foreground_executor: ForegroundExecutor,
    pub(super) text_system: Arc<dyn PlatformTextSystem>,
    pub(super) renderer_context: renderer::Context,
    pub(super) headless: bool,
    pub(super) general_pasteboard: Pasteboard,
    pub(super) find_pasteboard: Pasteboard,
    pub(super) reopen: Option<Box<dyn FnMut()>>,
    pub(super) on_keyboard_layout_change: Option<Box<dyn FnMut()>>,
    pub(super) on_thermal_state_change: Option<Box<dyn FnMut()>>,
    pub(super) quit: Option<Box<dyn FnMut()>>,
    pub(super) menu_command: Option<Box<dyn FnMut(&dyn Action)>>,
    pub(super) validate_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    pub(super) will_open_menu: Option<Box<dyn FnMut()>>,
    pub(super) menu_actions: Vec<Box<dyn Action>>,
    pub(super) open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    pub(super) finish_launching: Option<Box<dyn FnOnce()>>,
    pub(super) dock_menu: Option<id>,
    pub(super) menus: Option<Vec<OwnedMenu>>,
    pub(super) keyboard_mapper: Rc<MacKeyboardMapper>,
}

impl MacPlatform {
    /// Creates a new MacPlatform.
    pub fn new(headless: bool) -> Self {
        let dispatcher = Arc::new(MacDispatcher::new());

        #[cfg(feature = "font-kit")]
        let text_system = Arc::new(super::super::MacTextSystem::new());

        #[cfg(not(feature = "font-kit"))]
        let text_system = Arc::new(inazuma::NoopTextSystem::new());

        let keyboard_layout = MacKeyboardLayout::new();
        let keyboard_mapper = Rc::new(MacKeyboardMapper::new(keyboard_layout.id()));

        Self(Mutex::new(MacPlatformState {
            headless,
            text_system,
            background_executor: BackgroundExecutor::new(dispatcher.clone()),
            foreground_executor: ForegroundExecutor::new(dispatcher),
            renderer_context: renderer::Context::default(),
            general_pasteboard: Pasteboard::general(),
            find_pasteboard: Pasteboard::find(),
            reopen: None,
            quit: None,
            menu_command: None,
            validate_menu_command: None,
            will_open_menu: None,
            menu_actions: Default::default(),
            open_urls: None,
            finish_launching: None,
            dock_menu: None,
            on_keyboard_layout_change: None,
            on_thermal_state_change: None,
            menus: None,
            keyboard_mapper,
        }))
    }

    pub(super) unsafe fn create_menu_bar(
        &self,
        menus: &Vec<Menu>,
        delegate: id,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> id {
        unsafe {
            let application_menu = NSMenu::new(nil).autorelease();
            application_menu.setDelegate_(delegate);

            for menu_config in menus {
                let menu = NSMenu::new(nil).autorelease();
                let menu_title = ns_string(&menu_config.name);
                menu.setTitle_(menu_title);
                menu.setDelegate_(delegate);

                for item_config in &menu_config.items {
                    menu.addItem_(Self::create_menu_item(
                        item_config,
                        delegate,
                        actions,
                        keymap,
                    ));
                }

                let menu_item = NSMenuItem::new(nil).autorelease();
                menu_item.setTitle_(menu_title);
                menu_item.setSubmenu_(menu);
                application_menu.addItem_(menu_item);

                if menu_config.name == "Window" {
                    let app: id = msg_send![APP_CLASS, sharedApplication];
                    app.setWindowsMenu_(menu);
                }
            }

            application_menu
        }
    }

    pub(super) unsafe fn create_dock_menu(
        &self,
        menu_items: Vec<MenuItem>,
        delegate: id,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> id {
        unsafe {
            let dock_menu = NSMenu::new(nil);
            dock_menu.setDelegate_(delegate);
            for item_config in menu_items {
                dock_menu.addItem_(Self::create_menu_item(
                    &item_config,
                    delegate,
                    actions,
                    keymap,
                ));
            }

            dock_menu
        }
    }

    unsafe fn create_menu_item(
        item: &MenuItem,
        delegate: id,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> id {
        static DEFAULT_CONTEXT: OnceLock<Vec<KeyContext>> = OnceLock::new();

        unsafe {
            match item {
                MenuItem::Separator => NSMenuItem::separatorItem(nil),
                MenuItem::Action {
                    name,
                    action,
                    os_action,
                    checked,
                    disabled,
                } => {
                    let keystrokes = keymap
                        .bindings_for_action(action.as_ref())
                        .find_or_first(|binding| {
                            binding.predicate().is_none_or(|predicate| {
                                predicate.eval(DEFAULT_CONTEXT.get_or_init(|| {
                                    let mut workspace_context = KeyContext::new_with_defaults();
                                    workspace_context.add("Workspace");
                                    let mut pane_context = KeyContext::new_with_defaults();
                                    pane_context.add("Pane");
                                    let mut editor_context = KeyContext::new_with_defaults();
                                    editor_context.add("Editor");

                                    pane_context.extend(&editor_context);
                                    workspace_context.extend(&pane_context);
                                    vec![workspace_context]
                                }))
                            })
                        })
                        .map(|binding| binding.keystrokes());

                    let selector = match os_action {
                        Some(inazuma::OsAction::Cut) => selector("cut:"),
                        Some(inazuma::OsAction::Copy) => selector("copy:"),
                        Some(inazuma::OsAction::Paste) => selector("paste:"),
                        Some(inazuma::OsAction::SelectAll) => selector("selectAll:"),
                        Some(inazuma::OsAction::Undo) => selector("handleGPUIMenuItem:"),
                        Some(inazuma::OsAction::Redo) => selector("handleGPUIMenuItem:"),
                        None => selector("handleGPUIMenuItem:"),
                    };

                    let item;
                    if let Some(keystrokes) = keystrokes {
                        if keystrokes.len() == 1 {
                            let keystroke = &keystrokes[0];
                            let mut mask = NSEventModifierFlags::empty();
                            for (modifier, flag) in &[
                                (keystroke.modifiers().platform, NSEventModifierFlags::NSCommandKeyMask),
                                (keystroke.modifiers().control, NSEventModifierFlags::NSControlKeyMask),
                                (keystroke.modifiers().alt, NSEventModifierFlags::NSAlternateKeyMask),
                                (keystroke.modifiers().shift, NSEventModifierFlags::NSShiftKeyMask),
                            ] {
                                if *modifier {
                                    mask |= *flag;
                                }
                            }

                            item = NSMenuItem::alloc(nil)
                                .initWithTitle_action_keyEquivalent_(
                                    ns_string(name),
                                    selector,
                                    ns_string(key_to_native(keystroke.key()).as_ref()),
                                )
                                .autorelease();
                            if Self::os_version() >= Version::new(12, 0, 0) {
                                let _: () = msg_send![item, setAllowsAutomaticKeyEquivalentLocalization: NO];
                            }
                            item.setKeyEquivalentModifierMask_(mask);
                        } else {
                            item = NSMenuItem::alloc(nil)
                                .initWithTitle_action_keyEquivalent_(ns_string(name), selector, ns_string(""))
                                .autorelease();
                        }
                    } else {
                        item = NSMenuItem::alloc(nil)
                            .initWithTitle_action_keyEquivalent_(ns_string(name), selector, ns_string(""))
                            .autorelease();
                    }

                    if *checked {
                        item.setState_(NSVisualEffectState::Active);
                    }
                    item.setEnabled_(if *disabled { NO } else { YES });

                    let tag = actions.len() as NSInteger;
                    let _: () = msg_send![item, setTag: tag];
                    actions.push(action.boxed_clone());
                    item
                }
                MenuItem::Submenu(Menu { name, items, disabled }) => {
                    let item = NSMenuItem::new(nil).autorelease();
                    let submenu = NSMenu::new(nil).autorelease();
                    submenu.setDelegate_(delegate);
                    for item in items {
                        submenu.addItem_(Self::create_menu_item(item, delegate, actions, keymap));
                    }
                    item.setSubmenu_(submenu);
                    item.setEnabled_(if *disabled { NO } else { YES });
                    item.setTitle_(ns_string(name));
                    item
                }
                MenuItem::SystemMenu(OsMenu { name, menu_type }) => {
                    let item = NSMenuItem::new(nil).autorelease();
                    let submenu = NSMenu::new(nil).autorelease();
                    submenu.setDelegate_(delegate);
                    item.setSubmenu_(submenu);
                    item.setTitle_(ns_string(name));

                    match menu_type {
                        SystemMenuType::Services => {
                            let app: id = msg_send![APP_CLASS, sharedApplication];
                            app.setServicesMenu_(item);
                        }
                    }

                    item
                }
            }
        }
    }

    pub(super) fn os_version() -> Version {
        let version = unsafe {
            let process_info = NSProcessInfo::processInfo(nil);
            process_info.operatingSystemVersion()
        };
        Version::new(
            version.majorVersion,
            version.minorVersion,
            version.patchVersion,
        )
    }
}
