use std::sync::Arc;

use raijin_fs::Fs;
use raijin_language_model::LanguageModel;
use inazuma_settings_framework::{LanguageModelSelection, update_settings_file};
use raijin_ui::App;

fn language_model_to_selection(model: &Arc<dyn LanguageModel>) -> LanguageModelSelection {
    LanguageModelSelection {
        provider: model.provider_id().to_string().into(),
        model: model.id().0.to_string(),
        enable_thinking: false,
        effort: None,
    }
}

pub fn toggle_in_settings(
    model: Arc<dyn LanguageModel>,
    should_be_favorite: bool,
    fs: Arc<dyn Fs>,
    cx: &mut App,
) {
    let selection = language_model_to_selection(&model);
    update_settings_file(fs, cx, move |settings, _| {
        let agent = settings.agent.get_or_insert_default();
        if should_be_favorite {
            agent.add_favorite_model(selection.clone());
        } else {
            agent.remove_favorite_model(&selection);
        }
    });
}
