mod terminal_element;
mod workspace;

use inazuma::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use inazuma_component::Root;
use workspace::Workspace;

fn main() {
    Application::new()
        .with_assets(inazuma_component_assets::Assets)
        .run(|cx: &mut App| {
        inazuma_component::init(cx);
        inazuma_component::theme::Theme::change(
            inazuma_component::theme::ThemeMode::Dark,
            None,
            cx,
        );

        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
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
