use super::*;

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
    pub(super) dock_menu: Option<Retained<NSMenu>>,
    pub(super) menus: Option<Vec<OwnedMenu>>,
    pub(super) keyboard_mapper: Rc<MacKeyboardMapper>,
}

impl MacPlatform {
    /// Creates a new MacPlatform.
    pub fn new(headless: bool) -> Self {
        let dispatcher = Arc::new(MacDispatcher::new());

        let text_system = Arc::new(super::super::MacTextSystem::new());

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

    pub(super) fn create_menu_bar(
        &self,
        menus: &Vec<Menu>,
        delegate: &ProtocolObject<dyn NSMenuDelegate>,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenu> {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let application_menu = NSMenu::new(mtm);
            application_menu.setDelegate(Some(delegate));

            for menu_config in menus {
                let menu = NSMenu::new(mtm);
                let menu_title = NSString::from_str(&menu_config.name);
                menu.setTitle(&menu_title);
                menu.setDelegate(Some(delegate));

                for item_config in &menu_config.items {
                    let item = Self::create_menu_item(item_config, delegate, actions, keymap);
                    menu.addItem(&item);
                }

                let menu_item = NSMenuItem::new(mtm);
                menu_item.setTitle(&menu_title);
                menu_item.setSubmenu(Some(&menu));
                application_menu.addItem(&menu_item);

                if menu_config.name == "Window" {
                    let app = NSApplication::sharedApplication(mtm);
                    app.setWindowsMenu(Some(&menu));
                }
            }

            application_menu
        }
    }

    pub(super) fn create_dock_menu(
        &self,
        menu_items: Vec<MenuItem>,
        delegate: &ProtocolObject<dyn NSMenuDelegate>,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenu> {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let dock_menu = NSMenu::new(mtm);
            dock_menu.setDelegate(Some(delegate));
            for item_config in menu_items {
                let item = Self::create_menu_item(&item_config, delegate, actions, keymap);
                dock_menu.addItem(&item);
            }

            dock_menu
        }
    }

    fn create_menu_item(
        item: &MenuItem,
        delegate: &ProtocolObject<dyn NSMenuDelegate>,
        actions: &mut Vec<Box<dyn Action>>,
        keymap: &Keymap,
    ) -> Retained<NSMenuItem> {
        static DEFAULT_CONTEXT: OnceLock<Vec<KeyContext>> = OnceLock::new();

        unsafe {
            let mtm = MainThreadMarker::new_unchecked();

            match item {
                MenuItem::Separator => NSMenuItem::separatorItem(mtm),
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
                        Some(inazuma::OsAction::Cut) => sel!(cut:),
                        Some(inazuma::OsAction::Copy) => sel!(copy:),
                        Some(inazuma::OsAction::Paste) => sel!(paste:),
                        Some(inazuma::OsAction::SelectAll) => sel!(selectAll:),
                        Some(inazuma::OsAction::Undo) => sel!(handleGPUIMenuItem:),
                        Some(inazuma::OsAction::Redo) => sel!(handleGPUIMenuItem:),
                        None => sel!(handleGPUIMenuItem:),
                    };

                    let item;
                    if let Some(keystrokes) = keystrokes {
                        if keystrokes.len() == 1 {
                            let keystroke = &keystrokes[0];
                            let mut mask = NSEventModifierFlags::empty();
                            for (modifier, flag) in &[
                                (keystroke.modifiers().platform, NSEventModifierFlags::Command),
                                (keystroke.modifiers().control, NSEventModifierFlags::Control),
                                (keystroke.modifiers().alt, NSEventModifierFlags::Option),
                                (keystroke.modifiers().shift, NSEventModifierFlags::Shift),
                            ] {
                                if *modifier {
                                    mask |= *flag;
                                }
                            }

                            let title = NSString::from_str(name);
                            let key_equiv =
                                NSString::from_str(key_to_native(keystroke.key()).as_ref());
                            item = NSMenuItem::initWithTitle_action_keyEquivalent(
                                mtm.alloc(),
                                &title,
                                Some(selector),
                                &key_equiv,
                            );
                            if Self::os_version() >= Version::new(12, 0, 0) {
                                item.setAllowsAutomaticKeyEquivalentLocalization(false);
                            }
                            item.setKeyEquivalentModifierMask(mask);
                        } else {
                            let title = NSString::from_str(name);
                            let empty = NSString::from_str("");
                            item = NSMenuItem::initWithTitle_action_keyEquivalent(
                                mtm.alloc(),
                                &title,
                                Some(selector),
                                &empty,
                            );
                        }
                    } else {
                        let title = NSString::from_str(name);
                        let empty = NSString::from_str("");
                        item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            mtm.alloc(),
                            &title,
                            Some(selector),
                            &empty,
                        );
                    }

                    if *checked {
                        item.setState(objc2_app_kit::NSControlStateValueOn);
                    }
                    item.setEnabled(!*disabled);

                    let tag = actions.len() as NSInteger;
                    item.setTag(tag);
                    actions.push(action.boxed_clone());
                    item
                }
                MenuItem::Submenu(Menu { name, items, disabled }) => {
                    let item = NSMenuItem::new(mtm);
                    let submenu = NSMenu::new(mtm);
                    submenu.setDelegate(Some(delegate));
                    for sub_item in items {
                        let mi = Self::create_menu_item(sub_item, delegate, actions, keymap);
                        submenu.addItem(&mi);
                    }
                    item.setSubmenu(Some(&submenu));
                    item.setEnabled(!*disabled);
                    item.setTitle(&NSString::from_str(name));
                    item
                }
                MenuItem::SystemMenu(OsMenu { name, menu_type }) => {
                    let item = NSMenuItem::new(mtm);
                    let submenu = NSMenu::new(mtm);
                    submenu.setDelegate(Some(delegate));
                    item.setSubmenu(Some(&submenu));
                    item.setTitle(&NSString::from_str(name));

                    match menu_type {
                        SystemMenuType::Services => {
                            let app = NSApplication::sharedApplication(mtm);
                            app.setServicesMenu(Some(&submenu));
                        }
                    }

                    item
                }
            }
        }
    }

    pub(super) fn os_version() -> Version {
        let version = NSProcessInfo::processInfo().operatingSystemVersion();
        Version::new(
            version.majorVersion as u64,
            version.minorVersion as u64,
            version.patchVersion as u64,
        )
    }
}
