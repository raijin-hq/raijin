mod interactive_text;
mod text_element;

use crate::{
    ActiveTooltip, AnyView, App, Bounds, DispatchPhase, Element, ElementId, GlobalElementId,
    HighlightStyle, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, Size, TextOverflow,
    TextRun, TextStyle, TooltipId, TruncateFrom, WhiteSpace, Window, WrappedLine,
    WrappedLineLayout, register_tooltip_mouse_handlers, set_tooltip_on_window,
};
use anyhow::Context as _;
use itertools::Itertools;
use smallvec::SmallVec;
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    mem,
    ops::Range,
    rc::Rc,
    sync::Arc,
};
use inazuma_util::ResultExt;

pub use interactive_text::*;
pub use text_element::*;
