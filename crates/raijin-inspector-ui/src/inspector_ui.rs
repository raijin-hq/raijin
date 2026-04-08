#[cfg(debug_assertions)]
mod div_inspector;
#[cfg(debug_assertions)]
mod inspector;

#[cfg(debug_assertions)]
pub use inspector::init;

#[cfg(not(debug_assertions))]
pub fn init(_app_state: std::sync::Arc<raijin_workspace::AppState>, cx: &mut inazuma::App) {
    use std::any::TypeId;
    use raijin_workspace::notifications::NotifyResultExt as _;

    cx.on_action(|_: &raijin_actions::dev::ToggleInspector, cx| {
        Err::<(), anyhow::Error>(anyhow::anyhow!(
            "dev::ToggleInspector is only available in debug builds"
        ))
        .notify_app_err(cx);
    });

    raijin_command_palette_hooks::CommandPaletteFilter::update_global(cx, |filter, _cx| {
        filter.hide_action_types(&[TypeId::of::<raijin_actions::dev::ToggleInspector>()]);
    });
}
