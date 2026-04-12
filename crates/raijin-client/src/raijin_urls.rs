//! Contains helper functions for constructing URLs to various Raijin-related pages.
//!
//! These URLs will adapt to the configured server URL in order to construct
//! links appropriate for the environment.

use inazuma::App;
use inazuma_settings_framework::Settings;

use crate::ClientSettings;

fn server_url(cx: &App) -> &str {
    &ClientSettings::get_global(cx).server_url
}

/// Returns the URL to the account page.
pub fn account_url(cx: &App) -> String {
    format!("{server_url}/account", server_url = server_url(cx))
}

/// Returns the URL to the start trial page.
pub fn start_trial_url(cx: &App) -> String {
    format!(
        "{server_url}/account/start-trial",
        server_url = server_url(cx)
    )
}

/// Returns the URL to the upgrade page.
pub fn upgrade_to_raijin_pro_url(cx: &App) -> String {
    format!("{server_url}/account/upgrade", server_url = server_url(cx))
}

/// Returns the URL to Raijin's terms of service.
pub fn terms_of_service(cx: &App) -> String {
    format!("{server_url}/terms-of-service", server_url = server_url(cx))
}

/// Returns the URL to Raijin AI's privacy and security docs.
pub fn ai_privacy_and_security(cx: &App) -> String {
    format!(
        "{server_url}/docs/ai/privacy-and-security",
        server_url = server_url(cx)
    )
}

/// Returns the URL to Raijin's edit prediction documentation.
pub fn edit_prediction_docs(cx: &App) -> String {
    format!(
        "{server_url}/docs/ai/edit-prediction",
        server_url = server_url(cx)
    )
}

/// Returns the URL to Raijin's ACP registry blog post.
pub fn acp_registry_blog(cx: &App) -> String {
    format!(
        "{server_url}/blog/acp-registry",
        server_url = server_url(cx)
    )
}

pub fn shared_agent_thread_url(session_id: &str) -> String {
    format!("raijin://agent/shared/{}", session_id)
}
