//! # UI – Raijin UI Primitives & Components
//!
//! This crate provides a set of UI primitives and components that are used to build all of the elements in Raijin's UI.
//!
//! ## Related Crates:
//!
//! - [`ui_macros`] - proc_macros support for this crate
//! - `ui_input` - the single line input component

pub mod component_prelude;
mod components;
pub mod prelude;
mod styles;
mod traits;
pub mod utils;

pub use components::*;
pub use prelude::*;
pub use styles::*;
pub use traits::animation_ext::*;
pub use traits::collapsible::*;
pub use traits::element_ext::*;
pub use traits::event::*;
pub use traits::focusable_ext::*;
pub use traits::selectable::*;
pub use traits::size::*;
pub use traits::styled_ext::box_shadow;
pub use utils::index_path::IndexPath;
pub use utils::virtual_list::{VirtualListScrollHandle, v_virtual_list};
pub use utils::with_rem_size::WithRemSize;
pub use utils::{MenuGlobalState, PopoverGlobalState, TextGlobalState};

// Re-export inazuma geometry types used by IC components
pub use inazuma::{Anchor, Placement, Side, AxisExt};

// Re-export utils submodules used by IC components
pub use utils::actions;
pub use utils::history::{History, HistoryItem};
pub use utils::window_ext::WindowExt;
pub use utils::focus_trap::{FocusTrapContainer, FocusTrapElement};
pub use utils::capitalize;

/// Initialize all raijin-ui components, registering keybindings and global state.
///
/// Must be called once during application startup.
pub fn init(cx: &mut inazuma::App) {
    components::init(cx);
    utils::focus_trap::init(cx);
}
