use std::path::PathBuf;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};

/// Chip provider for the active Azure subscription.
///
/// Parses `~/.azure/azureProfile.json` directly (no `az` CLI — too slow).
/// Finds the subscription with `"isDefault": true` and shows its name.
///
/// Config dir: `$AZURE_CONFIG_DIR` or `~/.azure`.
pub struct AzureProvider;

impl ChipProvider for AzureProvider {
    fn id(&self) -> ChipId {
        "azure"
    }

    fn display_name(&self) -> &str {
        "Azure"
    }

    fn is_available(&self, ctx: &ChipContext) -> bool {
        azure_profile_path(ctx).is_some_and(|p| p.exists())
    }

    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_azure(ctx).unwrap_or_default();

        ChipOutput {
            id: self.id(),
            label,
            icon: Some("Cloud"),
            tooltip: Some("Azure subscription".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_azure(ctx: &ChipContext) -> Option<String> {
    let profile_path = azure_profile_path(ctx)?;
    let content = std::fs::read_to_string(profile_path).ok()?;
    parse_default_subscription_name(&content)
}

fn azure_config_dir(ctx: &ChipContext) -> Option<PathBuf> {
    if let Some(dir) = ctx.get_env("AZURE_CONFIG_DIR") {
        Some(PathBuf::from(dir))
    } else {
        dirs::home_dir().map(|h| h.join(".azure"))
    }
}

fn azure_profile_path(ctx: &ChipContext) -> Option<PathBuf> {
    azure_config_dir(ctx).map(|d| d.join("azureProfile.json"))
}

/// Parse the default subscription name from azureProfile.json.
///
/// The file contains a `subscriptions` array. We find the entry with
/// `"isDefault": true` and extract its `"name"` field.
///
/// This uses a simple line-by-line state machine instead of serde_json,
/// keeping the dependency graph clean.
fn parse_default_subscription_name(content: &str) -> Option<String> {
    // State machine: we track whether we're inside a subscription object
    // that has isDefault: true.
    let mut in_subscriptions = false;
    let mut brace_depth: i32 = 0;
    let mut current_name: Option<String> = None;
    let mut current_is_default = false;
    let mut in_object = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect subscriptions array
        if trimmed.contains("\"subscriptions\"") {
            in_subscriptions = true;
            continue;
        }

        if !in_subscriptions {
            continue;
        }

        // Track braces for object boundaries
        for ch in trimmed.chars() {
            match ch {
                '{' => {
                    brace_depth += 1;
                    if brace_depth == 1 {
                        in_object = true;
                        current_name = None;
                        current_is_default = false;
                    }
                }
                '}' => {
                    if brace_depth == 1 && in_object {
                        // End of subscription object
                        if current_is_default {
                            if let Some(name) = current_name.take() {
                                return Some(name);
                            }
                        }
                        in_object = false;
                        current_name = None;
                        current_is_default = false;
                    }
                    brace_depth -= 1;
                    if brace_depth < 0 {
                        // Left the subscriptions array
                        return None;
                    }
                }
                ']' if brace_depth == 0 => {
                    // End of subscriptions array
                    return None;
                }
                _ => {}
            }
        }

        if !in_object || brace_depth != 1 {
            continue;
        }

        // Extract "name": "value"
        if let Some(val) = extract_json_string_value(trimmed, "name") {
            current_name = Some(val);
        }

        // Detect "isDefault": true
        if trimmed.contains("\"isDefault\"") && trimmed.contains("true") {
            current_is_default = true;
        }
    }

    None
}

/// Extract a JSON string value for a given key from a single line.
/// e.g., `"name": "My Subscription",` → `Some("My Subscription")`
fn extract_json_string_value(line: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let idx = line.find(&pattern)?;
    let after_key = &line[idx + pattern.len()..];
    let after_colon = after_key.trim().strip_prefix(':')?;
    let trimmed = after_colon.trim();

    // Find the string value between quotes
    let start = trimmed.find('"')? + 1;
    let rest = &trimmed[start..];
    let end = rest.find('"')?;
    let value = &rest[..end];

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
