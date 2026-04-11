use std::sync::Arc;

use inazuma::{
    App, AppContext, Axis, Bounds, Entity, Pixels, WeakEntity, Window,
};

use super::{
    DockArea, Panel, PanelView, StackPanel, TabPanel, TileItem, TileMeta, Tiles,
};

/// DockItem is a tree structure that represents the layout of the dock.
#[derive(Clone)]
pub enum DockItem {
    /// Split layout
    Split {
        axis: Axis,
        /// Self size, only used for build split panels
        size: Option<Pixels>,
        items: Vec<DockItem>,
        /// Items sizes
        sizes: Vec<Option<Pixels>>,
        view: Entity<StackPanel>,
    },
    /// Tab layout
    Tabs {
        /// Self size, only used for build split panels
        size: Option<Pixels>,
        items: Vec<Arc<dyn PanelView>>,
        active_ix: usize,
        view: Entity<TabPanel>,
    },
    /// Panel layout
    Panel {
        /// Self size, only used for build split panels
        size: Option<Pixels>,
        view: Arc<dyn PanelView>,
    },
    /// Tiles layout
    Tiles {
        /// Self size, only used for build split panels
        size: Option<Pixels>,
        items: Vec<TileItem>,
        view: Entity<Tiles>,
    },
}

impl std::fmt::Debug for DockItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DockItem::Split {
                axis, items, sizes, ..
            } => f
                .debug_struct("Split")
                .field("axis", axis)
                .field("items", &items.len())
                .field("sizes", sizes)
                .finish(),
            DockItem::Tabs {
                items, active_ix, ..
            } => f
                .debug_struct("Tabs")
                .field("items", &items.len())
                .field("active_ix", active_ix)
                .finish(),
            DockItem::Panel { .. } => f.debug_struct("Panel").finish(),
            DockItem::Tiles { .. } => f.debug_struct("Tiles").finish(),
        }
    }
}

impl DockItem {
    /// Get the size of the DockItem.
    pub(super) fn get_size(&self) -> Option<Pixels> {
        match self {
            Self::Split { size, .. } => *size,
            Self::Tabs { size, .. } => *size,
            Self::Panel { size, .. } => *size,
            Self::Tiles { size, .. } => *size,
        }
    }

    /// Set size for the DockItem.
    pub fn size(mut self, new_size: impl Into<Pixels>) -> Self {
        let new_size: Option<Pixels> = Some(new_size.into());
        match self {
            Self::Split { ref mut size, .. } => *size = new_size,
            Self::Tabs { ref mut size, .. } => *size = new_size,
            Self::Tiles { ref mut size, .. } => *size = new_size,
            Self::Panel { ref mut size, .. } => *size = new_size,
        }
        self
    }

    /// Set active index for the DockItem, only valid for [`DockItem::Tabs`].
    pub fn active_index(mut self, new_active_ix: usize, cx: &mut App) -> Self {
        debug_assert!(
            matches!(self, Self::Tabs { .. }),
            "active_ix can only be set for DockItem::Tabs"
        );

        if let Self::Tabs {
            ref mut active_ix,
            ref mut view,
            ..
        } = self
        {
            *active_ix = new_active_ix;
            view.update(cx, |tab_panel, _| {
                tab_panel.active_ix = new_active_ix;
            });
        }
        self
    }

    /// Create DockItem::Split with given split layout.
    pub fn split(
        axis: Axis,
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let sizes = items.iter().map(|item| item.get_size()).collect();
        Self::split_with_sizes(axis, items, sizes, dock_area, window, cx)
    }

    /// Create DockItem with vertical split layout.
    pub fn v_split(
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self::split(Axis::Vertical, items, dock_area, window, cx)
    }

    /// Create DockItem with horizontal split layout.
    pub fn h_split(
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self::split(Axis::Horizontal, items, dock_area, window, cx)
    }

    /// Create DockItem with split layout, each item of panel have specified size.
    ///
    /// Please note that the `items` and `sizes` must have the same length.
    /// Set `None` in `sizes` to make the index of panel have auto size.
    pub fn split_with_sizes(
        axis: Axis,
        items: Vec<DockItem>,
        sizes: Vec<Option<Pixels>>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let mut items = items;
        let stack_panel = cx.new(|cx| {
            let mut stack_panel = StackPanel::new(axis, window, cx);
            for (i, item) in items.iter_mut().enumerate() {
                let view = item.view();
                let size = sizes.get(i).copied().flatten();
                stack_panel.add_panel(view.clone(), size, dock_area.clone(), window, cx)
            }

            for (i, item) in items.iter().enumerate() {
                let view = item.view();
                let size = sizes.get(i).copied().flatten();
                stack_panel.add_panel(view.clone(), size, dock_area.clone(), window, cx)
            }

            stack_panel
        });

        window.defer(cx, {
            let stack_panel = stack_panel.clone();
            let dock_area = dock_area.clone();
            move |window, cx| {
                _ = dock_area.update(cx, |this, cx| {
                    this.subscribe_panel(&stack_panel, window, cx);
                });
            }
        });

        Self::Split {
            axis,
            size: None,
            items,
            sizes,
            view: stack_panel,
        }
    }

    /// Create DockItem with panel layout
    pub fn panel(panel: Arc<dyn PanelView>) -> Self {
        Self::Panel {
            size: None,
            view: panel,
        }
    }

    /// Create DockItem with tiles layout
    ///
    /// This items and metas should have the same length.
    pub fn tiles(
        items: Vec<DockItem>,
        metas: Vec<impl Into<TileMeta> + Copy>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        assert!(items.len() == metas.len());

        let tile_panel = cx.new(|cx| {
            let mut tiles = Tiles::new(window, cx);
            for (ix, item) in items.clone().into_iter().enumerate() {
                match item {
                    DockItem::Tabs { view, .. } => {
                        let meta: TileMeta = metas[ix].into();
                        let tile_item =
                            TileItem::new(Arc::new(view), meta.bounds).z_index(meta.z_index);
                        tiles.add_item(tile_item, dock_area, window, cx);
                    }
                    DockItem::Panel { view, .. } => {
                        let meta: TileMeta = metas[ix].into();
                        let tile_item =
                            TileItem::new(view.clone(), meta.bounds).z_index(meta.z_index);
                        tiles.add_item(tile_item, dock_area, window, cx);
                    }
                    _ => {
                        // Ignore non-tabs items
                    }
                }
            }
            tiles
        });

        window.defer(cx, {
            let tile_panel = tile_panel.clone();
            let dock_area = dock_area.clone();
            move |window, cx| {
                _ = dock_area.update(cx, |this, cx| {
                    this.subscribe_panel(&tile_panel, window, cx);
                    this.subscribe_tiles_item_drop(&tile_panel, window, cx);
                });
            }
        });

        Self::Tiles {
            size: None,
            items: tile_panel.read(cx).panels.clone(),
            view: tile_panel,
        }
    }

    /// Create DockItem with tabs layout, items are displayed as tabs.
    ///
    /// The `active_ix` is the index of the active tab, if `None` the first tab is active.
    pub fn tabs(
        items: Vec<Arc<dyn PanelView>>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let mut new_items: Vec<Arc<dyn PanelView>> = vec![];
        for item in items.into_iter() {
            new_items.push(item)
        }
        Self::new_tabs(new_items, None, dock_area, window, cx)
    }

    pub fn tab<P: Panel>(
        item: Entity<P>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self::new_tabs(vec![Arc::new(item.clone())], None, dock_area, window, cx)
    }

    fn new_tabs(
        items: Vec<Arc<dyn PanelView>>,
        active_ix: Option<usize>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let active_ix = active_ix.unwrap_or(0);
        let tab_panel = cx.new(|cx| {
            let mut tab_panel = TabPanel::new(None, dock_area.clone(), window, cx);
            for item in items.iter() {
                tab_panel.add_panel(item.clone(), window, cx)
            }
            tab_panel.active_ix = active_ix;
            tab_panel
        });

        Self::Tabs {
            size: None,
            items,
            active_ix,
            view: tab_panel,
        }
    }

    /// Returns the views of the dock item.
    pub fn view(&self) -> Arc<dyn PanelView> {
        match self {
            Self::Split { view, .. } => Arc::new(view.clone()),
            Self::Tabs { view, .. } => Arc::new(view.clone()),
            Self::Tiles { view, .. } => Arc::new(view.clone()),
            Self::Panel { view, .. } => view.clone(),
        }
    }

    /// Find existing panel in the dock item.
    pub fn find_panel(&self, panel: Arc<dyn PanelView>) -> Option<Arc<dyn PanelView>> {
        match self {
            Self::Split { items, .. } => {
                items.iter().find_map(|item| item.find_panel(panel.clone()))
            }
            Self::Tabs { items, .. } => items.iter().find(|item| *item == &panel).cloned(),
            Self::Panel { view, .. } => Some(view.clone()),
            Self::Tiles { items, .. } => items.iter().find_map(|item| {
                if &item.panel == &panel {
                    Some(item.panel.clone())
                } else {
                    None
                }
            }),
        }
    }

    /// Add a panel to the dock item.
    pub fn add_panel(
        &mut self,
        panel: Arc<dyn PanelView>,
        dock_area: &WeakEntity<DockArea>,
        bounds: Option<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        match self {
            Self::Tabs { view, items, .. } => {
                items.push(panel.clone());
                view.update(cx, |tab_panel, cx| {
                    tab_panel.add_panel(panel, window, cx);
                });
            }
            Self::Split { view, items, .. } => {
                // Iter items to add panel to the first tabs
                for item in items.into_iter() {
                    if let DockItem::Tabs { view, .. } = item {
                        view.update(cx, |tab_panel, cx| {
                            tab_panel.add_panel(panel.clone(), window, cx);
                        });
                        return;
                    }
                }

                // Unable to find tabs, create new tabs
                let new_item = Self::tabs(vec![panel.clone()], dock_area, window, cx);
                items.push(new_item.clone());
                view.update(cx, |stack_panel, cx| {
                    stack_panel.add_panel(new_item.view(), None, dock_area.clone(), window, cx);
                });
            }
            Self::Tiles { view, items, .. } => {
                let tile_item = TileItem::new(
                    Arc::new(cx.new(|cx| {
                        let mut tab_panel = TabPanel::new(None, dock_area.clone(), window, cx);
                        tab_panel.add_panel(panel.clone(), window, cx);
                        tab_panel
                    })),
                    bounds.unwrap_or_else(|| TileMeta::default().bounds),
                );

                items.push(tile_item.clone());
                view.update(cx, |tiles, cx| {
                    tiles.add_item(tile_item, dock_area, window, cx);
                });
            }
            Self::Panel { .. } => {}
        }
    }

    /// Remove a panel from the dock item.
    pub fn remove_panel(&self, panel: Arc<dyn PanelView>, window: &mut Window, cx: &mut App) {
        match self {
            DockItem::Tabs { view, .. } => {
                view.update(cx, |tab_panel, cx| {
                    tab_panel.remove_panel(panel, window, cx);
                });
            }
            DockItem::Split { items, view, .. } => {
                // For each child item, set collapsed state
                for item in items {
                    item.remove_panel(panel.clone(), window, cx);
                }
                view.update(cx, |split, cx| {
                    split.remove_panel(panel, window, cx);
                });
            }
            DockItem::Tiles { view, .. } => {
                view.update(cx, |tiles, cx| {
                    tiles.remove(panel, window, cx);
                });
            }
            DockItem::Panel { .. } => {}
        }
    }

    pub fn set_collapsed(&self, collapsed: bool, window: &mut Window, cx: &mut App) {
        match self {
            DockItem::Tabs { view, .. } => {
                view.update(cx, |tab_panel, cx| {
                    tab_panel.set_collapsed(collapsed, window, cx);
                });
            }
            DockItem::Split { items, .. } => {
                // For each child item, set collapsed state
                for item in items {
                    item.set_collapsed(collapsed, window, cx);
                }
            }
            DockItem::Tiles { .. } => {}
            DockItem::Panel { view, .. } => view.set_active(!collapsed, window, cx),
        }
    }

    /// Recursively traverses to find the left-most and top-most TabPanel.
    pub(crate) fn left_top_tab_panel(&self, cx: &App) -> Option<Entity<TabPanel>> {
        match self {
            DockItem::Tabs { view, .. } => Some(view.clone()),
            DockItem::Split { view, .. } => view.read(cx).left_top_tab_panel(true, cx),
            DockItem::Tiles { .. } => None,
            DockItem::Panel { .. } => None,
        }
    }

    /// Recursively traverses to find the right-most and top-most TabPanel.
    pub(crate) fn right_top_tab_panel(&self, cx: &App) -> Option<Entity<TabPanel>> {
        match self {
            DockItem::Tabs { view, .. } => Some(view.clone()),
            DockItem::Split { view, .. } => view.read(cx).right_top_tab_panel(true, cx),
            DockItem::Tiles { .. } => None,
            DockItem::Panel { .. } => None,
        }
    }
}
