mod command_history;
mod completions;
mod input;
mod settings_view;
mod shell_install;
mod terminal;
mod workspace;

use std::borrow::Cow;
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
        // Register bundled terminal font (DankMono Nerd Font Mono)
        let bundled_fonts: Vec<Cow<'static, [u8]>> = vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/dankmono-nerd-font-mono/DankMonoNerdFontMono-Regular.otf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/dankmono-nerd-font-mono/DankMonoNerdFontMono-Bold.otf")),
        ];
        cx.text_system().add_fonts(bundled_fonts).expect("failed to register bundled fonts");

        inazuma_component::init(cx);
        inazuma_component::theme::Theme::change(
            inazuma_component::theme::ThemeMode::Dark,
            None,
            cx,
        );

        // Load Raijin config and theme, set both as globals
        let config = raijin_settings::RaijinConfig::load().unwrap_or_default();
        let theme = raijin_settings::RaijinTheme::load(&config.appearance.theme)
            .unwrap_or_default();
        let resolved_theme = raijin_settings::ResolvedTheme::from_theme(&theme);
        cx.set_global(config);
        cx.set_global(resolved_theme);
        cx.set_global(workspace::PendingShellSwitch(None));
        cx.set_global(workspace::PendingShellInstallName(None));

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
