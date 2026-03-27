//! Bottom-anchored, virtualized block list using Inazuma's `list()` element.
//!
//! Uses `ListAlignment::Bottom` for Warp-style bottom-anchoring:
//! - Content at the bottom, empty space above when few blocks
//! - Normal scrolling when content exceeds viewport
//! - Auto-scroll-to-bottom on new blocks
//! - Virtualized rendering: only visible blocks are rendered
//! - Overlay scrollbar via Scrollbar component

use inazuma::{
    div, list, Font, InteractiveElement, IntoElement, ListState, ParentElement, Styled,
};
use inazuma_component::scroll::{Scrollbar, ScrollbarShow};
use raijin_terminal::TerminalHandle;

use super::block_element::render_block;
use super::grid_snapshot::{BlockSnapshotCache, extract_all_block_snapshots};

/// Sync the ListState item count with the actual block count.
fn sync_block_count(list_state: &ListState, block_count: usize) {
    let current_count = list_state.item_count();
    if block_count != current_count {
        if block_count > current_count {
            list_state.splice(current_count..current_count, block_count - current_count);
        } else {
            list_state.reset(block_count);
        }
    }
}

/// Build the block list element with scrollbar.
///
/// Extracts all block data with a single lock, syncs the ListState,
/// and returns a container with `list()` + overlay scrollbar.
pub fn render_block_list(
    handle: &TerminalHandle,
    list_state: &ListState,
    cache: &mut BlockSnapshotCache,
    symbol_maps: &[raijin_settings::ResolvedSymbolMap],
    font: &Font,
    font_size: f32,
    selected_block: Option<usize>,
) -> impl IntoElement {
    let snapshots = extract_all_block_snapshots(handle, cache, symbol_maps);
    sync_block_count(list_state, snapshots.len());

    let font = font.clone();
    let block_list = list(list_state.clone(), move |ix, _window, _cx| {
        if let Some(snapshot) = snapshots.get(ix).cloned() {
            let is_selected = selected_block == Some(ix);
            render_block(snapshot, &font, font_size, is_selected).into_any_element()
        } else {
            div().into_any_element()
        }
    });

    div()
        .id("block-list-container")
        .flex_1()
        .min_h_0()
        .relative()
        .overflow_hidden()
        .child(block_list.size_full())
        .child(Scrollbar::vertical(list_state).scrollbar_show(ScrollbarShow::Always))
}
