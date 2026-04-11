use std::{
    any::Any,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use crate::{
    ActiveTheme,
    History, HistoryItem,
    ScrollbarShow,
};

use super::{
    DockArea, Panel, PanelEvent, PanelInfo, PanelState, PanelView, StackPanel, TabPanel, TileMeta,
};
use inazuma::{
    App, Bounds, Context, Empty, EntityId, EventEmitter, FocusHandle,
    IntoElement, Pixels, Point, Render, ScrollHandle, Size, WeakEntity, Window,
    actions, px, size,
};

actions!(tiles, [Undo, Redo]);

pub(super) const MINIMUM_SIZE: Size<Pixels> = size(px(100.), px(100.));
pub(super) const DRAG_BAR_HEIGHT: Pixels = px(30.);
pub(super) const HANDLE_SIZE: Pixels = px(5.0);

#[derive(Clone, PartialEq, Debug)]
pub(super) struct TileChange {
    pub(super) tile_id: EntityId,
    pub(super) old_bounds: Option<Bounds<Pixels>>,
    pub(super) new_bounds: Option<Bounds<Pixels>>,
    pub(super) old_order: Option<usize>,
    pub(super) new_order: Option<usize>,
    pub(super) version: usize,
}

impl HistoryItem for TileChange {
    fn version(&self) -> usize {
        self.version
    }

    fn set_version(&mut self, version: usize) {
        self.version = version;
    }
}

#[derive(Clone)]
pub struct DragMoving(pub(super) EntityId);
impl Render for DragMoving {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, PartialEq)]
pub(super) enum ResizeSide {
    Left,
    Right,
    Top,
    Bottom,
    BottomRight,
}

#[derive(Clone)]
pub struct DragResizing(pub(super) EntityId);

impl Render for DragResizing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
pub(super) struct ResizeDrag {
    pub(super) side: ResizeSide,
    pub(super) last_position: Point<Pixels>,
    pub(super) last_bounds: Bounds<Pixels>,
}

/// TileItem is a moveable and resizable panel that can be added to a Tiles view.
#[derive(Clone)]
pub struct TileItem {
    pub(super) id: EntityId,
    pub(crate) panel: Arc<dyn PanelView>,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) z_index: usize,
}

impl Debug for TileItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileItem")
            .field("bounds", &self.bounds)
            .field("z_index", &self.z_index)
            .finish()
    }
}

impl TileItem {
    pub fn new(panel: Arc<dyn PanelView>, bounds: Bounds<Pixels>) -> Self {
        Self {
            id: panel.view().entity_id(),
            panel,
            bounds,
            z_index: 0,
        }
    }

    pub fn z_index(mut self, z_index: usize) -> Self {
        self.z_index = z_index;
        self
    }
}

#[derive(Clone, Debug)]
pub struct AnyDrag {
    pub value: Arc<dyn Any>,
}

impl AnyDrag {
    pub fn new(value: impl Any) -> Self {
        Self {
            value: Arc::new(value),
        }
    }
}

/// Tiles is a canvas that can contain multiple panels, each of which can be dragged and resized.
pub struct Tiles {
    pub(super) focus_handle: FocusHandle,
    pub(crate) panels: Vec<TileItem>,
    pub(super) dragging_id: Option<EntityId>,
    pub(super) dragging_initial_mouse: Point<Pixels>,
    pub(super) dragging_initial_bounds: Bounds<Pixels>,
    pub(super) resizing_id: Option<EntityId>,
    pub(super) resizing_drag_data: Option<ResizeDrag>,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) history: History<TileChange>,
    pub(super) scroll_handle: ScrollHandle,
    pub(super) scrollbar_show: Option<ScrollbarShow>,
}

impl Panel for Tiles {
    fn panel_name(&self) -> &'static str {
        "Tiles"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Tiles".into_any_element()
    }

    fn dump(&self, cx: &App) -> PanelState {
        let panels = self
            .panels
            .iter()
            .map(|item: &TileItem| item.panel.dump(cx))
            .collect();

        let metas = self
            .panels
            .iter()
            .map(|item: &TileItem| TileMeta {
                bounds: item.bounds,
                z_index: item.z_index,
            })
            .collect();

        let mut state = PanelState::new(self);
        state.panel_name = self.panel_name().to_string();
        state.children = panels;
        state.info = PanelInfo::Tiles { metas };
        state
    }
}

#[derive(Clone, Debug)]
pub struct DragDrop(pub AnyDrag);

impl EventEmitter<DragDrop> for Tiles {}

impl Tiles {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            panels: vec![],
            dragging_id: None,
            dragging_initial_mouse: Point::default(),
            dragging_initial_bounds: Bounds::default(),
            resizing_id: None,
            scrollbar_show: None,
            resizing_drag_data: None,
            bounds: Bounds::default(),
            history: History::new().group_interval(std::time::Duration::from_millis(100)),
            scroll_handle: ScrollHandle::default(),
        }
    }

    /// Set the scrollbar show mode [`ScrollbarShow`], if not set use the `cx.theme().scrollbar_show`.
    pub fn set_scrollbar_show(
        &mut self,
        scrollbar_show: Option<ScrollbarShow>,
        cx: &mut Context<Self>,
    ) {
        self.scrollbar_show = scrollbar_show;
        cx.notify();
    }

    pub fn panels(&self) -> &[TileItem] {
        &self.panels
    }

    pub(super) fn sorted_panels(&self) -> Vec<TileItem> {
        let mut items: Vec<(usize, TileItem)> = self.panels.iter().cloned().enumerate().collect();
        items.sort_by(|a, b| a.1.z_index.cmp(&b.1.z_index).then_with(|| a.0.cmp(&b.0)));
        items.into_iter().map(|(_, item)| item).collect()
    }

    /// Return the index of the panel.
    #[inline]
    pub(crate) fn index_of(&self, id: &EntityId) -> Option<usize> {
        self.panels.iter().position(|p| &p.id == id)
    }

    #[inline]
    pub(crate) fn panel(&self, id: &EntityId) -> Option<&TileItem> {
        self.panels.iter().find(|p| &p.id == id)
    }

    /// Remove panel from the children.
    pub fn remove(&mut self, panel: Arc<dyn PanelView>, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.index_of(&panel.panel_id(cx)) {
            self.panels.remove(ix);

            cx.emit(PanelEvent::LayoutChanged);
        }
    }

    /// Calculate magnetic snap position for the dragging panel
    fn calculate_magnetic_snap(
        &self,
        dragging_bounds: Bounds<Pixels>,
        item_ix: usize,
        snap_threshold: Pixels,
    ) -> (Option<Pixels>, Option<Pixels>) {
        // Only check nearby panels
        let search_bounds = Bounds {
            origin: Point {
                x: dragging_bounds.left() - snap_threshold,
                y: dragging_bounds.top() - snap_threshold,
            },
            size: Size {
                width: dragging_bounds.size.width + snap_threshold * 2.0,
                height: dragging_bounds.size.height + snap_threshold * 2.0,
            },
        };

        let mut snap_x: Option<Pixels> = None;
        let mut snap_y: Option<Pixels> = None;
        let mut min_x_dist = snap_threshold;
        let mut min_y_dist = snap_threshold;

        // Pre-calculate dragging bounds edges to avoid repeated method calls
        let drag_left = dragging_bounds.left();
        let drag_right = dragging_bounds.right();
        let drag_top = dragging_bounds.top();
        let drag_bottom = dragging_bounds.bottom();
        let drag_width = dragging_bounds.size.width;
        let drag_height = dragging_bounds.size.height;

        // Check for edge snapping first (top and left boundaries)
        let edge_snap_pos = px(0.);

        // Snap to top edge
        let top_dist = drag_top.abs();
        if top_dist < snap_threshold {
            snap_y = Some(edge_snap_pos);
            min_y_dist = top_dist;
        }

        // Snap to left edge
        let left_dist = drag_left.abs();
        if left_dist < snap_threshold {
            snap_x = Some(edge_snap_pos);
            min_x_dist = left_dist;
        }

        // If both edges are snapped, return early
        if snap_x.is_some() && snap_y.is_some() {
            return (snap_x, snap_y);
        }

        for (ix, other) in self.panels.iter().enumerate() {
            if ix == item_ix {
                continue;
            }

            // Pre-calculate other bounds edges
            let other_left = other.bounds.left();
            let other_right = other.bounds.right();
            let other_top = other.bounds.top();
            let other_bottom = other.bounds.bottom();

            // Skip panels that are far away
            if other_right < search_bounds.left()
                || other_left > search_bounds.right()
                || other_bottom < search_bounds.top()
                || other_top > search_bounds.bottom()
            {
                continue;
            }

            // Horizontal snapping (X axis) - find closest snap point
            if snap_x.is_none() {
                let candidates = [
                    ((drag_left - other_left).abs(), other_left),
                    ((drag_left - other_right).abs(), other_right),
                    ((drag_right - other_left).abs(), other_left - drag_width),
                    ((drag_right - other_right).abs(), other_right - drag_width),
                ];

                for (dist, snap_pos) in candidates {
                    if dist < min_x_dist {
                        min_x_dist = dist;
                        snap_x = Some(snap_pos);
                    }
                }
            }

            // Vertical snapping (Y axis) - find closest snap point
            if snap_y.is_none() {
                let candidates = [
                    ((drag_top - other_top).abs(), other_top),
                    ((drag_top - other_bottom).abs(), other_bottom),
                    ((drag_bottom - other_top).abs(), other_top - drag_height),
                    (
                        (drag_bottom - other_bottom).abs(),
                        other_bottom - drag_height,
                    ),
                ];

                for (dist, snap_pos) in candidates {
                    if dist < min_y_dist {
                        min_y_dist = dist;
                        snap_y = Some(snap_pos);
                    }
                }
            }

            // Early exit if both axes are snapped
            if snap_x.is_some() && snap_y.is_some() {
                break;
            }
        }

        (snap_x, snap_y)
    }

    /// Apply boundary constraints to the panel origin
    fn apply_boundary_constraints(&self, mut origin: Point<Pixels>) -> Point<Pixels> {
        // Top boundary
        if origin.y < px(0.) {
            origin.y = px(0.);
        }

        // Left boundary (allow partial off-screen but keep 64px visible)
        let min_left = -self.dragging_initial_bounds.size.width + px(64.);
        if origin.x < min_left {
            origin.x = min_left;
        }

        origin
    }

    pub(super) fn update_position(&mut self, mouse_position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(dragging_id) = self.dragging_id else {
            return;
        };

        let Some(item_ix) = self.panels.iter().position(|p| p.id == dragging_id) else {
            return;
        };

        let previous_bounds = self.panels[item_ix].bounds;
        let adjusted_position = mouse_position - self.bounds.origin;
        let delta = adjusted_position - self.dragging_initial_mouse;
        let mut new_origin = self.dragging_initial_bounds.origin + delta;

        // Apply magnetic snap before boundary checks
        let snap_threshold = px(4.);
        let dragging_bounds = Bounds {
            origin: new_origin,
            size: self.dragging_initial_bounds.size,
        };

        let (snap_x, snap_y) =
            self.calculate_magnetic_snap(dragging_bounds, item_ix, snap_threshold);

        // Apply snapping
        if let Some(x) = snap_x {
            new_origin.x = x;
        }
        if let Some(y) = snap_y {
            new_origin.y = y;
        }

        // Apply boundary constraints after snapping
        new_origin = self.apply_boundary_constraints(new_origin);

        // Update position without grid rounding (smooth dragging)
        if new_origin != previous_bounds.origin {
            self.panels[item_ix].bounds.origin = new_origin;
            let item = &self.panels[item_ix];
            let bounds = item.bounds;
            let entity_id = item.panel.view().entity_id();

            if !self.history.ignore {
                self.history.push(TileChange {
                    tile_id: entity_id,
                    old_bounds: Some(previous_bounds),
                    new_bounds: Some(bounds),
                    old_order: None,
                    new_order: None,
                    version: 0,
                });
            }
            cx.notify();
        }
    }

    pub(super) fn resize(
        &mut self,
        new_x: Option<Pixels>,
        new_y: Option<Pixels>,
        new_width: Option<Pixels>,
        new_height: Option<Pixels>,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(resizing_id) = self.resizing_id else {
            return;
        };
        let Some(item) = self.panels.iter_mut().find(|item| item.id == resizing_id) else {
            return;
        };

        let previous_bounds = item.bounds;
        let final_x = if let Some(x) = new_x {
            round_to_nearest_ten(x, cx)
        } else {
            previous_bounds.origin.x
        };
        let final_y = if let Some(y) = new_y {
            round_to_nearest_ten(y, cx)
        } else {
            previous_bounds.origin.y
        };
        let final_width = if let Some(width) = new_width {
            round_to_nearest_ten(width, cx)
        } else {
            previous_bounds.size.width
        };

        let final_height = if let Some(height) = new_height {
            round_to_nearest_ten(height, cx)
        } else {
            previous_bounds.size.height
        };

        // Only push to history if size has changed
        if final_width != item.bounds.size.width
            || final_height != item.bounds.size.height
            || final_x != item.bounds.origin.x
            || final_y != item.bounds.origin.y
        {
            item.bounds.origin.x = final_x;
            item.bounds.origin.y = final_y;
            item.bounds.size.width = final_width;
            item.bounds.size.height = final_height;

            // Only push if not during history operations
            if !self.history.ignore {
                self.history.push(TileChange {
                    tile_id: item.panel.view().entity_id(),
                    old_bounds: Some(previous_bounds),
                    new_bounds: Some(item.bounds),
                    old_order: None,
                    new_order: None,
                    version: 0,
                });
            }
        }

        cx.notify();
    }

    pub fn add_item(
        &mut self,
        item: TileItem,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(tab_panel) = item.panel.view().downcast::<TabPanel>() else {
            panic!("only allows to add TabPanel type")
        };

        tab_panel.update(cx, |tab_panel, _| {
            tab_panel.set_in_tiles(true);
        });

        self.panels.push(item.clone());
        window.defer(cx, {
            let panel = item.panel.clone();
            let dock_area = dock_area.clone();

            move |window, cx| {
                // Subscribe to the panel's layout change event.
                _ = dock_area.update(cx, |this, cx| {
                    if let Ok(tab_panel) = panel.view().downcast::<TabPanel>() {
                        this.subscribe_panel(&tab_panel, window, cx);
                    }
                });
            }
        });

        cx.emit(PanelEvent::LayoutChanged);
        cx.notify();
    }

    #[inline]
    pub(super) fn reset_current_index(&mut self) {
        self.dragging_id = None;
        self.resizing_id = None;
    }

    /// Bring the panel of target_index to front, returns (old_index, new_index) if successful
    pub(super) fn bring_to_front(
        &mut self,
        target_id: Option<EntityId>,
        cx: &mut Context<Self>,
    ) -> Option<EntityId> {
        let Some(old_id) = target_id else {
            return None;
        };

        let old_ix = self.panels.iter().position(|item| item.id == old_id)?;
        if old_ix < self.panels.len() {
            let item = self.panels.remove(old_ix);
            self.panels.push(item);
            let new_ix = self.panels.len() - 1;
            let new_id = self.panels[new_ix].id;
            self.history.push(TileChange {
                tile_id: new_id,
                old_bounds: None,
                new_bounds: None,
                old_order: Some(old_ix),
                new_order: Some(new_ix),
                version: 0,
            });
            cx.notify();
            return Some(new_id);
        }
        None
    }

    /// Handle the undo action
    pub fn undo(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.history.ignore = true;

        if let Some(changes) = self.history.undo() {
            for change in changes {
                if let Some(index) = self
                    .panels
                    .iter()
                    .position(|item| item.panel.view().entity_id() == change.tile_id)
                {
                    if let Some(old_bounds) = change.old_bounds {
                        self.panels[index].bounds = old_bounds;
                    }
                    if let Some(old_order) = change.old_order {
                        let item = self.panels.remove(index);
                        self.panels.insert(old_order, item);
                    }
                }
            }
            cx.emit(PanelEvent::LayoutChanged);
        }

        self.history.ignore = false;
        cx.notify();
    }

    /// Handle the redo action
    pub fn redo(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.history.ignore = true;

        if let Some(changes) = self.history.redo() {
            for change in changes {
                if let Some(index) = self
                    .panels
                    .iter()
                    .position(|item| item.panel.view().entity_id() == change.tile_id)
                {
                    if let Some(new_bounds) = change.new_bounds {
                        self.panels[index].bounds = new_bounds;
                    }
                    if let Some(new_order) = change.new_order {
                        let item = self.panels.remove(index);
                        self.panels.insert(new_order, item);
                    }
                }
            }
            cx.emit(PanelEvent::LayoutChanged);
        }

        self.history.ignore = false;
        cx.notify();
    }

    /// Returns the active panel, if any.
    pub fn active_panel(&self, cx: &App) -> Option<Arc<dyn PanelView>> {
        self.panels.last().and_then(|item| {
            if let Ok(tab_panel) = item.panel.view().downcast::<TabPanel>() {
                tab_panel.read(cx).active_panel(cx)
            } else if let Ok(_) = item.panel.view().downcast::<StackPanel>() {
                None
            } else {
                Some(item.panel.clone())
            }
        })
    }

}

#[inline]
pub(super) fn round_to_nearest_ten(value: Pixels, cx: &App) -> Pixels {
    (value / px(4.)).round() * px(4.)
}

#[inline]
pub(super) fn round_point_to_nearest_ten(point: Point<Pixels>, cx: &App) -> Point<Pixels> {
    Point::new(
        round_to_nearest_ten(point.x, cx),
        round_to_nearest_ten(point.y, cx),
    )
}
