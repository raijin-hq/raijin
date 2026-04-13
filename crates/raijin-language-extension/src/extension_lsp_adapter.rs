use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use inazuma_collections::{HashMap, HashSet};
use raijin_extension::{Extension, ExtensionLanguageServerProxy, WorktreeDelegate};
use futures::{FutureExt, future::join_all, lock::OwnedMutexGuard};
use inazuma::{App, AppContext, AsyncApp, Task};
use raijin_language::{
    BinaryStatus, CodeLabel, DynLspInstaller, HighlightId, Language, LanguageName,
    LanguageServerBinaryLocations, LspAdapter, LspAdapterDelegate, Toolchain,
};
use raijin_lsp::{
    CodeActionKind, LanguageServerBinary, LanguageServerBinaryOptions, LanguageServerName,
    LanguageServerSelector, Uri,
};
use serde::Serialize;
use serde_json::Value;
use inazuma_util::{ResultExt, fs::make_file_executable, maybe, rel_path::RelPath};

use crate::{LanguageServerRegistryProxy, LspAccess};

/// An adapter that allows an [`LspAdapterDelegate`] to be used as a [`WorktreeDelegate`].
struct WorktreeDelegateAdapter(pub Arc<dyn LspAdapterDelegate>);

#[async_trait]
impl WorktreeDelegate for WorktreeDelegateAdapter {
    fn id(&self) -> u64 {
        self.0.worktree_id().to_proto()
    }

    fn root_path(&self) -> String {
        self.0.worktree_root_path().to_string_lossy().into_owned()
    }

    async fn read_text_file(&self, path: &RelPath) -> Result<String> {
        self.0.read_text_file(path).await
    }

    async fn which(&self, binary_name: String) -> Option<String> {
        self.0
            .which(binary_name.as_ref())
            .await
            .map(|path| path.to_string_lossy().into_owned())
    }

    async fn shell_env(&self) -> Vec<(String, String)> {
        self.0.shell_env().await.into_iter().collect()
    }
}

impl ExtensionLanguageServerProxy for LanguageServerRegistryProxy {
    fn register_language_server(
        &self,
        extension: Arc<dyn Extension>,
        language_server_id: LanguageServerName,
        language: LanguageName,
    ) {
        self.language_registry.register_lsp_adapter(
            language.clone(),
            Arc::new(ExtensionLspAdapter::new(
                extension,
                language_server_id,
                language,
            )),
        );
    }

    fn remove_language_server(
        &self,
        language: &LanguageName,
        language_server_name: &LanguageServerName,
        cx: &mut App,
    ) -> Task<Result<()>> {
        self.language_registry
            .remove_lsp_adapter(language, language_server_name);

        let mut tasks = Vec::new();
        match &self.lsp_access {
            LspAccess::ViaLspStore(lsp_store) => lsp_store.update(cx, |lsp_store, cx| {
                let stop_task = lsp_store.stop_language_servers_for_buffers(
                    Vec::new(),
                    HashSet::from_iter([LanguageServerSelector::Name(
                        language_server_name.clone(),
                    )]),
                    cx,
                );
                tasks.push(stop_task);
            }),
            LspAccess::ViaWorkspaces(lsp_store_provider) => {
                if let Ok(lsp_stores) = lsp_store_provider(cx) {
                    for lsp_store in lsp_stores {
                        lsp_store.update(cx, |lsp_store, cx| {
                            let stop_task = lsp_store.stop_language_servers_for_buffers(
                                Vec::new(),
                                HashSet::from_iter([LanguageServerSelector::Name(
                                    language_server_name.clone(),
                                )]),
                                cx,
                            );
                            tasks.push(stop_task);
                        });
                    }
                }
            }
            LspAccess::Noop => {}
        }

        cx.background_spawn(async move {
            let results = join_all(tasks).await;
            for result in results {
                result?;
            }
            Ok(())
        })
    }

    fn update_language_server_status(
        &self,
        language_server_id: LanguageServerName,
        status: BinaryStatus,
    ) {
        log::debug!(
            "updating binary status for {} to {:?}",
            language_server_id,
            status
        );
        self.language_registry
            .update_lsp_binary_status(language_server_id, status);
    }
}

struct ExtensionLspAdapter {
    extension: Arc<dyn Extension>,
    language_server_id: LanguageServerName,
    language_name: LanguageName,
}

impl ExtensionLspAdapter {
    fn new(
        extension: Arc<dyn Extension>,
        language_server_id: LanguageServerName,
        language_name: LanguageName,
    ) -> Self {
        Self {
            extension,
            language_server_id,
            language_name,
        }
    }
}

#[async_trait(?Send)]
impl DynLspInstaller for ExtensionLspAdapter {
    fn get_language_server_command(
        self: Arc<Self>,
        delegate: Arc<dyn LspAdapterDelegate>,
        _: Option<Toolchain>,
        _: LanguageServerBinaryOptions,
        _: OwnedMutexGuard<Option<(bool, LanguageServerBinary)>>,
        _: AsyncApp,
    ) -> LanguageServerBinaryLocations {
        async move {
            let ret = maybe!(async move {
                let delegate = Arc::new(WorktreeDelegateAdapter(delegate.clone())) as _;
                let command = self
                    .extension
                    .language_server_command(
                        self.language_server_id.clone(),
                        self.language_name.clone(),
                        delegate,
                    )
                    .await?;

                // on windows, extensions might produce weird paths
                // that start with a leading slash due to WASI
                // requiring that for PWD and friends so account for
                // that here and try to transform those paths back
                // to windows paths
                //
                // if we don't do this, std will interpret the path as relative,
                // which changes join behavior
                let command_path: &Path = if cfg!(windows)
                    && let Some(command) = command.command.to_str()
                {
                    let mut chars = command.chars();
                    if chars.next().is_some_and(|c| c == '/')
                        && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                        && chars.next().is_some_and(|c| c == ':')
                        && chars.next().is_some_and(|c| c == '\\' || c == '/')
                    {
                        // looks like a windows path with a leading slash, so strip it
                        command.strip_prefix('/').unwrap().as_ref()
                    } else {
                        command.as_ref()
                    }
                } else {
                    command.command.as_ref()
                };
                let path = self.extension.path_from_extension(command_path);

                // TODO: This should now be done via the `make_file_executable` function in
                // the extension API, but we're leaving these existing usages in place temporarily
                // to avoid any compatibility issues with the extension versions.
                //
                // We can remove once the following extension versions no longer see any use:
                // - toml@0.0.2
                // - zig@0.0.1
                if ["toml", "zig"].contains(&self.extension.manifest().id.as_ref())
                    && path.starts_with(&self.extension.work_dir())
                {
                    make_file_executable(&path)
                        .await
                        .context("failed to set file permissions")?;
                }

                Ok(LanguageServerBinary {
                    path,
                    arguments: command
                        .args
                        .into_iter()
                        .map(|arg| {
                            // on windows, extensions might produce weird paths
                            // that start with a leading slash due to WASI
                            // requiring that for PWD and friends so account for
                            // that here and try to transform those paths back
                            // to windows paths
                            if cfg!(windows) {
                                let mut chars = arg.chars();
                                if chars.next().is_some_and(|c| c == '/')
                                    && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                                    && chars.next().is_some_and(|c| c == ':')
                                    && chars.next().is_some_and(|c| c == '\\' || c == '/')
                                {
                                    // looks like a windows path with a leading slash, so strip it
                                    arg.strip_prefix('/').unwrap().into()
                                } else {
                                    arg.into()
                                }
                            } else {
                                arg.into()
                            }
                        })
                        .collect(),
                    env: Some(command.env.into_iter().collect()),
                })
            })
            .await;
            (ret, None)
        }
        .boxed_local()
    }

    async fn try_fetch_server_binary(
        &self,
        _: &Arc<dyn LspAdapterDelegate>,
        _: PathBuf,
        _: bool,
        _: &mut AsyncApp,
    ) -> Result<LanguageServerBinary> {
        unreachable!("get_language_server_command is overridden")
    }
}

#[async_trait(?Send)]
impl LspAdapter for ExtensionLspAdapter {
    fn name(&self) -> LanguageServerName {
        self.language_server_id.clone()
    }

    fn code_action_kinds(&self) -> Option<Vec<CodeActionKind>> {
        let code_action_kinds = self
            .extension
            .manifest()
            .language_servers
            .get(&self.language_server_id)
            .and_then(|server| server.code_action_kinds.clone());

        code_action_kinds.or(Some(vec![
            CodeActionKind::EMPTY,
            CodeActionKind::QUICKFIX,
            CodeActionKind::REFACTOR,
            CodeActionKind::REFACTOR_EXTRACT,
            CodeActionKind::SOURCE,
        ]))
    }

    fn language_ids(&self) -> HashMap<LanguageName, String> {
        // TODO: The language IDs can be provided via the language server options
        // in `extension.toml now but we're leaving these existing usages in place temporarily
        // to avoid any compatibility issues between Raijin and the extension versions.
        //
        // We can remove once the following extension versions no longer see any use:
        // - php@0.0.1
        if self.extension.manifest().id.as_ref() == "php" {
            return HashMap::from_iter([(LanguageName::new_static("PHP"), "php".into())]);
        }

        self.extension
            .manifest()
            .language_servers
            .get(&self.language_server_id)
            .map(|server| server.language_ids.clone())
            .unwrap_or_default()
    }

    async fn initialization_options(
        self: Arc<Self>,
        delegate: &Arc<dyn LspAdapterDelegate>,
        _: &mut AsyncApp,
    ) -> Result<Option<serde_json::Value>> {
        let delegate = Arc::new(WorktreeDelegateAdapter(delegate.clone())) as _;
        let json_options = self
            .extension
            .language_server_initialization_options(
                self.language_server_id.clone(),
                self.language_name.clone(),
                delegate,
            )
            .await?;
        Ok(if let Some(json_options) = json_options {
            serde_json::from_str(&json_options).with_context(|| {
                format!("failed to parse initialization_options from extension: {json_options}")
            })?
        } else {
            None
        })
    }

    async fn workspace_configuration(
        self: Arc<Self>,
        delegate: &Arc<dyn LspAdapterDelegate>,
        _: Option<Toolchain>,
        _: Option<Uri>,
        _cx: &mut AsyncApp,
    ) -> Result<Value> {
        let delegate = Arc::new(WorktreeDelegateAdapter(delegate.clone())) as _;
        let json_options: Option<String> = self
            .extension
            .language_server_workspace_configuration(self.language_server_id.clone(), delegate)
            .await?;
        Ok(if let Some(json_options) = json_options {
            serde_json::from_str(&json_options).with_context(|| {
                format!("failed to parse workspace_configuration from extension: {json_options}")
            })?
        } else {
            serde_json::json!({})
        })
    }

    async fn initialization_options_schema(
        self: Arc<Self>,
        delegate: &Arc<dyn LspAdapterDelegate>,
        _cached_binary: OwnedMutexGuard<Option<(bool, LanguageServerBinary)>>,
        _cx: &mut AsyncApp,
    ) -> Option<serde_json::Value> {
        let delegate = Arc::new(WorktreeDelegateAdapter(delegate.clone())) as _;
        let json_schema: Option<String> = self
            .extension
            .language_server_initialization_options_schema(
                self.language_server_id.clone(),
                delegate,
            )
            .await
            .ok()
            .flatten();
        json_schema.and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn settings_schema(
        self: Arc<Self>,
        delegate: &Arc<dyn LspAdapterDelegate>,
        _cached_binary: OwnedMutexGuard<Option<(bool, LanguageServerBinary)>>,
        _cx: &mut AsyncApp,
    ) -> Option<serde_json::Value> {
        let delegate = Arc::new(WorktreeDelegateAdapter(delegate.clone())) as _;
        let json_schema: Option<String> = self
            .extension
            .language_server_workspace_configuration_schema(
                self.language_server_id.clone(),
                delegate,
            )
            .await
            .ok()
            .flatten();
        json_schema.and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn additional_initialization_options(
        self: Arc<Self>,
        target_language_server_id: LanguageServerName,
        delegate: &Arc<dyn LspAdapterDelegate>,
    ) -> Result<Option<serde_json::Value>> {
        let delegate = Arc::new(WorktreeDelegateAdapter(delegate.clone())) as _;
        let json_options: Option<String> = self
            .extension
            .language_server_additional_initialization_options(
                self.language_server_id.clone(),
                target_language_server_id.clone(),
                delegate,
            )
            .await?;
        Ok(if let Some(json_options) = json_options {
            serde_json::from_str(&json_options).with_context(|| {
                format!(
                    "failed to parse additional_initialization_options from extension: {json_options}"
                )
            })?
        } else {
            None
        })
    }

    async fn additional_workspace_configuration(
        self: Arc<Self>,
        target_language_server_id: LanguageServerName,

        delegate: &Arc<dyn LspAdapterDelegate>,

        _cx: &mut AsyncApp,
    ) -> Result<Option<serde_json::Value>> {
        let delegate = Arc::new(WorktreeDelegateAdapter(delegate.clone())) as _;
        let json_options: Option<String> = self
            .extension
            .language_server_additional_workspace_configuration(
                self.language_server_id.clone(),
                target_language_server_id.clone(),
                delegate,
            )
            .await?;
        Ok(if let Some(json_options) = json_options {
            serde_json::from_str(&json_options).with_context(|| {
                format!("failed to parse additional_workspace_configuration from extension: {json_options}")
            })?
        } else {
            None
        })
    }

    async fn labels_for_completions(
        self: Arc<Self>,
        completions: &[raijin_lsp::CompletionItem],
        language: &Arc<Language>,
    ) -> Result<Vec<Option<CodeLabel>>> {
        let completions = completions
            .iter()
            .cloned()
            .map(lsp_completion_to_extension)
            .collect::<Vec<_>>();

        let labels = self
            .extension
            .labels_for_completions(self.language_server_id.clone(), completions)
            .await?;

        Ok(labels_from_extension(labels, language))
    }

    async fn labels_for_symbols(
        self: Arc<Self>,
        symbols: &[raijin_language::Symbol],
        language: &Arc<Language>,
    ) -> Result<Vec<Option<CodeLabel>>> {
        let symbols = symbols
            .iter()
            .cloned()
            .map(
                |raijin_language::Symbol {
                     name,
                     kind,
                     container_name,
                 }| raijin_extension::Symbol {
                    name,
                    kind: lsp_symbol_kind_to_extension(kind),
                    container_name,
                },
            )
            .collect::<Vec<_>>();

        let labels = self
            .extension
            .labels_for_symbols(self.language_server_id.clone(), symbols)
            .await?;

        Ok(labels_from_extension(labels, language))
    }

    fn is_extension(&self) -> bool {
        true
    }
}

fn labels_from_extension(
    labels: Vec<Option<raijin_extension::CodeLabel>>,
    language: &Arc<Language>,
) -> Vec<Option<CodeLabel>> {
    labels
        .into_iter()
        .map(|label| {
            let label = label?;
            let runs = if label.code.is_empty() {
                Vec::new()
            } else {
                language.highlight_text(&label.code.as_str().into(), 0..label.code.len())
            };
            build_code_label(&label, &runs, language)
        })
        .collect()
}

fn build_code_label(
    label: &raijin_extension::CodeLabel,
    parsed_runs: &[(Range<usize>, HighlightId)],
    language: &Arc<Language>,
) -> Option<CodeLabel> {
    let mut text = String::new();
    let mut runs = vec![];

    for span in &label.spans {
        match span {
            raijin_extension::CodeLabelSpan::CodeRange(range) => {
                let code_span = &label.code.get(range.clone())?;
                let mut input_ix = range.start;
                let mut output_ix = text.len();
                for (run_range, id) in parsed_runs {
                    if run_range.start >= range.end {
                        break;
                    }
                    if run_range.end <= input_ix {
                        continue;
                    }

                    if run_range.start > input_ix {
                        let len = run_range.start - input_ix;
                        output_ix += len;
                        input_ix += len;
                    }

                    let len = range.end.min(run_range.end) - input_ix;
                    runs.push((output_ix..output_ix + len, *id));
                    output_ix += len;
                    input_ix += len;
                }

                text.push_str(code_span);
            }
            raijin_extension::CodeLabelSpan::Literal(span) => {
                if let Some(highlight_id) = language
                    .grammar()
                    .zip(span.highlight_name.as_ref())
                    .and_then(|(grammar, highlight_name)| {
                        grammar.highlight_id_for_name(highlight_name)
                    })
                {
                    let ix = text.len();
                    runs.push((ix..ix + span.text.len(), highlight_id));
                }
                text.push_str(&span.text);
            }
        }
    }

    let filter_range = label.filter_range.clone();
    text.get(filter_range.clone())?;
    Some(CodeLabel::new(text, filter_range, runs))
}

fn lsp_completion_to_extension(value: raijin_lsp::CompletionItem) -> raijin_extension::Completion {
    raijin_extension::Completion {
        label: value.label,
        label_details: value
            .label_details
            .map(lsp_completion_item_label_details_to_extension),
        detail: value.detail,
        kind: value.kind.map(lsp_completion_item_kind_to_extension),
        insert_text_format: value
            .insert_text_format
            .map(lsp_insert_text_format_to_extension),
    }
}

fn lsp_completion_item_label_details_to_extension(
    value: raijin_lsp::CompletionItemLabelDetails,
) -> raijin_extension::CompletionLabelDetails {
    raijin_extension::CompletionLabelDetails {
        detail: value.detail,
        description: value.description,
    }
}

fn lsp_completion_item_kind_to_extension(
    value: raijin_lsp::CompletionItemKind,
) -> raijin_extension::CompletionKind {
    match value {
        raijin_lsp::CompletionItemKind::TEXT => raijin_extension::CompletionKind::Text,
        raijin_lsp::CompletionItemKind::METHOD => raijin_extension::CompletionKind::Method,
        raijin_lsp::CompletionItemKind::FUNCTION => raijin_extension::CompletionKind::Function,
        raijin_lsp::CompletionItemKind::CONSTRUCTOR => raijin_extension::CompletionKind::Constructor,
        raijin_lsp::CompletionItemKind::FIELD => raijin_extension::CompletionKind::Field,
        raijin_lsp::CompletionItemKind::VARIABLE => raijin_extension::CompletionKind::Variable,
        raijin_lsp::CompletionItemKind::CLASS => raijin_extension::CompletionKind::Class,
        raijin_lsp::CompletionItemKind::INTERFACE => raijin_extension::CompletionKind::Interface,
        raijin_lsp::CompletionItemKind::MODULE => raijin_extension::CompletionKind::Module,
        raijin_lsp::CompletionItemKind::PROPERTY => raijin_extension::CompletionKind::Property,
        raijin_lsp::CompletionItemKind::UNIT => raijin_extension::CompletionKind::Unit,
        raijin_lsp::CompletionItemKind::VALUE => raijin_extension::CompletionKind::Value,
        raijin_lsp::CompletionItemKind::ENUM => raijin_extension::CompletionKind::Enum,
        raijin_lsp::CompletionItemKind::KEYWORD => raijin_extension::CompletionKind::Keyword,
        raijin_lsp::CompletionItemKind::SNIPPET => raijin_extension::CompletionKind::Snippet,
        raijin_lsp::CompletionItemKind::COLOR => raijin_extension::CompletionKind::Color,
        raijin_lsp::CompletionItemKind::FILE => raijin_extension::CompletionKind::File,
        raijin_lsp::CompletionItemKind::REFERENCE => raijin_extension::CompletionKind::Reference,
        raijin_lsp::CompletionItemKind::FOLDER => raijin_extension::CompletionKind::Folder,
        raijin_lsp::CompletionItemKind::ENUM_MEMBER => raijin_extension::CompletionKind::EnumMember,
        raijin_lsp::CompletionItemKind::CONSTANT => raijin_extension::CompletionKind::Constant,
        raijin_lsp::CompletionItemKind::STRUCT => raijin_extension::CompletionKind::Struct,
        raijin_lsp::CompletionItemKind::EVENT => raijin_extension::CompletionKind::Event,
        raijin_lsp::CompletionItemKind::OPERATOR => raijin_extension::CompletionKind::Operator,
        raijin_lsp::CompletionItemKind::TYPE_PARAMETER => raijin_extension::CompletionKind::TypeParameter,
        _ => raijin_extension::CompletionKind::Other(extract_int(value)),
    }
}

fn lsp_insert_text_format_to_extension(
    value: raijin_lsp::InsertTextFormat,
) -> raijin_extension::InsertTextFormat {
    match value {
        raijin_lsp::InsertTextFormat::PLAIN_TEXT => raijin_extension::InsertTextFormat::PlainText,
        raijin_lsp::InsertTextFormat::SNIPPET => raijin_extension::InsertTextFormat::Snippet,
        _ => raijin_extension::InsertTextFormat::Other(extract_int(value)),
    }
}

fn lsp_symbol_kind_to_extension(value: raijin_lsp::SymbolKind) -> raijin_extension::SymbolKind {
    match value {
        raijin_lsp::SymbolKind::FILE => raijin_extension::SymbolKind::File,
        raijin_lsp::SymbolKind::MODULE => raijin_extension::SymbolKind::Module,
        raijin_lsp::SymbolKind::NAMESPACE => raijin_extension::SymbolKind::Namespace,
        raijin_lsp::SymbolKind::PACKAGE => raijin_extension::SymbolKind::Package,
        raijin_lsp::SymbolKind::CLASS => raijin_extension::SymbolKind::Class,
        raijin_lsp::SymbolKind::METHOD => raijin_extension::SymbolKind::Method,
        raijin_lsp::SymbolKind::PROPERTY => raijin_extension::SymbolKind::Property,
        raijin_lsp::SymbolKind::FIELD => raijin_extension::SymbolKind::Field,
        raijin_lsp::SymbolKind::CONSTRUCTOR => raijin_extension::SymbolKind::Constructor,
        raijin_lsp::SymbolKind::ENUM => raijin_extension::SymbolKind::Enum,
        raijin_lsp::SymbolKind::INTERFACE => raijin_extension::SymbolKind::Interface,
        raijin_lsp::SymbolKind::FUNCTION => raijin_extension::SymbolKind::Function,
        raijin_lsp::SymbolKind::VARIABLE => raijin_extension::SymbolKind::Variable,
        raijin_lsp::SymbolKind::CONSTANT => raijin_extension::SymbolKind::Constant,
        raijin_lsp::SymbolKind::STRING => raijin_extension::SymbolKind::String,
        raijin_lsp::SymbolKind::NUMBER => raijin_extension::SymbolKind::Number,
        raijin_lsp::SymbolKind::BOOLEAN => raijin_extension::SymbolKind::Boolean,
        raijin_lsp::SymbolKind::ARRAY => raijin_extension::SymbolKind::Array,
        raijin_lsp::SymbolKind::OBJECT => raijin_extension::SymbolKind::Object,
        raijin_lsp::SymbolKind::KEY => raijin_extension::SymbolKind::Key,
        raijin_lsp::SymbolKind::NULL => raijin_extension::SymbolKind::Null,
        raijin_lsp::SymbolKind::ENUM_MEMBER => raijin_extension::SymbolKind::EnumMember,
        raijin_lsp::SymbolKind::STRUCT => raijin_extension::SymbolKind::Struct,
        raijin_lsp::SymbolKind::EVENT => raijin_extension::SymbolKind::Event,
        raijin_lsp::SymbolKind::OPERATOR => raijin_extension::SymbolKind::Operator,
        raijin_lsp::SymbolKind::TYPE_PARAMETER => raijin_extension::SymbolKind::TypeParameter,
        _ => raijin_extension::SymbolKind::Other(extract_int(value)),
    }
}

fn extract_int<T: Serialize>(value: T) -> i32 {
    maybe!({
        let kind = serde_json::to_value(&value)?;
        serde_json::from_value(kind)
    })
    .log_err()
    .unwrap_or(-1)
}

#[test]
fn test_build_code_label() {
    use inazuma_util::test::marked_text_ranges;

    let (code, code_ranges) = marked_text_ranges(
        "«const» «a»: «fn»(«Bcd»(«Efgh»)) -> «Ijklm» = pqrs.tuv",
        false,
    );
    let code_runs = code_ranges
        .into_iter()
        .map(|range| (range, HighlightId(0)))
        .collect::<Vec<_>>();

    let label = build_code_label(
        &raijin_extension::CodeLabel {
            spans: vec![
                raijin_extension::CodeLabelSpan::CodeRange(code.find("pqrs").unwrap()..code.len()),
                raijin_extension::CodeLabelSpan::CodeRange(
                    code.find(": fn").unwrap()..code.find(" = ").unwrap(),
                ),
            ],
            filter_range: 0.."pqrs.tuv".len(),
            code,
        },
        &code_runs,
        &raijin_language::PLAIN_TEXT,
    )
    .unwrap();

    let (label_text, label_ranges) =
        marked_text_ranges("pqrs.tuv: «fn»(«Bcd»(«Efgh»)) -> «Ijklm»", false);
    let label_runs = label_ranges
        .into_iter()
        .map(|range| (range, HighlightId(0)))
        .collect::<Vec<_>>();

    assert_eq!(
        label,
        CodeLabel::new(label_text, label.filter_range.clone(), label_runs)
    )
}

#[test]
fn test_build_code_label_with_invalid_ranges() {
    use inazuma_util::test::marked_text_ranges;

    let (code, code_ranges) = marked_text_ranges("const «a»: «B» = '🏀'", false);
    let code_runs = code_ranges
        .into_iter()
        .map(|range| (range, HighlightId(0)))
        .collect::<Vec<_>>();

    // A span uses a code range that is invalid because it starts inside of
    // a multi-byte character.
    let label = build_code_label(
        &raijin_extension::CodeLabel {
            spans: vec![
                raijin_extension::CodeLabelSpan::CodeRange(
                    code.find('B').unwrap()..code.find(" = ").unwrap(),
                ),
                raijin_extension::CodeLabelSpan::CodeRange((code.find('🏀').unwrap() + 1)..code.len()),
            ],
            filter_range: 0.."B".len(),
            code,
        },
        &code_runs,
        &raijin_language::PLAIN_TEXT,
    );
    assert!(label.is_none());

    // Filter range extends beyond actual text
    let label = build_code_label(
        &raijin_extension::CodeLabel {
            spans: vec![raijin_extension::CodeLabelSpan::Literal(
                raijin_extension::CodeLabelSpanLiteral {
                    text: "abc".into(),
                    highlight_name: Some("type".into()),
                },
            )],
            filter_range: 0..5,
            code: String::new(),
        },
        &code_runs,
        &raijin_language::PLAIN_TEXT,
    );
    assert!(label.is_none());
}
