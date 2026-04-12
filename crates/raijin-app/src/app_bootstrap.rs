use std::sync::Arc;

use inazuma::{App, AppContext};
use raijin_client::{Client, UserStore};
use raijin_db::kvp::KeyValueStore;
use raijin_fs::RealFs;
use raijin_language::LanguageRegistry;
use raijin_node_runtime::NodeRuntime;
use raijin_session::{AppSession, Session};
use raijin_workspace::{AppState, WorkspaceStore};

/// Build the full `AppState` needed by `raijin_workspace::Workspace::new()`.
///
/// This follows the same pattern as Zed's production init, but without an active
/// collaboration server connection. All subsystems (Client, UserStore, WorkspaceStore,
/// LanguageRegistry, Fs, Session) are created with real implementations — they're
/// just not connected to any remote server.
///
/// **Prerequisites (must be called before this function):**
/// - `raijin_release_channel::init(app_version, cx)` — sets `GlobalReleaseChannel`
/// - `inazuma_settings_framework::init(cx)` — sets up `SettingsStore` with defaults
pub fn build_app_state(cx: &mut App) -> Arc<AppState> {
    // Client::production(cx) creates:
    // - Arc<RealSystemClock>
    // - Arc<HttpClientWithUrl> from ClientSettings::get_global(cx).server_url
    // - Client::new(clock, http, cx) with CredentialsProvider
    let client = Client::production(cx);

    // Session is async: pub async fn new(session_id: String, db: KeyValueStore)
    // Only ForegroundExecutor has block_on() (executor.rs:456)
    let db = KeyValueStore::global(cx);
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = cx
        .foreground_executor()
        .block_on(Session::new(session_id, db));
    let session = cx.new(|cx| AppSession::new(session, cx));

    let user_store = cx.new(|cx| raijin_client::UserStore::new(client.clone(), cx));
    let workspace_store = cx.new(|cx| WorkspaceStore::new(client.clone(), cx));

    let languages = Arc::new(LanguageRegistry::new(cx.background_executor().clone()));
    let fs: Arc<dyn raijin_fs::Fs> = Arc::new(RealFs::new(None, cx.background_executor().clone()));

    raijin_client::init(&client, cx);

    let app_state = Arc::new(AppState {
        client,
        fs,
        languages,
        user_store,
        workspace_store,
        node_runtime: NodeRuntime::unavailable(),
        build_window_options: |_, _| Default::default(),
        session,
    });

    AppState::set_global(Arc::downgrade(&app_state), cx);

    app_state
}
