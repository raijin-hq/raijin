use inazuma::App;

mod column;
mod data_table;
mod delegate;
mod loading;
mod state;
mod state_actions;
mod state_render;
mod table;

pub use column::*;
pub use delegate::*;
pub use state::*;
pub use table::*;

pub(crate) fn init(cx: &mut App) {
    data_table::init(cx);
}
