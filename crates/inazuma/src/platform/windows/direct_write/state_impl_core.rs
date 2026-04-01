use super::*;

impl DirectWriteState {
    fn select_and_cache_font(
        &mut self,
        components: &DirectWriteComponents,
        font: &Font,
    ) -> Option<FontId> {
        let select_font = |this: &mut DirectWriteState, font: &Font| -> Option<FontId> {
            let info = [&this.custom_font_collection, &this.system_font_collection]
                .into_iter()
                .find_map(|font_collection| unsafe {
                    DirectWriteState::make_font_from_font_collection(
                        font,
                        font_collection,
                        &components.factory,
                        &this.system_font_collection,
                        &components.system_ui_font_name,
                    )
                })?;

            let font_id = FontId(this.fonts.len());
            let font_face_key = info.font_face.cast::<IUnknown>().unwrap().as_raw().addr();
            this.fonts.push(info);
            this.font_info_cache.insert(font_face_key, font_id);
            Some(font_id)
        };

        let mut font_id = select_font(self, font);
        if font_id.is_none() {
            // try updating system fonts and reselect
            let mut collection = None;
            let font_collection_updated = unsafe {
                components
                    .factory
                    .GetSystemFontCollection(false, &mut collection, true)
            }
            .log_err()
            .is_some();
            if font_collection_updated && let Some(collection) = collection {
                self.system_font_collection = collection;
            }
            font_id = select_font(self, font);
        };
        let font_id = font_id?;
        self.font_to_font_id.insert(font.clone(), font_id);
        Some(font_id)
    }

    fn add_fonts(
        &mut self,
        components: &DirectWriteComponents,
        fonts: Vec<Cow<'static, [u8]>>,
    ) -> Result<()> {
        for font_data in fonts {
            match font_data {
                Cow::Borrowed(data) => unsafe {
                    let font_file = components
                        .in_memory_loader
                        .CreateInMemoryFontFileReference(
                            &components.factory,
                            data.as_ptr().cast(),
                            data.len() as _,
                            None,
                        )?;
                    components.builder.AddFontFile(&font_file)?;
                },
                Cow::Owned(data) => unsafe {
                    let font_file = components
                        .in_memory_loader
                        .CreateInMemoryFontFileReference(
                            &components.factory,
                            data.as_ptr().cast(),
                            data.len() as _,
                            None,
                        )?;
                    components.builder.AddFontFile(&font_file)?;
                },
            }
        }
        let set = unsafe { components.builder.CreateFontSet()? };
        let collection = unsafe { components.factory.CreateFontCollectionFromFontSet(&set)? };
        self.custom_font_collection = collection;

        Ok(())
    }

    fn generate_font_fallbacks(
        fallbacks: &FontFallbacks,
        factory: &IDWriteFactory5,
        system_font_collection: &IDWriteFontCollection1,
    ) -> Result<Option<IDWriteFontFallback>> {
        let fallback_list = fallbacks.fallback_list();
        if fallback_list.is_empty() {
            return Ok(None);
        }
        unsafe {
            let builder = factory.CreateFontFallbackBuilder()?;
            let font_set = &system_font_collection.GetFontSet()?;
            let mut unicode_ranges = Vec::new();
            for family_name in fallback_list {
                let family_name = HSTRING::from(family_name);
                let Some(fonts) = font_set
                    .GetMatchingFonts(
                        &family_name,
                        DWRITE_FONT_WEIGHT_NORMAL,
                        DWRITE_FONT_STRETCH_NORMAL,
                        DWRITE_FONT_STYLE_NORMAL,
                    )
                    .log_err()
                else {
                    continue;
                };
                let Ok(font_face) = fonts.GetFontFaceReference(0) else {
                    continue;
                };
                let font = font_face.CreateFontFace()?;
                let mut count = 0;
                font.GetUnicodeRanges(None, &mut count).ok();
                if count == 0 {
                    continue;
                }
                unicode_ranges.clear();
                unicode_ranges.resize_with(count as usize, DWRITE_UNICODE_RANGE::default);
                let Some(_) = font
                    .GetUnicodeRanges(Some(&mut unicode_ranges), &mut count)
                    .log_err()
                else {
                    continue;
                };
                builder.AddMapping(
                    &unicode_ranges,
                    &[family_name.as_ptr()],
                    None,
                    None,
                    None,
                    1.0,
                )?;
            }
            let system_fallbacks = factory.GetSystemFontFallback()?;
            builder.AddMappings(&system_fallbacks)?;
            Ok(Some(builder.CreateFontFallback()?))
        }
    }

    unsafe fn generate_font_features(
        factory: &IDWriteFactory5,
        font_features: &FontFeatures,
    ) -> Result<IDWriteTypography> {
        let direct_write_features = unsafe { factory.CreateTypography()? };
        apply_font_features(&direct_write_features, font_features)?;
        Ok(direct_write_features)
    }

    unsafe fn make_font_from_font_collection(
        &Font {
            ref family,
            ref features,
            ref fallbacks,
            weight,
            style,
        }: &Font,
        collection: &IDWriteFontCollection1,
        factory: &IDWriteFactory5,
        system_font_collection: &IDWriteFontCollection1,
        system_ui_font_name: &SharedString,
    ) -> Option<FontInfo> {
        const SYSTEM_UI_FONT_NAME: &str = ".SystemUIFont";
        let family = if family == SYSTEM_UI_FONT_NAME {
            system_ui_font_name
        } else {
            inazuma::font_name_with_fallbacks_shared(&family, &system_ui_font_name)
        };
        let fontset = unsafe { collection.GetFontSet().log_err()? };
        let font_family_h = HSTRING::from(family.as_str());
        let font = unsafe {
            fontset
                .GetMatchingFonts(
                    &font_family_h,
                    font_weight_to_dwrite(weight),
                    DWRITE_FONT_STRETCH_NORMAL,
                    font_style_to_dwrite(style),
                )
                .log_err()?
        };
        let total_number = unsafe { font.GetFontCount() };
        for index in 0..total_number {
            let res = maybe!({
                let font_face_ref = unsafe { font.GetFontFaceReference(index).log_err()? };
                let font_face = unsafe { font_face_ref.CreateFontFace().log_err()? };
                let direct_write_features =
                    unsafe { Self::generate_font_features(factory, features).log_err()? };
                let fallbacks = fallbacks.as_ref().and_then(|fallbacks| {
                    Self::generate_font_fallbacks(fallbacks, factory, system_font_collection)
                        .log_err()
                        .flatten()
                });
                let font_info = FontInfo {
                    font_family_h: font_family_h.clone(),
                    font_face,
                    features: direct_write_features,
                    fallbacks,
                    font_collection: collection.clone(),
                };
                Some(font_info)
            });
            if res.is_some() {
                return res;
            }
        }
        None
    }

    fn layout_line(
        &mut self,
        components: &DirectWriteComponents,
        text: &str,
        font_size: Pixels,
        font_runs: &[FontRun],
    ) -> Result<LineLayout> {
        if font_runs.is_empty() {
            return Ok(LineLayout {
                font_size,
                ..Default::default()
            });
        }
        unsafe {
            self.layout_line_scratch.clear();
            self.layout_line_scratch.extend(text.encode_utf16());
            let text_wide = &*self.layout_line_scratch;

            let mut utf8_offset = 0usize;
            let mut utf16_offset = 0u32;
            let text_layout = {
                let first_run = &font_runs[0];
                let font_info = &self.fonts[first_run.font_id.0];
                let collection = &font_info.font_collection;
                let format: IDWriteTextFormat1 = components
                    .factory
                    .CreateTextFormat(
                        &font_info.font_family_h,
                        collection,
                        font_info.font_face.GetWeight(),
                        font_info.font_face.GetStyle(),
                        DWRITE_FONT_STRETCH_NORMAL,
                        font_size.as_f32(),
                        &components.locale,
                    )?
                    .cast()?;
                if let Some(ref fallbacks) = font_info.fallbacks {
                    format.SetFontFallback(fallbacks)?;
                }

                let layout = components.factory.CreateTextLayout(
                    text_wide,
                    &format,
                    f32::INFINITY,
                    f32::INFINITY,
                )?;
                let current_text = &text[utf8_offset..(utf8_offset + first_run.len)];
                utf8_offset += first_run.len;
                let current_text_utf16_length = current_text.encode_utf16().count() as u32;
                let text_range = DWRITE_TEXT_RANGE {
                    startPosition: utf16_offset,
                    length: current_text_utf16_length,
                };
                layout.SetTypography(&font_info.features, text_range)?;
                utf16_offset += current_text_utf16_length;

                layout
            };

            let (ascent, descent) = {
                let mut first_metrics = [DWRITE_LINE_METRICS::default(); 4];
                let mut line_count = 0u32;
                text_layout.GetLineMetrics(Some(&mut first_metrics), &mut line_count)?;
                (
                    px(first_metrics[0].baseline),
                    px(first_metrics[0].height - first_metrics[0].baseline),
                )
            };
            let mut break_ligatures = true;
            for run in &font_runs[1..] {
                let font_info = &self.fonts[run.font_id.0];
                let current_text = &text[utf8_offset..(utf8_offset + run.len)];
                utf8_offset += run.len;
                let current_text_utf16_length = current_text.encode_utf16().count() as u32;

                let collection = &font_info.font_collection;
                let text_range = DWRITE_TEXT_RANGE {
                    startPosition: utf16_offset,
                    length: current_text_utf16_length,
                };
                utf16_offset += current_text_utf16_length;
                text_layout.SetFontCollection(collection, text_range)?;
                text_layout.SetFontFamilyName(&font_info.font_family_h, text_range)?;
                let font_size = if break_ligatures {
                    font_size.as_f32().next_up()
                } else {
                    font_size.as_f32()
                };
                text_layout.SetFontSize(font_size, text_range)?;
                text_layout.SetFontStyle(font_info.font_face.GetStyle(), text_range)?;
                text_layout.SetFontWeight(font_info.font_face.GetWeight(), text_range)?;
                text_layout.SetTypography(&font_info.features, text_range)?;

                break_ligatures = !break_ligatures;
            }

            let mut runs = Vec::new();
            let renderer_context = RendererContext {
                text_system: self,
                components,
                index_converter: StringIndexConverter::new(text),
                runs: &mut runs,
                width: 0.0,
            };
            text_layout.Draw(
                Some((&raw const renderer_context).cast::<c_void>()),
                &components.text_renderer.0,
                0.0,
                0.0,
            )?;
            let width = px(renderer_context.width);

            Ok(LineLayout {
                font_size,
                width,
                ascent,
                descent,
                runs,
                len: text.len(),
            })
        }
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        unsafe {
            let font_info = &self.fonts[font_id.0];
            let mut metrics = std::mem::zeroed();
            font_info.font_face.GetMetrics(&mut metrics);

            FontMetrics {
                units_per_em: metrics.Base.designUnitsPerEm as _,
                ascent: metrics.Base.ascent as _,
                descent: -(metrics.Base.descent as f32),
                line_gap: metrics.Base.lineGap as _,
                underline_position: metrics.Base.underlinePosition as _,
                underline_thickness: metrics.Base.underlineThickness as _,
                cap_height: metrics.Base.capHeight as _,
                x_height: metrics.Base.xHeight as _,
                bounding_box: Bounds {
                    origin: Point {
                        x: metrics.glyphBoxLeft as _,
                        y: metrics.glyphBoxBottom as _,
                    },
                    size: Size {
                        width: (metrics.glyphBoxRight - metrics.glyphBoxLeft) as _,
                        height: (metrics.glyphBoxTop - metrics.glyphBoxBottom) as _,
                    },
                },
            }
        }
    }

    fn create_glyph_run_analysis(
        &self,
        components: &DirectWriteComponents,
        params: &RenderGlyphParams,
    ) -> Result<IDWriteGlyphRunAnalysis> {
        let font = &self.fonts[params.font_id.0];
        let glyph_id = [params.glyph_id.0 as u16];
        let advance = [0.0];
        let offset = [DWRITE_GLYPH_OFFSET::default()];
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
        let transform = DWRITE_MATRIX {
            m11: params.scale_factor,
            m12: 0.0,
            m21: 0.0,
            m22: params.scale_factor,
            dx: 0.0,
            dy: 0.0,
        };
        let baseline_origin_x =
            params.subpixel_variant.x as f32 / SUBPIXEL_VARIANTS_X as f32 / params.scale_factor;
        let baseline_origin_y = params.subpixel_variant.y as f32
            / inazuma::SUBPIXEL_VARIANTS_Y as f32
            / params.scale_factor;

        let mut rendering_mode = DWRITE_RENDERING_MODE1::default();
        let mut grid_fit_mode = DWRITE_GRID_FIT_MODE::default();
        unsafe {
            font.font_face.GetRecommendedRenderingMode(
                params.font_size.as_f32(),
                // Using 96 as scale is applied by the transform
                96.0,
                96.0,
                Some(&transform),
                false,
                DWRITE_OUTLINE_THRESHOLD_ANTIALIASED,
                DWRITE_MEASURING_MODE_NATURAL,
                None,
                &mut rendering_mode,
                &mut grid_fit_mode,
            )?;
        }
        let rendering_mode = match rendering_mode {
            DWRITE_RENDERING_MODE1_OUTLINE => DWRITE_RENDERING_MODE1_NATURAL_SYMMETRIC,
            m => m,
        };

        let antialias_mode = if params.subpixel_rendering {
            DWRITE_TEXT_ANTIALIAS_MODE_CLEARTYPE
        } else {
            DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE
        };

        let glyph_analysis = unsafe {
            components.factory.CreateGlyphRunAnalysis(
                &glyph_run,
                Some(&transform),
                rendering_mode,
                DWRITE_MEASURING_MODE_NATURAL,
                grid_fit_mode,
                antialias_mode,
                baseline_origin_x,
                baseline_origin_y,
            )
        }?;
        Ok(glyph_analysis)
    }

    fn raster_bounds(
        &self,
        components: &DirectWriteComponents,
        params: &RenderGlyphParams,
    ) -> Result<Bounds<DevicePixels>> {
        let glyph_analysis = self.create_glyph_run_analysis(components, params)?;

        let texture_type = if params.subpixel_rendering {
            DWRITE_TEXTURE_CLEARTYPE_3x1
        } else {
            DWRITE_TEXTURE_ALIASED_1x1
        };

        let bounds = unsafe { glyph_analysis.GetAlphaTextureBounds(texture_type)? };

        if bounds.right < bounds.left {
            Ok(Bounds {
                origin: point(0.into(), 0.into()),
                size: size(0.into(), 0.into()),
            })
        } else {
            Ok(Bounds {
                origin: point(bounds.left.into(), bounds.top.into()),
                size: size(
                    (bounds.right - bounds.left).into(),
                    (bounds.bottom - bounds.top).into(),
                ),
            })
        }
    }

