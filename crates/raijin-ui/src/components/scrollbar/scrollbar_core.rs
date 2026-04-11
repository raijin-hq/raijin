use std::{
    cell::Cell,
    ops::Deref,
    panic::Location,
    rc::Rc,
};

use instant::Instant;

use raijin_theme::ActiveTheme;
use inazuma::AxisExt;
use inazuma::{
    App, Axis, ElementId, Oklch, IntoElement, ListState, Pixels, Point, ScrollHandle, Size,
    UniformListScrollHandle, point, px,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The width of the scrollbar (THUMB_ACTIVE_INSET * 2 + THUMB_ACTIVE_WIDTH)
pub(super) const WIDTH: Pixels = px(4. * 2. + 8.);
pub(super) const MIN_THUMB_SIZE: f32 = 48.;

pub(super) const THUMB_WIDTH: Pixels = px(6.);
pub(super) const THUMB_RADIUS: Pixels = px(6. / 2.);
pub(super) const THUMB_INSET: Pixels = px(4.);

pub(super) const THUMB_ACTIVE_WIDTH: Pixels = px(8.);
pub(super) const THUMB_ACTIVE_RADIUS: Pixels = px(8. / 2.);
pub(super) const THUMB_ACTIVE_INSET: Pixels = px(4.);

pub(super) const FADE_OUT_DURATION: f32 = 3.0;
pub(super) const FADE_OUT_DELAY: f32 = 2.0;

/// Scrollbar show mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Default, JsonSchema)]
pub enum ScrollbarShow {
    /// Show scrollbar when scrolling, will fade out after idle.
    #[default]
    Scrolling,
    /// Show scrollbar on hover.
    Hover,
    /// Always show scrollbar.
    Always,
}

impl ScrollbarShow {
    pub(super) fn is_hover(&self) -> bool {
        matches!(self, Self::Hover)
    }

    pub(super) fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }
}

/// A trait for scroll handles that can get and set offset.
pub trait ScrollbarHandle: 'static {
    /// Get the current offset of the scroll handle.
    fn offset(&self) -> Point<Pixels>;
    /// Set the offset of the scroll handle.
    fn set_offset(&self, offset: Point<Pixels>);
    /// The full size of the content, including padding.
    fn content_size(&self) -> Size<Pixels>;
    /// Called when start dragging the scrollbar thumb.
    fn start_drag(&self) {}
    /// Called when end dragging the scrollbar thumb.
    fn end_drag(&self) {}
}

impl ScrollbarHandle for ScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.offset()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset(offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        (self.max_offset() + self.bounds().size.into()).into()
    }
}

impl ScrollbarHandle for UniformListScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.0.borrow().base_handle.offset()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.0.borrow_mut().base_handle.set_offset(offset)
    }

    fn content_size(&self) -> Size<Pixels> {
        let base_handle = &self.0.borrow().base_handle;
        (base_handle.max_offset() + base_handle.bounds().size.into()).into()
    }
}

impl ScrollbarHandle for ListState {
    fn offset(&self) -> Point<Pixels> {
        self.scroll_px_offset_for_scrollbar()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset_from_scrollbar(offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        self.viewport_bounds().size + self.max_offset_for_scrollbar().into()
    }

    fn start_drag(&self) {
        self.scrollbar_drag_started();
    }

    fn end_drag(&self) {
        self.scrollbar_drag_ended();
    }
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub(super) struct ScrollbarState(Rc<Cell<ScrollbarStateInner>>);

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub(super) struct ScrollbarStateInner {
    pub(super) hovered_axis: Option<Axis>,
    pub(super) hovered_on_thumb: Option<Axis>,
    pub(super) dragged_axis: Option<Axis>,
    pub(super) drag_pos: Point<Pixels>,
    pub(super) last_scroll_offset: Point<Pixels>,
    pub(super) last_scroll_time: Option<Instant>,
    pub(super) last_update: Instant,
    pub(super) idle_timer_scheduled: bool,
}

impl Default for ScrollbarState {
    fn default() -> Self {
        Self(Rc::new(Cell::new(ScrollbarStateInner {
            hovered_axis: None,
            hovered_on_thumb: None,
            dragged_axis: None,
            drag_pos: point(px(0.), px(0.)),
            last_scroll_offset: point(px(0.), px(0.)),
            last_scroll_time: None,
            last_update: Instant::now(),
            idle_timer_scheduled: false,
        })))
    }
}

impl Deref for ScrollbarState {
    type Target = Rc<Cell<ScrollbarStateInner>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ScrollbarStateInner {
    pub(super) fn with_drag_pos(&self, axis: Axis, pos: Point<Pixels>) -> Self {
        let mut state = *self;
        if axis.is_vertical() {
            state.drag_pos.y = pos.y;
        } else {
            state.drag_pos.x = pos.x;
        }

        state.dragged_axis = Some(axis);
        state
    }

    pub(super) fn with_unset_drag_pos(&self) -> Self {
        let mut state = *self;
        state.dragged_axis = None;
        state
    }

    pub(super) fn with_hovered(&self, axis: Option<Axis>) -> Self {
        let mut state = *self;
        state.hovered_axis = axis;
        if axis.is_some() {
            state.last_scroll_time = Some(Instant::now());
        }
        state
    }

    pub(super) fn with_hovered_on_thumb(&self, axis: Option<Axis>) -> Self {
        let mut state = *self;
        state.hovered_on_thumb = axis;
        if self.is_scrollbar_visible() {
            if axis.is_some() {
                state.last_scroll_time = Some(Instant::now());
            }
        }
        state
    }

    pub(super) fn with_last_scroll(
        &self,
        last_scroll_offset: Point<Pixels>,
        last_scroll_time: Option<Instant>,
    ) -> Self {
        let mut state = *self;
        state.last_scroll_offset = last_scroll_offset;
        state.last_scroll_time = last_scroll_time;
        state
    }

    pub(super) fn with_last_scroll_time(&self, t: Option<Instant>) -> Self {
        let mut state = *self;
        state.last_scroll_time = t;
        state
    }

    pub(super) fn with_last_update(&self, t: Instant) -> Self {
        let mut state = *self;
        state.last_update = t;
        state
    }

    pub(super) fn with_idle_timer_scheduled(&self, scheduled: bool) -> Self {
        let mut state = *self;
        state.idle_timer_scheduled = scheduled;
        state
    }

    pub(super) fn is_scrollbar_visible(&self) -> bool {
        // On drag
        if self.dragged_axis.is_some() {
            return true;
        }

        if let Some(last_time) = self.last_scroll_time {
            let elapsed = Instant::now().duration_since(last_time).as_secs_f32();
            elapsed < FADE_OUT_DURATION
        } else {
            false
        }
    }
}

/// Scrollbar axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarAxis {
    /// Vertical scrollbar.
    Vertical,
    /// Horizontal scrollbar.
    Horizontal,
    /// Show both vertical and horizontal scrollbars.
    Both,
}

impl From<Axis> for ScrollbarAxis {
    fn from(axis: Axis) -> Self {
        match axis {
            Axis::Vertical => Self::Vertical,
            Axis::Horizontal => Self::Horizontal,
        }
    }
}

impl ScrollbarAxis {
    /// Return true if the scrollbar axis is vertical.
    #[inline]
    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Vertical)
    }

    /// Return true if the scrollbar axis is horizontal.
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal)
    }

    /// Return true if the scrollbar axis is both vertical and horizontal.
    #[inline]
    pub fn is_both(&self) -> bool {
        matches!(self, Self::Both)
    }

    /// Return true if the scrollbar has vertical axis.
    #[inline]
    pub fn has_vertical(&self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }

    /// Return true if the scrollbar has horizontal axis.
    #[inline]
    pub fn has_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    #[inline]
    pub(super) fn all(&self) -> Vec<Axis> {
        match self {
            Self::Vertical => vec![Axis::Vertical],
            Self::Horizontal => vec![Axis::Horizontal],
            // This should keep Horizontal first, Vertical is the primary axis
            // if Vertical not need display, then Horizontal will not keep right margin.
            Self::Both => vec![Axis::Horizontal, Axis::Vertical],
        }
    }
}

/// Scrollbar control for scroll-area or a uniform-list.
pub struct Scrollbar {
    pub(super) id: ElementId,
    pub(super) axis: ScrollbarAxis,
    pub(super) scrollbar_show: Option<ScrollbarShow>,
    pub(super) scroll_handle: Rc<dyn ScrollbarHandle>,
    pub(super) scroll_size: Option<Size<Pixels>>,
    /// Maximum frames per second for scrolling by drag. Default is 120 FPS.
    ///
    /// This is used to limit the update rate of the scrollbar when it is
    /// being dragged for some complex interactions for reducing CPU usage.
    pub(super) max_fps: usize,
}

impl Scrollbar {
    /// Create a new scrollbar.
    ///
    /// This will have both vertical and horizontal scrollbars.
    #[track_caller]
    pub fn new<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        let caller = Location::caller();
        Self {
            id: ElementId::CodeLocation(*caller),
            axis: ScrollbarAxis::Both,
            scrollbar_show: None,
            scroll_handle: Rc::new(scroll_handle.clone()),
            max_fps: 120,
            scroll_size: None,
        }
    }

    /// Create with horizontal scrollbar.
    #[track_caller]
    pub fn horizontal<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        Self::new(scroll_handle).axis(ScrollbarAxis::Horizontal)
    }

    /// Create with vertical scrollbar.
    #[track_caller]
    pub fn vertical<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        Self::new(scroll_handle).axis(ScrollbarAxis::Vertical)
    }

    /// Set a specific element id, default is the [`Location::caller`].
    ///
    /// NOTE: In most cases, you don't need to set a specific id for scrollbar.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the scrollbar show mode [`ScrollbarShow`], if not set use the `cx.theme().scrollbar_show`.
    pub fn scrollbar_show(mut self, scrollbar_show: ScrollbarShow) -> Self {
        self.scrollbar_show = Some(scrollbar_show);
        self
    }

    /// Set a special scroll size of the content area, default is None.
    ///
    /// Default will sync the `content_size` from `scroll_handle`.
    pub fn scroll_size(mut self, scroll_size: Size<Pixels>) -> Self {
        self.scroll_size = Some(scroll_size);
        self
    }

    /// Set scrollbar axis.
    pub fn axis(mut self, axis: impl Into<ScrollbarAxis>) -> Self {
        self.axis = axis.into();
        self
    }

    /// Set maximum frames per second for scrolling by drag. Default is 120 FPS.
    ///
    /// If you have very high CPU usage, consider reducing this value to improve performance.
    ///
    /// Available values: 30..120
    pub(crate) fn max_fps(mut self, max_fps: usize) -> Self {
        self.max_fps = max_fps.clamp(30, 120);
        self
    }

    // Get the width of the scrollbar.
    pub(crate) const fn width() -> Pixels {
        WIDTH
    }

    pub(super) fn style_for_active(cx: &App) -> (Oklch, Oklch, Oklch, Pixels, Pixels, Pixels) {
        (
            cx.theme().colors().scrollbar.thumb_hover_background,
            cx.theme().colors().scrollbar.track_background,
            cx.theme().colors().border,
            THUMB_ACTIVE_WIDTH,
            THUMB_ACTIVE_INSET,
            THUMB_ACTIVE_RADIUS,
        )
    }

    pub(super) fn style_for_hovered_thumb(cx: &App) -> (Oklch, Oklch, Oklch, Pixels, Pixels, Pixels) {
        (
            cx.theme().colors().scrollbar.thumb_hover_background,
            cx.theme().colors().scrollbar.track_background,
            cx.theme().colors().border,
            THUMB_ACTIVE_WIDTH,
            THUMB_ACTIVE_INSET,
            THUMB_ACTIVE_RADIUS,
        )
    }

    pub(super) fn style_for_hovered_bar(cx: &App) -> (Oklch, Oklch, Oklch, Pixels, Pixels, Pixels) {
        (
            cx.theme().colors().scrollbar.thumb_background,
            cx.theme().colors().scrollbar.track_background,
            Oklch::transparent_black(),
            THUMB_ACTIVE_WIDTH,
            THUMB_ACTIVE_INSET,
            THUMB_ACTIVE_RADIUS,
        )
    }

    pub(super) fn style_for_normal(&self, cx: &App) -> (Oklch, Oklch, Oklch, Pixels, Pixels, Pixels) {
        let scrollbar_show = self.scrollbar_show.unwrap_or(ScrollbarShow::Scrolling);
        let (width, inset, radius) = match scrollbar_show {
            ScrollbarShow::Scrolling => (THUMB_WIDTH, THUMB_INSET, THUMB_RADIUS),
            _ => (THUMB_ACTIVE_WIDTH, THUMB_ACTIVE_INSET, THUMB_ACTIVE_RADIUS),
        };

        (
            cx.theme().colors().scrollbar.thumb_background,
            cx.theme().colors().scrollbar.track_background,
            Oklch::transparent_black(),
            width,
            inset,
            radius,
        )
    }

    pub(super) fn style_for_idle(&self, cx: &App) -> (Oklch, Oklch, Oklch, Pixels, Pixels, Pixels) {
        let scrollbar_show = self.scrollbar_show.unwrap_or(ScrollbarShow::Scrolling);
        let (width, inset, radius) = match scrollbar_show {
            ScrollbarShow::Scrolling => (THUMB_WIDTH, THUMB_INSET, THUMB_RADIUS),
            _ => (THUMB_ACTIVE_WIDTH, THUMB_ACTIVE_INSET, THUMB_ACTIVE_RADIUS),
        };

        (
            Oklch::transparent_black(),
            Oklch::transparent_black(),
            Oklch::transparent_black(),
            width,
            inset,
            radius,
        )
    }
}

impl IntoElement for Scrollbar {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[doc(hidden)]
pub struct PrepaintState {
    pub(super) hitbox: inazuma::Hitbox,
    pub(super) scrollbar_state: ScrollbarState,
    pub(super) states: Vec<AxisPrepaintState>,
}

#[doc(hidden)]
pub struct AxisPrepaintState {
    pub(super) axis: Axis,
    pub(super) bar_hitbox: inazuma::Hitbox,
    pub(super) bounds: inazuma::Bounds<Pixels>,
    pub(super) radius: Pixels,
    pub(super) bg: Oklch,
    pub(super) border: Oklch,
    pub(super) thumb_bounds: inazuma::Bounds<Pixels>,
    pub(super) thumb_fill_bounds: inazuma::Bounds<Pixels>,
    pub(super) thumb_bg: Oklch,
    pub(super) scroll_size: Pixels,
    pub(super) container_size: Pixels,
    pub(super) thumb_size: Pixels,
    pub(super) margin_end: Pixels,
}
