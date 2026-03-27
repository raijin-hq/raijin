//! Renderable terminal content for the UI layer.

use crate::grid::GridIterator;
use crate::index::Point;
use crate::selection::SelectionRange;
use crate::term::cell::{Cell, Flags};
use crate::term::color;
use crate::term::{Term, TermMode};
use crate::vte::ansi::CursorShape;

/// Terminal cursor rendering information.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct RenderableCursor {
    pub shape: CursorShape,
    pub point: Point,
}

impl RenderableCursor {
    pub(crate) fn new<T>(term: &Term<T>) -> Self {
        let vi_mode = term.mode().contains(TermMode::VI);
        let mut point = if vi_mode { term.vi_mode_cursor.point } else { term.block_router.active_grid().cursor.point };
        if term.block_router.active_grid()[point].flags.contains(Flags::WIDE_CHAR_SPACER) {
            point.column -= 1;
        }

        let shape = if !vi_mode && !term.mode().contains(TermMode::SHOW_CURSOR) {
            CursorShape::Hidden
        } else {
            term.cursor_style().shape
        };

        Self { shape, point }
    }
}

/// Visible terminal content — everything needed to render the current view.
pub struct RenderableContent<'a> {
    pub display_iter: GridIterator<'a, Cell>,
    pub selection: Option<SelectionRange>,
    pub cursor: RenderableCursor,
    pub display_offset: usize,
    pub colors: &'a color::Colors,
    pub mode: TermMode,
}

impl<'a> RenderableContent<'a> {
    pub(crate) fn new<T>(term: &'a Term<T>) -> Self {
        Self {
            display_iter: term.grid().display_iter(),
            display_offset: term.grid().display_offset(),
            cursor: RenderableCursor::new(term),
            selection: term.selection.as_ref().and_then(|s| s.to_range(term)),
            colors: &term.colors,
            mode: *term.mode(),
        }
    }
}
