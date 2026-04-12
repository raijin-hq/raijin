pub mod actions;
pub(crate) mod anchored;
pub mod async_util;
mod constants;
mod corner_solver;

pub use constants::*;
pub use corner_solver::{CornerSolver, inner_corner_radius};
pub mod color_contrast;
pub mod focus_trap;
pub mod history;
pub mod index_path;
pub mod menu_global_state;
pub mod popover_global_state;
pub mod search_input;
pub mod text_global_state;
pub mod virtual_list;
pub mod with_rem_size;
pub mod window_border;
pub mod window_ext;

pub use color_contrast::calculate_contrast_ratio;
pub use index_path::IndexPath;
pub use with_rem_size::WithRemSize;
pub use menu_global_state::MenuGlobalState;
pub use popover_global_state::PopoverGlobalState;
pub use text_global_state::TextGlobalState;
pub use search_input::SearchInputWidth;

/// Returns the platform-appropriate label for the "reveal in file manager" action.
pub fn reveal_in_file_manager_label(is_remote: bool) -> &'static str {
    if cfg!(target_os = "macos") && !is_remote {
        "Reveal in Finder"
    } else if cfg!(target_os = "windows") && !is_remote {
        "Reveal in File Explorer"
    } else {
        "Reveal in File Manager"
    }
}

use inazuma::SharedString;

/// Capitalize the first character of a string.
pub fn capitalize(s: &str) -> SharedString {
    let mut chars = s.chars();
    match chars.next() {
        None => SharedString::default(),
        Some(c) => {
            let upper: String = c.to_uppercase().chain(chars).collect();
            SharedString::from(upper)
        }
    }
}
