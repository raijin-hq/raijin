mod pipelines;
mod renderer_core;
mod rendering;
mod types;

use crate::{CompositorGpuHint, WgpuAtlas, WgpuContext};
use bytemuck::{Pod, Zeroable};
use inazuma::{
    AtlasTextureId, Background, Bounds, DevicePixels, GpuSpecs, MonochromeSprite, Path, Point,
    PolychromeSprite, PrimitiveBatch, Quad, ScaledPixels, Scene, Shadow, Size, SubpixelSprite,
    Underline, get_gamma_correction_ratios,
};
use log::warn;
#[cfg(not(target_family = "wasm"))]
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::cell::RefCell;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub use types::{GpuContext, WgpuRenderer, WgpuSurfaceConfig};
pub(crate) use types::*;
