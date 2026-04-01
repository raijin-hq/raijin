use crate::Arena;
use std::cell::{Cell, RefCell};

thread_local! {
    /// Fallback arena used when no app-specific arena is active.
    /// In production, each window draw sets CURRENT_ELEMENT_ARENA to the app's arena.
    pub(crate) static ELEMENT_ARENA: RefCell<Arena> = RefCell::new(Arena::new(1024 * 1024));

    /// Points to the current App's element arena during draw operations.
    /// This allows multiple test Apps to have isolated arenas, preventing
    /// cross-session corruption when the scheduler interleaves their tasks.
    static CURRENT_ELEMENT_ARENA: Cell<Option<*const RefCell<Arena>>> = const { Cell::new(None) };
}

/// Allocates an element in the current arena. Uses the app-specific arena if one
/// is active (during draw), otherwise falls back to the thread-local ELEMENT_ARENA.
pub(crate) fn with_element_arena<R>(f: impl FnOnce(&mut Arena) -> R) -> R {
    CURRENT_ELEMENT_ARENA.with(|current| {
        if let Some(arena_ptr) = current.get() {
            // SAFETY: The pointer is valid for the duration of the draw operation
            // that set it, and we're being called during that same draw.
            let arena_cell = unsafe { &*arena_ptr };
            f(&mut arena_cell.borrow_mut())
        } else {
            ELEMENT_ARENA.with_borrow_mut(f)
        }
    })
}

/// RAII guard that sets CURRENT_ELEMENT_ARENA for the duration of a draw operation.
/// When dropped, restores the previous arena (supporting nested draws).
pub(crate) struct ElementArenaScope {
    previous: Option<*const RefCell<Arena>>,
}

impl ElementArenaScope {
    /// Enter a scope where element allocations use the given arena.
    pub(crate) fn enter(arena: &RefCell<Arena>) -> Self {
        let previous = CURRENT_ELEMENT_ARENA.with(|current| {
            let prev = current.get();
            current.set(Some(arena as *const RefCell<Arena>));
            prev
        });
        Self { previous }
    }
}

impl Drop for ElementArenaScope {
    fn drop(&mut self) {
        CURRENT_ELEMENT_ARENA.with(|current| {
            current.set(self.previous);
        });
    }
}

/// Returned when the element arena has been used and so must be cleared before the next draw.
#[must_use]
pub struct ArenaClearNeeded {
    arena: *const RefCell<Arena>,
}

impl ArenaClearNeeded {
    /// Create a new ArenaClearNeeded that will clear the given arena.
    pub(crate) fn new(arena: &RefCell<Arena>) -> Self {
        Self {
            arena: arena as *const RefCell<Arena>,
        }
    }

    /// Clear the element arena.
    pub fn clear(self) {
        // SAFETY: The arena pointer is valid because ArenaClearNeeded is created
        // at the end of draw() and must be cleared before the next draw.
        let arena_cell = unsafe { &*self.arena };
        arena_cell.borrow_mut().clear();
    }
}
