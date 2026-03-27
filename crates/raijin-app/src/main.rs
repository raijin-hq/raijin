mod command_history;
mod completions;
mod input;
mod settings_view;
mod terminal;
mod workspace;

use inazuma::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use inazuma_component::Root;
use inazuma_component::TitleBar;
use workspace::Workspace;

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,raijin=debug,raijin_term=debug")
    ).init();

    Application::new()
        .with_assets(inazuma_component_assets::Assets)
        .run(|cx: &mut App| {
        inazuma_component::init(cx);
        inazuma_component::theme::Theme::change(
            inazuma_component::theme::ThemeMode::Dark,
            None,
            cx,
        );

        // Load Raijin config and set as global
        let config = raijin_settings::RaijinConfig::load().unwrap_or_default();
        cx.set_global(config);

        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                ..Default::default()
            },
            |window, cx| {
                let workspace = cx.new(|cx| Workspace::new(window, cx));
                cx.new(|cx| Root::new(workspace, window, cx))
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
