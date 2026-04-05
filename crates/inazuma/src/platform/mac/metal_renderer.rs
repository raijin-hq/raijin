mod render_pass;
mod setup;
mod types;

use super::metal_atlas::MetalAtlas;
use anyhow::Result;
use inazuma::{
    AtlasTextureId, Background, Bounds, ContentMask, DevicePixels, MonochromeSprite, PaintSurface,
    Path, Point, PolychromeSprite, PrimitiveBatch, Quad, ScaledPixels, Scene, Shadow, Size,
    Surface, Underline, point, size,
};
#[cfg(any(test, feature = "test-support"))]
use image::RgbaImage;

use objc2_core_video::{
    CVMetalTextureCache,
    CVMetalTextureGetTexture, CVPixelBufferGetHeightOfPlane, CVPixelBufferGetWidthOfPlane,
    CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth, CVPixelBufferGetHeight,
    kCVPixelFormatType_420YpCbCr8BiPlanarFullRange, kCVReturnSuccess,
};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSRange;
use objc2_metal::*;
use objc2_quartz_core::*;
use parking_lot::Mutex;

use std::{ffi::c_void, mem, ptr, sync::Arc};
use std::ptr::NonNull;

pub(crate) use types::{PointF, Context, InstanceBuffer, InstanceBufferPool, Renderer, new_renderer};
pub use types::{PathRasterizationVertex, PathSprite, SurfaceBounds};
pub(crate) use setup::MetalRenderer;
#[cfg(any(test, feature = "test-support"))]
pub use render_pass::MetalHeadlessRenderer;
use types::*;
