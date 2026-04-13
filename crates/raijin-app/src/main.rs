mod app_bootstrap;

use std::borrow::Cow;
use std::sync::Arc;

use futures::StreamExt;
use inazuma::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, actions, px, size};
use inazuma_settings_framework::{SettingsStore, watch_config_file};
use raijin_settings::AppearanceSettings;
use raijin_ui::AppShell;
use raijin_ui::TitleBar;
use raijin_actions::Quit;

// Global actions — available everywhere (OpenSettings and Quit come from raijin-actions)
actions!(
    raijin,
    [
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
    if std::env::args().any(|arg| arg == "--printenv") {
        inazuma_util::shell_env::print_env();
        return;
    }

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

        // 1. Initialize SettingsStore with defaults from assets/settings/default.toml
        //    All #[derive(RegisterSetting)] types are automatically registered via inventory
        inazuma_settings_framework::init(cx);

        // 2. Initialize ReleaseChannel (required before Client::production)
        let app_version = semver::Version::new(0, 1, 0);
        raijin_release_channel::init(app_version, cx);

        // 3. Initialize database (required before build_app_state which uses KeyValueStore)
        let app_db = raijin_db::AppDatabase::new();
        cx.set_global(app_db);

        // 4. Build AppState (Client, Session, UserStore, WorkspaceStore, FS, Languages)
        let app_state = app_bootstrap::build_app_state(cx);

        // 4. Watch settings.toml + keymap.toml via Fs::watch
        let fs = app_state.fs.clone();
        handle_settings_file(fs.clone(), cx);
        handle_keymap_file(fs.clone(), cx);

        // 5. Initialize ProjectRegistry (reactive git-root-based project detection)
        raijin_project_registry::ProjectRegistry::init(
            app_state.client.clone(),
            app_state.user_store.clone(),
            app_state.languages.clone(),
            app_state.fs.clone(),
            raijin_node_runtime::NodeRuntime::unavailable(),
            cx,
        );

        // 5. Initialize theme system (ThemeSettings via SettingsStore + Provider registration)
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

        // Set up globals for shell switching (used by raijin-terminal-view)
        cx.set_global(raijin_terminal_view::terminal_pane::PendingShellSwitch(None));
        cx.set_global(raijin_terminal_view::terminal_pane::PendingBranchSwitch(None));
        cx.set_global(raijin_terminal_view::terminal_pane::PendingShellInstallName(None));

        // Initialize UI components (keybindings, global state)
        raijin_ui::init(cx);

        // Initialize workspace system + feature crates
        raijin_workspace::init(app_state.clone(), cx);
        raijin_call::init(app_state.client.clone(), app_state.user_store.clone(), cx);
        raijin_terminal_view::init(cx);
        raijin_title_bar::init(cx);
        raijin_command_palette::init(cx);
        raijin_tab_switcher::init(cx);
        raijin_settings_ui::init(cx);

        // Read colorspace from SettingsStore (already loaded by handle_settings_file)
        let colorspace = {
            use inazuma_settings_framework::Settings;
            AppearanceSettings::get_global(cx).window_colorspace
        };

        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                colorspace,
                ..Default::default()
            },
            |window, cx| {
                // Empty project — worktrees are added reactively when the
                // shell CWD enters a git repo (via OSC 7/7777 + git root detection).
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

                // Create the Workspace
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
                    let terminal = cx.new(|cx| raijin_terminal_view::terminal_pane::TerminalPane::new(window, cx));
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
                cx.new(|cx| AppShell::new(workspace, window, cx))
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

/// Watches ~/.raijin/settings.toml via Fs::watch and loads changes into SettingsStore.
///
/// Uses the same pattern as the reference: `watch_config_file()` from inazuma-settings-framework,
/// initial blocking load + async continuous watching.
fn handle_settings_file(fs: Arc<dyn raijin_fs::Fs>, cx: &mut App) {
    let settings_path = raijin_paths::settings_file().clone();

    let (mut rx, _task) = watch_config_file(
        &cx.background_executor(),
        fs,
        settings_path,
    );

    // Initial load (blocking) — settings must be available before window opens
    if let Some(content) = cx.foreground_executor().block_on(rx.next()) {
        SettingsStore::update(cx, |store, cx| {
            let _ = store.set_user_settings(&content, cx);
        });
    }

    // Continuous watching (async)
    cx.spawn(async move |cx| {
        while let Some(content) = rx.next().await {
            cx.update(|cx| {
                SettingsStore::update(cx, |store, cx| {
                    let _ = store.set_user_settings(&content, cx);
                });
                cx.refresh_windows();
                log::info!("Settings reloaded from disk");
            });
        }
    })
    .detach();
}

/// Watches ~/.raijin/keymap.toml via Fs::watch and reloads keybindings on change.
fn handle_keymap_file(fs: Arc<dyn raijin_fs::Fs>, cx: &mut App) {
    let keymap_path = raijin_paths::keymap_file().clone();

    let (mut rx, _task) = watch_config_file(
        &cx.background_executor(),
        fs,
        keymap_path,
    );

    // Continuous watching — keymaps are loaded separately via load_default_and_user_keymap
    cx.spawn(async move |cx| {
        while let Some(_content) = rx.next().await {
            cx.update(|cx| {
                cx.clear_key_bindings();
                raijin_settings::keymap_file::load_default_and_user_keymap(cx);
                log::info!("Keybindings reloaded from disk");
            });
        }
    })
    .detach();
}
