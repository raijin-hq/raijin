use std::path::{Path, PathBuf};

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active Google Cloud project.
///
/// Parses gcloud config files directly (no `gcloud` CLI — too slow).
///
/// Resolution :
/// 1. Find config dir: `$CLOUDSDK_CONFIG` or `~/.config/gcloud`
/// 2. Find active config name: `$CLOUDSDK_ACTIVE_CONFIG_NAME` or read `active_config` file
/// 3. Parse `configurations/config_<name>` INI for `[core]` project and account
///
/// Label: project name (or account if no project set).
pub struct GcloudProvider;

impl ChipProvider for GcloudProvider {
    fn id(&self) -> ChipId {
        "gcloud"
    }

    fn display_name(&self) -> &str {
        "Google Cloud"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        get_config_dir(ctx)
            .map(|d| d.join("configurations").exists() || d.join("properties").exists())
            .unwrap_or(false)
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_gcloud(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Cloud"),
            tooltip: Some("Google Cloud project".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_gcloud(ctx: &ChipContext) -> Option<String> {
    let config_dir = get_config_dir(ctx)?;
    let config_name = get_active_config(ctx, &config_dir)?;

    if config_name == "NONE" {
        return None;
    }

    let config_path = config_dir
        .join("configurations")
        .join(format!("config_{config_name}"));

    let content = std::fs::read_to_string(&config_path)
        .or_else(|_| {
            // Fall back to the legacy `properties` file
            std::fs::read_to_string(config_dir.join("properties"))
        })
        .ok()?;

    // Try project first, fall back to account
    let project = parse_ini_value(&content, "core", "project");
    if project.is_some() {
        return project;
    }

    // Fall back to account (just the user part before @)
    let account = parse_ini_value(&content, "core", "account")?;
    let user_part = account.split('@').next().unwrap_or(&account);
    Some(user_part.to_string())
}

fn get_config_dir(ctx: &ChipContext) -> Option<PathBuf> {
    if let Some(dir) = ctx.get_env("CLOUDSDK_CONFIG") {
        Some(PathBuf::from(dir))
    } else {
        dirs::home_dir().map(|h| h.join(".config").join("gcloud"))
    }
}

fn get_active_config(ctx: &ChipContext, config_dir: &Path) -> Option<String> {
    // Priority 1: Environment variable
    if let Some(name) = ctx.get_env("CLOUDSDK_ACTIVE_CONFIG_NAME") {
        return Some(name.to_string());
    }

    // Priority 2: Read active_config file
    let active_config_path = config_dir.join("active_config");
    let content = std::fs::read_to_string(active_config_path).ok()?;
    content.lines().next().map(|s| s.trim().to_string())
}

/// Simple INI parser: find a value for `key` in `[section]`.
fn parse_ini_value(content: &str, section: &str, key: &str) -> Option<String> {
    let target_section = format!("[{section}]");
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') {
            in_section = trimmed == target_section;
            continue;
        }

        if in_section
            && let Some(rest) = trimmed.strip_prefix(key)
        {
            let rest = rest.trim();
            if let Some(value) = rest.strip_prefix('=') {
                let val = value.trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }

    None
}
