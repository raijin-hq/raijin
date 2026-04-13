use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AzureProfile {
    installation_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    subscriptions: Vec<Subscription>,
}

#[derive(Serialize, Deserialize, Clone)]
struct User {
    name: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Subscription {
    name: String,
    user: User,
    is_default: bool,
}

pub struct AzureProvider;

impl ChipProvider for AzureProvider {
    fn id(&self) -> ChipId { "azure" }
    fn display_name(&self) -> &str { "Azure" }
    fn is_available(&self, ctx: &ChipContext) -> bool {
        ctx.has_env("AZURE_CONFIG_DIR")
            || dirs::home_dir().map(|h| h.join(".azure").exists()).unwrap_or(false)
    }
    fn gather(&self, ctx: &ChipContext) -> ChipOutput {
        let label = gather_azure(ctx).unwrap_or_default();
        ChipOutput {
            id: self.id(), label,
            icon: Some("Azure"),
            tooltip: Some("Azure".into()),
            ..ChipOutput::default()
        }
    }
}

fn gather_azure(ctx: &ChipContext) -> Option<String> {
    let subscription = get_azure_profile_info(ctx)?;
    Some(subscription.name)
}


fn get_azure_profile_info(ctx: &ChipContext) -> Option<Subscription> {
    let mut config_path = get_config_file_location(ctx)?;
    config_path.push("azureProfile.json");

    let azure_profile = load_azure_profile(&config_path)?;
    azure_profile
        .subscriptions
        .into_iter()
        .find(|s| s.is_default)
}

fn load_azure_profile(config_path: &PathBuf) -> Option<AzureProfile> {
    let json_data = fs::read_to_string(config_path).ok()?;
    let sanitized_json_data = json_data.strip_prefix('\u{feff}').unwrap_or(&json_data);
    if let Ok(azure_profile) = serde_json::from_str::<AzureProfile>(sanitized_json_data) {
        Some(azure_profile)
    } else {
        log::info!("Failed to parse azure profile.");
        None
    }
}

fn get_config_file_location(ctx: &ChipContext) -> Option<PathBuf> {
    ctx
        .get_env("AZURE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            let mut home = dirs::home_dir()?;
            home.push(".azure");
            Some(home)
        })
}
