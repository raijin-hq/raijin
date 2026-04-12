mod sign_in;

use std::sync::Arc;

use raijin_copilot::GlobalCopilotAuth;
use inazuma::AppContext;
use raijin_language::language_settings::{AllLanguageSettings, EditPredictionProvider};
use raijin_project::DisableAiSettings;
use inazuma_settings_framework::SettingsStore;
pub use sign_in::{
    ConfigurationMode, ConfigurationView, CopilotCodeVerification, initiate_sign_in,
    initiate_sign_out, reinstall_and_sign_in,
};
use raijin_ui::App;
use raijin_workspace::AppState;

pub fn init(app_state: &Arc<AppState>, cx: &mut App) {
    let disable_ai = cx.read_global(|settings: &SettingsStore, _| {
        settings.get::<DisableAiSettings>(None).disable_ai
    });
    let provider = cx.read_global(|settings: &SettingsStore, _| {
        settings
            .get::<AllLanguageSettings>(None)
            .edit_predictions
            .provider
    });
    if !disable_ai && provider == EditPredictionProvider::Copilot {
        GlobalCopilotAuth::set_global(
            app_state.languages.next_language_server_id(),
            app_state.fs.clone(),
            app_state.node_runtime.clone(),
            cx,
        );
    }
}
