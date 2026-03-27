//! Scrollable list of terminal blocks.
//!
//! Locks the terminal ONCE via extract_all_block_snapshots(), then renders
//! all blocks from pre-extracted snapshots. No per-block mutex locking.
//! Finished blocks are served from the cache (zero extraction cost).

use inazuma::{
    div, Font, IntoElement, ParentElement, Styled,
};
use raijin_terminal::TerminalHandle;

use super::block_element::render_block;
use super::grid_snapshot::{BlockSnapshotCache, extract_all_block_snapshots};

/// Render the block list from snapshots extracted with a single lock.
/// Finished blocks come from the cache — only active blocks are freshly extracted.
pub fn render_block_list(
    handle: &TerminalHandle,
    font: &Font,
    font_size: f32,
    selected_block: Option<usize>,
    cache: &mut BlockSnapshotCache,
    symbol_maps: &[raijin_settings::ResolvedSymbolMap],
) -> impl IntoElement {
    let snapshots = extract_all_block_snapshots(handle, cache, symbol_maps);

    let mut list = div()
        .flex()
        .flex_col()
        .w_full();

    for (i, snapshot) in snapshots.into_iter().enumerate() {
        let is_selected = selected_block == Some(i);
        list = list.child(render_block(snapshot, font, font_size, is_selected));
    }

    list
}
