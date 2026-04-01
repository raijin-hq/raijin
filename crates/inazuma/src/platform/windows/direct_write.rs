mod helpers;
mod state_impl_core;
mod state_impl_rasterize;
mod text_renderer;
mod text_system_impl;
mod types;

use std::{
    borrow::Cow,
    ffi::{c_uint, c_void},
    mem::ManuallyDrop,
};

use ::util::{ResultExt, maybe};
use anyhow::{Context, Result};
use collections::HashMap;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use windows::{
    Win32::{
        Foundation::*,
        Globalization::GetUserDefaultLocaleName,
        Graphics::{
            Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, Direct3D11::*, DirectWrite::*,
            Dxgi::Common::*, Gdi::LOGFONTW,
        },
        System::SystemServices::LOCALE_NAME_MAX_LENGTH,
        UI::WindowsAndMessaging::*,
    },
    core::*,
};
use windows_numerics::Vector2;

use crate::*;
use inazuma::*;

pub(crate) use types::*;
pub(crate) use helpers::DEFAULT_LOCALE_NAME;
