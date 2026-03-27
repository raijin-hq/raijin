//! Scrollable list of terminal blocks.
//!
//! Renders all blocks (finished + active) as a vertical list.
//! Each block is rendered via block_element::render_block().

use inazuma::{
    div, px, IntoElement, ParentElement, Styled, Window,
};
use raijin_term::block_grid::BlockGridRouter;
use raijin_term::term::color::Colors;

use super::block_element::render_block;
use super::constants::*;

/// Render the scrollable block list from the BlockGridRouter.
pub fn render_block_list(
    router: &BlockGridRouter,
    colors: &Colors,
    font: &inazuma::Font,
    font_size: inazuma::Pixels,
    cell_width: inazuma::Pixels,
    cell_height: inazuma::Pixels,
    selected_block: Option<usize>,
    window: &mut Window,
) -> impl IntoElement {
    let blocks = router.blocks();

    let mut list = div()
        .flex()
        .flex_col()
        .w_full()
        .gap(px(BLOCK_GAP));

    for (i, block) in blocks.iter().enumerate() {
        let is_selected = selected_block == Some(i);
        list = list.child(render_block(
            block,
            colors,
            font,
            font_size,
            cell_width,
            cell_height,
            is_selected,
            window,
        ));
    }

    list
}
