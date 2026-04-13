mod codelldb;
mod gdb;
mod go;
mod javascript;
mod python;

#[cfg(test)]
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use codelldb::CodeLldbDebugAdapter;
use raijin_dap::{
    DapRegistry,
    adapters::{
        self, AdapterVersion, DapDelegate, DebugAdapter, DebugAdapterBinary, DebugAdapterName,
    },
    configure_tcp_connection,
};
use gdb::GdbDebugAdapter;
use go::GoDebugAdapter;
use inazuma::{App, BorrowAppContext};
use javascript::JsDebugAdapter;
use python::PythonDebugAdapter;
use serde_json::json;
use raijin_task::{DebugScenario, RaijinDebugConfig};

pub fn init(cx: &mut App) {
    cx.update_default_global(|registry: &mut DapRegistry, _cx| {
        registry.add_adapter(Arc::from(CodeLldbDebugAdapter::default()));
        registry.add_adapter(Arc::from(PythonDebugAdapter::default()));
        registry.add_adapter(Arc::from(JsDebugAdapter::default()));
        registry.add_adapter(Arc::from(GoDebugAdapter::default()));
        registry.add_adapter(Arc::from(GdbDebugAdapter));

        #[cfg(any(test, feature = "test-support"))]
        {
            registry.add_adapter(Arc::from(raijin_dap::FakeAdapter {}));
        }
    })
}

#[cfg(test)]
mod test_mocks {
    use super::*;

    pub(crate) struct MockDelegate {
        worktree_root: PathBuf,
    }

    impl MockDelegate {
        pub(crate) fn new() -> Arc<dyn adapters::DapDelegate> {
            Arc::new(Self {
                worktree_root: PathBuf::from("/tmp/test"),
            })
        }
    }

    #[async_trait::async_trait]
    impl adapters::DapDelegate for MockDelegate {
        fn worktree_id(&self) -> inazuma_settings_framework::WorktreeId {
            inazuma_settings_framework::WorktreeId::from_usize(0)
        }

        fn worktree_root_path(&self) -> &std::path::Path {
            &self.worktree_root
        }

        fn http_client(&self) -> Arc<dyn raijin_http_client::HttpClient> {
            unimplemented!("Not needed for tests")
        }

        fn node_runtime(&self) -> raijin_node_runtime::NodeRuntime {
            unimplemented!("Not needed for tests")
        }

        fn toolchain_store(&self) -> Arc<dyn raijin_language::LanguageToolchainStore> {
            unimplemented!("Not needed for tests")
        }

        fn fs(&self) -> Arc<dyn raijin_fs::Fs> {
            unimplemented!("Not needed for tests")
        }

        fn output_to_console(&self, _msg: String) {}

        async fn which(&self, _command: &std::ffi::OsStr) -> Option<PathBuf> {
            None
        }

        async fn read_text_file(&self, _path: &inazuma_util::rel_path::RelPath) -> Result<String> {
            Ok(String::new())
        }

        async fn shell_env(&self) -> inazuma_collections::HashMap<String, String> {
            inazuma_collections::HashMap::default()
        }

        fn is_headless(&self) -> bool {
            false
        }
    }
}
