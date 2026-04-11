mod dock;
mod dock_area;
mod dock_item;
mod invalid_panel;
mod panel;
mod stack_panel;
mod state;
mod tab_panel;
mod tab_panel_actions;
mod tab_panel_render;
mod tiles;
mod tiles_render;

use inazuma::{App, actions};

pub use dock::*;
pub use dock_area::*;
pub use dock_item::*;
pub use panel::*;
pub use stack_panel::*;
pub use state::*;
pub use tab_panel::*;
pub use tiles::*;

pub(crate) fn init(cx: &mut App) {
    PanelRegistry::init(cx);
}

actions!(dock, [ToggleZoom, ClosePanel]);

/// Events emitted by the dock area.
pub enum DockEvent {
    /// The layout of the dock has changed, subscribers this to save the layout.
    ///
    /// This event is emitted when every time the layout of the dock has changed,
    /// So it emits may be too frequently, you may want to debounce the event.
    LayoutChanged,

    /// The drag item drop event.
    DragDrop(AnyDrag),
}
