mod app_bootstrap;
mod command_history;
mod completions;
mod input;
mod settings_view;
mod shell_install;
mod terminal;
mod terminal_pane;
mod workspace;

use std::borrow::Cow;
use inazuma::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, actions, px, size};
use inazuma_component::Root;
use inazuma_component::TitleBar;
use inazuma_settings_framework::SettingsStore;

// Global actions — available everywhere
actions!(
    raijin,
    [
        Quit,
        OpenSettings,
        ToggleCommandPalette,
        ToggleThemeSelector,
        NewWindow,
        CloseWindow,
        IncreaseFontSize,
        DecreaseFontSize,
        ResetFontSize,
    ]
);

// Terminal actions
actions!(
    terminal,
    [
        Copy,
        Paste,
        Clear,
        NewTab,
        NextTab,
        PreviousTab,
        ScrollPageUp,
        ScrollPageDown,
        ScrollToTop,
        ScrollToBottom,
        Find,
    ]
);

// Input actions (Raijin Mode)
actions!(
    input,
    [
        Submit,
        AcceptCompletion,
        HistoryPrev,
        HistoryNext,
        Cancel,
        Interrupt,
    ]
);

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,raijin=debug,raijin_term=debug")
    ).init();

    Application::new()
        .with_assets(raijin_assets::Assets)
        .run(|cx: &mut App| {
        // Register bundled terminal fonts via asset pipeline
        let font_paths = ["fonts/dankmono-nerd-font-mono/DankMonoNerdFontMono-Regular.otf", "fonts/dankmono-nerd-font-mono/DankMonoNerdFontMono-Bold.otf"];
        let bundled_fonts: Vec<Cow<'static, [u8]>> = font_paths
            .iter()
            .filter_map(|path| cx.asset_source().load(path).ok().flatten())
            .collect();
        cx.text_system().add_fonts(bundled_fonts).expect("failed to register bundled fonts");

        inazuma_component::init(cx);
        inazuma_component::theme::Theme::change(
            inazuma_component::theme::ThemeMode::Dark,
            None,
            cx,
        );

        // 1. Initialize SettingsStore with defaults from assets/settings/default.toml
        //    All #[derive(RegisterSetting)] types are automatically registered via inventory
        inazuma_settings_framework::init(cx);

        // 2. Load user settings from ~/.raijin/settings.toml into SettingsStore
        let settings_path = raijin_settings::RaijinSettings::settings_path();
        if let Ok(content) = std::fs::read_to_string(&settings_path) {
            SettingsStore::update_global(cx, |store, cx| {
                store.set_user_settings(&content, cx);
            });
        }

        // 3. Also keep RaijinSettings loaded for Raijin-specific settings
        //    (working_directory, input_mode, colorspace, symbol_map, etc.)
        //    These will migrate to SettingsStore once GeneralSettings/AppearanceSettings are added
        let raijin_settings = raijin_settings::RaijinSettings::load().unwrap_or_default();
        let colorspace = raijin_settings.appearance.window_colorspace;
        cx.set_global(raijin_settings);

        // 4. Initialize theme system (ThemeSettings via SettingsStore + Provider registration)
        //    MUST happen after SettingsStore init — registers ThemeSettingsProvider for raijin-ui
        raijin_theme_settings::init(raijin_theme::LoadThemes::All(Box::new(raijin_assets::Assets)), cx);

        // Register global action handlers
        cx.on_action::<Quit>(|_, cx| cx.quit());
        cx.on_action::<CloseWindow>(|_, cx| {
            cx.defer(|cx| {
                cx.windows().iter().find(|window| {
                    window
                        .update(cx, |_, window, _| {
                            if window.is_window_active() {
                                window.remove_window();
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false)
                });
            });
        });

        // Load keybindings from default + user keymap
        raijin_settings::keymap_file::load_default_and_user_keymap(cx);

        // File watchers — hot-reload settings, themes, and keymaps on change
        start_file_watchers(cx);

        // Set up globals for shell switching (used by terminal_pane)
        cx.set_global(terminal_pane::PendingShellSwitch(None));
        cx.set_global(terminal_pane::PendingShellInstallName(None));

        // Initialize ReleaseChannel (required before Client::production)
        let app_version = semver::Version::new(0, 1, 0);
        raijin_release_channel::init(app_version, cx);

        // Build AppState (Client, Session, UserStore, WorkspaceStore, FS, Languages)
        let app_state = app_bootstrap::build_app_state(cx);

        // Initialize workspace system + feature crates
        raijin_workspace::init(app_state.clone(), cx);
        raijin_title_bar::init(cx);
        raijin_command_palette::init(cx);
        raijin_tab_switcher::init(cx);

        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                colorspace,
                ..Default::default()
            },
            |window, cx| {
                // Create a local Project (no remote connection)
                let project = raijin_project::Project::local(
                    app_state.client.clone(),
                    raijin_node_runtime::NodeRuntime::unavailable(),
                    app_state.user_store.clone(),
                    app_state.languages.clone(),
                    app_state.fs.clone(),
                    None,
                    raijin_project::LocalProjectFlags::default(),
                    cx,
                );

                // Create the Workspace (Zed's full workspace system)
                let workspace = cx.new(|cx| {
                    raijin_workspace::Workspace::new(
                        None,
                        project,
                        app_state.clone(),
                        window,
                        cx,
                    )
                });

                // Add our terminal as the first item + configure workspace
                workspace.update(cx, |ws, cx| {
                    // Create and add terminal pane
                    let terminal = cx.new(|cx| terminal_pane::TerminalPane::new(window, cx));
                    ws.add_item_to_active_pane(
                        Box::new(terminal),
                        None,
                        true,
                        window,
                        cx,
                    );

                    // Hide docks (not needed for terminal-first experience)
                    ws.left_dock().update(cx, |d, cx| d.set_open(false, window, cx));
                    ws.right_dock().update(cx, |d, cx| d.set_open(false, window, cx));
                    ws.bottom_dock().update(cx, |d, cx| d.set_open(false, window, cx));
                });

                // Root wraps Workspace (provides Sheets, Dialogs, Notifications)
                cx.new(|cx| Root::new(workspace, window, cx))
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

/// Starts file watchers for settings.toml and the themes directory.
/// Changes are hot-reloaded without requiring an app restart.
fn start_file_watchers(cx: &mut App) {
    // Watch settings.toml — reload into SettingsStore + RaijinSettings on change
    let settings_path = raijin_settings::RaijinSettings::settings_path();
    let (settings_rx, _settings_handle) = raijin_settings::watcher::watch_file(settings_path);
    cx.spawn(async move |cx| {
        while let Ok(_path) = settings_rx.recv_async().await {
            cx.update(|cx| {
                let settings_path = raijin_settings::RaijinSettings::settings_path();
                if let Ok(content) = std::fs::read_to_string(&settings_path) {
                    // Update SettingsStore (triggers theme/font observers automatically)
                    SettingsStore::update_global(cx, |store, cx| {
                        store.set_user_settings(&content, cx);
                    });
                }
                // Also reload RaijinSettings for Raijin-specific values
                match raijin_settings::RaijinSettings::load() {
                    Ok(new_settings) => {
                        cx.set_global(new_settings);
                        cx.refresh_windows();
                        log::info!("Settings reloaded from disk");
                    }
                    Err(err) => {
                        log::warn!("Failed to reload settings: {err}");
                    }
                }
            });
        }
    })
    .detach();

    // Watch ~/.raijin/themes/ — reload changed theme files
    let themes_dir = raijin_settings::RaijinSettings::themes_dir();
    if !themes_dir.exists() {
        std::fs::create_dir_all(&themes_dir).ok();
    }
    let (themes_rx, _themes_handle) = raijin_settings::watcher::watch_dir(themes_dir);
    cx.spawn(async move |cx| {
        while let Ok(changed_path) = themes_rx.recv_async().await {
            if changed_path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            cx.update(|cx| {
                let content = match std::fs::read_to_string(&changed_path) {
                    Ok(c) => c,
                    Err(err) => {
                        log::warn!("Failed to read changed theme '{}': {err}", changed_path.display());
                        return;
                    }
                };
                let id = changed_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                let base_dir = changed_path.parent().map(|p| p.to_path_buf());

                match raijin_theme::load_theme_from_toml_with_base_dir(id, &content, base_dir) {
                    Ok(theme) => {
                        let theme_name = theme.name.clone();
                        let registry = raijin_theme::ThemeRegistry::global(cx);
                        registry.insert_theme(theme);
                        log::info!("Hot-reloaded theme: {theme_name}");

                        // If the active theme was the one that changed, refresh it
                        let active = raijin_theme::GlobalTheme::theme(cx).clone();
                        if active.name == theme_name {
                            raijin_theme_settings::reload_theme(cx);
                        }
                    }
                    Err(err) => {
                        log::warn!("Failed to parse changed theme '{}': {err}", changed_path.display());
                    }
                }
            });
        }
    })
    .detach();

    // Watch ~/.raijin/keymap.toml — reload keybindings on change
    let keymap_path = raijin_settings::RaijinSettings::keymap_path();
    let (keymap_rx, _keymap_handle) = raijin_settings::watcher::watch_file(keymap_path);
    cx.spawn(async move |cx| {
        while let Ok(_path) = keymap_rx.recv_async().await {
            cx.update(|cx| {
                cx.clear_key_bindings();
                raijin_settings::keymap_file::load_default_and_user_keymap(cx);
                log::info!("Keybindings reloaded from disk");
            });
        }
    })
    .detach();
}
