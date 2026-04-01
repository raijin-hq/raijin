mod storage;
mod weak;

use crate::{App, AppContext, GpuiBorrow, VisualContext, Window, seal::Sealed};
use anyhow::{Context as _, Result};
use collections::FxHashSet;
use derive_more::{Deref, DerefMut};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use slotmap::{KeyData, SecondaryMap, SlotMap};
use std::{
    any::{Any, TypeId, type_name},
    cell::RefCell,
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU64,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, AtomicUsize, Ordering::SeqCst},
    },
    thread::panicking,
};

use super::Context;
use crate::local_util::atomic_incr_if_not_zero;
#[cfg(any(test, feature = "leak-detection"))]
use collections::HashMap;

pub use storage::*;
pub use weak::*;
