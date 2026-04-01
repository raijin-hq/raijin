use super::*;

impl DirectWriteState {
    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        let font_info = &self.fonts[font_id.0];
        let codepoints = ch as u32;
        let mut glyph_indices = 0u16;
        unsafe {
            font_info
                .font_face
                .GetGlyphIndices(&raw const codepoints, 1, &raw mut glyph_indices)
                .log_err()
        }
        .map(|_| GlyphId(glyph_indices as u32))
    }

    fn rasterize_glyph(
        &self,
        components: &DirectWriteComponents,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        if glyph_bounds.size.width.0 == 0 || glyph_bounds.size.height.0 == 0 {
            anyhow::bail!("glyph bounds are empty");
        }

        let bitmap_data = if params.is_emoji {
            if let Ok(color) = self.rasterize_color(components, params, glyph_bounds) {
                color
            } else {
                let monochrome = self.rasterize_monochrome(components, params, glyph_bounds)?;
                monochrome
                    .into_iter()
                    .flat_map(|pixel| [0, 0, 0, pixel])
                    .collect::<Vec<_>>()
            }
        } else {
            self.rasterize_monochrome(components, params, glyph_bounds)?
        };

        Ok((glyph_bounds.size, bitmap_data))
    }

    fn rasterize_monochrome(
        &self,
        components: &DirectWriteComponents,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
    ) -> Result<Vec<u8>> {
        let glyph_analysis = self.create_glyph_run_analysis(components, params)?;
        if !params.subpixel_rendering {
            let mut bitmap_data =
                vec![0u8; glyph_bounds.size.width.0 as usize * glyph_bounds.size.height.0 as usize];
            unsafe {
                glyph_analysis.CreateAlphaTexture(
                    DWRITE_TEXTURE_ALIASED_1x1,
                    &RECT {
                        left: glyph_bounds.origin.x.0,
                        top: glyph_bounds.origin.y.0,
                        right: glyph_bounds.size.width.0 + glyph_bounds.origin.x.0,
                        bottom: glyph_bounds.size.height.0 + glyph_bounds.origin.y.0,
                    },
                    &mut bitmap_data,
                )?;
            }

            return Ok(bitmap_data);
        }

        let width = glyph_bounds.size.width.0 as usize;
        let height = glyph_bounds.size.height.0 as usize;
        let pixel_count = width * height;

        let mut bitmap_data = vec![0u8; pixel_count * 4];

        unsafe {
            glyph_analysis.CreateAlphaTexture(
                DWRITE_TEXTURE_CLEARTYPE_3x1,
                &RECT {
                    left: glyph_bounds.origin.x.0,
                    top: glyph_bounds.origin.y.0,
                    right: glyph_bounds.size.width.0 + glyph_bounds.origin.x.0,
                    bottom: glyph_bounds.size.height.0 + glyph_bounds.origin.y.0,
                },
                &mut bitmap_data[..pixel_count * 3],
            )?;
        }

        // The output buffer expects RGBA data, so pad the alpha channel with zeros.
        for pixel_ix in (0..pixel_count).rev() {
            let src = pixel_ix * 3;
            let dst = pixel_ix * 4;
            (
                bitmap_data[dst],
                bitmap_data[dst + 1],
                bitmap_data[dst + 2],
                bitmap_data[dst + 3],
            ) = (
                bitmap_data[src],
                bitmap_data[src + 1],
                bitmap_data[src + 2],
                0,
            );
        }

        Ok(bitmap_data)
    }

    fn rasterize_color(
        &self,
        components: &DirectWriteComponents,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
    ) -> Result<Vec<u8>> {
        let bitmap_size = glyph_bounds.size;
        let subpixel_shift = params
            .subpixel_variant
            .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);
        let baseline_origin_x = subpixel_shift.x / params.scale_factor;
        let baseline_origin_y = subpixel_shift.y / params.scale_factor;

        let transform = DWRITE_MATRIX {
            m11: params.scale_factor,
            m12: 0.0,
            m21: 0.0,
            m22: params.scale_factor,
            dx: 0.0,
            dy: 0.0,
        };

        let font = &self.fonts[params.font_id.0];
        let glyph_id = [params.glyph_id.0 as u16];
        let advance = [glyph_bounds.size.width.0 as f32];
        let offset = [DWRITE_GLYPH_OFFSET {
            advanceOffset: -glyph_bounds.origin.x.0 as f32 / params.scale_factor,
            ascenderOffset: glyph_bounds.origin.y.0 as f32 / params.scale_factor,
        }];
        let glyph_run = DWRITE_GLYPH_RUN {
            fontFace: ManuallyDrop::new(Some(unsafe { std::ptr::read(&***font.font_face) })),
            fontEmSize: params.font_size.as_f32(),
            glyphCount: 1,
            glyphIndices: glyph_id.as_ptr(),
            glyphAdvances: advance.as_ptr(),
            glyphOffsets: offset.as_ptr(),
            isSideways: BOOL(0),
            bidiLevel: 0,
        };

        // todo: support formats other than COLR
        let color_enumerator = unsafe {
            components.factory.TranslateColorGlyphRun(
                Vector2::new(baseline_origin_x, baseline_origin_y),
                &glyph_run,
                None,
                DWRITE_GLYPH_IMAGE_FORMATS_COLR,
                DWRITE_MEASURING_MODE_NATURAL,
                Some(&transform),
                0,
            )
        }?;

        let mut glyph_layers = Vec::new();
        let mut alpha_data = Vec::new();
        loop {
            let color_run = unsafe { color_enumerator.GetCurrentRun() }?;
            let color_run = unsafe { &*color_run };
            let image_format = color_run.glyphImageFormat & !DWRITE_GLYPH_IMAGE_FORMATS_TRUETYPE;
            if image_format == DWRITE_GLYPH_IMAGE_FORMATS_COLR {
                let color_analysis = unsafe {
                    components.factory.CreateGlyphRunAnalysis(
                        &color_run.Base.glyphRun as *const _,
                        Some(&transform),
                        DWRITE_RENDERING_MODE1_NATURAL_SYMMETRIC,
                        DWRITE_MEASURING_MODE_NATURAL,
                        DWRITE_GRID_FIT_MODE_DEFAULT,
                        DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE,
                        baseline_origin_x,
                        baseline_origin_y,
                    )
                }?;

                let color_bounds =
                    unsafe { color_analysis.GetAlphaTextureBounds(DWRITE_TEXTURE_ALIASED_1x1) }?;

                let color_size = size(
                    color_bounds.right - color_bounds.left,
                    color_bounds.bottom - color_bounds.top,
                );
                if color_size.width > 0 && color_size.height > 0 {
                    alpha_data.clear();
                    alpha_data.resize((color_size.width * color_size.height) as usize, 0);
                    unsafe {
                        color_analysis.CreateAlphaTexture(
                            DWRITE_TEXTURE_ALIASED_1x1,
                            &color_bounds,
                            &mut alpha_data,
                        )
                    }?;

                    let run_color = {
                        let run_color = color_run.Base.runColor;
                        Rgba {
                            r: run_color.r,
                            g: run_color.g,
                            b: run_color.b,
                            a: run_color.a,
                        }
                    };
                    let bounds = bounds(point(color_bounds.left, color_bounds.top), color_size);
                    glyph_layers.push(GlyphLayerTexture::new(
                        &self.gpu_state,
                        run_color,
                        bounds,
                        &alpha_data,
                    )?);
                }
            }

            let has_next = unsafe { color_enumerator.MoveNext() }
                .map(|e| e.as_bool())
                .unwrap_or(false);
            if !has_next {
                break;
            }
        }

        let gpu_state = &self.gpu_state;
        let params_buffer = {
            let desc = D3D11_BUFFER_DESC {
                ByteWidth: std::mem::size_of::<GlyphLayerTextureParams>() as u32,
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                MiscFlags: 0,
                StructureByteStride: 0,
            };

            let mut buffer = None;
            unsafe {
                gpu_state
                    .device
                    .CreateBuffer(&desc, None, Some(&mut buffer))
            }?;
            buffer
        };

        let render_target_texture = {
            let mut texture = None;
            let desc = D3D11_TEXTURE2D_DESC {
                Width: bitmap_size.width.0 as u32,
                Height: bitmap_size.height.0 as u32,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            unsafe {
                gpu_state
                    .device
                    .CreateTexture2D(&desc, None, Some(&mut texture))
            }?;
            texture.unwrap()
        };

        let render_target_view = {
            let desc = D3D11_RENDER_TARGET_VIEW_DESC {
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 },
                },
            };
            let mut rtv = None;
            unsafe {
                gpu_state.device.CreateRenderTargetView(
                    &render_target_texture,
                    Some(&desc),
                    Some(&mut rtv),
                )
            }?;
            rtv
        };

        let staging_texture = {
            let mut texture = None;
            let desc = D3D11_TEXTURE2D_DESC {
                Width: bitmap_size.width.0 as u32,
                Height: bitmap_size.height.0 as u32,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            unsafe {
                gpu_state
                    .device
                    .CreateTexture2D(&desc, None, Some(&mut texture))
            }?;
            texture.unwrap()
        };

        let device_context = &gpu_state.device_context;
        unsafe { device_context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP) };
        unsafe { device_context.VSSetShader(&gpu_state.vertex_shader, None) };
        unsafe { device_context.PSSetShader(&gpu_state.pixel_shader, None) };
        unsafe {
            device_context.VSSetConstantBuffers(0, Some(std::slice::from_ref(&params_buffer)))
        };
        unsafe {
            device_context.PSSetConstantBuffers(0, Some(std::slice::from_ref(&params_buffer)))
        };
        unsafe {
            device_context.OMSetRenderTargets(Some(std::slice::from_ref(&render_target_view)), None)
        };
        unsafe { device_context.PSSetSamplers(0, Some(std::slice::from_ref(&gpu_state.sampler))) };
        unsafe { device_context.OMSetBlendState(&gpu_state.blend_state, None, 0xffffffff) };

        let crate::FontInfo {
            gamma_ratios,
            grayscale_enhanced_contrast,
            ..
        } = DirectXRenderer::get_font_info();

        for layer in glyph_layers {
            let params = GlyphLayerTextureParams {
                run_color: layer.run_color,
                bounds: layer.bounds,
                gamma_ratios: *gamma_ratios,
                grayscale_enhanced_contrast: *grayscale_enhanced_contrast,
                _pad: [0f32; 3],
            };
            unsafe {
                let mut dest = std::mem::zeroed();
                gpu_state.device_context.Map(
                    params_buffer.as_ref().unwrap(),
                    0,
                    D3D11_MAP_WRITE_DISCARD,
                    0,
                    Some(&mut dest),
                )?;
                std::ptr::copy_nonoverlapping(&params as *const _, dest.pData as *mut _, 1);
                gpu_state
                    .device_context
                    .Unmap(params_buffer.as_ref().unwrap(), 0);
            };

            let texture = [Some(layer.texture_view)];
            unsafe { device_context.PSSetShaderResources(0, Some(&texture)) };

            let viewport = [D3D11_VIEWPORT {
                TopLeftX: layer.bounds.origin.x as f32,
                TopLeftY: layer.bounds.origin.y as f32,
                Width: layer.bounds.size.width as f32,
                Height: layer.bounds.size.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }];
            unsafe { device_context.RSSetViewports(Some(&viewport)) };

            unsafe { device_context.Draw(4, 0) };
        }

        unsafe { device_context.CopyResource(&staging_texture, &render_target_texture) };

        let mapped_data = {
            let mut mapped_data = D3D11_MAPPED_SUBRESOURCE::default();
            unsafe {
                device_context.Map(
                    &staging_texture,
                    0,
                    D3D11_MAP_READ,
                    0,
                    Some(&mut mapped_data),
                )
            }?;
            mapped_data
        };
        let mut rasterized =
            vec![0u8; (bitmap_size.width.0 as u32 * bitmap_size.height.0 as u32 * 4) as usize];

        for y in 0..bitmap_size.height.0 as usize {
            let width = bitmap_size.width.0 as usize;
            unsafe {
                std::ptr::copy_nonoverlapping::<u8>(
                    (mapped_data.pData as *const u8).byte_add(mapped_data.RowPitch as usize * y),
                    rasterized
                        .as_mut_ptr()
                        .byte_add(width * y * std::mem::size_of::<u32>()),
                    width * std::mem::size_of::<u32>(),
                )
            };
        }

        // Convert from premultiplied to straight alpha
        for chunk in rasterized.chunks_exact_mut(4) {
            let b = chunk[0] as f32;
            let g = chunk[1] as f32;
            let r = chunk[2] as f32;
            let a = chunk[3] as f32;
            if a > 0.0 {
                let inv_a = 255.0 / a;
                chunk[0] = (b * inv_a).clamp(0.0, 255.0) as u8;
                chunk[1] = (g * inv_a).clamp(0.0, 255.0) as u8;
                chunk[2] = (r * inv_a).clamp(0.0, 255.0) as u8;
            }
        }

        Ok(rasterized)
    }

    fn get_typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        unsafe {
            let font = &self.fonts[font_id.0].font_face;
            let glyph_indices = [glyph_id.0 as u16];
            let mut metrics = [DWRITE_GLYPH_METRICS::default()];
            font.GetDesignGlyphMetrics(glyph_indices.as_ptr(), 1, metrics.as_mut_ptr(), false)?;

            let metrics = &metrics[0];
            let advance_width = metrics.advanceWidth as i32;
            let advance_height = metrics.advanceHeight as i32;
            let left_side_bearing = metrics.leftSideBearing;
            let right_side_bearing = metrics.rightSideBearing;
            let top_side_bearing = metrics.topSideBearing;
            let bottom_side_bearing = metrics.bottomSideBearing;
            let vertical_origin_y = metrics.verticalOriginY;

            let y_offset = vertical_origin_y + bottom_side_bearing - advance_height;
            let width = advance_width - (left_side_bearing + right_side_bearing);
            let height = advance_height - (top_side_bearing + bottom_side_bearing);

            Ok(Bounds {
                origin: Point {
                    x: left_side_bearing as f32,
                    y: y_offset as f32,
                },
                size: Size {
                    width: width as f32,
                    height: height as f32,
                },
            })
        }
    }

    fn get_advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        unsafe {
            let font = &self.fonts[font_id.0].font_face;
            let glyph_indices = [glyph_id.0 as u16];
            let mut metrics = [DWRITE_GLYPH_METRICS::default()];
            font.GetDesignGlyphMetrics(glyph_indices.as_ptr(), 1, metrics.as_mut_ptr(), false)?;

            let metrics = &metrics[0];

            Ok(Size {
                width: metrics.advanceWidth as f32,
                height: 0.0,
            })
        }
    }

    fn all_font_names(&self, components: &DirectWriteComponents) -> Vec<String> {
        let mut result =
            get_font_names_from_collection(&self.system_font_collection, &components.locale);
        result.extend(get_font_names_from_collection(
            &self.custom_font_collection,
            &components.locale,
        ));
        result
    }

    fn handle_gpu_lost(&mut self, directx_devices: &DirectXDevices) -> Result<()> {
        try_to_recover_from_device_lost(|| {
            GPUState::new(directx_devices).context("Recreating GPU state for DirectWrite")
        })
        .map(|gpu_state| self.gpu_state = gpu_state)
    }
}

