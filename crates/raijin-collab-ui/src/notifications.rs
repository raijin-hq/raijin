pub mod incoming_call_notification;
pub mod project_shared_notification;

use inazuma::App;
use std::sync::Arc;
use raijin_workspace::AppState;

pub fn init(app_state: &Arc<AppState>, cx: &mut App) {
    incoming_call_notification::init(app_state, cx);
    project_shared_notification::init(app_state, cx);
}
