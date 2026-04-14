//! Component Preview Example
//!
//! Run with: `cargo run -p raijin-component-preview --example component_preview"`
use raijin_fs::RealFs;
use inazuma::{AppContext as _, Bounds, KeyBinding, WindowBounds, WindowOptions, actions, size};

use raijin_client::{Client, UserStore};
use raijin_language::LanguageRegistry;
use raijin_node_runtime::NodeRuntime;
use raijin_project::Project;
use raijin_reqwest_client::ReqwestClient;
use raijin_session::{AppSession, Session};
use std::sync::Arc;
use raijin_ui::{App, px};
use raijin_workspace::{AppState, Workspace};

use raijin_component_preview::{ComponentPreview, init};

actions!(raijin, [Quit]);

fn quit(_: &Quit, cx: &mut App) {
    cx.quit();
}

fn main() {
    inazuma_platform::application().run(|cx| {
        raijin_component::init();

        cx.on_action(quit);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        let version = raijin_release_channel::AppVersion::load(env!("CARGO_PKG_VERSION"), None, None);
        raijin_release_channel::init(version, cx);

        let http_client =
            ReqwestClient::user_agent("component_preview").expect("Failed to create HTTP client");
        cx.set_http_client(Arc::new(http_client));

        let fs = Arc::new(RealFs::new(None, cx.background_executor().clone()));
        <dyn raijin_fs::Fs>::set_global(fs.clone(), cx);

        inazuma_settings_framework::init(cx);
        raijin_theme_settings::init(raijin_theme::LoadThemes::JustBase, cx);

        let languages = Arc::new(LanguageRegistry::new(cx.background_executor().clone()));
        let client = Client::production(cx);
        raijin_client::init(&client, cx);

        let user_store = cx.new(|cx| UserStore::new(client.clone(), cx));
        let session_id = uuid::Uuid::new_v4().to_string();
        let kvp = raijin_db::kvp::KeyValueStore::global(cx);
        let session = cx
            .foreground_executor()
            .block_on(Session::new(session_id, kvp));
        let session = cx.new(|cx| AppSession::new(session, cx));
        let node_runtime = NodeRuntime::unavailable();

        let app_state = Arc::new(AppState {
            languages,
            client,
            user_store,
            fs,
            build_window_options: |_, _| Default::default(),
            node_runtime,
            session,
        });
        AppState::set_global(Arc::downgrade(&app_state), cx);

        raijin_workspace::init(app_state.clone(), cx);
        init(app_state.clone(), cx);

        let size = size(px(1200.), px(800.));
        let bounds = Bounds::centered(None, size, cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            {
                move |window, cx| {
                    let app_state = app_state;
                    raijin_theme_settings::setup_ui_font(window, cx);

                    let project = Project::local(
                        app_state.client.clone(),
                        app_state.node_runtime.clone(),
                        app_state.user_store.clone(),
                        app_state.languages.clone(),
                        app_state.fs.clone(),
                        None,
                        raijin_project::LocalProjectFlags {
                            init_worktree_trust: false,
                            ..Default::default()
                        },
                        cx,
                    );

                    let workspace = cx.new(|cx| {
                        Workspace::new(
                            Default::default(),
                            project.clone(),
                            app_state.clone(),
                            window,
                            cx,
                        )
                    });

                    workspace.update(cx, |workspace, cx| {
                        let weak_workspace = cx.entity().downgrade();
                        let language_registry = app_state.languages.clone();
                        let user_store = app_state.user_store.clone();

                        let component_preview = cx.new(|cx| {
                            ComponentPreview::new(
                                weak_workspace,
                                project,
                                language_registry,
                                user_store,
                                None,
                                None,
                                window,
                                cx,
                            )
                            .expect("Failed to create component preview")
                        });

                        workspace.add_item_to_active_pane(
                            Box::new(component_preview),
                            None,
                            true,
                            window,
                            cx,
                        );
                    });

                    workspace
                }
            },
        )
        .expect("Failed to open component preview window");

        cx.activate(true);
    });
}
