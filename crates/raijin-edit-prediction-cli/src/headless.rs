use raijin_client::{Client, ProxySettings, UserStore};
use raijin_db::AppDatabase;
use raijin_extension::ExtensionHostProxy;
use raijin_fs::RealFs;
use inazuma::http_client::read_proxy_from_env;
use inazuma::{App, AppContext, Entity};
use inazuma_tokio::Tokio;
use raijin_language::LanguageRegistry;
use raijin_language_extension::LspAccess;
use raijin_node_runtime::{NodeBinaryOptions, NodeRuntime};
use raijin_project::project_settings::ProjectSettings;
use raijin_release_channel::{AppCommitSha, AppVersion};
use raijin_reqwest_client::ReqwestClient;
use inazuma_settings_framework::{Settings, SettingsStore};
use std::path::PathBuf;
use std::sync::Arc;
use inazuma_util::ResultExt as _;

/// Headless subset of `workspace::AppState`.
pub struct EpAppState {
    pub languages: Arc<LanguageRegistry>,
    pub client: Arc<Client>,
    pub user_store: Entity<UserStore>,
    pub fs: Arc<dyn raijin_fs::Fs>,
    pub node_runtime: NodeRuntime,
}

pub fn init(cx: &mut App) -> EpAppState {
    let app_commit_sha = option_env!("RAIJIN_COMMIT_SHA").map(|s| AppCommitSha::new(s.to_owned()));

    let app_version = AppVersion::load(
        env!("RAIJIN_PKG_VERSION"),
        option_env!("RAIJIN_BUILD_ID"),
        app_commit_sha,
    );
    raijin_release_channel::init(app_version.clone(), cx);
    inazuma_tokio::init(cx);

    let settings_store = SettingsStore::new(cx, &inazuma_settings_framework::default_settings());
    cx.set_global(settings_store);

    // Set User-Agent so we can download language servers from GitHub
    let user_agent = format!(
        "Raijin Edit Prediction CLI/{} ({}; {})",
        app_version,
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let proxy_str = ProxySettings::get_global(cx).proxy.to_owned();
    let proxy_url = proxy_str
        .as_ref()
        .and_then(|input| input.parse().ok())
        .or_else(read_proxy_from_env);
    let http = {
        let _guard = Tokio::handle(cx).enter();

        ReqwestClient::proxy_and_user_agent(proxy_url, &user_agent)
            .expect("could not start HTTP client")
    };
    cx.set_http_client(Arc::new(http));

    let client = Client::production(cx);
    cx.set_http_client(client.http_client());

    let app_db = AppDatabase::new();
    cx.set_global(app_db);

    let git_binary_path = None;
    let fs = Arc::new(RealFs::new(
        git_binary_path,
        cx.background_executor().clone(),
    ));

    let mut languages = LanguageRegistry::new(cx.background_executor().clone());
    languages.set_language_server_download_dir(raijin_paths::languages_dir().clone());
    let languages = Arc::new(languages);

    let user_store = cx.new(|cx| UserStore::new(client.clone(), cx));

    raijin_extension::init(cx);

    let (mut tx, rx) = raijin_watch::channel(None);
    cx.observe_global::<SettingsStore>(move |cx| {
        let settings = &ProjectSettings::get_global(cx).node;
        let options = NodeBinaryOptions {
            allow_path_lookup: !settings.ignore_system_version,
            allow_binary_download: true,
            use_paths: settings.path.as_ref().map(|node_path| {
                let node_path = PathBuf::from(shellexpand::tilde(node_path).as_ref());
                let npm_path = settings
                    .npm_path
                    .as_ref()
                    .map(|path| PathBuf::from(shellexpand::tilde(&path).as_ref()));
                (
                    node_path.clone(),
                    npm_path.unwrap_or_else(|| {
                        let base_path = PathBuf::new();
                        node_path.parent().unwrap_or(&base_path).join("npm")
                    }),
                )
            }),
        };
        tx.send(Some(options)).log_err();
    })
    .detach();
    let node_runtime = NodeRuntime::new(client.http_client(), None, rx);

    let extension_host_proxy = ExtensionHostProxy::global(cx);

    raijin_debug_adapter_extension::init(extension_host_proxy.clone(), cx);
    raijin_language_extension::init(LspAccess::Noop, extension_host_proxy, languages.clone());
    raijin_language_model::init(user_store.clone(), client.clone(), cx);
    raijin_language_models::init(user_store.clone(), client.clone(), cx);
    raijin_languages::init(languages.clone(), fs.clone(), node_runtime.clone(), cx);
    raijin_prompt_store::init(cx);

    EpAppState {
        languages,
        client,
        user_store,
        fs,
        node_runtime,
    }
}
