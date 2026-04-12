pub(crate) mod latest;

use raijin_dap::DebugRequest;
use raijin_extension::{DebugTaskDefinition, KeyValueStoreDelegate, WorktreeDelegate};
use inazuma::BackgroundExecutor;
use raijin_lsp::LanguageServerName;
use raijin_task::{DebugScenario, SpawnInTerminal, TaskTemplate, ZedDebugConfig};

use super::{WasmState, wasm_engine};
use anyhow::{Context as _, Result, anyhow};
use std::{path::PathBuf, sync::Arc};
use wasmtime::{
    Store,
    component::{Component, Linker, Resource},
};

#[cfg(test)]
pub use latest::CodeLabelSpanLiteral;
pub use latest::{
    CodeLabel, CodeLabelSpan, Command, DebugAdapterBinary, ExtensionProject, Range, SlashCommand,
    raijin::extension::context_server::ContextServerConfiguration,
    raijin::extension::lsp::{
        Completion, CompletionKind, CompletionLabelDetails, InsertTextFormat, Symbol, SymbolKind,
    },
    raijin::extension::slash_command::{SlashCommandArgumentCompletion, SlashCommandOutput},
};

pub fn new_linker(
    executor: &BackgroundExecutor,
    f: impl Fn(&mut Linker<WasmState>) -> Result<()>,
) -> Linker<WasmState> {
    let mut linker = Linker::new(&wasm_engine(executor));
    wasmtime_wasi::p2::add_to_linker_async(&mut linker).unwrap();
    f(&mut linker).unwrap();
    linker
}

pub struct Extension(latest::Extension);

impl Extension {
    pub async fn instantiate_async(
        executor: &BackgroundExecutor,
        store: &mut Store<WasmState>,
        component: &Component,
    ) -> Result<Self> {
        let ext =
            latest::Extension::instantiate_async(store, component, latest::linker(executor))
                .await
                .context("failed to instantiate wasm extension")?;
        Ok(Self(ext))
    }

    pub async fn call_init_extension(&self, store: &mut Store<WasmState>) -> Result<()> {
        self.0.call_init_extension(store).await
    }

    pub async fn call_language_server_command(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Result<Command, String>> {
        self.0
            .call_language_server_command(store, &language_server_id.0, resource)
            .await
    }

    pub async fn call_language_server_initialization_options(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Result<Option<String>, String>> {
        self.0
            .call_language_server_initialization_options(
                store,
                &language_server_id.0,
                resource,
            )
            .await
    }

    pub async fn call_language_server_workspace_configuration(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Result<Option<String>, String>> {
        self.0
            .call_language_server_workspace_configuration(
                store,
                &language_server_id.0,
                resource,
            )
            .await
    }

    pub async fn call_language_server_initialization_options_schema(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Option<String>> {
        self.0
            .call_language_server_initialization_options_schema(
                store,
                &language_server_id.0,
                resource,
            )
            .await
    }

    pub async fn call_language_server_workspace_configuration_schema(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Option<String>> {
        self.0
            .call_language_server_workspace_configuration_schema(
                store,
                &language_server_id.0,
                resource,
            )
            .await
    }

    pub async fn call_language_server_additional_initialization_options(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        target_language_server_id: &LanguageServerName,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Result<Option<String>, String>> {
        self.0
            .call_language_server_additional_initialization_options(
                store,
                &language_server_id.0,
                &target_language_server_id.0,
                resource,
            )
            .await
    }

    pub async fn call_language_server_additional_workspace_configuration(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        target_language_server_id: &LanguageServerName,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Result<Option<String>, String>> {
        self.0
            .call_language_server_additional_workspace_configuration(
                store,
                &language_server_id.0,
                &target_language_server_id.0,
                resource,
            )
            .await
    }

    pub async fn call_labels_for_completions(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        completions: Vec<latest::Completion>,
    ) -> Result<Result<Vec<Option<CodeLabel>>, String>> {
        self.0
            .call_labels_for_completions(store, &language_server_id.0, &completions)
            .await
    }

    pub async fn call_labels_for_symbols(
        &self,
        store: &mut Store<WasmState>,
        language_server_id: &LanguageServerName,
        symbols: Vec<latest::Symbol>,
    ) -> Result<Result<Vec<Option<CodeLabel>>, String>> {
        self.0
            .call_labels_for_symbols(store, &language_server_id.0, &symbols)
            .await
    }

    pub async fn call_complete_slash_command_argument(
        &self,
        store: &mut Store<WasmState>,
        command: &SlashCommand,
        arguments: &[String],
    ) -> Result<Result<Vec<SlashCommandArgumentCompletion>, String>> {
        self.0
            .call_complete_slash_command_argument(store, command, arguments)
            .await
    }

    pub async fn call_run_slash_command(
        &self,
        store: &mut Store<WasmState>,
        command: &SlashCommand,
        arguments: &[String],
        resource: Option<Resource<Arc<dyn WorktreeDelegate>>>,
    ) -> Result<Result<SlashCommandOutput, String>> {
        self.0
            .call_run_slash_command(store, command, arguments, resource)
            .await
    }

    pub async fn call_context_server_command(
        &self,
        store: &mut Store<WasmState>,
        context_server_id: Arc<str>,
        project: Resource<ExtensionProject>,
    ) -> Result<Result<Command, String>> {
        self.0
            .call_context_server_command(store, &context_server_id, project)
            .await
    }

    pub async fn call_context_server_configuration(
        &self,
        store: &mut Store<WasmState>,
        context_server_id: Arc<str>,
        project: Resource<ExtensionProject>,
    ) -> Result<Result<Option<ContextServerConfiguration>, String>> {
        self.0
            .call_context_server_configuration(store, &context_server_id, project)
            .await
    }

    pub async fn call_suggest_docs_packages(
        &self,
        store: &mut Store<WasmState>,
        provider: &str,
    ) -> Result<Result<Vec<String>, String>> {
        self.0.call_suggest_docs_packages(store, provider).await
    }

    pub async fn call_index_docs(
        &self,
        store: &mut Store<WasmState>,
        provider: &str,
        package_name: &str,
        kv_store: Resource<Arc<dyn KeyValueStoreDelegate>>,
    ) -> Result<Result<(), String>> {
        self.0
            .call_index_docs(store, provider, package_name, kv_store)
            .await
    }

    pub async fn call_get_dap_binary(
        &self,
        store: &mut Store<WasmState>,
        adapter_name: Arc<str>,
        task: DebugTaskDefinition,
        user_installed_path: Option<PathBuf>,
        resource: Resource<Arc<dyn WorktreeDelegate>>,
    ) -> Result<Result<DebugAdapterBinary, String>> {
        let dap_binary = self
            .0
            .call_get_dap_binary(
                store,
                &adapter_name,
                &task.try_into()?,
                user_installed_path.as_ref().and_then(|p| p.to_str()),
                resource,
            )
            .await?
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(Ok(dap_binary))
    }

    pub async fn call_dap_request_kind(
        &self,
        store: &mut Store<WasmState>,
        adapter_name: Arc<str>,
        config: serde_json::Value,
    ) -> Result<Result<latest::dap::StartDebuggingRequestArgumentsRequest, String>> {
        let config =
            serde_json::to_string(&config).context("Adapter config is not a valid JSON")?;
        let result = self
            .0
            .call_dap_request_kind(store, &adapter_name, &config)
            .await?
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(Ok(result))
    }

    pub async fn call_dap_config_to_scenario(
        &self,
        store: &mut Store<WasmState>,
        config: ZedDebugConfig,
    ) -> Result<Result<DebugScenario, String>> {
        let config = config.into();
        let result = self
            .0
            .call_dap_config_to_scenario(store, &config)
            .await?
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(Ok(result.try_into()?))
    }

    pub async fn call_dap_locator_create_scenario(
        &self,
        store: &mut Store<WasmState>,
        locator_name: String,
        build_config_template: TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Result<Option<DebugScenario>> {
        let build_config_template = build_config_template.into();
        let result = self
            .0
            .call_dap_locator_create_scenario(
                store,
                &locator_name,
                &build_config_template,
                &resolved_label,
                &debug_adapter_name,
            )
            .await?;

        Ok(result.map(TryInto::try_into).transpose()?)
    }

    pub async fn call_run_dap_locator(
        &self,
        store: &mut Store<WasmState>,
        locator_name: String,
        resolved_build_task: SpawnInTerminal,
    ) -> Result<Result<DebugRequest, String>> {
        let build_config_template = resolved_build_task.try_into()?;
        let dap_request = self
            .0
            .call_run_dap_locator(store, &locator_name, &build_config_template)
            .await?
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(Ok(dap_request.into()))
    }
}

trait ToWasmtimeResult<T> {
    fn to_wasmtime_result(self) -> wasmtime::Result<Result<T, String>>;
}

impl<T> ToWasmtimeResult<T> for Result<T> {
    fn to_wasmtime_result(self) -> wasmtime::Result<Result<T, String>> {
        Ok(self.map_err(|error| format!("{error:?}")))
    }
}
