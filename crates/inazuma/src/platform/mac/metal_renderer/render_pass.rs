use super::*;

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
        let viewport_size = layer.drawable_size();
        let viewport_size: Size<DevicePixels> = size(
            (viewport_size.width.ceil() as i32).into(),
            (viewport_size.height.ceil() as i32).into(),
        );
        let drawable = if let Some(drawable) = layer.next_drawable() {
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
                self.draw_primitives(scene, &mut instance_buffer, drawable, viewport_size);

            match command_buffer {
                Ok(command_buffer) => {
                    let instance_buffer_pool = self.instance_buffer_pool.clone();
                    let instance_buffer = Cell::new(Some(instance_buffer));
                    let block = ConcreteBlock::new(move |_| {
                        if let Some(instance_buffer) = instance_buffer.take() {
                            instance_buffer_pool.lock().release(instance_buffer);
                        }
                    });
                    let block = block.copy();
                    command_buffer.add_completed_handler(&block);

                    if self.presents_with_transaction {
                        command_buffer.commit();
                        command_buffer.wait_until_scheduled();
                        drawable.present();
                    } else {
                        command_buffer.present_drawable(drawable);
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
        let viewport_size = layer.drawable_size();
        let viewport_size: Size<DevicePixels> = size(
            (viewport_size.width.ceil() as i32).into(),
            (viewport_size.height.ceil() as i32).into(),
        );
        let drawable = layer
            .next_drawable()
            .ok_or_else(|| anyhow::anyhow!("Failed to get drawable for render_to_image"))?;

        loop {
            let mut instance_buffer = self
                .instance_buffer_pool
                .lock()
                .acquire(&self.device, self.is_unified_memory);

            let command_buffer =
                self.draw_primitives(scene, &mut instance_buffer, drawable, viewport_size);

            match command_buffer {
                Ok(command_buffer) => {
                    let instance_buffer_pool = self.instance_buffer_pool.clone();
                    let instance_buffer = Cell::new(Some(instance_buffer));
                    let block = ConcreteBlock::new(move |_| {
                        if let Some(instance_buffer) = instance_buffer.take() {
                            instance_buffer_pool.lock().release(instance_buffer);
                        }
                    });
                    let block = block.copy();
                    command_buffer.add_completed_handler(&block);

                    command_buffer.commit();
                    command_buffer.wait_until_completed();

                    let texture = drawable.texture();
                    let width = texture.width() as u32;
                    let height = texture.height() as u32;
                    let bytes_per_row = width as usize * 4;
                    let buffer_size = height as usize * bytes_per_row;

                    let mut pixels = vec![0u8; buffer_size];

                    let region = metal::MTLRegion {
                        origin: metal::MTLOrigin { x: 0, y: 0, z: 0 },
                        size: metal::MTLSize {
                            width: width as u64,
                            height: height as u64,
                            depth: 1,
                        },
                    };

                    texture.get_bytes(
                        pixels.as_mut_ptr() as *mut std::ffi::c_void,
                        bytes_per_row as u64,
                        region,
                        0,
                    );

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

        let texture_descriptor = metal::TextureDescriptor::new();
        texture_descriptor.set_width(size.width.0 as u64);
        texture_descriptor.set_height(size.height.0 as u64);
        texture_descriptor.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        texture_descriptor
            .set_usage(metal::MTLTextureUsage::RenderTarget | metal::MTLTextureUsage::ShaderRead);
        texture_descriptor.set_storage_mode(metal::MTLStorageMode::Managed);
        let target_texture = self.device.new_texture(&texture_descriptor);

        loop {
            let mut instance_buffer = self
                .instance_buffer_pool
                .lock()
                .acquire(&self.device, self.is_unified_memory);

            let command_buffer =
                self.draw_primitives_to_texture(scene, &mut instance_buffer, &target_texture, size);

            match command_buffer {
                Ok(command_buffer) => {
                    let instance_buffer_pool = self.instance_buffer_pool.clone();
                    let instance_buffer = Cell::new(Some(instance_buffer));
                    let block = ConcreteBlock::new(move |_| {
                        if let Some(instance_buffer) = instance_buffer.take() {
                            instance_buffer_pool.lock().release(instance_buffer);
                        }
                    });
                    let block = block.copy();
                    command_buffer.add_completed_handler(&block);

                    if !self.is_unified_memory {
                        let blit = command_buffer.new_blit_command_encoder();
                        blit.synchronize_resource(&target_texture);
                        blit.end_encoding();
                    }

                    command_buffer.commit();
                    command_buffer.wait_until_completed();

                    let width = size.width.0 as u32;
                    let height = size.height.0 as u32;
                    let bytes_per_row = width as usize * 4;
                    let buffer_size = height as usize * bytes_per_row;

                    let mut pixels = vec![0u8; buffer_size];

                    let region = metal::MTLRegion {
                        origin: metal::MTLOrigin { x: 0, y: 0, z: 0 },
                        size: metal::MTLSize {
                            width: width as u64,
                            height: height as u64,
                            depth: 1,
                        },
                    };

                    target_texture.get_bytes(
                        pixels.as_mut_ptr() as *mut std::ffi::c_void,
                        bytes_per_row as u64,
                        region,
                        0,
                    );

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
        drawable: &metal::MetalDrawableRef,
        viewport_size: Size<DevicePixels>,
    ) -> Result<metal::CommandBuffer> {
        self.draw_primitives_to_texture(scene, instance_buffer, drawable.texture(), viewport_size)
    }

    fn draw_primitives_to_texture(
        &mut self,
        scene: &Scene,
        instance_buffer: &mut InstanceBuffer,
        texture: &metal::TextureRef,
        viewport_size: Size<DevicePixels>,
    ) -> Result<metal::CommandBuffer> {
        let command_queue = self.command_queue.clone();
        let command_buffer = command_queue.new_command_buffer();
        let alpha = if self.opaque { 1. } else { 0. };
        let mut instance_offset = 0;

        let mut command_encoder = new_command_encoder_for_texture(
            command_buffer,
            texture,
            viewport_size,
            |color_attachment| {
                color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                color_attachment.set_clear_color(metal::MTLClearColor::new(0., 0., 0., alpha));
            },
        );

        for batch in scene.batches() {
            let ok = match batch {
                PrimitiveBatch::Shadows(range) => self.draw_shadows(
                    &scene.shadows[range],
                    instance_buffer,
                    &mut instance_offset,
                    viewport_size,
                    command_encoder,
                ),
                PrimitiveBatch::Quads(range) => self.draw_quads(
                    &scene.quads[range],
                    instance_buffer,
                    &mut instance_offset,
                    viewport_size,
                    command_encoder,
                ),
                PrimitiveBatch::Paths(range) => {
                    let paths = &scene.paths[range];
                    command_encoder.end_encoding();

                    let did_draw = self.draw_paths_to_intermediate(
                        paths,
                        instance_buffer,
                        &mut instance_offset,
                        viewport_size,
                        command_buffer,
                    );

                    command_encoder = new_command_encoder_for_texture(
                        command_buffer,
                        texture,
                        viewport_size,
                        |color_attachment| {
                            color_attachment.set_load_action(metal::MTLLoadAction::Load);
                        },
                    );

                    if did_draw {
                        self.draw_paths_from_intermediate(
                            paths,
                            instance_buffer,
                            &mut instance_offset,
                            viewport_size,
                            command_encoder,
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
                    command_encoder,
                ),
                PrimitiveBatch::MonochromeSprites { texture_id, range } => self
                    .draw_monochrome_sprites(
                        texture_id,
                        &scene.monochrome_sprites[range],
                        instance_buffer,
                        &mut instance_offset,
                        viewport_size,
                        command_encoder,
                    ),
                PrimitiveBatch::PolychromeSprites { texture_id, range } => self
                    .draw_polychrome_sprites(
                        texture_id,
                        &scene.polychrome_sprites[range],
                        instance_buffer,
                        &mut instance_offset,
                        viewport_size,
                        command_encoder,
                    ),
                PrimitiveBatch::Surfaces(range) => self.draw_surfaces(
                    &scene.surfaces[range],
                    instance_buffer,
                    &mut instance_offset,
                    viewport_size,
                    command_encoder,
                ),
                PrimitiveBatch::SubpixelSprites { .. } => unreachable!(),
            };
            if !ok {
                command_encoder.end_encoding();
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

        command_encoder.end_encoding();

        if !self.is_unified_memory {
            instance_buffer.metal_buffer.did_modify_range(NSRange {
                location: 0,
                length: instance_offset as NSUInteger,
            });
        }

        Ok(command_buffer.to_owned())
    }

    fn draw_paths_to_intermediate(
        &self,
        paths: &[Path<ScaledPixels>],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_buffer: &metal::CommandBufferRef,
    ) -> bool {
        if paths.is_empty() {
            return true;
        }
        let Some(intermediate_texture) = &self.path_intermediate_texture else {
            return false;
        };

        let render_pass_descriptor = metal::RenderPassDescriptor::new();
        let color_attachment = render_pass_descriptor
            .color_attachments()
            .object_at(0)
            .unwrap();
        color_attachment.set_load_action(metal::MTLLoadAction::Clear);
        color_attachment.set_clear_color(metal::MTLClearColor::new(0., 0., 0., 0.));

        if let Some(msaa_texture) = &self.path_intermediate_msaa_texture {
            color_attachment.set_texture(Some(msaa_texture));
            color_attachment.set_resolve_texture(Some(intermediate_texture));
            color_attachment.set_store_action(metal::MTLStoreAction::MultisampleResolve);
        } else {
            color_attachment.set_texture(Some(intermediate_texture));
            color_attachment.set_store_action(metal::MTLStoreAction::Store);
        }

        let command_encoder = command_buffer.new_render_command_encoder(render_pass_descriptor);
        command_encoder.set_render_pipeline_state(&self.paths_rasterization_pipeline_state);

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
            command_encoder.end_encoding();
            return false;
        }
        command_encoder.set_vertex_buffer(
            PathRasterizationInputIndex::Vertices as u64,
            Some(&instance_buffer.metal_buffer),
            *instance_offset as u64,
        );
        command_encoder.set_vertex_bytes(
            PathRasterizationInputIndex::ViewportSize as u64,
            mem::size_of_val(&viewport_size) as u64,
            &viewport_size as *const Size<DevicePixels> as *const _,
        );
        command_encoder.set_fragment_buffer(
            PathRasterizationInputIndex::Vertices as u64,
            Some(&instance_buffer.metal_buffer),
            *instance_offset as u64,
        );
        let buffer_contents =
            unsafe { (instance_buffer.metal_buffer.contents() as *mut u8).add(*instance_offset) };
        unsafe {
            ptr::copy_nonoverlapping(
                vertices.as_ptr() as *const u8,
                buffer_contents,
                vertices_bytes_len,
            );
        }
        command_encoder.draw_primitives(
            metal::MTLPrimitiveType::Triangle,
            0,
            vertices.len() as u64,
        );
        *instance_offset = next_offset;

        command_encoder.end_encoding();
        true
    }

    fn draw_shadows(
        &self,
        shadows: &[Shadow],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &metal::RenderCommandEncoderRef,
    ) -> bool {
        if shadows.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        command_encoder.set_render_pipeline_state(&self.shadows_pipeline_state);
        command_encoder.set_vertex_buffer(ShadowInputIndex::Vertices as u64, Some(&self.unit_vertices), 0);
        command_encoder.set_vertex_buffer(ShadowInputIndex::Shadows as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_fragment_buffer(ShadowInputIndex::Shadows as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_vertex_bytes(ShadowInputIndex::ViewportSize as u64, mem::size_of_val(&viewport_size) as u64, &viewport_size as *const Size<DevicePixels> as *const _);

        let shadow_bytes_len = mem::size_of_val(shadows);
        let buffer_contents = unsafe { (instance_buffer.metal_buffer.contents() as *mut u8).add(*instance_offset) };
        let next_offset = *instance_offset + shadow_bytes_len;
        if next_offset > instance_buffer.size {
            return false;
        }
        unsafe { ptr::copy_nonoverlapping(shadows.as_ptr() as *const u8, buffer_contents, shadow_bytes_len) };
        command_encoder.draw_primitives_instanced(metal::MTLPrimitiveType::Triangle, 0, 6, shadows.len() as u64);
        *instance_offset = next_offset;
        true
    }

    fn draw_quads(
        &self,
        quads: &[Quad],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &metal::RenderCommandEncoderRef,
    ) -> bool {
        if quads.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        command_encoder.set_render_pipeline_state(&self.quads_pipeline_state);
        command_encoder.set_vertex_buffer(QuadInputIndex::Vertices as u64, Some(&self.unit_vertices), 0);
        command_encoder.set_vertex_buffer(QuadInputIndex::Quads as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_fragment_buffer(QuadInputIndex::Quads as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_vertex_bytes(QuadInputIndex::ViewportSize as u64, mem::size_of_val(&viewport_size) as u64, &viewport_size as *const Size<DevicePixels> as *const _);

        let quad_bytes_len = mem::size_of_val(quads);
        let buffer_contents = unsafe { (instance_buffer.metal_buffer.contents() as *mut u8).add(*instance_offset) };
        let next_offset = *instance_offset + quad_bytes_len;
        if next_offset > instance_buffer.size {
            return false;
        }
        unsafe { ptr::copy_nonoverlapping(quads.as_ptr() as *const u8, buffer_contents, quad_bytes_len) };
        command_encoder.draw_primitives_instanced(metal::MTLPrimitiveType::Triangle, 0, 6, quads.len() as u64);
        *instance_offset = next_offset;
        true
    }

    fn draw_paths_from_intermediate(
        &self,
        paths: &[Path<ScaledPixels>],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &metal::RenderCommandEncoderRef,
    ) -> bool {
        let Some(first_path) = paths.first() else {
            return true;
        };
        let Some(ref intermediate_texture) = self.path_intermediate_texture else {
            return false;
        };

        command_encoder.set_render_pipeline_state(&self.path_sprites_pipeline_state);
        command_encoder.set_vertex_buffer(SpriteInputIndex::Vertices as u64, Some(&self.unit_vertices), 0);
        command_encoder.set_vertex_bytes(SpriteInputIndex::ViewportSize as u64, mem::size_of_val(&viewport_size) as u64, &viewport_size as *const Size<DevicePixels> as *const _);
        command_encoder.set_fragment_texture(SpriteInputIndex::AtlasTexture as u64, Some(intermediate_texture));

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

        command_encoder.set_vertex_buffer(SpriteInputIndex::Sprites as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        let buffer_contents = unsafe { (instance_buffer.metal_buffer.contents() as *mut u8).add(*instance_offset) };
        unsafe { ptr::copy_nonoverlapping(sprites.as_ptr() as *const u8, buffer_contents, sprite_bytes_len) };
        command_encoder.draw_primitives_instanced(metal::MTLPrimitiveType::Triangle, 0, 6, sprites.len() as u64);
        *instance_offset = next_offset;
        true
    }

    fn draw_underlines(
        &self,
        underlines: &[Underline],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &metal::RenderCommandEncoderRef,
    ) -> bool {
        if underlines.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        command_encoder.set_render_pipeline_state(&self.underlines_pipeline_state);
        command_encoder.set_vertex_buffer(UnderlineInputIndex::Vertices as u64, Some(&self.unit_vertices), 0);
        command_encoder.set_vertex_buffer(UnderlineInputIndex::Underlines as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_fragment_buffer(UnderlineInputIndex::Underlines as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_vertex_bytes(UnderlineInputIndex::ViewportSize as u64, mem::size_of_val(&viewport_size) as u64, &viewport_size as *const Size<DevicePixels> as *const _);

        let underline_bytes_len = mem::size_of_val(underlines);
        let buffer_contents = unsafe { (instance_buffer.metal_buffer.contents() as *mut u8).add(*instance_offset) };
        let next_offset = *instance_offset + underline_bytes_len;
        if next_offset > instance_buffer.size {
            return false;
        }
        unsafe { ptr::copy_nonoverlapping(underlines.as_ptr() as *const u8, buffer_contents, underline_bytes_len) };
        command_encoder.draw_primitives_instanced(metal::MTLPrimitiveType::Triangle, 0, 6, underlines.len() as u64);
        *instance_offset = next_offset;
        true
    }

    fn draw_monochrome_sprites(
        &self,
        texture_id: AtlasTextureId,
        sprites: &[MonochromeSprite],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &metal::RenderCommandEncoderRef,
    ) -> bool {
        if sprites.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        let sprite_bytes_len = mem::size_of_val(sprites);
        let buffer_contents = unsafe { (instance_buffer.metal_buffer.contents() as *mut u8).add(*instance_offset) };
        let next_offset = *instance_offset + sprite_bytes_len;
        if next_offset > instance_buffer.size {
            return false;
        }

        let texture = self.sprite_atlas.metal_texture(texture_id);
        let texture_size = size(DevicePixels(texture.width() as i32), DevicePixels(texture.height() as i32));
        command_encoder.set_render_pipeline_state(&self.monochrome_sprites_pipeline_state);
        command_encoder.set_vertex_buffer(SpriteInputIndex::Vertices as u64, Some(&self.unit_vertices), 0);
        command_encoder.set_vertex_buffer(SpriteInputIndex::Sprites as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_vertex_bytes(SpriteInputIndex::ViewportSize as u64, mem::size_of_val(&viewport_size) as u64, &viewport_size as *const Size<DevicePixels> as *const _);
        command_encoder.set_vertex_bytes(SpriteInputIndex::AtlasTextureSize as u64, mem::size_of_val(&texture_size) as u64, &texture_size as *const Size<DevicePixels> as *const _);
        command_encoder.set_fragment_buffer(SpriteInputIndex::Sprites as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_fragment_texture(SpriteInputIndex::AtlasTexture as u64, Some(&texture));

        unsafe { ptr::copy_nonoverlapping(sprites.as_ptr() as *const u8, buffer_contents, sprite_bytes_len) };
        command_encoder.draw_primitives_instanced(metal::MTLPrimitiveType::Triangle, 0, 6, sprites.len() as u64);
        *instance_offset = next_offset;
        true
    }

    fn draw_polychrome_sprites(
        &self,
        texture_id: AtlasTextureId,
        sprites: &[PolychromeSprite],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &metal::RenderCommandEncoderRef,
    ) -> bool {
        if sprites.is_empty() {
            return true;
        }
        align_offset(instance_offset);

        let texture = self.sprite_atlas.metal_texture(texture_id);
        let texture_size = size(DevicePixels(texture.width() as i32), DevicePixels(texture.height() as i32));
        command_encoder.set_render_pipeline_state(&self.polychrome_sprites_pipeline_state);
        command_encoder.set_vertex_buffer(SpriteInputIndex::Vertices as u64, Some(&self.unit_vertices), 0);
        command_encoder.set_vertex_buffer(SpriteInputIndex::Sprites as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_vertex_bytes(SpriteInputIndex::ViewportSize as u64, mem::size_of_val(&viewport_size) as u64, &viewport_size as *const Size<DevicePixels> as *const _);
        command_encoder.set_vertex_bytes(SpriteInputIndex::AtlasTextureSize as u64, mem::size_of_val(&texture_size) as u64, &texture_size as *const Size<DevicePixels> as *const _);
        command_encoder.set_fragment_buffer(SpriteInputIndex::Sprites as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
        command_encoder.set_fragment_texture(SpriteInputIndex::AtlasTexture as u64, Some(&texture));

        let sprite_bytes_len = mem::size_of_val(sprites);
        let buffer_contents = unsafe { (instance_buffer.metal_buffer.contents() as *mut u8).add(*instance_offset) };
        let next_offset = *instance_offset + sprite_bytes_len;
        if next_offset > instance_buffer.size {
            return false;
        }
        unsafe { ptr::copy_nonoverlapping(sprites.as_ptr() as *const u8, buffer_contents, sprite_bytes_len) };
        command_encoder.draw_primitives_instanced(metal::MTLPrimitiveType::Triangle, 0, 6, sprites.len() as u64);
        *instance_offset = next_offset;
        true
    }

    fn draw_surfaces(
        &mut self,
        surfaces: &[PaintSurface],
        instance_buffer: &mut InstanceBuffer,
        instance_offset: &mut usize,
        viewport_size: Size<DevicePixels>,
        command_encoder: &metal::RenderCommandEncoderRef,
    ) -> bool {
        command_encoder.set_render_pipeline_state(&self.surfaces_pipeline_state);
        command_encoder.set_vertex_buffer(SurfaceInputIndex::Vertices as u64, Some(&self.unit_vertices), 0);
        command_encoder.set_vertex_bytes(SurfaceInputIndex::ViewportSize as u64, mem::size_of_val(&viewport_size) as u64, &viewport_size as *const Size<DevicePixels> as *const _);

        for surface in surfaces {
            let texture_size = size(
                DevicePixels::from(surface.image_buffer.get_width() as i32),
                DevicePixels::from(surface.image_buffer.get_height() as i32),
            );

            assert_eq!(
                surface.image_buffer.get_pixel_format(),
                kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
            );

            let y_texture = self.core_video_texture_cache
                .create_texture_from_image(surface.image_buffer.as_concrete_TypeRef(), None, MTLPixelFormat::R8Unorm, surface.image_buffer.get_width_of_plane(0), surface.image_buffer.get_height_of_plane(0), 0)
                .unwrap();
            let cb_cr_texture = self.core_video_texture_cache
                .create_texture_from_image(surface.image_buffer.as_concrete_TypeRef(), None, MTLPixelFormat::RG8Unorm, surface.image_buffer.get_width_of_plane(1), surface.image_buffer.get_height_of_plane(1), 1)
                .unwrap();

            align_offset(instance_offset);
            let next_offset = *instance_offset + mem::size_of::<Surface>();
            if next_offset > instance_buffer.size {
                return false;
            }

            command_encoder.set_vertex_buffer(SurfaceInputIndex::Surfaces as u64, Some(&instance_buffer.metal_buffer), *instance_offset as u64);
            command_encoder.set_vertex_bytes(SurfaceInputIndex::TextureSize as u64, mem::size_of_val(&texture_size) as u64, &texture_size as *const Size<DevicePixels> as *const _);
            command_encoder.set_fragment_texture(SurfaceInputIndex::YTexture as u64, unsafe {
                let texture = CVMetalTextureGetTexture(y_texture.as_concrete_TypeRef());
                Some(metal::TextureRef::from_ptr(texture as *mut _))
            });
            command_encoder.set_fragment_texture(SurfaceInputIndex::CbCrTexture as u64, unsafe {
                let texture = CVMetalTextureGetTexture(cb_cr_texture.as_concrete_TypeRef());
                Some(metal::TextureRef::from_ptr(texture as *mut _))
            });

            unsafe {
                let buffer_contents = (instance_buffer.metal_buffer.contents() as *mut u8)
                    .add(*instance_offset)
                    as *mut SurfaceBounds;
                ptr::write(
                    buffer_contents,
                    SurfaceBounds {
                        bounds: surface.bounds,
                        content_mask: surface.content_mask.clone(),
                    },
                );
            }

            command_encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 6);
            *instance_offset = next_offset;
        }
        true
    }
}

pub(super) fn new_command_encoder_for_texture<'a>(
    command_buffer: &'a metal::CommandBufferRef,
    texture: &'a metal::TextureRef,
    viewport_size: Size<DevicePixels>,
    configure_color_attachment: impl Fn(&RenderPassColorAttachmentDescriptorRef),
) -> &'a metal::RenderCommandEncoderRef {
    let render_pass_descriptor = metal::RenderPassDescriptor::new();
    let color_attachment = render_pass_descriptor
        .color_attachments()
        .object_at(0)
        .unwrap();
    color_attachment.set_texture(Some(texture));
    color_attachment.set_store_action(metal::MTLStoreAction::Store);
    configure_color_attachment(color_attachment);

    let command_encoder = command_buffer.new_render_command_encoder(render_pass_descriptor);
    command_encoder.set_viewport(metal::MTLViewport {
        originX: 0.0,
        originY: 0.0,
        width: i32::from(viewport_size.width) as f64,
        height: i32::from(viewport_size.height) as f64,
        znear: 0.0,
        zfar: 1.0,
    });
    command_encoder
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
