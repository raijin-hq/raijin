mod helpers;
mod platform_impl;
mod types;

use std::{
    env,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};
#[cfg(any(feature = "wayland", feature = "x11"))]
use std::{
    ffi::OsString,
    fs::File,
    io::Read as _,
    os::fd::{AsFd, FromRawFd, IntoRawFd},
    time::Duration,
};

use crate::command::{new_command, new_std_command};
use anyhow::{Context as _, anyhow};
use calloop::LoopSignal;
use futures::channel::oneshot;
use inazuma_util::ResultExt as _;
#[cfg(any(feature = "wayland", feature = "x11"))]
use xkbcommon::xkb::{self, Keycode, Keysym, State};

use crate::platform::linux::{LinuxDispatcher, PriorityQueueCalloopReceiver};
use inazuma::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, DisplayId,
    ForegroundExecutor, Keymap, Menu, MenuItem, OwnedMenu, PathPromptOptions, Platform,
    PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem,
    PlatformWindow, Result, RunnableVariant, Task, ThermalState, WindowAppearance, WindowParams,
};
#[cfg(any(feature = "wayland", feature = "x11"))]
use inazuma::{Pixels, Point, px};

pub use types::*;
pub(crate) use helpers::*;
