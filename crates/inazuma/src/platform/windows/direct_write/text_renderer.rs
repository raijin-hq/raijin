use super::*;

pub(super) struct GlyphLayerTexture {
    pub(super) run_color: Rgba,
    pub(super) bounds: Bounds<i32>,
    pub(super) texture_view: ID3D11ShaderResourceView,
    // holding on to the texture to not RAII drop it
    pub(super) _texture: ID3D11Texture2D,
}

impl GlyphLayerTexture {
    fn new(
        gpu_state: &GPUState,
        run_color: Rgba,
        bounds: Bounds<i32>,
        alpha_data: &[u8],
    ) -> Result<Self> {
        let texture_size = bounds.size;

        let desc = D3D11_TEXTURE2D_DESC {
            Width: texture_size.width as u32,
            Height: texture_size.height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };

        let texture = {
            let mut texture: Option<ID3D11Texture2D> = None;
            unsafe {
                gpu_state
                    .device
                    .CreateTexture2D(&desc, None, Some(&mut texture))?
            };
            texture.unwrap()
        };
        let texture_view = {
            let mut view: Option<ID3D11ShaderResourceView> = None;
            unsafe {
                gpu_state
                    .device
                    .CreateShaderResourceView(&texture, None, Some(&mut view))?
            };
            view.unwrap()
        };

        unsafe {
            gpu_state.device_context.UpdateSubresource(
                &texture,
                0,
                None,
                alpha_data.as_ptr() as _,
                texture_size.width as u32,
                0,
            )
        };

        Ok(GlyphLayerTexture {
            run_color,
            bounds,
            texture_view,
            _texture: texture,
        })
    }
}

#[repr(C)]
pub(super) struct GlyphLayerTextureParams {
    pub(super) bounds: Bounds<i32>,
    pub(super) run_color: Rgba,
    pub(super) gamma_ratios: [f32; 4],
    pub(super) grayscale_enhanced_contrast: f32,
    pub(super) _pad: [f32; 3],
}

pub(super) struct TextRendererWrapper(IDWriteTextRenderer);

impl TextRendererWrapper {
    pub(super) fn new(locale_str: HSTRING) -> Self {
        pub(super) let inner = TextRenderer::new(locale_str);
        TextRendererWrapper(inner.into())
    }
}

#[implement(IDWriteTextRenderer)]
pub(super) struct TextRenderer {
    pub(super) locale: HSTRING,
}

impl TextRenderer {
    fn new(locale: HSTRING) -> Self {
        TextRenderer { locale }
    }
}

pub(super) struct RendererContext<'t, 'a, 'b> {
    pub(super) text_system: &'t mut DirectWriteState,
    pub(super) components: &'a DirectWriteComponents,
    pub(super) index_converter: StringIndexConverter<'a>,
    pub(super) runs: &'b mut Vec<ShapedRun>,
    pub(super) width: f32,
}

#[derive(Debug)]
pub(super) struct ClusterAnalyzer<'t> {
    pub(super) utf16_idx: usize,
    pub(super) glyph_idx: usize,
    pub(super) glyph_count: usize,
    pub(super) cluster_map: &'t [u16],
}

impl<'t> ClusterAnalyzer<'t> {
    fn new(cluster_map: &'t [u16], glyph_count: usize) -> Self {
        ClusterAnalyzer {
            utf16_idx: 0,
            glyph_idx: 0,
            glyph_count,
            cluster_map,
        }
    }
}

impl Iterator for ClusterAnalyzer<'_> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<(usize, usize)> {
        if self.utf16_idx >= self.cluster_map.len() {
            return None; // No more clusters
        }
        let start_utf16_idx = self.utf16_idx;
        let current_glyph = self.cluster_map[start_utf16_idx] as usize;

        // Find the end of current cluster (where glyph index changes)
        let mut end_utf16_idx = start_utf16_idx + 1;
        while end_utf16_idx < self.cluster_map.len()
            && self.cluster_map[end_utf16_idx] as usize == current_glyph
        {
            end_utf16_idx += 1;
        }

        let utf16_len = end_utf16_idx - start_utf16_idx;

        // Calculate glyph count for this cluster
        let next_glyph = if end_utf16_idx < self.cluster_map.len() {
            self.cluster_map[end_utf16_idx] as usize
        } else {
            self.glyph_count
        };

        let glyph_count = next_glyph - current_glyph;

        // Update state for next call
        self.utf16_idx = end_utf16_idx;
        self.glyph_idx = next_glyph;

        Some((utf16_len, glyph_count))
    }
}

#[allow(non_snake_case)]
impl IDWritePixelSnapping_Impl for TextRenderer_Impl {
    fn IsPixelSnappingDisabled(
        &self,
        _clientdrawingcontext: *const ::core::ffi::c_void,
    ) -> windows::core::Result<BOOL> {
        Ok(BOOL(0))
    }

    fn GetCurrentTransform(
        &self,
        _clientdrawingcontext: *const ::core::ffi::c_void,
        transform: *mut DWRITE_MATRIX,
    ) -> windows::core::Result<()> {
        unsafe {
            *transform = DWRITE_MATRIX {
                m11: 1.0,
                m12: 0.0,
                m21: 0.0,
                m22: 1.0,
                dx: 0.0,
                dy: 0.0,
            };
        }
        Ok(())
    }

    fn GetPixelsPerDip(
        &self,
        _clientdrawingcontext: *const ::core::ffi::c_void,
    ) -> windows::core::Result<f32> {
        Ok(1.0)
    }
}

#[allow(non_snake_case)]
impl IDWriteTextRenderer_Impl for TextRenderer_Impl {
    fn DrawGlyphRun(
        &self,
        clientdrawingcontext: *const ::core::ffi::c_void,
        _baselineoriginx: f32,
        _baselineoriginy: f32,
        _measuringmode: DWRITE_MEASURING_MODE,
        glyphrun: *const DWRITE_GLYPH_RUN,
        glyphrundescription: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _clientdrawingeffect: windows::core::Ref<windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        let glyphrun = unsafe { &*glyphrun };
        let glyph_count = glyphrun.glyphCount as usize;
        if glyph_count == 0 {
            return Ok(());
        }
        let desc = unsafe { &*glyphrundescription };
        let context = unsafe { &mut *(clientdrawingcontext.cast::<RendererContext>().cast_mut()) };
        let Some(font_face) = glyphrun.fontFace.as_ref() else {
            return Ok(());
        };
        // This `cast()` action here should never fail since we are running on Win10+, and
        // `IDWriteFontFace3` requires Win10
        let Ok(font_face) = &font_face.cast::<IDWriteFontFace3>() else {
            return Err(Error::new(
                DWRITE_E_UNSUPPORTEDOPERATION,
                "Failed to cast font face",
            ));
        };

        let font_face_key = font_face.cast::<IUnknown>().unwrap().as_raw().addr();
        let font_id = context
            .text_system
            .font_info_cache
            .get(&font_face_key)
            .copied()
            // in some circumstances, we might be getting served a FontFace that we did not create ourselves
            // so create a new font from it and cache it accordingly. The usual culprit here seems to be Segoe UI Symbol
            .map_or_else(
                || {
                    let font = font_face_to_font(font_face, &self.locale)
                        .ok_or_else(|| Error::new(DWRITE_E_NOFONT, "Failed to create font"))?;
                    let font_id = match context.text_system.font_to_font_id.get(&font) {
                        Some(&font_id) => font_id,
                        None => context
                            .text_system
                            .select_and_cache_font(context.components, &font)
                            .ok_or_else(|| Error::new(DWRITE_E_NOFONT, "Failed to create font"))?,
                    };
                    context
                        .text_system
                        .font_info_cache
                        .insert(font_face_key, font_id);
                    windows::core::Result::Ok(font_id)
                },
                Ok,
            )?;

        let color_font = unsafe { font_face.IsColorFont().as_bool() };

        let glyph_ids = unsafe { std::slice::from_raw_parts(glyphrun.glyphIndices, glyph_count) };
        let glyph_advances =
            unsafe { std::slice::from_raw_parts(glyphrun.glyphAdvances, glyph_count) };
        let glyph_offsets =
            unsafe { std::slice::from_raw_parts(glyphrun.glyphOffsets, glyph_count) };
        let cluster_map =
            unsafe { std::slice::from_raw_parts(desc.clusterMap, desc.stringLength as usize) };

        let cluster_analyzer = ClusterAnalyzer::new(cluster_map, glyph_count);
        let mut utf16_idx = desc.textPosition as usize;
        let mut glyph_idx = 0;
        let mut glyphs = Vec::with_capacity(glyph_count);
        for (cluster_utf16_len, cluster_glyph_count) in cluster_analyzer {
            context.index_converter.advance_to_utf16_ix(utf16_idx);
            utf16_idx += cluster_utf16_len;
            for (cluster_glyph_idx, glyph_id) in glyph_ids
                [glyph_idx..(glyph_idx + cluster_glyph_count)]
                .iter()
                .enumerate()
            {
                let id = GlyphId(*glyph_id as u32);
                let is_emoji =
                    color_font && is_color_glyph(font_face, id, &context.components.factory);
                let this_glyph_idx = glyph_idx + cluster_glyph_idx;
                glyphs.push(ShapedGlyph {
                    id,
                    position: point(
                        px(context.width + glyph_offsets[this_glyph_idx].advanceOffset),
                        px(-glyph_offsets[this_glyph_idx].ascenderOffset),
                    ),
                    index: context.index_converter.utf8_ix,
                    is_emoji,
                });
                context.width += glyph_advances[this_glyph_idx];
            }
            glyph_idx += cluster_glyph_count;
        }
        context.runs.push(ShapedRun { font_id, glyphs });
        Ok(())
    }

    fn DrawUnderline(
        &self,
        _clientdrawingcontext: *const ::core::ffi::c_void,
        _baselineoriginx: f32,
        _baselineoriginy: f32,
        _underline: *const DWRITE_UNDERLINE,
        _clientdrawingeffect: windows::core::Ref<windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            E_NOTIMPL,
            "DrawUnderline unimplemented",
        ))
    }

    fn DrawStrikethrough(
        &self,
        _clientdrawingcontext: *const ::core::ffi::c_void,
        _baselineoriginx: f32,
        _baselineoriginy: f32,
        _strikethrough: *const DWRITE_STRIKETHROUGH,
        _clientdrawingeffect: windows::core::Ref<windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            E_NOTIMPL,
            "DrawStrikethrough unimplemented",
        ))
    }

    fn DrawInlineObject(
        &self,
        _clientdrawingcontext: *const ::core::ffi::c_void,
        _originx: f32,
        _originy: f32,
        _inlineobject: windows::core::Ref<IDWriteInlineObject>,
        _issideways: BOOL,
        _isrighttoleft: BOOL,
        _clientdrawingeffect: windows::core::Ref<windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            E_NOTIMPL,
            "DrawInlineObject unimplemented",
        ))
    }
}

pub(super) struct StringIndexConverter<'a> {
    pub(super) text: &'a str,
    pub(super) utf8_ix: usize,
    pub(super) utf16_ix: usize,
}

impl<'a> StringIndexConverter<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            utf8_ix: 0,
            utf16_ix: 0,
        }
    }

    #[allow(dead_code)]
    fn advance_to_utf8_ix(&mut self, utf8_target: usize) {
        for (ix, c) in self.text[self.utf8_ix..].char_indices() {
            if self.utf8_ix + ix >= utf8_target {
                self.utf8_ix += ix;
                return;
            }
            self.utf16_ix += c.len_utf16();
        }
        self.utf8_ix = self.text.len();
    }

    fn advance_to_utf16_ix(&mut self, utf16_target: usize) {
        for (ix, c) in self.text[self.utf8_ix..].char_indices() {
            if self.utf16_ix >= utf16_target {
                self.utf8_ix += ix;
                return;
            }
            self.utf16_ix += c.len_utf16();
        }
        self.utf8_ix = self.text.len();
    }
}

