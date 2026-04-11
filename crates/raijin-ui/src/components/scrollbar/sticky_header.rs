//! Sticky header overlay for [`ListState`]-backed lists.
//!
//! When a list item is partially scrolled out of the viewport, the sticky
//! header renders a pinned overlay at the top showing that item's header
//! content. This gives the user context about which item produced the
//! currently visible content — the same UX pattern as Warp terminal's
//! sticky command blocks.
//!
//! Usage:
//! ```ignore
//! StickyHeader::new(&list_state, item_count, |ix, window, cx| {
//!     // Return the header element for item `ix`
//!     div().child(format!("Header for item {}", ix)).into_any_element()
//! })
//! ```

use inazuma::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ListState, Pixels, Style, Window,
    px, size,
};

/// A sticky header overlay that pins the topmost partially-scrolled item's
/// header to the top of the viewport.
///
/// Generic over the header content — the consumer provides a closure that
/// builds the header element for a given item index. The closure receives
/// the same `(index, window, cx)` signature as the list's render callback.
pub struct StickyHeader {
    list_state: ListState,
    item_count: usize,
    header_height: Pixels,
    render_header: Box<dyn FnOnce(usize, &mut Window, &mut App) -> AnyElement>,
    /// The item index whose header should be sticky (computed during prepaint).
    sticky_index: Option<usize>,
}

impl StickyHeader {
    /// Create a new sticky header bound to a list.
    ///
    /// - `list_state` — the list whose scroll position is tracked
    /// - `item_count` — total number of items in the list
    /// - `header_height` — fixed height of each item's header region
    /// - `render_header` — closure that builds the header element for a given item index
    pub fn new(
        list_state: &ListState,
        item_count: usize,
        header_height: Pixels,
        render_header: impl FnOnce(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            list_state: list_state.clone(),
            item_count,
            header_height,
            render_header: Box::new(render_header),
            sticky_index: None,
        }
    }

    /// Find the item whose header should be pinned.
    ///
    /// Returns the index of the first item whose top edge is above the
    /// viewport top but whose bottom edge is still visible (i.e., the item
    /// is partially scrolled off).
    fn find_sticky_item(&self) -> Option<usize> {
        let viewport = self.list_state.viewport_bounds();
        if viewport.size.height <= px(0.) {
            return None;
        }

        let viewport_top = viewport.origin.y;

        for ix in 0..self.item_count {
            if let Some(item_bounds) = self.list_state.bounds_for_item(ix) {
                let item_top = item_bounds.origin.y;
                let item_bottom = item_top + item_bounds.size.height;

                // Item's top is above viewport AND its content is still visible
                // (bottom extends below viewport_top + header_height)
                if item_top < viewport_top
                    && item_bottom > viewport_top + self.header_height
                {
                    return Some(ix);
                }
            }
        }
        None
    }
}

impl IntoElement for StickyHeader {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Prepaint state for the sticky header.
pub struct StickyHeaderPrepaint {
    header_element: Option<AnyElement>,
    bounds: Bounds<Pixels>,
}

impl Element for StickyHeader {
    type RequestLayoutState = ();
    type PrepaintState = StickyHeaderPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Absolute positioning — overlays the parent container.
        let mut style = Style::default();
        style.position = inazuma::Position::Absolute;
        style.inset.top = px(0.).into();
        style.inset.right = px(0.).into();
        style.inset.bottom = px(0.).into();
        style.inset.left = px(0.).into();

        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // Determine which item needs a sticky header
        self.sticky_index = self.find_sticky_item();

        let header_element = if let Some(ix) = self.sticky_index {
            let render = std::mem::replace(
                &mut self.render_header,
                Box::new(|_, _, _| inazuma::div().into_any_element()),
            );
            let mut element = render(ix, window, cx);

            // Layout the header element within the sticky bounds
            let header_bounds = Bounds::new(
                bounds.origin,
                size(bounds.size.width, self.header_height),
            );
            element.prepaint_as_root(
                header_bounds.origin,
                inazuma::Size {
                    width: inazuma::AvailableSpace::Definite(header_bounds.size.width),
                    height: inazuma::AvailableSpace::Definite(header_bounds.size.height),
                },
                window,
                cx,
            );

            Some(element)
        } else {
            None
        };

        StickyHeaderPrepaint {
            header_element,
            bounds,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(ref mut element) = prepaint.header_element {
            let header_bounds = Bounds::new(
                prepaint.bounds.origin,
                size(prepaint.bounds.size.width, self.header_height),
            );

            // The header element (provided by the consumer) is responsible for its
            // own background, border, and text. We just clip it to the header region.
            window.with_content_mask(
                Some(inazuma::ContentMask { bounds: header_bounds }),
                |window| {
                    element.paint(window, cx);
                },
            );
        }
    }
}
