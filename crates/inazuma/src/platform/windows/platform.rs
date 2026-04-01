mod helpers;
mod platform_impl;
mod platform_new;
mod types;

use std::{
    cell::{Cell, RefCell},
    ffi::OsStr,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use ::util::{ResultExt, paths::SanitizedPath};
use anyhow::{Context as _, Result, anyhow};
use futures::channel::oneshot::{self, Receiver};
use itertools::Itertools;
use parking_lot::RwLock;
use smallvec::SmallVec;
use windows::{
    UI::ViewManagement::UISettings,
    Win32::{
        Foundation::*,
        Graphics::{Direct3D11::ID3D11Device, Gdi::*},
        Security::Credentials::*,
        System::{Com::*, LibraryLoader::*, Ole::*, SystemInformation::*},
        UI::{Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
    core::*,
};

use crate::*;
use inazuma::*;

pub use types::WindowsPlatform;
pub(crate) use types::*;
