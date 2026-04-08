#![deny(unsafe_op_in_unsafe_fn)]

mod helpers;
mod platform_window;
mod types;
mod window_impl;

use std::{
    cell::{Cell, RefCell},
    num::NonZeroIsize,
    path::PathBuf,
    rc::{Rc, Weak},
    str::FromStr,
    sync::{Arc, Once, atomic::AtomicBool},
    time::{Duration, Instant},
};

use ::inazuma_util::ResultExt;
use anyhow::{Context as _, Result};
use futures::channel::oneshot::{self, Receiver};
use raw_window_handle as rwh;
use smallvec::SmallVec;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Dwm::*,
        Graphics::Gdi::*,
        System::{Com::*, LibraryLoader::*, Ole::*, SystemServices::*},
        UI::{Controls::*, HiDpi::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
    core::*,
};

use crate::*;
use inazuma::*;

pub(crate) use types::*;
pub(crate) use helpers::{WindowBorderOffset, window_from_hwnd};
