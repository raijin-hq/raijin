// Sub-modules
mod arena;
mod focus;
mod frame;
mod hitbox;
mod invalidator;
mod paint_quad;
mod prompts;
mod window_actions;
mod window_api;
mod window_bounds;
mod window_draw;
mod window_element;
mod window_events;
mod window_handle;
mod window_init;
mod window_inspector;
mod window_layout;
mod window_paint;
mod window_state;

// Re-exports — public types
pub use arena::*;
pub use focus::*;
pub use hitbox::*;
pub use paint_quad::*;
pub use prompts::*;
pub use window_handle::*;
pub use window_init::*;
pub use window_state::*;

// Re-exports — crate-internal types
pub(crate) use frame::*;
pub(crate) use invalidator::*;

use crate::{Pixels, Size};

// Shared imports for sub-modules (accessible via `use super::*`)
#[cfg(any(feature = "inspector", debug_assertions))]
pub(super) use crate::Inspector;
pub(super) use crate::scheduler::Instant;
pub(super) use crate::{
    AnyElement, AnyTooltip, CursorStyle, DispatchNodeId, Entity, EntityId,
    IsZero, KeyDownEvent, Keystroke, Modifiers, ResizeEdge, SubscriberSet,
    TaffyLayoutEngine, ThermalState, WindowBounds, WindowDecorations,
    WindowTextSystem, point, px, size,
};
pub(super) use anyhow::Context as AnyhowContext;
pub(super) use inazuma_util::post_inc;
pub(super) use inazuma_util::{ResultExt, measure};

// Std imports shared across sub-modules
pub(super) use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

/// Default window size used when no explicit size is provided.
pub const DEFAULT_WINDOW_SIZE: Size<Pixels> = size(px(1536.), px(864.));

/// A 6:5 aspect ratio minimum window size to be used for functional,
/// additional-to-main-Raijin windows, like the settings and rules library windows.
pub const DEFAULT_ADDITIONAL_WINDOW_SIZE: Size<Pixels> = Size {
    width: Pixels(900.),
    height: Pixels(750.),
};
