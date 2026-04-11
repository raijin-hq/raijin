use crate::wasm_host::wit::since_v0_6_0::{
    dap::{
        BuildTaskDefinition, BuildTaskDefinitionTemplatePayload, StartDebuggingRequestArguments,
        TcpArguments, TcpArgumentsTemplate,
    },
    slash_command::SlashCommandOutputSection,
};
use crate::wasm_host::wit::{CompletionKind, CompletionLabelDetails, InsertTextFormat, SymbolKind};
use crate::wasm_host::{WasmState, wit::ToWasmtimeResult};
use ::raijin_http_client::{AsyncBody, HttpRequestExt};
use ::inazuma_settings_framework::{Settings, WorktreeId};
use anyhow::{Context as _, Result, bail};
use async_compression::futures::bufread::GzipDecoder;
use async_tar::Archive;
use async_trait::async_trait;
use raijin_extension::{
    ExtensionLanguageServerProxy, KeyValueStoreDelegate, ProjectDelegate, WorktreeDelegate,
};
use futures::{AsyncReadExt, lock::Mutex};
use futures::{FutureExt as _, io::BufReader};
use inazuma::{BackgroundExecutor, SharedString};
use raijin_language::{BinaryStatus, LanguageName, language_settings::AllLanguageSettings};
use raijin_project::project_settings::ProjectSettings;
use semver::Version;
use std::{
    env,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, OnceLock},
};
use raijin_task::{SpawnInTerminal, ZedDebugConfig};
use url::Url;
use inazuma_util::{
    archive::extract_zip, fs::make_file_executable, maybe, paths::PathStyle, rel_path::RelPath,
};
use wasmtime::component::{Linker, Resource};

pub const MIN_VERSION: Version = Version::new(0, 8, 0);
pub const MAX_VERSION: Version = Version::new(0, 8, 0);

wasmtime::component::bindgen!({
    async: true,
    trappable_imports: true,
    path: "../raijin-extension-api/wit/since_v0.8.0",
    with: {
         "worktree": ExtensionWorktree,
         "project": ExtensionProject,
         "key-value-store": ExtensionKeyValueStore,
         "zed:extension/http-client/http-response-stream": ExtensionHttpResponseStream
    },
});

pub use self::zed::extension::*;

mod settings {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/since_v0.8.0/settings.rs"));
}

pub type ExtensionWorktree = Arc<dyn WorktreeDelegate>;
pub type ExtensionProject = Arc<dyn ProjectDelegate>;
pub type ExtensionKeyValueStore = Arc<dyn KeyValueStoreDelegate>;
pub type ExtensionHttpResponseStream = Arc<Mutex<::raijin_http_client::Response<AsyncBody>>>;

pub fn linker(executor: &BackgroundExecutor) -> &'static Linker<WasmState> {
    static LINKER: OnceLock<Linker<WasmState>> = OnceLock::new();
    LINKER.get_or_init(|| super::new_linker(executor, Extension::add_to_linker))
}

impl From<Range> for std::ops::Range<usize> {
    fn from(range: Range) -> Self {
        let start = range.start as usize;
        let end = range.end as usize;
        start..end
    }
}

impl From<Command> for raijin_extension::Command {
    fn from(value: Command) -> Self {
        Self {
            command: value.command.into(),
            args: value.args,
            env: value.env,
        }
    }
}

impl From<StartDebuggingRequestArgumentsRequest>
    for raijin_extension::StartDebuggingRequestArgumentsRequest
{
    fn from(value: StartDebuggingRequestArgumentsRequest) -> Self {
        match value {
            StartDebuggingRequestArgumentsRequest::Launch => Self::Launch,
            StartDebuggingRequestArgumentsRequest::Attach => Self::Attach,
        }
    }
}
impl TryFrom<StartDebuggingRequestArguments> for raijin_extension::StartDebuggingRequestArguments {
    type Error = anyhow::Error;

    fn try_from(value: StartDebuggingRequestArguments) -> Result<Self, Self::Error> {
        Ok(Self {
            configuration: serde_json::from_str(&value.configuration)?,
            request: value.request.into(),
        })
    }
}
impl From<TcpArguments> for raijin_extension::TcpArguments {
    fn from(value: TcpArguments) -> Self {
        Self {
            host: value.host.into(),
            port: value.port,
            timeout: value.timeout,
        }
    }
}

impl From<raijin_extension::TcpArgumentsTemplate> for TcpArgumentsTemplate {
    fn from(value: raijin_extension::TcpArgumentsTemplate) -> Self {
        Self {
            host: value.host.map(Ipv4Addr::to_bits),
            port: value.port,
            timeout: value.timeout,
        }
    }
}

impl From<TcpArgumentsTemplate> for raijin_extension::TcpArgumentsTemplate {
    fn from(value: TcpArgumentsTemplate) -> Self {
        Self {
            host: value.host.map(Ipv4Addr::from_bits),
            port: value.port,
            timeout: value.timeout,
        }
    }
}

impl TryFrom<raijin_extension::DebugTaskDefinition> for DebugTaskDefinition {
    type Error = anyhow::Error;
    fn try_from(value: raijin_extension::DebugTaskDefinition) -> Result<Self, Self::Error> {
        Ok(Self {
            label: value.label.to_string(),
            adapter: value.adapter.to_string(),
            config: value.config.to_string(),
            tcp_connection: value.tcp_connection.map(Into::into),
        })
    }
}

impl From<raijin_task::DebugRequest> for DebugRequest {
    fn from(value: raijin_task::DebugRequest) -> Self {
        match value {
            raijin_task::DebugRequest::Launch(launch_request) => Self::Launch(launch_request.into()),
            raijin_task::DebugRequest::Attach(attach_request) => Self::Attach(attach_request.into()),
        }
    }
}

impl From<DebugRequest> for raijin_task::DebugRequest {
    fn from(value: DebugRequest) -> Self {
        match value {
            DebugRequest::Launch(launch_request) => Self::Launch(launch_request.into()),
            DebugRequest::Attach(attach_request) => Self::Attach(attach_request.into()),
        }
    }
}

impl From<raijin_task::LaunchRequest> for LaunchRequest {
    fn from(value: raijin_task::LaunchRequest) -> Self {
        Self {
            program: value.program,
            cwd: value.cwd.map(|p| p.to_string_lossy().into_owned()),
            args: value.args,
            envs: value.env.into_iter().collect(),
        }
    }
}

impl From<raijin_task::AttachRequest> for AttachRequest {
    fn from(value: raijin_task::AttachRequest) -> Self {
        Self {
            process_id: value.process_id,
        }
    }
}

impl From<LaunchRequest> for raijin_task::LaunchRequest {
    fn from(value: LaunchRequest) -> Self {
        Self {
            program: value.program,
            cwd: value.cwd.map(|p| p.into()),
            args: value.args,
            env: value.envs.into_iter().collect(),
        }
    }
}
impl From<AttachRequest> for raijin_task::AttachRequest {
    fn from(value: AttachRequest) -> Self {
        Self {
            process_id: value.process_id,
        }
    }
}

impl From<ZedDebugConfig> for DebugConfig {
    fn from(value: ZedDebugConfig) -> Self {
        Self {
            label: value.label.into(),
            adapter: value.adapter.into(),
            request: value.request.into(),
            stop_on_entry: value.stop_on_entry,
        }
    }
}
impl TryFrom<DebugAdapterBinary> for raijin_extension::DebugAdapterBinary {
    type Error = anyhow::Error;
    fn try_from(value: DebugAdapterBinary) -> Result<Self, Self::Error> {
        Ok(Self {
            command: value.command,
            arguments: value.arguments,
            envs: value.envs.into_iter().collect(),
            cwd: value.cwd.map(|s| s.into()),
            connection: value.connection.map(Into::into),
            request_args: value.request_args.try_into()?,
        })
    }
}

impl From<BuildTaskDefinition> for raijin_extension::BuildTaskDefinition {
    fn from(value: BuildTaskDefinition) -> Self {
        match value {
            BuildTaskDefinition::ByName(name) => Self::ByName(name.into()),
            BuildTaskDefinition::Template(build_task_template) => Self::Template {
                task_template: build_task_template.template.into(),
                locator_name: build_task_template.locator_name.map(SharedString::from),
            },
        }
    }
}

impl From<raijin_extension::BuildTaskDefinition> for BuildTaskDefinition {
    fn from(value: raijin_extension::BuildTaskDefinition) -> Self {
        match value {
            raijin_extension::BuildTaskDefinition::ByName(name) => Self::ByName(name.into()),
            raijin_extension::BuildTaskDefinition::Template {
                task_template,
                locator_name,
            } => Self::Template(BuildTaskDefinitionTemplatePayload {
                template: task_template.into(),
                locator_name: locator_name.map(String::from),
            }),
        }
    }
}
impl From<BuildTaskTemplate> for raijin_extension::BuildTaskTemplate {
    fn from(value: BuildTaskTemplate) -> Self {
        Self {
            label: value.label,
            command: value.command,
            args: value.args,
            env: value.env.into_iter().collect(),
            cwd: value.cwd,
            ..Default::default()
        }
    }
}
impl From<raijin_extension::BuildTaskTemplate> for BuildTaskTemplate {
    fn from(value: raijin_extension::BuildTaskTemplate) -> Self {
        Self {
            label: value.label,
            command: value.command,
            args: value.args,
            env: value.env.into_iter().collect(),
            cwd: value.cwd,
        }
    }
}

impl TryFrom<DebugScenario> for raijin_extension::DebugScenario {
    type Error = anyhow::Error;

    fn try_from(value: DebugScenario) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            adapter: value.adapter.into(),
            label: value.label.into(),
            build: value.build.map(Into::into),
            config: serde_json::Value::from_str(&value.config)?,
            tcp_connection: value.tcp_connection.map(Into::into),
        })
    }
}

impl From<raijin_extension::DebugScenario> for DebugScenario {
    fn from(value: raijin_extension::DebugScenario) -> Self {
        Self {
            adapter: value.adapter.into(),
            label: value.label.into(),
            build: value.build.map(Into::into),
            config: value.config.to_string(),
            tcp_connection: value.tcp_connection.map(Into::into),
        }
    }
}

impl TryFrom<SpawnInTerminal> for ResolvedTask {
    type Error = anyhow::Error;

    fn try_from(value: SpawnInTerminal) -> Result<Self, Self::Error> {
        Ok(Self {
            label: value.label,
            command: value.command.context("missing command")?,
            args: value.args,
            env: value.env.into_iter().collect(),
            cwd: value.cwd.map(|s| {
                let s = s.to_string_lossy();
                if cfg!(target_os = "windows") {
                    s.replace('\\', "/")
                } else {
                    s.into_owned()
                }
            }),
        })
    }
}

impl From<CodeLabel> for raijin_extension::CodeLabel {
    fn from(value: CodeLabel) -> Self {
        Self {
            code: value.code,
            spans: value.spans.into_iter().map(Into::into).collect(),
            filter_range: value.filter_range.into(),
        }
    }
}

impl From<CodeLabelSpan> for raijin_extension::CodeLabelSpan {
    fn from(value: CodeLabelSpan) -> Self {
        match value {
            CodeLabelSpan::CodeRange(range) => Self::CodeRange(range.into()),
            CodeLabelSpan::Literal(literal) => Self::Literal(literal.into()),
        }
    }
}

impl From<CodeLabelSpanLiteral> for raijin_extension::CodeLabelSpanLiteral {
    fn from(value: CodeLabelSpanLiteral) -> Self {
        Self {
            text: value.text,
            highlight_name: value.highlight_name,
        }
    }
}

impl From<raijin_extension::Completion> for Completion {
    fn from(value: raijin_extension::Completion) -> Self {
        Self {
            label: value.label,
            label_details: value.label_details.map(Into::into),
            detail: value.detail,
            kind: value.kind.map(Into::into),
            insert_text_format: value.insert_text_format.map(Into::into),
        }
    }
}

impl From<raijin_extension::CompletionLabelDetails> for CompletionLabelDetails {
    fn from(value: raijin_extension::CompletionLabelDetails) -> Self {
        Self {
            detail: value.detail,
            description: value.description,
        }
    }
}

impl From<raijin_extension::CompletionKind> for CompletionKind {
    fn from(value: raijin_extension::CompletionKind) -> Self {
        match value {
            raijin_extension::CompletionKind::Text => Self::Text,
            raijin_extension::CompletionKind::Method => Self::Method,
            raijin_extension::CompletionKind::Function => Self::Function,
            raijin_extension::CompletionKind::Constructor => Self::Constructor,
            raijin_extension::CompletionKind::Field => Self::Field,
            raijin_extension::CompletionKind::Variable => Self::Variable,
            raijin_extension::CompletionKind::Class => Self::Class,
            raijin_extension::CompletionKind::Interface => Self::Interface,
            raijin_extension::CompletionKind::Module => Self::Module,
            raijin_extension::CompletionKind::Property => Self::Property,
            raijin_extension::CompletionKind::Unit => Self::Unit,
            raijin_extension::CompletionKind::Value => Self::Value,
            raijin_extension::CompletionKind::Enum => Self::Enum,
            raijin_extension::CompletionKind::Keyword => Self::Keyword,
            raijin_extension::CompletionKind::Snippet => Self::Snippet,
            raijin_extension::CompletionKind::Color => Self::Color,
            raijin_extension::CompletionKind::File => Self::File,
            raijin_extension::CompletionKind::Reference => Self::Reference,
            raijin_extension::CompletionKind::Folder => Self::Folder,
            raijin_extension::CompletionKind::EnumMember => Self::EnumMember,
            raijin_extension::CompletionKind::Constant => Self::Constant,
            raijin_extension::CompletionKind::Struct => Self::Struct,
            raijin_extension::CompletionKind::Event => Self::Event,
            raijin_extension::CompletionKind::Operator => Self::Operator,
            raijin_extension::CompletionKind::TypeParameter => Self::TypeParameter,
            raijin_extension::CompletionKind::Other(value) => Self::Other(value),
        }
    }
}

impl From<raijin_extension::InsertTextFormat> for InsertTextFormat {
    fn from(value: raijin_extension::InsertTextFormat) -> Self {
        match value {
            raijin_extension::InsertTextFormat::PlainText => Self::PlainText,
            raijin_extension::InsertTextFormat::Snippet => Self::Snippet,
            raijin_extension::InsertTextFormat::Other(value) => Self::Other(value),
        }
    }
}

impl From<raijin_extension::Symbol> for Symbol {
    fn from(value: raijin_extension::Symbol) -> Self {
        Self {
            kind: value.kind.into(),
            name: value.name,
            container_name: value.container_name,
        }
    }
}

impl From<raijin_extension::SymbolKind> for SymbolKind {
    fn from(value: raijin_extension::SymbolKind) -> Self {
        match value {
            raijin_extension::SymbolKind::File => Self::File,
            raijin_extension::SymbolKind::Module => Self::Module,
            raijin_extension::SymbolKind::Namespace => Self::Namespace,
            raijin_extension::SymbolKind::Package => Self::Package,
            raijin_extension::SymbolKind::Class => Self::Class,
            raijin_extension::SymbolKind::Method => Self::Method,
            raijin_extension::SymbolKind::Property => Self::Property,
            raijin_extension::SymbolKind::Field => Self::Field,
            raijin_extension::SymbolKind::Constructor => Self::Constructor,
            raijin_extension::SymbolKind::Enum => Self::Enum,
            raijin_extension::SymbolKind::Interface => Self::Interface,
            raijin_extension::SymbolKind::Function => Self::Function,
            raijin_extension::SymbolKind::Variable => Self::Variable,
            raijin_extension::SymbolKind::Constant => Self::Constant,
            raijin_extension::SymbolKind::String => Self::String,
            raijin_extension::SymbolKind::Number => Self::Number,
            raijin_extension::SymbolKind::Boolean => Self::Boolean,
            raijin_extension::SymbolKind::Array => Self::Array,
            raijin_extension::SymbolKind::Object => Self::Object,
            raijin_extension::SymbolKind::Key => Self::Key,
            raijin_extension::SymbolKind::Null => Self::Null,
            raijin_extension::SymbolKind::EnumMember => Self::EnumMember,
            raijin_extension::SymbolKind::Struct => Self::Struct,
            raijin_extension::SymbolKind::Event => Self::Event,
            raijin_extension::SymbolKind::Operator => Self::Operator,
            raijin_extension::SymbolKind::TypeParameter => Self::TypeParameter,
            raijin_extension::SymbolKind::Other(value) => Self::Other(value),
        }
    }
}

impl From<raijin_extension::SlashCommand> for SlashCommand {
    fn from(value: raijin_extension::SlashCommand) -> Self {
        Self {
            name: value.name,
            description: value.description,
            tooltip_text: value.tooltip_text,
            requires_argument: value.requires_argument,
        }
    }
}

impl From<SlashCommandOutput> for raijin_extension::SlashCommandOutput {
    fn from(value: SlashCommandOutput) -> Self {
        Self {
            text: value.text,
            sections: value.sections.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SlashCommandOutputSection> for raijin_extension::SlashCommandOutputSection {
    fn from(value: SlashCommandOutputSection) -> Self {
        Self {
            range: value.range.start as usize..value.range.end as usize,
            label: value.label,
        }
    }
}

impl From<SlashCommandArgumentCompletion> for raijin_extension::SlashCommandArgumentCompletion {
    fn from(value: SlashCommandArgumentCompletion) -> Self {
        Self {
            label: value.label,
            new_text: value.new_text,
            run_command: value.run_command,
        }
    }
}

impl TryFrom<ContextServerConfiguration> for raijin_extension::ContextServerConfiguration {
    type Error = anyhow::Error;

    fn try_from(value: ContextServerConfiguration) -> Result<Self, Self::Error> {
        let settings_schema: serde_json::Value = serde_json::from_str(&value.settings_schema)
            .context("Failed to parse settings_schema")?;

        Ok(Self {
            installation_instructions: value.installation_instructions,
            default_settings: value.default_settings,
            settings_schema,
        })
    }
}

impl HostKeyValueStore for WasmState {
    async fn insert(
        &mut self,
        kv_store: Resource<ExtensionKeyValueStore>,
        key: String,
        value: String,
    ) -> wasmtime::Result<Result<(), String>> {
        let kv_store = self.table.get(&kv_store)?;
        kv_store.insert(key, value).await.to_wasmtime_result()
    }

    async fn drop(&mut self, _worktree: Resource<ExtensionKeyValueStore>) -> Result<()> {
        // We only ever hand out borrows of key-value stores.
        Ok(())
    }
}

impl HostProject for WasmState {
    async fn worktree_ids(
        &mut self,
        project: Resource<ExtensionProject>,
    ) -> wasmtime::Result<Vec<u64>> {
        let project = self.table.get(&project)?;
        Ok(project.worktree_ids())
    }

    async fn drop(&mut self, _project: Resource<Project>) -> Result<()> {
        // We only ever hand out borrows of projects.
        Ok(())
    }
}

impl HostWorktree for WasmState {
    async fn id(&mut self, delegate: Resource<Arc<dyn WorktreeDelegate>>) -> wasmtime::Result<u64> {
        let delegate = self.table.get(&delegate)?;
        Ok(delegate.id())
    }

    async fn root_path(
        &mut self,
        delegate: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> wasmtime::Result<String> {
        let delegate = self.table.get(&delegate)?;
        Ok(delegate.root_path())
    }

    async fn read_text_file(
        &mut self,
        delegate: Resource<Arc<dyn WorktreeDelegate>>,
        path: String,
    ) -> wasmtime::Result<Result<String, String>> {
        let delegate = self.table.get(&delegate)?;
        Ok(delegate
            .read_text_file(&RelPath::new(Path::new(&path), PathStyle::Posix)?)
            .await
            .map_err(|error| error.to_string()))
    }

    async fn shell_env(
        &mut self,
        delegate: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> wasmtime::Result<EnvVars> {
        let delegate = self.table.get(&delegate)?;
        Ok(delegate.shell_env().await.into_iter().collect())
    }

    async fn which(
        &mut self,
        delegate: Resource<Arc<dyn WorktreeDelegate>>,
        binary_name: String,
    ) -> wasmtime::Result<Option<String>> {
        let delegate = self.table.get(&delegate)?;
        Ok(delegate.which(binary_name).await)
    }

    async fn drop(&mut self, _worktree: Resource<Worktree>) -> Result<()> {
        // We only ever hand out borrows of worktrees.
        Ok(())
    }
}

impl common::Host for WasmState {}

impl http_client::Host for WasmState {
    async fn fetch(
        &mut self,
        request: http_client::HttpRequest,
    ) -> wasmtime::Result<Result<http_client::HttpResponse, String>> {
        maybe!(async {
            let url = &request.url;
            let request = convert_request(&request)?;
            let mut response = self.host.http_client.send(request).await?;

            if response.status().is_client_error() || response.status().is_server_error() {
                bail!("failed to fetch '{url}': status code {}", response.status())
            }
            convert_response(&mut response).await
        })
        .await
        .to_wasmtime_result()
    }

    async fn fetch_stream(
        &mut self,
        request: http_client::HttpRequest,
    ) -> wasmtime::Result<Result<Resource<ExtensionHttpResponseStream>, String>> {
        let request = convert_request(&request)?;
        let response = self.host.http_client.send(request);
        maybe!(async {
            let response = response.await?;
            let stream = Arc::new(Mutex::new(response));
            let resource = self.table.push(stream)?;
            Ok(resource)
        })
        .await
        .to_wasmtime_result()
    }
}

impl http_client::HostHttpResponseStream for WasmState {
    async fn next_chunk(
        &mut self,
        resource: Resource<ExtensionHttpResponseStream>,
    ) -> wasmtime::Result<Result<Option<Vec<u8>>, String>> {
        let stream = self.table.get(&resource)?.clone();
        maybe!(async move {
            let mut response = stream.lock().await;
            let mut buffer = vec![0; 8192]; // 8KB buffer
            let bytes_read = response.body_mut().read(&mut buffer).await?;
            if bytes_read == 0 {
                Ok(None)
            } else {
                buffer.truncate(bytes_read);
                Ok(Some(buffer))
            }
        })
        .await
        .to_wasmtime_result()
    }

    async fn drop(&mut self, _resource: Resource<ExtensionHttpResponseStream>) -> Result<()> {
        Ok(())
    }
}

impl From<http_client::HttpMethod> for ::raijin_http_client::Method {
    fn from(value: http_client::HttpMethod) -> Self {
        match value {
            http_client::HttpMethod::Get => Self::GET,
            http_client::HttpMethod::Post => Self::POST,
            http_client::HttpMethod::Put => Self::PUT,
            http_client::HttpMethod::Delete => Self::DELETE,
            http_client::HttpMethod::Head => Self::HEAD,
            http_client::HttpMethod::Options => Self::OPTIONS,
            http_client::HttpMethod::Patch => Self::PATCH,
        }
    }
}

fn convert_request(
    extension_request: &http_client::HttpRequest,
) -> anyhow::Result<::raijin_http_client::Request<AsyncBody>> {
    let mut request = ::raijin_http_client::Request::builder()
        .method(::raijin_http_client::Method::from(extension_request.method))
        .uri(&extension_request.url)
        .follow_redirects(match extension_request.redirect_policy {
            http_client::RedirectPolicy::NoFollow => ::raijin_http_client::RedirectPolicy::NoFollow,
            http_client::RedirectPolicy::FollowLimit(limit) => {
                ::raijin_http_client::RedirectPolicy::FollowLimit(limit)
            }
            http_client::RedirectPolicy::FollowAll => ::raijin_http_client::RedirectPolicy::FollowAll,
        });
    for (key, value) in &extension_request.headers {
        request = request.header(key, value);
    }
    let body = extension_request
        .body
        .clone()
        .map(AsyncBody::from)
        .unwrap_or_default();
    request.body(body).map_err(anyhow::Error::from)
}

async fn convert_response(
    response: &mut ::raijin_http_client::Response<AsyncBody>,
) -> anyhow::Result<http_client::HttpResponse> {
    let mut extension_response = http_client::HttpResponse {
        body: Vec::new(),
        headers: Vec::new(),
    };

    for (key, value) in response.headers() {
        extension_response
            .headers
            .push((key.to_string(), value.to_str().unwrap_or("").to_string()));
    }

    response
        .body_mut()
        .read_to_end(&mut extension_response.body)
        .await?;

    Ok(extension_response)
}

impl nodejs::Host for WasmState {
    async fn node_binary_path(&mut self) -> wasmtime::Result<Result<String, String>> {
        self.host
            .node_runtime
            .binary_path()
            .await
            .map(|path| path.to_string_lossy().into_owned())
            .to_wasmtime_result()
    }

    async fn npm_package_latest_version(
        &mut self,
        package_name: String,
    ) -> wasmtime::Result<Result<String, String>> {
        self.host
            .node_runtime
            .npm_package_latest_version(&package_name)
            .await
            .map(|v| v.to_string())
            .to_wasmtime_result()
    }

    async fn npm_package_installed_version(
        &mut self,
        package_name: String,
    ) -> wasmtime::Result<Result<Option<String>, String>> {
        self.host
            .node_runtime
            .npm_package_installed_version(&self.work_dir(), &package_name)
            .await
            .map(|option| option.map(|version| version.to_string()))
            .to_wasmtime_result()
    }

    async fn npm_install_package(
        &mut self,
        package_name: String,
        version: String,
    ) -> wasmtime::Result<Result<(), String>> {
        self.capability_granter
            .grant_npm_install_package(&package_name)?;

        self.host
            .node_runtime
            .npm_install_packages(&self.work_dir(), &[(&package_name, &version)])
            .await
            .to_wasmtime_result()
    }
}

#[async_trait]
impl lsp::Host for WasmState {}

impl From<::raijin_http_client::github::GithubRelease> for github::GithubRelease {
    fn from(value: ::raijin_http_client::github::GithubRelease) -> Self {
        Self {
            version: value.tag_name,
            assets: value.assets.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<::raijin_http_client::github::GithubReleaseAsset> for github::GithubReleaseAsset {
    fn from(value: ::raijin_http_client::github::GithubReleaseAsset) -> Self {
        Self {
            name: value.name,
            download_url: value.browser_download_url,
            digest: value.digest,
        }
    }
}

impl github::Host for WasmState {
    async fn latest_github_release(
        &mut self,
        repo: String,
        options: github::GithubReleaseOptions,
    ) -> wasmtime::Result<Result<github::GithubRelease, String>> {
        maybe!(async {
            let release = ::raijin_http_client::github::latest_github_release(
                &repo,
                options.require_assets,
                options.pre_release,
                self.host.http_client.clone(),
            )
            .await?;
            Ok(release.into())
        })
        .await
        .to_wasmtime_result()
    }

    async fn github_release_by_tag_name(
        &mut self,
        repo: String,
        tag: String,
    ) -> wasmtime::Result<Result<github::GithubRelease, String>> {
        maybe!(async {
            let release = ::raijin_http_client::github::get_release_by_tag_name(
                &repo,
                &tag,
                self.host.http_client.clone(),
            )
            .await?;
            Ok(release.into())
        })
        .await
        .to_wasmtime_result()
    }
}

impl platform::Host for WasmState {
    async fn current_platform(&mut self) -> Result<(platform::Os, platform::Architecture)> {
        Ok((
            match env::consts::OS {
                "macos" => platform::Os::Mac,
                "linux" => platform::Os::Linux,
                "windows" => platform::Os::Windows,
                _ => panic!("unsupported os"),
            },
            match env::consts::ARCH {
                "aarch64" => platform::Architecture::Aarch64,
                "x86" => platform::Architecture::X86,
                "x86_64" => platform::Architecture::X8664,
                _ => panic!("unsupported architecture"),
            },
        ))
    }
}

impl From<std::process::Output> for process::Output {
    fn from(output: std::process::Output) -> Self {
        Self {
            status: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

impl process::Host for WasmState {
    async fn run_command(
        &mut self,
        command: process::Command,
    ) -> wasmtime::Result<Result<process::Output, String>> {
        maybe!(async {
            self.capability_granter
                .grant_exec(&command.command, &command.args)?;

            let output = inazuma_util::command::new_command(command.command.as_str())
                .args(&command.args)
                .envs(command.env)
                .output()
                .await?;

            Ok(output.into())
        })
        .await
        .to_wasmtime_result()
    }
}

#[async_trait]
impl slash_command::Host for WasmState {}

#[async_trait]
impl context_server::Host for WasmState {}

impl dap::Host for WasmState {
    async fn resolve_tcp_template(
        &mut self,
        template: TcpArgumentsTemplate,
    ) -> wasmtime::Result<Result<TcpArguments, String>> {
        maybe!(async {
            let (host, port, timeout) =
                ::raijin_dap::configure_tcp_connection(raijin_task::TcpArgumentsTemplate {
                    port: template.port,
                    host: template.host.map(Ipv4Addr::from_bits),
                    timeout: template.timeout,
                })
                .await?;
            Ok(TcpArguments {
                port,
                host: host.to_bits(),
                timeout,
            })
        })
        .await
        .to_wasmtime_result()
    }
}

impl ExtensionImports for WasmState {
    async fn get_settings(
        &mut self,
        location: Option<self::SettingsLocation>,
        category: String,
        key: Option<String>,
    ) -> wasmtime::Result<Result<String, String>> {
        self.on_main_thread(|cx| {
            async move {
                let path = location.as_ref().and_then(|location| {
                    RelPath::new(Path::new(&location.path), PathStyle::Posix).ok()
                });
                let location = path
                    .as_ref()
                    .zip(location.as_ref())
                    .map(|(path, location)| ::inazuma_settings_framework::SettingsLocation {
                        worktree_id: WorktreeId::from_proto(location.worktree_id),
                        path,
                    });

                cx.update(|cx| match category.as_str() {
                    "language" => {
                        let key = key.map(|k| LanguageName::new(&k));
                        let settings = AllLanguageSettings::get(location, cx).language(
                            location,
                            key.as_ref(),
                            cx,
                        );
                        Ok(serde_json::to_string(&settings::LanguageSettings {
                            tab_size: settings.tab_size,
                            preferred_line_length: settings.preferred_line_length,
                        })?)
                    }
                    "lsp" => {
                        let settings = key
                            .and_then(|key| {
                                ProjectSettings::get(location, cx)
                                    .lsp
                                    .get(&::raijin_lsp::LanguageServerName::from_proto(key))
                            })
                            .cloned()
                            .unwrap_or_default();
                        Ok(serde_json::to_string(&settings::LspSettings {
                            binary: settings.binary.map(|binary| settings::CommandSettings {
                                path: binary.path,
                                arguments: binary.arguments,
                                env: binary.env.map(|env| env.into_iter().collect()),
                            }),
                            settings: settings.settings,
                            initialization_options: settings.initialization_options,
                        })?)
                    }
                    "context_servers" => {
                        let settings = key
                            .and_then(|key| {
                                ProjectSettings::get(location, cx)
                                    .context_servers
                                    .get(key.as_str())
                            })
                            .cloned()
                            .unwrap_or_else(|| {
                                raijin_project::project_settings::ContextServerSettings::default_extension(
                                )
                            });

                        match settings {
                            raijin_project::project_settings::ContextServerSettings::Stdio {
                                enabled: _,
                                command,
                                ..
                            } => Ok(serde_json::to_string(&settings::ContextServerSettings {
                                command: Some(settings::CommandSettings {
                                    path: command.path.to_str().map(|path| path.to_string()),
                                    arguments: Some(command.args),
                                    env: command.env.map(|env| env.into_iter().collect()),
                                }),
                                settings: None,
                            })?),
                            raijin_project::project_settings::ContextServerSettings::Extension {
                                enabled: _,
                                settings,
                                ..
                            } => Ok(serde_json::to_string(&settings::ContextServerSettings {
                                command: None,
                                settings: Some(settings),
                            })?),
                            raijin_project::project_settings::ContextServerSettings::Http { .. } => {
                                bail!("remote context server settings not supported in 0.6.0")
                            }
                        }
                    }
                    _ => {
                        bail!("Unknown settings category: {}", category);
                    }
                })
            }
            .boxed_local()
        })
        .await
        .to_wasmtime_result()
    }

    async fn set_language_server_installation_status(
        &mut self,
        server_name: String,
        status: LanguageServerInstallationStatus,
    ) -> wasmtime::Result<()> {
        let status = match status {
            LanguageServerInstallationStatus::CheckingForUpdate => BinaryStatus::CheckingForUpdate,
            LanguageServerInstallationStatus::Downloading => BinaryStatus::Downloading,
            LanguageServerInstallationStatus::None => BinaryStatus::None,
            LanguageServerInstallationStatus::Failed(error) => BinaryStatus::Failed { error },
        };

        self.host
            .proxy
            .update_language_server_status(::raijin_lsp::LanguageServerName(server_name.into()), status);

        Ok(())
    }

    async fn download_file(
        &mut self,
        url: String,
        path: String,
        file_type: DownloadedFileType,
    ) -> wasmtime::Result<Result<(), String>> {
        maybe!(async {
            let parsed_url = Url::parse(&url)?;
            self.capability_granter.grant_download_file(&parsed_url)?;

            let path = PathBuf::from(path);
            let extension_work_dir = self.host.work_dir.join(self.manifest.id.as_ref());

            self.host.fs.create_dir(&extension_work_dir).await?;

            let destination_path = self
                .host
                .writeable_path_from_extension(&self.manifest.id, &path)
                .await?;

            let mut response = self
                .host
                .http_client
                .get(&url, Default::default(), true)
                .await
                .context("downloading release")?;

            anyhow::ensure!(
                response.status().is_success(),
                "download failed with status {}",
                response.status()
            );
            let body = BufReader::new(response.body_mut());

            match file_type {
                DownloadedFileType::Uncompressed => {
                    futures::pin_mut!(body);
                    self.host
                        .fs
                        .create_file_with(&destination_path, body)
                        .await?;
                }
                DownloadedFileType::Gzip => {
                    let body = GzipDecoder::new(body);
                    futures::pin_mut!(body);
                    self.host
                        .fs
                        .create_file_with(&destination_path, body)
                        .await?;
                }
                DownloadedFileType::GzipTar => {
                    let body = GzipDecoder::new(body);
                    futures::pin_mut!(body);
                    self.host
                        .fs
                        .extract_tar_file(&destination_path, Archive::new(body))
                        .await?;
                }
                DownloadedFileType::Zip => {
                    futures::pin_mut!(body);
                    extract_zip(&destination_path, body)
                        .await
                        .with_context(|| format!("unzipping {path:?} archive"))?;
                }
            }

            Ok(())
        })
        .await
        .to_wasmtime_result()
    }

    async fn make_file_executable(&mut self, path: String) -> wasmtime::Result<Result<(), String>> {
        let path = self
            .host
            .writeable_path_from_extension(&self.manifest.id, Path::new(&path))
            .await?;

        make_file_executable(&path)
            .await
            .with_context(|| format!("setting permissions for path {path:?}"))
            .to_wasmtime_result()
    }
}
