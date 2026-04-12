use inazuma::App;

mod app_menu_bar;
mod context_menu;
mod dropdown_menu;
mod menu_item;
mod popup_menu;
mod popup_menu_actions;
mod popup_menu_render;

pub use app_menu_bar::AppMenuBar;
pub use context_menu::{ContextMenuExt, WithContextMenu, WithContextMenuState};
pub use dropdown_menu::{PopupMenuExt, PopupMenuPopover};
pub use popup_menu::{PopupMenu, PopupMenuItem};

pub(crate) fn init(cx: &mut App) {
    app_menu_bar::init(cx);
    popup_menu::init(cx);
}
