mod helpers;
mod pipeline_state;
mod renderer_impl;
mod resources_impl;
mod types;

use std::{
    slice,
    sync::{Arc, OnceLock},
};

use ::inazuma_util::ResultExt;
use anyhow::{Context, Result};
use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::*,
            Direct3D11::*,
            DirectComposition::*,
            DirectWrite::*,
            Dxgi::{Common::*, *},
        },
    },
    core::Interface,
};

use crate::directx_renderer::shader_resources::{RawShaderBytes, ShaderModule, ShaderTarget};
use crate::*;
use inazuma::*;

pub(crate) use types::*;
pub(crate) use helpers::shader_resources;
