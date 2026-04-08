mod key_handling;
mod wndproc;
mod wndproc_ext;

use std::rc::Rc;

use ::inazuma_util::ResultExt;
use anyhow::Context as _;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::SystemServices::*,
        UI::{
            Controls::*,
            HiDpi::*,
            Input::{Ime::*, KeyboardAndMouse::*},
            WindowsAndMessaging::*,
        },
    },
    core::PCWSTR,
};

use crate::*;
use inazuma::*;

pub(crate) use wndproc::*;
pub(crate) use key_handling::{current_capslock, current_modifiers};
