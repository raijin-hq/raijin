mod app_context_impl;
mod entity_context;

use crate::{
    AnyView, AnyWindowHandle, AppContext, AsyncApp, DispatchPhase, Effect, EntityId, EventEmitter,
    FocusHandle, FocusOutEvent, Focusable, Global, KeystrokeObserver, Priority, Reservation,
    SubscriberSet, Subscription, Task, WeakEntity, WeakFocusHandle, Window, WindowHandle,
};
use anyhow::Result;
use futures::FutureExt;
use std::{
    any::{Any, TypeId},
    borrow::{Borrow, BorrowMut},
    future::Future,
    ops,
    sync::Arc,
};
use inazuma_util::Deferred;

use super::{App, AsyncWindowContext, Entity, KeystrokeEvent};

pub use entity_context::*;
