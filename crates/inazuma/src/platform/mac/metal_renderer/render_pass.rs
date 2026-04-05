use super::*;

use std::cell::Cell;


impl MetalRenderer {
    pub fn draw(&mut self, scene: &Scene) {
        let layer = match &self.layer {
            Some(l) => l.clone(),
            None => {
                log::error!(
                    "draw() called on headless renderer - use render_scene_to_image() instead"
                );
                return;
            }
        };
        let drawable_size = layer.drawableSize();
        let viewport_size: Size<DevicePixels> = size(
            (drawable_size.width.ceil() as i32).into(),
            (drawable_size.height.ceil() as i32).into(),
        );
        let drawable = if let Some(drawable) = layer.nextDrawable() {
            drawable
        } else {
            log::error!(
                "failed to retrieve next drawable, drawable size: {:?}",
                viewport_size
            );
            return;
        };

        loop {
            let mut instance_buffer = self
                .instance_buffer_pool
                .lock()
                .acquire(&self.device, self.is_unified_memory);

            let command_buffer =
                self.draw_primitives(scene, &mut instance_buffer, &drawable, viewport_size);

            match command_buffer {
                Ok(command_buffer) => {
                    let instance_buffer_pool = self.instance_buffer_pool.clone();
                    let instance_buffer = Cell::new(Some(instance_buffer));
                    let block = block2::RcBlock::new(
                        move |_cmd_buf: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
                            if let Some(instance_buffer) = instance_buffer.take() {
                                instance_buffer_pool.lock().release(instance_buffer);
                            }
                        },
                    );
                    unsafe { command_buffer.addCompletedHandler(block2::RcBlock::as_ptr(&block) as *mut _) };

                    if self.presents_with_transaction {
                        command_buffer.commit();
                        command_buffer.waitUntilScheduled();
                        drawable.present();
                    } else {
                        let drawable_as_mtl: &ProtocolObject<dyn MTLDrawable> =
                            ProtocolObject::from_ref(&*drawable);
                        command_buffer.presentDrawable(drawable_as_mtl);
                        command_buffer.commit();
                    }
                    return;
                }
                Err(err) => {
                    log::error!(
                        "failed to render: {}. retrying with larger instance buffer size",
                        err
                    );
                    let mut instance_buffer_pool = self.instance_buffer_pool.lock();
                    let buffer_size = instance_buffer_pool.buffer_size;
                    if buffer_size >= 256 * 1024 * 1024 {
                        log::error!("instance buffer size grew too large: {}", buffer_size);
                        break;
                    }
                    instance_buffer_pool.reset(buffer_size * 2);
                    log::info!(
                        "increased instance buffer size to {}",
                        instance_buffer_pool.buffer_size
                    );
                }
            }
        }
    }

    /// Renders the scene to a texture and returns the pixel data as an RGBA image.
    #[cfg(any(test, feature = "test-support"))]
    pub fn render_to_image(&mut self, scene: &Scene) -> Result<RgbaImage> {
        let layer = self
            .layer
            .clone()
            .ok_or_else(|| anyhow::anyhow!("render_to_image requires a layer-backed renderer"))?;
        let drawable_size = layer.drawableSize();
        let viewport_size: Size<DevicePixels> = size(
            (drawable_size.width.ceil() as i32).into(),
            (drawable_size.height.ceil() as i32).into(),
        );
        let drawable = layer
            .nextDrawable()
            .ok_or_else(|| anyhow::anyhow!("Failed to get drawable for render_to_image"))?;

        loop {
            let mut instance_buffer = self
                .instance_buffer_pool
                .lock()
                .acquire(&self.device, self.is_unified_memory);

            let command_buffer =
                self.draw_primitives(scene, &mut instance_buffer, &drawable, viewport_size);

            match command_buffer {
                Ok(command_buffer) => {
                    let instance_buffer_pool = self.instance_buffer_pool.clone();
                    let instance_buffer = Cell::new(Some(instance_buffer));
                    let block = block2::RcBlock::new(
                        move |_cmd_buf: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
                            if let Some(instance_buffer) = instance_buffer.take() {
                                instance_buffer_pool.lock().release(instance_buffer);
                            }
                        },
                    );
                    unsafe { command_buffer.addCompletedHandler(block2::RcBlock::as_ptr(&block) as *mut _) };

                    command_buffer.commit();
                    command_buffer.waitUntilCompleted();

                    let texture = drawable.texture();
                    let width = texture.width() as u32;
                    let height = texture.height() as u32;
                    let bytes_per_row = width as usize * 4;
                    let buffer_size = height as usize * bytes_per_row;

                    let mut pixels = vec![0u8; buffer_size];

                    let region = MTLRegion {
                        origin: MTLOrigin { x: 0, y: 0, z: 0 },
                        size: MTLSize {
                            width: width as usize,
                            height: height as usize,
                            depth: 1,
                        },
                    };

                    unsafe {
                        texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(
                            NonNull::new_unchecked(pixels.as_mut_ptr() as *mut c_void),
                            bytes_per_row,
                            region,
                            0,
                        );
                    }

                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                    }

                    return RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
                        anyhow::anyhow!("Failed to create RgbaImage from pixel data")
                    });
                }
                Err(err) => {
                    log::error!(
                        "failed to render: {}. retrying with larger instance buffer size",
                        err
                    );
                    let mut instance_buffer_pool = self.instance_buffer_pool.lock();
                    let buffer_size = instance_buffer_pool.buffer_size;
                    if buffer_size >= 256 * 1024 * 1024 {
                        anyhow::bail!("instance buffer size grew too large: {}", buffer_size);
                    }
                    instance_buffer_pool.reset(buffer_size * 2);
                    log::info!(
                        "increased instance buffer size to {}",
                        instance_buffer_pool.buffer_size
                    );
                }
            }
        }
    }

    /// Renders a scene to an image without requiring a window or CAMetalLayer.
    #[cfg(any(test, feature = "test-support"))]
    pub fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> Result<RgbaImage> {
        if size.width.0 <= 0 || size.height.0 <= 0 {
            anyhow::bail!("Invalid size for render_scene_to_image: {:?}", size);
        }

        self.update_path_intermediate_textures(size);

        let target_texture = unsafe {
            let texture_descriptor = MTLTextureDescriptor::new();
            texture_descriptor.setWidth(size.width.0 as usize);
            texture_descriptor.setHeight(size.height.0 as usize);
            texture_descriptor.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
            texture_descriptor
                .setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
            texture_descriptor.setStorageMode(MTLStorageMode::Managed);
            self.device
                .newTextureWithDescriptor(&texture_descriptor)
                .expect("failed to create target texture for render_scene_to_image")
        };

        loop {
            let mut instance_buffer = self
                .instance_buffer_pool
                .lock()
                .acquire(&self.device, self.is_unified_memory);

            let command_buffer = self.draw_primitives_to_texture(
                scene,
                &mut instance_buffer,
                &target_texture,
                size,
            );

            match command_buffer {
                Ok(command_buffer) => {
                    let instance_buffer_pool = self.instance_buffer_pool.clone();
                    let instance_buffer = Cell::new(Some(instance_buffer));
                    let block = block2::RcBlock::new(
                        move |_cmd_buf: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
                            if let Some(instance_buffer) = instance_buffer.take() {
                                instance_buffer_pool.lock().release(instance_buffer);
                            }
                        },
                    );
                    unsafe { command_buffer.addCompletedHandler(block2::RcBlock::as_ptr(&block) as *mut _) };

                    if !self.is_unified_memory {
                        let blit = command_buffer
                            .blitCommandEncoder()
                            .expect("failed to create blit command encoder");
                        blit.synchronizeResource(ProtocolObject::from_ref(&*target_texture));
                        blit.endEncoding();
                    }

                    command_buffer.commit();
                    command_buffer.waitUntilCompleted();

                    let width = size.width.0 as u32;
                    let height = size.height.0 as u32;
                    let bytes_per_row = width as usize * 4;
                    let buffer_size = height as usize * bytes_per_row;

                    let mut pixels = vec![0u8; buffer_size];

                    let region = MTLRegion {
                        origin: MTLOrigin { x: 0, y: 0, z: 0 },
                        size: MTLSize {
                            width: width as usize,
                            height: height as usize,
                            depth: 1,
                        },
                    };

                    unsafe {
                        target_texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(
                            NonNull::new_unchecked(pixels.as_mut_ptr() as *mut c_void),
                            bytes_per_row,
                            region,
                            0,
                        );
                    }

                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                    }

                    return RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
                        anyhow::anyhow!("Failed to create RgbaImage from pixel data")
                    });
                }
                Err(err) => {
                    log::error!(
                        "failed to render: {}. retrying with larger instance buffer size",
                        err
                    );
                    let mut instance_buffer_pool = self.instance_buffer_pool.lock();
                    let buffer_size = instance_buffer_pool.buffer_size;
                    if buffer_size >= 256 * 1024 * 1024 {
                        anyhow::bail!("instance buffer size grew too large: {}", buffer_size);
                    }
                    instance_buffer_pool.reset(buffer_size * 2);
                    log::info!(
                        "increased instance buffer size to {}",
                        instance_buffer_pool.buffer_size
                    );
                }
            }
        }
    }

    fn draw_primitives(
        &mut self,
        scene: &Scene,
        instance_buffer: &mut InstanceBuffer,
        drawable: &ProtocolObject<dyn CAMetalDrawable>,
        viewport_size: Size<DevicePixels>,
    ) -> Result<Retained<ProtocolObject<dyn MTLCommandBuffer>>> {
        let texture = drawable.texture();
        self.draw_primitives_to_texture(scene, instance_buffer, &texture, viewport_size)
    }

    fn draw_primitives_to_texture(
        &mut self,
        scene: &Scene,
        instance_buffer: &mut InstanceBuffer,
        texture: &ProtocolObject<dyn MTLTexture>,
        viewport_size: Size<DevicePixels>,
    ) -> Result<Retained<ProtocolObject<dyn MTLCommandBuffer>>> {
        let command_buffer = self
            .command_queue
            .commandBuffer()
            .expect("failed to create command buffer");
        let alpha = if self.opaque { 1. } else { 0. };
        let mut instance_offset = 0;

        let mut command_encoder = new_command_encoder_for_texture(
            &command_buffer,
            texture,
            viewport_size,
            |color_attachment| {
                color_attachment.setLoadAction(MTLLoadAction::Clear);
                color_attachment.setClearColor(MTLClearColor {
                    red: 0.,
                    green: 0.,
                    blue: 0.,
                    alpha,
                });
            },
        );

        for batch in scene.batches() {
            let ok = match batch {
                PrimitiveBatch::Shadows(range) => self.draw_shadows(
                    &scene.shadows[range],
                    instance_buffer,
                    &mut instance_offset,
                    viewport_size,
                    &command_encoder,
                ),
                PrimitiveBatch::Quads(range) => self.draw_quads(
                    &scene.quads[range],
                    instance_buffer,
                    &mut instance_offset,
                    viewport_size,
                    &command_encoder,
                ),
                PrimitiveBatch::Paths(range) => {
                    let paths = &scene.paths[range];
                    command_encoder.endEncoding();

                    let did_draw = self.draw_paths_to_intermediate(
                        paths,
                        instance_buffer,
                        &mut instance_offset,
                        viewport_size,
                        &command_buffer,
                    );

                    command_encoder = new_command_encoder_for_texture(
                        &command_buffer,
                        texture,
                        viewport_size,
                        |color_attachment| {
                            color_attachment.setLoadAction(MTLLoadAction::Load);
                        },
                    );

                    if did_draw {
                        self.draw_paths_from_intermediate(
                            paths,
                            instance_buffer,
                            &mut instance_offset,
                            viewport_size,
                            &command_encoder,
                        )
                    } else {
                        false
                    }
                }
                PrimitiveBatch::Underlines(range) => self.draw_underlines(
                    &scene.underlines[range],
                    instance_buffer,
                    &mut instance_offset,
                    viewport_size,
                    &command_encoder,
                ),
                PrimitiveBatch::MonochromeSprites { texture_id, range } => self
                    .draw_monochrome_sprites(
                        texture_id,
                        &scene.monochrome_sprites[range],
                        instance_buffer,
                        &mut instance_offset,
                        viewport_size,
                        &command_encoder,
                    ),
                PrimitiveBatch::PolychromeSprites { texture_id, range } => self
                    .draw_polychrome_sprites(
                        texture_id,
                        &scene.polychrome_sprites[range],
                        instance_buffer,
                        &mut instance_offset,
                        viewport_size,
                        &command_encoder,
                    ),
                PrimitiveBatch::Surfaces(range) => self.draw_surfaces(
                    &scene.surfaces[range],
                    instance_buffer,
                    &mut instance_offset,
                    viewport_size,
                    &command_encoder,
                ),
                PrimitiveBatch::SubpixelSprites { .. } => unreachable!(),
            };
            if !ok {
                command_encoder.endEncoding();
                anyhow::bail!(
                    "scene too large: {} paths, {} shadows, {} quads, {} underlines, {} mono, {} poly, {} surfaces",
                    scene.paths.len(),
                    scene.shadows.len(),
                    scene.quads.len(),
                    scene.underlines.len(),
                    scene.monochrome_sprites.len(),
                    scene.polychrome_sprites.len(),
                    scene.surfaces.len(),
                );
            }
        }

        command_encoder.endEncoding();

        if !self.is_unified_memory {
            let metal_buffer = &*instance_buffer.metal_buffer;
            metal_buffer.didModifyRange(NSRange::new(0, instance_offset));
        }

        Ok(command_buffer)
    }

    fn draw_paths_to_intermediate(
        &self,
        paths: &[Path<ScaledPixels>],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_buffer: &ProtocolObject<dyn MTLCommandBuffer>,
    ) -> bool {
        if paths.is_empty() {
            return true;
        }
        let Some(intermediate_texture) = &self.path_intermediate_texture else {
            return false;
        };

        unsafe {
            let render_pass_descriptor = MTLRenderPassDescriptor::new();
            let color_attachment = render_pass_descriptor
                .colorAttachments()
                .objectAtIndexedSubscript(0);
            color_attachment.setLoadAction(MTLLoadAction::Clear);
            color_attachment.setClearColor(MTLClearColor {
                red: 0.,
                green: 0.,
                blue: 0.,
                alpha: 0.,
            });

            if let Some(msaa_texture) = &self.path_intermediate_msaa_texture {
                color_attachment.setTexture(Some(msaa_texture));
                color_attachment.setResolveTexture(Some(intermediate_texture));
                color_attachment.setStoreAction(MTLStoreAction::MultisampleResolve);
            } else {
                color_attachment.setTexture(Some(intermediate_texture));
                color_attachment.setStoreAction(MTLStoreAction::Store);
            }

            let command_encoder = command_buffer
                .renderCommandEncoderWithDescriptor(&render_pass_descriptor)
                .expect("failed to create render command encoder for path rasterization");
            command_encoder.setRenderPipelineState(&self.paths_rasterization_pipeline_state);

            align_offset(instance_offset);
            let mut vertices = Vec::new();
            for path in paths {
                vertices.extend(path.vertices.iter().map(|v| PathRasterizationVertex {
                    xy_position: v.xy_position,
                    st_position: v.st_position,
                    color: path.color,
                    bounds: path.bounds.intersect(&path.content_mask.bounds),
                }));
            }
            let vertices_bytes_len = mem::size_of_val(vertices.as_slice());
            let next_offset = *instance_offset + vertices_bytes_len;
            if next_offset > instance_buffer.size {
                command_encoder.endEncoding();
                return false;
            }

            let metal_buffer = &*instance_buffer.metal_buffer;

            command_encoder.setVertexBuffer_offset_atIndex(
                Some(metal_buffer),
                *instance_offset,
                PathRasterizationInputIndex::Vertices as usize,
            );
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(
                    &viewport_size as *const Size<DevicePixels> as *mut c_void,
                ),
                mem::size_of_val(&viewport_size),
                PathRasterizationInputIndex::ViewportSize as usize,
            );
            command_encoder.setFragmentBuffer_offset_atIndex(
                Some(metal_buffer),
                *instance_offset,
                PathRasterizationInputIndex::Vertices as usize,
            );

            let buffer_contents =
                (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8).add(*instance_offset);
            ptr::copy_nonoverlapping(
                vertices.as_ptr() as *const u8,
                buffer_contents,
                vertices_bytes_len,
            );
            command_encoder.drawPrimitives_vertexStart_vertexCount(
                MTLPrimitiveType::Triangle,
                0,
                vertices.len(),
            );
            *instance_offset = next_offset;

            command_encoder.endEncoding();
            true
        }
    }

    fn draw_shadows(
        &self,
        shadows: &[Shadow],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> bool {
        if shadows.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        unsafe {
            let metal_buffer = &*instance_buffer.metal_buffer;

            command_encoder.setRenderPipelineState(&self.shadows_pipeline_state);
            command_encoder.setVertexBuffer_offset_atIndex(Some(&*self.unit_vertices), 0, ShadowInputIndex::Vertices as usize);
            command_encoder.setVertexBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, ShadowInputIndex::Shadows as usize);
            command_encoder.setFragmentBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, ShadowInputIndex::Shadows as usize);
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&viewport_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&viewport_size),
                ShadowInputIndex::ViewportSize as usize,
            );

            let shadow_bytes_len = mem::size_of_val(shadows);
            let buffer_contents = (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8).add(*instance_offset);
            let next_offset = *instance_offset + shadow_bytes_len;
            if next_offset > instance_buffer.size {
                return false;
            }
            ptr::copy_nonoverlapping(shadows.as_ptr() as *const u8, buffer_contents, shadow_bytes_len);
            command_encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(MTLPrimitiveType::Triangle, 0, 6, shadows.len());
            *instance_offset = next_offset;
            true
        }
    }

    fn draw_quads(
        &self,
        quads: &[Quad],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> bool {
        if quads.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        unsafe {
            let metal_buffer = &*instance_buffer.metal_buffer;
            let unit_vertices = &*self.unit_vertices;

            command_encoder.setRenderPipelineState(&self.quads_pipeline_state);
            command_encoder.setVertexBuffer_offset_atIndex(Some(unit_vertices), 0, QuadInputIndex::Vertices as usize);
            command_encoder.setVertexBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, QuadInputIndex::Quads as usize);
            command_encoder.setFragmentBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, QuadInputIndex::Quads as usize);
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&viewport_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&viewport_size),
                QuadInputIndex::ViewportSize as usize,
            );

            let quad_bytes_len = mem::size_of_val(quads);
            let buffer_contents = (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8).add(*instance_offset);
            let next_offset = *instance_offset + quad_bytes_len;
            if next_offset > instance_buffer.size {
                return false;
            }
            ptr::copy_nonoverlapping(quads.as_ptr() as *const u8, buffer_contents, quad_bytes_len);
            command_encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(MTLPrimitiveType::Triangle, 0, 6, quads.len());
            *instance_offset = next_offset;
            true
        }
    }

    fn draw_paths_from_intermediate(
        &self,
        paths: &[Path<ScaledPixels>],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> bool {
        let Some(first_path) = paths.first() else {
            return true;
        };
        let Some(ref intermediate_texture) = self.path_intermediate_texture else {
            return false;
        };

        unsafe {
            let metal_buffer = &*instance_buffer.metal_buffer;
            let unit_vertices = &*self.unit_vertices;

            command_encoder.setRenderPipelineState(&self.path_sprites_pipeline_state);
            command_encoder.setVertexBuffer_offset_atIndex(Some(unit_vertices), 0, SpriteInputIndex::Vertices as usize);
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&viewport_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&viewport_size),
                SpriteInputIndex::ViewportSize as usize,
            );
            command_encoder.setFragmentTexture_atIndex(Some(intermediate_texture), SpriteInputIndex::AtlasTexture as usize);

            let sprites;
            if paths.last().unwrap().order == first_path.order {
                sprites = paths.iter().map(|path| PathSprite { bounds: path.clipped_bounds() }).collect();
            } else {
                let mut bounds = first_path.clipped_bounds();
                for path in paths.iter().skip(1) {
                    bounds = bounds.union(&path.clipped_bounds());
                }
                sprites = vec![PathSprite { bounds }];
            }

            align_offset(instance_offset);
            let sprite_bytes_len = mem::size_of_val(sprites.as_slice());
            let next_offset = *instance_offset + sprite_bytes_len;
            if next_offset > instance_buffer.size {
                return false;
            }

            command_encoder.setVertexBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, SpriteInputIndex::Sprites as usize);
            let buffer_contents = (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8).add(*instance_offset);
            ptr::copy_nonoverlapping(sprites.as_ptr() as *const u8, buffer_contents, sprite_bytes_len);
            command_encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(MTLPrimitiveType::Triangle, 0, 6, sprites.len());
            *instance_offset = next_offset;
            true
        }
    }

    fn draw_underlines(
        &self,
        underlines: &[Underline],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> bool {
        if underlines.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        unsafe {
            let metal_buffer = &*instance_buffer.metal_buffer;
            let unit_vertices = &*self.unit_vertices;

            command_encoder.setRenderPipelineState(&self.underlines_pipeline_state);
            command_encoder.setVertexBuffer_offset_atIndex(Some(unit_vertices), 0, UnderlineInputIndex::Vertices as usize);
            command_encoder.setVertexBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, UnderlineInputIndex::Underlines as usize);
            command_encoder.setFragmentBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, UnderlineInputIndex::Underlines as usize);
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&viewport_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&viewport_size),
                UnderlineInputIndex::ViewportSize as usize,
            );

            let underline_bytes_len = mem::size_of_val(underlines);
            let buffer_contents = (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8).add(*instance_offset);
            let next_offset = *instance_offset + underline_bytes_len;
            if next_offset > instance_buffer.size {
                return false;
            }
            ptr::copy_nonoverlapping(underlines.as_ptr() as *const u8, buffer_contents, underline_bytes_len);
            command_encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(MTLPrimitiveType::Triangle, 0, 6, underlines.len());
            *instance_offset = next_offset;
            true
        }
    }

    fn draw_monochrome_sprites(
        &self,
        texture_id: AtlasTextureId,
        sprites: &[MonochromeSprite],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> bool {
        if sprites.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        unsafe {
            let sprite_bytes_len = mem::size_of_val(sprites);
            let buffer_contents = (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8).add(*instance_offset);
            let next_offset = *instance_offset + sprite_bytes_len;
            if next_offset > instance_buffer.size {
                return false;
            }

            let metal_buffer = &*instance_buffer.metal_buffer;
            let unit_vertices = &*self.unit_vertices;

            let texture = self.sprite_atlas.metal_texture(texture_id);
            let texture_ref = &*texture;
            let texture_size = size(DevicePixels(texture_ref.width() as i32), DevicePixels(texture_ref.height() as i32));
            command_encoder.setRenderPipelineState(&self.monochrome_sprites_pipeline_state);
            command_encoder.setVertexBuffer_offset_atIndex(Some(unit_vertices), 0, SpriteInputIndex::Vertices as usize);
            command_encoder.setVertexBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, SpriteInputIndex::Sprites as usize);
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&viewport_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&viewport_size),
                SpriteInputIndex::ViewportSize as usize,
            );
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&texture_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&texture_size),
                SpriteInputIndex::AtlasTextureSize as usize,
            );
            command_encoder.setFragmentBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, SpriteInputIndex::Sprites as usize);
            command_encoder.setFragmentTexture_atIndex(Some(texture_ref), SpriteInputIndex::AtlasTexture as usize);

            ptr::copy_nonoverlapping(sprites.as_ptr() as *const u8, buffer_contents, sprite_bytes_len);
            command_encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(MTLPrimitiveType::Triangle, 0, 6, sprites.len());
            *instance_offset = next_offset;
            true
        }
    }

    fn draw_polychrome_sprites(
        &self,
        texture_id: AtlasTextureId,
        sprites: &[PolychromeSprite],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> bool {
        if sprites.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        unsafe {
            let metal_buffer = &*instance_buffer.metal_buffer;
            let unit_vertices = &*self.unit_vertices;

            let texture = self.sprite_atlas.metal_texture(texture_id);
            let texture_ref = &*texture;
            let texture_size = size(DevicePixels(texture_ref.width() as i32), DevicePixels(texture_ref.height() as i32));
            command_encoder.setRenderPipelineState(&self.polychrome_sprites_pipeline_state);
            command_encoder.setVertexBuffer_offset_atIndex(Some(unit_vertices), 0, SpriteInputIndex::Vertices as usize);
            command_encoder.setVertexBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, SpriteInputIndex::Sprites as usize);
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&viewport_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&viewport_size),
                SpriteInputIndex::ViewportSize as usize,
            );
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&texture_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&texture_size),
                SpriteInputIndex::AtlasTextureSize as usize,
            );
            command_encoder.setFragmentBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, SpriteInputIndex::Sprites as usize);
            command_encoder.setFragmentTexture_atIndex(Some(texture_ref), SpriteInputIndex::AtlasTexture as usize);

            let sprite_bytes_len = mem::size_of_val(sprites);
            let buffer_contents = (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8).add(*instance_offset);
            let next_offset = *instance_offset + sprite_bytes_len;
            if next_offset > instance_buffer.size {
                return false;
            }
            ptr::copy_nonoverlapping(sprites.as_ptr() as *const u8, buffer_contents, sprite_bytes_len);
            command_encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(MTLPrimitiveType::Triangle, 0, 6, sprites.len());
            *instance_offset = next_offset;
            true
        }
    }

    fn draw_surfaces(
        &mut self,
        surfaces: &[PaintSurface],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> bool {
        unsafe {
            let metal_buffer = &*instance_buffer.metal_buffer;
            let unit_vertices = &*self.unit_vertices;

            command_encoder.setRenderPipelineState(&self.surfaces_pipeline_state);
            command_encoder.setVertexBuffer_offset_atIndex(Some(unit_vertices), 0, SurfaceInputIndex::Vertices as usize);
            command_encoder.setVertexBytes_length_atIndex(
                NonNull::new_unchecked(&viewport_size as *const Size<DevicePixels> as *mut c_void),
                mem::size_of_val(&viewport_size),
                SurfaceInputIndex::ViewportSize as usize,
            );

            for surface in surfaces {
                let img = &surface.image_buffer;
                let texture_size = size(
                    DevicePixels::from(CVPixelBufferGetWidth(img) as i32),
                    DevicePixels::from(CVPixelBufferGetHeight(img) as i32),
                );

                assert_eq!(
                    CVPixelBufferGetPixelFormatType(img),
                    kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
                );

                let cache = &*self.core_video_texture_cache;
                let mut y_texture: *mut objc2_core_video::CVMetalTexture = ptr::null_mut();
                let status = CVMetalTextureCache::create_texture_from_image(
                    None, cache, img, None,
                    MTLPixelFormat::R8Unorm,
                    CVPixelBufferGetWidthOfPlane(img, 0),
                    CVPixelBufferGetHeightOfPlane(img, 0),
                    0,
                    NonNull::new_unchecked(&mut y_texture),
                );
                assert_eq!(status, kCVReturnSuccess);

                let mut cbcr_texture: *mut objc2_core_video::CVMetalTexture = ptr::null_mut();
                let status = CVMetalTextureCache::create_texture_from_image(
                    None, cache, img, None,
                    MTLPixelFormat::RG8Unorm,
                    CVPixelBufferGetWidthOfPlane(img, 1),
                    CVPixelBufferGetHeightOfPlane(img, 1),
                    1,
                    NonNull::new_unchecked(&mut cbcr_texture),
                );
                assert_eq!(status, kCVReturnSuccess);

                align_offset(instance_offset);
                let next_offset = *instance_offset + mem::size_of::<Surface>();
                if next_offset > instance_buffer.size {
                    return false;
                }

                command_encoder.setVertexBuffer_offset_atIndex(Some(metal_buffer), *instance_offset, SurfaceInputIndex::Surfaces as usize);
                command_encoder.setVertexBytes_length_atIndex(
                    NonNull::new_unchecked(&texture_size as *const Size<DevicePixels> as *mut c_void),
                    mem::size_of_val(&texture_size),
                    SurfaceInputIndex::TextureSize as usize,
                );

                let y_tex = CVMetalTextureGetTexture(&*y_texture).expect("CVMetalTexture has no texture");
                command_encoder.setFragmentTexture_atIndex(Some(&y_tex), SurfaceInputIndex::YTexture as usize);

                let cbcr_tex = CVMetalTextureGetTexture(&*cbcr_texture).expect("CVMetalTexture has no texture");
                command_encoder.setFragmentTexture_atIndex(Some(&cbcr_tex), SurfaceInputIndex::CbCrTexture as usize);

                let buffer_contents = (instance_buffer.metal_buffer.contents().as_ptr() as *mut u8)
                    .add(*instance_offset)
                    as *mut SurfaceBounds;
                ptr::write(
                    buffer_contents,
                    SurfaceBounds {
                        bounds: surface.bounds,
                        content_mask: surface.content_mask.clone(),
                    },
                );

                command_encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 6);
                *instance_offset = next_offset;
            }
            true
        }
    }
}

pub(super) fn new_command_encoder_for_texture(
    command_buffer: &ProtocolObject<dyn MTLCommandBuffer>,
    texture: &ProtocolObject<dyn MTLTexture>,
    viewport_size: Size<DevicePixels>,
    configure_color_attachment: impl Fn(&MTLRenderPassColorAttachmentDescriptor),
) -> Retained<ProtocolObject<dyn MTLRenderCommandEncoder>> {
    unsafe {
        let render_pass_descriptor = MTLRenderPassDescriptor::new();
        let color_attachment = render_pass_descriptor
            .colorAttachments()
            .objectAtIndexedSubscript(0);
        color_attachment.setTexture(Some(texture));
        color_attachment.setStoreAction(MTLStoreAction::Store);
        configure_color_attachment(&color_attachment);

        let command_encoder = command_buffer
            .renderCommandEncoderWithDescriptor(&render_pass_descriptor)
            .expect("failed to create render command encoder");
        command_encoder.setViewport(MTLViewport {
            originX: 0.0,
            originY: 0.0,
            width: i32::from(viewport_size.width) as f64,
            height: i32::from(viewport_size.height) as f64,
            znear: 0.0,
            zfar: 1.0,
        });
        command_encoder
    }
}

#[cfg(any(test, feature = "test-support"))]
pub struct MetalHeadlessRenderer {
    renderer: MetalRenderer,
}

#[cfg(any(test, feature = "test-support"))]
impl MetalHeadlessRenderer {
    pub fn new() -> Self {
        let instance_buffer_pool = Arc::new(Mutex::new(InstanceBufferPool::default()));
        let renderer = MetalRenderer::new_headless(instance_buffer_pool);
        Self { renderer }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl inazuma::PlatformHeadlessRenderer for MetalHeadlessRenderer {
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> anyhow::Result<image::RgbaImage> {
        self.renderer.render_scene_to_image(scene, size)
    }

    fn sprite_atlas(&self) -> Arc<dyn inazuma::PlatformAtlas> {
        self.renderer.sprite_atlas().clone()
    }
}
