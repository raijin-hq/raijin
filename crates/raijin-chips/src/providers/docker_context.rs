use std::path::{Path, PathBuf};

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active Docker context.
///
/// Resolution order (matches Docker CLI and standard
/// 1. `$DOCKER_MACHINE_NAME`, `$DOCKER_HOST`, `$DOCKER_CONTEXT` env vars
/// 2. Parse `~/.docker/config.json` (or `$DOCKER_CONFIG/config.json`) for `currentContext`
/// 3. Fall back to `docker context show`
///
/// Skips display for default contexts ("default", "desktop-linux") and unix:// URIs.
pub struct DockerContextProvider;

impl ChipProvider for DockerContextProvider {
    fn id(&self) -> ChipId {
        "docker_context"
    }

    fn display_name(&self) -> &str {
        "Docker Context"
    }

    fn detect_files(&self) -> &[&str] {
        &["docker-compose.yml", "docker-compose.yaml", "Dockerfile"]
    }

    fn detect_extensions(&self) -> &[&str] {
        &["dockerfile"]
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        // Show if any Docker env var is set, or if docker-related files exist
        ctx.has_env("DOCKER_CONTEXT")
            || ctx.has_env("DOCKER_HOST")
            || ctx.has_env("DOCKER_MACHINE_NAME")
            || ctx.dir_contents.matches(
                self.detect_files(),
                self.detect_folders(),
                self.detect_extensions(),
            )
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_docker_context(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Container"),
            tooltip: Some("Docker context".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_docker_context(ctx: &ChipContext) -> Option<String> {
    // Priority 1: Environment variables (same order as Docker CLI)
    let env_context = ["DOCKER_MACHINE_NAME", "DOCKER_HOST", "DOCKER_CONTEXT"]
        .iter()
        .find_map(|key| ctx.get_env(key));

    let docker_ctx = if let Some(val) = env_context {
        val
    } else {
        // Priority 2: Parse config.json
        let config_path = docker_config_path(ctx);
        if let Some(ctx_name) = config_path.and_then(|p| parse_docker_config(&p)) {
            ctx_name
        } else {
            // Priority 3: Fall back to CLI
            ctx.exec_cmd("docker", &["context", "show"])
                .map(|o| o.stdout.trim().to_string())?
        }
    };

    // Skip default/uninteresting contexts
    let skip = ["default", "desktop-linux"];
    if skip.contains(&docker_ctx.as_str()) || docker_ctx.starts_with("unix://") {
        return None;
    }

    Some(docker_ctx)
}

fn docker_config_path(ctx: &ChipContext) -> Option<PathBuf> {
    if let Some(docker_config) = ctx.get_env("DOCKER_CONFIG") {
        Some(PathBuf::from(docker_config).join("config.json"))
    } else {
        dirs::home_dir().map(|h| h.join(".docker").join("config.json"))
    }
}

/// Parse `currentContext` from Docker's config.json without serde.
///
/// The file is small JSON. We scan line-by-line for the key.
fn parse_docker_config(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"currentContext\"") {
            // Expect: "currentContext": "value" or "currentContext":"value"
            let after_colon = rest.trim().strip_prefix(':')?;
            let value = after_colon
                .trim()
                .trim_start_matches('"')
                .trim_end_matches(['"', ',']);
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}
