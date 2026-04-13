use super::*;

use objc2_core_graphics::{
    CGColorSpace, kCGColorSpaceDisplayP3, kCGColorSpaceExtendedLinearDisplayP3, kCGColorSpaceSRGB,
};
use objc2_foundation::{NSSize, NSString};

pub(crate) struct MetalRenderer {
    pub(super) device: Retained<ProtocolObject<dyn MTLDevice>>,
    pub(super) layer: Option<Retained<CAMetalLayer>>,
    pub(super) is_apple_gpu: bool,
    pub(super) is_unified_memory: bool,
    pub(super) presents_with_transaction: bool,
    /// For headless rendering, tracks whether output should be opaque
    pub(super) opaque: bool,
    pub(super) command_queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pub(super) paths_rasterization_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) path_sprites_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) shadows_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) quads_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) underlines_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) monochrome_sprites_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) polychrome_sprites_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) surfaces_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub(super) unit_vertices: Retained<ProtocolObject<dyn MTLBuffer>>,
    #[allow(clippy::arc_with_non_send_sync)]
    pub(super) instance_buffer_pool: Arc<Mutex<InstanceBufferPool>>,
    pub(super) sprite_atlas: Arc<MetalAtlas>,
    pub(super) core_video_texture_cache: *mut CVMetalTextureCache,
    pub(super) path_intermediate_texture: Option<Retained<ProtocolObject<dyn MTLTexture>>>,
    pub(super) path_intermediate_msaa_texture: Option<Retained<ProtocolObject<dyn MTLTexture>>>,
    pub(super) path_sample_count: u32,
    pub(super) pixel_format: MTLPixelFormat,
}

impl MetalRenderer {
    /// Creates a new MetalRenderer with a CAMetalLayer for window-based rendering.
    pub fn new(
        instance_buffer_pool: Arc<Mutex<InstanceBufferPool>>,
        transparent: bool,
        colorspace: crate::WindowColorspace,
    ) -> Self {
        let device = Self::create_device();

        let is_hdr = matches!(colorspace, crate::WindowColorspace::Hdr);
        let pixel_format = if is_hdr {
            MTLPixelFormat::RGBA16Float
        } else {
            MTLPixelFormat::BGRA8Unorm
        };

        let layer = CAMetalLayer::new();
        layer.setDevice(Some(&device));
        layer.setPixelFormat(pixel_format);
        // Tag the layer with the correct colorspace:
        // - sRGB: safe default, prevents oversaturation on P3 displays
        // - Display P3: wider gamut (25% more colors), same gamma as sRGB
        // - HDR: extended linear P3, values > 1.0 = HDR brightness, RGBA16Float
        unsafe {
            let cs_name = match colorspace {
                crate::WindowColorspace::Hdr => kCGColorSpaceExtendedLinearDisplayP3,
                crate::WindowColorspace::DisplayP3 => kCGColorSpaceDisplayP3,
                _ => kCGColorSpaceSRGB,
            };
            if let Some(cs) = CGColorSpace::with_name(Some(cs_name)) {
                layer.setColorspace(Some(&cs));
            }
            if is_hdr {
                let _: () = msg_send![&layer, setWantsExtendedDynamicRangeContent: true];
            }
        }
        // Support direct-to-display rendering if the window is not transparent
        // https://developer.apple.com/documentation/metal/managing-your-game-window-for-metal-in-macos
        layer.setOpaque(!transparent);
        layer.setMaximumDrawableCount(3);
        // Allow texture reading for visual tests (captures screenshots without ScreenCaptureKit)
        #[cfg(any(test, feature = "test-support"))]
        layer.setFramebufferOnly(false);
        unsafe {
            let _: () = msg_send![&layer, setAllowsNextDrawableTimeout: false];
            let _: () = msg_send![&layer, setNeedsDisplayOnBoundsChange: true];
            let _: () = msg_send![
                &layer,
                setAutoresizingMask: 0x12u32
            ];
        }

        Self::new_internal(device, Some(layer), !transparent, instance_buffer_pool, pixel_format)
    }

    /// Creates a new headless MetalRenderer for offscreen rendering without a window.
    ///
    /// This renderer can render scenes to images without requiring a CAMetalLayer,
    /// window, or AppKit. Use `render_scene_to_image()` to render scenes.
    #[cfg(any(test, feature = "test-support"))]
    pub fn new_headless(instance_buffer_pool: Arc<Mutex<InstanceBufferPool>>) -> Self {
        let device = Self::create_device();
        Self::new_internal(device, None, true, instance_buffer_pool, MTLPixelFormat::BGRA8Unorm)
    }

    fn create_device() -> Retained<ProtocolObject<dyn MTLDevice>> {
        // Prefer low-power integrated GPUs on Intel Mac. On Apple
        // Silicon, there is only ever one GPU, so this is equivalent to
        // `MTLCreateSystemDefaultDevice()`.
        let all_devices = MTLCopyAllDevices();
        if let Some(d) = all_devices
            .into_iter()
            .min_by_key(|d| (d.isRemovable(), !d.isLowPower()))
        {
            d
        } else {
            // For some reason `MTLCopyAllDevices()` can return an empty list, see https://github.com/raijin-industries/raijin/issues/37689
            // In that case, we fall back to the system default device.
            log::error!(
                "Unable to enumerate Metal devices; attempting to use system default device"
            );
            MTLCreateSystemDefaultDevice().unwrap_or_else(|| {
                log::error!("unable to access a compatible graphics device");
                std::process::exit(1);
            })
        }
    }

    fn new_internal(
        device: Retained<ProtocolObject<dyn MTLDevice>>,
        layer: Option<Retained<CAMetalLayer>>,
        opaque: bool,
        instance_buffer_pool: Arc<Mutex<InstanceBufferPool>>,
        pixel_format: MTLPixelFormat,
    ) -> Self {
        #[cfg(feature = "runtime_shaders")]
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(&SHADERS_SOURCE_FILE), None)
            .expect("error building metal library");
        #[cfg(not(feature = "runtime_shaders"))]
        let library = device
            .newLibraryWithData_error(&dispatch2::DispatchData::from_bytes(SHADERS_METALLIB))
            .expect("error building metal library");

        fn to_float2_bits(point: PointF) -> u64 {
            let mut output = point.y.to_bits() as u64;
            output <<= 32;
            output |= point.x.to_bits() as u64;
            output
        }

        // Shared memory can be used only if CPU and GPU share the same memory space.
        // https://developer.apple.com/documentation/metal/setting-resource-storage-modes
        let is_unified_memory = device.hasUnifiedMemory();
        // Apple GPU families support memoryless textures, which can significantly reduce
        // memory usage by keeping render targets in on-chip tile memory instead of
        // allocating backing store in system memory.
        // https://developer.apple.com/documentation/metal/mtlgpufamily
        let is_apple_gpu = device.supportsFamily(MTLGPUFamily::Apple1);

        let unit_vertices = [
            to_float2_bits(point(0., 0.)),
            to_float2_bits(point(1., 0.)),
            to_float2_bits(point(0., 1.)),
            to_float2_bits(point(0., 1.)),
            to_float2_bits(point(1., 0.)),
            to_float2_bits(point(1., 1.)),
        ];
        let unit_vertices = unsafe {
            device.newBufferWithBytes_length_options(
                NonNull::new_unchecked(unit_vertices.as_ptr() as *mut c_void),
                mem::size_of_val(&unit_vertices),
                if is_unified_memory {
                    MTLResourceOptions::CPUCacheModeWriteCombined
                        | MTLResourceOptions::StorageModeShared
                } else {
                    MTLResourceOptions::StorageModeManaged
                },
            )
        }
        .expect("failed to create unit vertices buffer");

        let paths_rasterization_pipeline_state = build_path_rasterization_pipeline_state(
            &device,
            &library,
            "paths_rasterization",
            "path_rasterization_vertex",
            "path_rasterization_fragment",
            pixel_format,
            PATH_SAMPLE_COUNT,
        );
        let path_sprites_pipeline_state = build_path_sprite_pipeline_state(
            &device,
            &library,
            "path_sprites",
            "path_sprite_vertex",
            "path_sprite_fragment",
            pixel_format,
        );
        let shadows_pipeline_state = build_pipeline_state(
            &device,
            &library,
            "shadows",
            "shadow_vertex",
            "shadow_fragment",
            pixel_format,
        );
        let quads_pipeline_state = build_pipeline_state(
            &device,
            &library,
            "quads",
            "quad_vertex",
            "quad_fragment",
            pixel_format,
        );
        let underlines_pipeline_state = build_pipeline_state(
            &device,
            &library,
            "underlines",
            "underline_vertex",
            "underline_fragment",
            pixel_format,
        );
        let monochrome_sprites_pipeline_state = build_pipeline_state(
            &device,
            &library,
            "monochrome_sprites",
            "monochrome_sprite_vertex",
            "monochrome_sprite_fragment",
            pixel_format,
        );
        let polychrome_sprites_pipeline_state = build_pipeline_state(
            &device,
            &library,
            "polychrome_sprites",
            "polychrome_sprite_vertex",
            "polychrome_sprite_fragment",
            pixel_format,
        );
        let surfaces_pipeline_state = build_pipeline_state(
            &device,
            &library,
            "surfaces",
            "surface_vertex",
            "surface_fragment",
            pixel_format,
        );

        let command_queue = device.newCommandQueue().expect("failed to create command queue");
        let sprite_atlas = Arc::new(MetalAtlas::new(device.clone(), is_apple_gpu));

        let core_video_texture_cache = unsafe {
            let mut cache: *mut CVMetalTextureCache = ptr::null_mut();
            let status = CVMetalTextureCache::create(
                None,
                None,
                &device,
                None,
                NonNull::new_unchecked(&mut cache),
            );
            assert_eq!(status, kCVReturnSuccess, "failed to create CVMetalTextureCache");
            cache
        };

        Self {
            device,
            layer,
            presents_with_transaction: false,
            is_apple_gpu,
            is_unified_memory,
            opaque,
            command_queue,
            paths_rasterization_pipeline_state,
            path_sprites_pipeline_state,
            shadows_pipeline_state,
            quads_pipeline_state,
            underlines_pipeline_state,
            monochrome_sprites_pipeline_state,
            polychrome_sprites_pipeline_state,
            surfaces_pipeline_state,
            unit_vertices,
            instance_buffer_pool,
            sprite_atlas,
            core_video_texture_cache,
            path_intermediate_texture: None,
            path_intermediate_msaa_texture: None,
            path_sample_count: PATH_SAMPLE_COUNT,
            pixel_format,
        }
    }

    pub fn layer(&self) -> Option<&CAMetalLayer> {
        self.layer.as_deref()
    }

    pub fn layer_ptr(&self) -> *mut c_void {
        self.layer
            .as_ref()
            .map(|l| Retained::as_ptr(l) as *mut c_void)
            .unwrap_or(ptr::null_mut())
    }

    pub fn sprite_atlas(&self) -> &Arc<MetalAtlas> {
        &self.sprite_atlas
    }

    pub fn set_presents_with_transaction(&mut self, presents_with_transaction: bool) {
        self.presents_with_transaction = presents_with_transaction;
        if let Some(layer) = &self.layer {
            layer.setPresentsWithTransaction(presents_with_transaction);
        }
    }

    pub fn update_drawable_size(&mut self, size: Size<DevicePixels>) {
        if let Some(layer) = &self.layer {
            layer.setDrawableSize(NSSize {
                width: size.width.0 as f64,
                height: size.height.0 as f64,
            });
        }
        self.update_path_intermediate_textures(size);
    }

    pub(super) fn update_path_intermediate_textures(&mut self, size: Size<DevicePixels>) {
        // We are uncertain when this happens, but sometimes size can be 0 here. Most likely before
        // the layout pass on window creation. Zero-sized texture creation causes SIGABRT.
        // https://github.com/raijin-industries/raijin/issues/36229
        if size.width.0 <= 0 || size.height.0 <= 0 {
            self.path_intermediate_texture = None;
            self.path_intermediate_msaa_texture = None;
            return;
        }

        unsafe {
            let texture_descriptor = MTLTextureDescriptor::new();
            texture_descriptor.setWidth(size.width.0 as usize);
            texture_descriptor.setHeight(size.height.0 as usize);
            texture_descriptor.setPixelFormat(self.pixel_format);
            texture_descriptor.setStorageMode(MTLStorageMode::Private);
            texture_descriptor
                .setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
            self.path_intermediate_texture =
                Some(self.device.newTextureWithDescriptor(&texture_descriptor)
                    .expect("failed to create path intermediate texture"));

            if self.path_sample_count > 1 {
                // https://developer.apple.com/documentation/metal/choosing-a-resource-storage-mode-for-apple-gpus
                // Rendering MSAA textures are done in a single pass, so we can use memory-less storage on Apple Silicon
                let storage_mode = if self.is_apple_gpu {
                    MTLStorageMode::Memoryless
                } else {
                    MTLStorageMode::Private
                };

                let msaa_descriptor = texture_descriptor;
                msaa_descriptor.setTextureType(MTLTextureType::Type2DMultisample);
                msaa_descriptor.setStorageMode(storage_mode);
                msaa_descriptor.setSampleCount(self.path_sample_count as _);
                self.path_intermediate_msaa_texture =
                    Some(self.device.newTextureWithDescriptor(&msaa_descriptor)
                        .expect("failed to create path intermediate MSAA texture"));
            } else {
                self.path_intermediate_msaa_texture = None;
            }
        }
    }

    pub fn update_transparency(&mut self, transparent: bool) {
        self.opaque = !transparent;
        if let Some(layer) = &self.layer {
            layer.setOpaque(!transparent);
        }
    }

    pub fn destroy(&self) {
        // nothing to do
    }
}

pub(super) fn build_pipeline_state(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    label: &str,
    vertex_fn_name: &str,
    fragment_fn_name: &str,
    pixel_format: MTLPixelFormat,
) -> Retained<ProtocolObject<dyn MTLRenderPipelineState>> {
    let vertex_fn = library
        .newFunctionWithName(&NSString::from_str(vertex_fn_name))
        .expect("error locating vertex function");
    let fragment_fn = library
        .newFunctionWithName(&NSString::from_str(fragment_fn_name))
        .expect("error locating fragment function");

    let descriptor = MTLRenderPipelineDescriptor::new();
    descriptor.setLabel(Some(&NSString::from_str(label)));
    descriptor.setVertexFunction(Some(&vertex_fn));
    descriptor.setFragmentFunction(Some(&fragment_fn));
    let color_attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
    color_attachment.setPixelFormat(pixel_format);
    color_attachment.setBlendingEnabled(true);
    color_attachment.setRgbBlendOperation(MTLBlendOperation::Add);
    color_attachment.setAlphaBlendOperation(MTLBlendOperation::Add);
    color_attachment.setSourceRGBBlendFactor(MTLBlendFactor::SourceAlpha);
    color_attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
    color_attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    color_attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::One);

    device
        .newRenderPipelineStateWithDescriptor_error(&descriptor)
        .expect("could not create render pipeline state")
}

pub(super) fn build_path_sprite_pipeline_state(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    label: &str,
    vertex_fn_name: &str,
    fragment_fn_name: &str,
    pixel_format: MTLPixelFormat,
) -> Retained<ProtocolObject<dyn MTLRenderPipelineState>> {
    let vertex_fn = library
        .newFunctionWithName(&NSString::from_str(vertex_fn_name))
        .expect("error locating vertex function");
    let fragment_fn = library
        .newFunctionWithName(&NSString::from_str(fragment_fn_name))
        .expect("error locating fragment function");

    let descriptor = MTLRenderPipelineDescriptor::new();
    descriptor.setLabel(Some(&NSString::from_str(label)));
    descriptor.setVertexFunction(Some(&vertex_fn));
    descriptor.setFragmentFunction(Some(&fragment_fn));
    let color_attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
    color_attachment.setPixelFormat(pixel_format);
    color_attachment.setBlendingEnabled(true);
    color_attachment.setRgbBlendOperation(MTLBlendOperation::Add);
    color_attachment.setAlphaBlendOperation(MTLBlendOperation::Add);
    color_attachment.setSourceRGBBlendFactor(MTLBlendFactor::One);
    color_attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
    color_attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    color_attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::One);

    device
        .newRenderPipelineStateWithDescriptor_error(&descriptor)
        .expect("could not create render pipeline state")
}

pub(super) fn build_path_rasterization_pipeline_state(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    label: &str,
    vertex_fn_name: &str,
    fragment_fn_name: &str,
    pixel_format: MTLPixelFormat,
    path_sample_count: u32,
) -> Retained<ProtocolObject<dyn MTLRenderPipelineState>> {
    let vertex_fn = library
        .newFunctionWithName(&NSString::from_str(vertex_fn_name))
        .expect("error locating vertex function");
    let fragment_fn = library
        .newFunctionWithName(&NSString::from_str(fragment_fn_name))
        .expect("error locating fragment function");

    let descriptor = MTLRenderPipelineDescriptor::new();
    descriptor.setLabel(Some(&NSString::from_str(label)));
    descriptor.setVertexFunction(Some(&vertex_fn));
    descriptor.setFragmentFunction(Some(&fragment_fn));
    if path_sample_count > 1 {
        descriptor.setRasterSampleCount(path_sample_count as _);
        descriptor.setAlphaToCoverageEnabled(false);
    }
    let color_attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
    color_attachment.setPixelFormat(pixel_format);
    color_attachment.setBlendingEnabled(true);
    color_attachment.setRgbBlendOperation(MTLBlendOperation::Add);
    color_attachment.setAlphaBlendOperation(MTLBlendOperation::Add);
    color_attachment.setSourceRGBBlendFactor(MTLBlendFactor::One);
    color_attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
    color_attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    color_attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);

    device
        .newRenderPipelineStateWithDescriptor_error(&descriptor)
        .expect("could not create render pipeline state")
}
