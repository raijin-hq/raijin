use super::*;

/// Represents an element that can be scrolled *to* in its parent element.
/// Contrary to [ScrollHandle::scroll_to_active_item], an anchored element does not have to be an immediate child of the parent.
#[derive(Clone)]
pub struct ScrollAnchor {
    pub(super) handle: ScrollHandle,
    pub(super) last_origin: Rc<RefCell<Point<Pixels>>>,
}

impl ScrollAnchor {
    /// Creates a [ScrollAnchor] associated with a given [ScrollHandle].
    pub fn for_handle(handle: ScrollHandle) -> Self {
        Self {
            handle,
            last_origin: Default::default(),
        }
    }
    /// Request scroll to this item on the next frame.
    pub fn scroll_to(&self, window: &mut Window, _cx: &mut App) {
        let this = self.clone();

        window.on_next_frame(move |_, _| {
            let viewport_bounds = this.handle.bounds();
            let self_bounds = *this.last_origin.borrow();
            this.handle.set_offset(viewport_bounds.origin - self_bounds);
        });
    }
}

#[derive(Default, Debug)]
pub(super) struct ScrollHandleState {
    pub(super) offset: Rc<RefCell<Point<Pixels>>>,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) max_offset: Point<Pixels>,
    pub(super) child_bounds: Vec<Bounds<Pixels>>,
    pub(super) scroll_to_bottom: bool,
    pub(super) overflow: Point<Overflow>,
    pub(super) active_item: Option<ScrollActiveItem>,
}

#[derive(Default, Debug, Clone, Copy)]
pub(super) struct ScrollActiveItem {
    pub(super) index: usize,
    pub(super) strategy: ScrollStrategy,
}

#[derive(Default, Debug, Clone, Copy)]
pub(super) enum ScrollStrategy {
    #[default]
    FirstVisible,
    Top,
}

/// A handle to the scrollable aspects of an element.
/// Used for accessing scroll state, like the current scroll offset,
/// and for mutating the scroll state, like scrolling to a specific child.
#[derive(Clone, Debug)]
pub struct ScrollHandle(pub(super) Rc<RefCell<ScrollHandleState>>);

impl Default for ScrollHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollHandle {
    /// Construct a new scroll handle.
    pub fn new() -> Self {
        Self(Rc::default())
    }

    /// Get the current scroll offset.
    pub fn offset(&self) -> Point<Pixels> {
        *self.0.borrow().offset.borrow()
    }

    /// Get the maximum scroll offset.
    pub fn max_offset(&self) -> Point<Pixels> {
        self.0.borrow().max_offset
    }

    /// Get the top child that's scrolled into view.
    pub fn top_item(&self) -> usize {
        let state = self.0.borrow();
        let top = state.bounds.top() - state.offset.borrow().y;

        match state.child_bounds.binary_search_by(|bounds| {
            if top < bounds.top() {
                Ordering::Greater
            } else if top > bounds.bottom() {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }) {
            Ok(ix) => ix,
            Err(ix) => ix.min(state.child_bounds.len().saturating_sub(1)),
        }
    }

    /// Get the bottom child that's scrolled into view.
    pub fn bottom_item(&self) -> usize {
        let state = self.0.borrow();
        let bottom = state.bounds.bottom() - state.offset.borrow().y;

        match state.child_bounds.binary_search_by(|bounds| {
            if bottom < bounds.top() {
                Ordering::Greater
            } else if bottom > bounds.bottom() {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }) {
            Ok(ix) => ix,
            Err(ix) => ix.min(state.child_bounds.len().saturating_sub(1)),
        }
    }

    /// Return the bounds into which this child is painted
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.0.borrow().bounds
    }

    /// Get the bounds for a specific child.
    pub fn bounds_for_item(&self, ix: usize) -> Option<Bounds<Pixels>> {
        self.0.borrow().child_bounds.get(ix).cloned()
    }

    /// Update [ScrollHandleState]'s active item for scrolling to in prepaint
    pub fn scroll_to_item(&self, ix: usize) {
        let mut state = self.0.borrow_mut();
        state.active_item = Some(ScrollActiveItem {
            index: ix,
            strategy: ScrollStrategy::default(),
        });
    }

    /// Update [ScrollHandleState]'s active item for scrolling to in prepaint
    /// This scrolls the minimal amount to ensure that the child is the first visible element
    pub fn scroll_to_top_of_item(&self, ix: usize) {
        let mut state = self.0.borrow_mut();
        state.active_item = Some(ScrollActiveItem {
            index: ix,
            strategy: ScrollStrategy::Top,
        });
    }

    /// Scrolls the minimal amount to either ensure that the child is
    /// fully visible or the top element of the view depends on the
    /// scroll strategy
    pub(super) fn scroll_to_active_item(&self) {
        let mut state = self.0.borrow_mut();

        let Some(active_item) = state.active_item else {
            return;
        };

        let active_item = match state.child_bounds.get(active_item.index) {
            Some(bounds) => {
                let mut scroll_offset = state.offset.borrow_mut();

                match active_item.strategy {
                    ScrollStrategy::FirstVisible => {
                        if state.overflow.y == Overflow::Scroll {
                            let child_height = bounds.size.height;
                            let viewport_height = state.bounds.size.height;
                            if child_height > viewport_height {
                                scroll_offset.y = state.bounds.top() - bounds.top();
                            } else if bounds.top() + scroll_offset.y < state.bounds.top() {
                                scroll_offset.y = state.bounds.top() - bounds.top();
                            } else if bounds.bottom() + scroll_offset.y > state.bounds.bottom() {
                                scroll_offset.y = state.bounds.bottom() - bounds.bottom();
                            }
                        }
                    }
                    ScrollStrategy::Top => {
                        scroll_offset.y = state.bounds.top() - bounds.top();
                    }
                }

                if state.overflow.x == Overflow::Scroll {
                    let child_width = bounds.size.width;
                    let viewport_width = state.bounds.size.width;
                    if child_width > viewport_width {
                        scroll_offset.x = state.bounds.left() - bounds.left();
                    } else if bounds.left() + scroll_offset.x < state.bounds.left() {
                        scroll_offset.x = state.bounds.left() - bounds.left();
                    } else if bounds.right() + scroll_offset.x > state.bounds.right() {
                        scroll_offset.x = state.bounds.right() - bounds.right();
                    }
                }
                None
            }
            None => Some(active_item),
        };
        state.active_item = active_item;
    }

    /// Scrolls to the bottom.
    pub fn scroll_to_bottom(&self) {
        let mut state = self.0.borrow_mut();
        state.scroll_to_bottom = true;
    }

    /// Set the offset explicitly. The offset is the distance from the top left of the
    /// parent container to the top left of the first child.
    /// As you scroll further down the offset becomes more negative.
    pub fn set_offset(&self, position: Point<Pixels>) {
        let state = self.0.borrow();
        *state.offset.borrow_mut() = position;
    }

    /// Get the logical scroll top, based on a child index and a pixel offset.
    pub fn logical_scroll_top(&self) -> (usize, Pixels) {
        let ix = self.top_item();
        let state = self.0.borrow();

        if let Some(child_bounds) = state.child_bounds.get(ix) {
            (
                ix,
                child_bounds.top() + state.offset.borrow().y - state.bounds.top(),
            )
        } else {
            (ix, px(0.))
        }
    }

    /// Get the logical scroll bottom, based on a child index and a pixel offset.
    pub fn logical_scroll_bottom(&self) -> (usize, Pixels) {
        let ix = self.bottom_item();
        let state = self.0.borrow();

        if let Some(child_bounds) = state.child_bounds.get(ix) {
            (
                ix,
                child_bounds.bottom() + state.offset.borrow().y - state.bounds.bottom(),
            )
        } else {
            (ix, px(0.))
        }
    }

    /// Get the count of children for scrollable item.
    pub fn children_count(&self) -> usize {
        self.0.borrow().child_bounds.len()
    }
}
