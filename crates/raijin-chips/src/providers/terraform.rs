use std::path::PathBuf;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active Terraform workspace.
///
/// Workspace resolution (matches Terraform's own logic and the reference):
/// 1. `$TF_WORKSPACE` environment variable
/// 2. Read `.terraform/environment` file (or `$TF_DATA_DIR/environment`)
/// 3. Default to "default"
///
/// Version is skipped to avoid slow `terraform version` calls.
pub struct TerraformProvider;

impl ChipProvider for TerraformProvider {
    fn id(&self) -> ChipId {
        "terraform"
    }

    fn display_name(&self) -> &str {
        "Terraform"
    }

    fn detect_files(&self) -> &[&str] {
        &["main.tf"]
    }

    fn detect_folders(&self) -> &[&str] {
        &[".terraform"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["tf", "tfplan", "tfstate"]
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let workspace = get_terraform_workspace(ctx);

        ChipOutput {
            id: self.id(),
            label: workspace,
            icon: Some("Layers"),
            tooltip: Some("Terraform workspace".into()),
            ..ChipOutput::default()
        }
    }
}

/// Determine the current Terraform workspace.
///
/// Follows the same resolution as Terraform's `meta.go`:
/// 1. `$TF_WORKSPACE` overrides everything
/// 2. Read `$TF_DATA_DIR/environment` or `.terraform/environment`
/// 3. If file doesn't exist, workspace is "default"
fn get_terraform_workspace(ctx: &ChipContext) -> String {
    // Priority 1: Explicit env var override
    if let Some(ws) = ctx.get_env("TF_WORKSPACE") {
        return ws.to_string();
    }

    // Priority 2: Read environment file from data dir
    let data_dir = match ctx.get_env("TF_DATA_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => ctx.cwd.join(".terraform"),
    };

    let env_file = data_dir.join("environment");
    match std::fs::read_to_string(&env_file) {
        Ok(content) => {
            let ws = content.trim().to_string();
            if ws.is_empty() {
                "default".to_string()
            } else {
                ws
            }
        }
        Err(_) => "default".to_string(),
    }
}
