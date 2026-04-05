use anyhow::anyhow;
use collections::HashMap;
use fontique::{
    Collection, CollectionOptions, QueryFamily, QueryFont, QueryStatus, SourceCache,
};
use inazuma::{
    Bounds, DevicePixels, Font, FontFallbacks, FontFeatures, FontId, FontMetrics, FontRun,
    FontStyle, GlyphId, LineLayout, Pixels, PlatformTextSystem, RenderGlyphParams, Result,
    SUBPIXEL_VARIANTS_X, ShapedGlyph, ShapedRun, SharedString, Size, TextRenderingMode, point, px,
    size, swap_rgba_pa_to_bgra,
};
use objc2_core_foundation::{
    CFData, CFIndex, CFMutableAttributedString, CFRange, CFRetained, CFString, CGFloat,
};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGDataProvider, CGFont, CGGlyph,
    CGImageAlphaInfo, CGTextDrawingMode,
};
use objc2_core_text::{
    CTFont, CTFontCollection, CTFontDescriptor, CTLine, CTRun, kCTFontAttributeName,
    kCTFontFamilyNameAttribute, kCTFontSlantTrait, kCTFontSymbolicTrait, kCTFontWeightTrait,
    kCTFontWidthTrait,
};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use smallvec::SmallVec;
use std::{borrow::Cow, ffi::c_void, ptr::NonNull, sync::Arc};
use swash::{
    CacheKey,
    scale::{Render, ScaleContext, Source, StrikeWith},
    zeno::{Format, Vector},
};

use super::open_type::apply_features_and_fallbacks;

/// A loaded font with its raw data, offsets, and cached CTFont for shaping.
struct LoadedFont {
    /// The raw font file data.
    data: Arc<Vec<u8>>,
    /// Byte offset into a font collection (0 for single fonts).
    offset: u32,
    /// Index of this font within a collection file (0 for single-font files).
    index: u32,
    /// Swash cache key for efficient scaler reuse.
    cache_key: CacheKey,
    /// CGFont created from the font data (used for CTFont creation).
    /// CGFont is thread-safe at the CoreGraphics level (immutable after creation).
    cg_font: CFRetained<CGFont>,
    /// Reference CTFont for system fonts without raw data (e.g. .SFNS variable instances).
    /// Stored so we can create sized copies via `copy_with_attributes` instead of going
    /// through CGFont (which loses variable font instance identity for system fonts).
    native_font: Option<CFRetained<CTFont>>,
}

/// SAFETY: CGFont is an immutable Core Graphics object created from font data.
/// Core Graphics font objects are safe to share across threads once created.
/// CTFont is likewise an immutable, thread-safe CoreText object.
unsafe impl Send for LoadedFont {}
/// SAFETY: CGFont and CTFont are immutable after creation - no interior mutability concerns.
unsafe impl Sync for LoadedFont {}

impl LoadedFont {
    /// Create a swash FontRef from this font's data.
    fn swash_font_ref(&self) -> swash::FontRef<'_> {
        swash::FontRef {
            data: &self.data,
            offset: self.offset,
            key: self.cache_key,
        }
    }

    /// Create a skrifa FontRef from this font's data.
    fn skrifa_font_ref(&self) -> Option<skrifa::FontRef<'_>> {
        skrifa::FontRef::from_index(&self.data, self.index).ok()
    }

    /// Create a CTFont at the given size.
    /// For system fonts with a stored native_font, uses `copy_with_attributes` to preserve
    /// the original font identity (variable font axes, system font resolution).
    /// For fonts with raw data, creates from CGFont.
    fn ct_font_at_size(&self, size: f64) -> CFRetained<CTFont> {
        if let Some(ref native) = self.native_font {
            unsafe { native.copy_with_attributes(size, std::ptr::null(), None) }
        } else {
            unsafe { CTFont::with_graphics_font(&self.cg_font, size, std::ptr::null(), None) }
        }
    }
}

/// macOS text system using CoreText for shaping, swash for rasterization,
/// and fontique + skrifa for font discovery and metrics.
pub struct MacTextSystem(RwLock<MacTextSystemState>);

#[derive(Clone, PartialEq, Eq, Hash)]
struct FontKey {
    font_family: SharedString,
    font_features: FontFeatures,
    font_fallbacks: Option<FontFallbacks>,
}

struct MacTextSystemState {
    /// Fontique collection for system + registered font discovery.
    collection: Collection,
    /// Fontique source cache for font data loading.
    source_cache: SourceCache,
    /// All loaded fonts, indexed by FontId.
    fonts: Vec<LoadedFont>,
    /// Cache: Font descriptor -> resolved FontId.
    font_selections: HashMap<Font, FontId>,
    /// PostScript name -> FontId index.
    font_ids_by_postscript_name: HashMap<String, FontId>,
    /// Family+features+fallbacks -> list of FontIds.
    font_ids_by_font_key: HashMap<FontKey, SmallVec<[FontId; 4]>>,
    /// FontId -> PostScript name.
    postscript_names_by_font_id: HashMap<FontId, String>,
    /// Swash scale context for glyph rasterization.
    scale_context: ScaleContext,
}

impl MacTextSystem {
    /// Create a new MacTextSystem.
    pub fn new() -> Self {
        Self(RwLock::new(MacTextSystemState {
            collection: Collection::new(CollectionOptions {
                shared: false,
                system_fonts: true,
            }),
            source_cache: SourceCache::default(),
            fonts: Vec::new(),
            font_selections: HashMap::default(),
            font_ids_by_postscript_name: HashMap::default(),
            font_ids_by_font_key: HashMap::default(),
            postscript_names_by_font_id: HashMap::default(),
            scale_context: ScaleContext::new(),
        }))
    }
}

impl Default for MacTextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTextSystem for MacTextSystem {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.0.write().add_fonts(fonts)
    }

    fn all_font_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        // Use CTFontCollection to enumerate system font families
        let collection = unsafe { CTFontCollection::from_available_fonts(None) };
        let descriptors = unsafe { collection.matching_font_descriptors() };
        if let Some(descriptors) = descriptors {
            let count = descriptors.count();
            for i in 0..count {
                let ptr = unsafe { descriptors.value_at_index(i) };
                if ptr.is_null() {
                    continue;
                }
                let desc_ref: &CTFontDescriptor =
                    unsafe { &*(ptr as *const CTFontDescriptor) };
                if let Some(family_name) =
                    unsafe { desc_ref.attribute(kCTFontFamilyNameAttribute) }
                {
                    if let Some(cf_str) = family_name.downcast_ref::<CFString>() {
                        names.push(cf_str.to_string());
                    }
                }
            }
        }
        // Also include registered (memory) font names from fontique
        let lock = self.0.write();
        // Note: we cannot add more names here without &mut, but the memory fonts
        // were already registered via add_fonts which uses CTFontCollection too.
        drop(lock);
        names
    }

    fn font_id(&self, font: &Font) -> Result<FontId> {
        let lock = self.0.upgradable_read();
        if let Some(font_id) = lock.font_selections.get(font) {
            Ok(*font_id)
        } else {
            let mut lock = RwLockUpgradableReadGuard::upgrade(lock);
            let font_key = FontKey {
                font_family: font.family.clone(),
                font_features: font.features.clone(),
                font_fallbacks: font.fallbacks.clone(),
            };
            let candidates = if let Some(font_ids) = lock.font_ids_by_font_key.get(&font_key) {
                font_ids.as_slice()
            } else {
                let font_ids =
                    lock.load_family(&font.family, &font.features, font.fallbacks.as_ref())?;
                lock.font_ids_by_font_key.insert(font_key.clone(), font_ids);
                lock.font_ids_by_font_key[&font_key].as_ref()
            };

            if candidates.is_empty() {
                anyhow::bail!("no fonts found for family {:?}", font.family);
            }

            // Match against candidates using swash attributes
            let target_weight = font.weight.0;
            let target_italic = matches!(font.style, FontStyle::Italic | FontStyle::Oblique);

            let mut best_ix = 0;
            let mut best_score = f32::MAX;

            for (ix, font_id) in candidates.iter().enumerate() {
                let loaded = &lock.fonts[font_id.0];
                let swash_ref = loaded.swash_font_ref();
                let attrs = swash_ref.attributes();
                let weight_diff = (attrs.weight().0 as f32 - target_weight).abs();
                let italic_diff = if target_italic {
                    if attrs.style() != swash::Style::Normal { 0.0 } else { 100.0 }
                } else {
                    if attrs.style() == swash::Style::Normal { 0.0 } else { 100.0 }
                };
                let score = weight_diff + italic_diff;
                if score < best_score {
                    best_score = score;
                    best_ix = ix;
                }
            }

            let font_id = candidates[best_ix];
            lock.font_selections.insert(font.clone(), font_id);
            Ok(font_id)
        }
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        let lock = self.0.read();
        let loaded = &lock.fonts[font_id.0];

        // System fonts without raw data (e.g. .SFNS) have empty data, so swash
        // returns all-zero metrics. Fall back to CTFont for these fonts.
        if loaded.data.is_empty() {
            let ct_font = loaded.ct_font_at_size(1000.0); // use 1000 upem for unscaled metrics
            let ascent = unsafe { ct_font.ascent() } as f32;
            let descent = unsafe { ct_font.descent() } as f32;
            let leading = unsafe { ct_font.leading() } as f32;
            let units_per_em = unsafe { ct_font.units_per_em() };
            let underline_pos = unsafe { ct_font.underline_position() } as f32;
            let underline_thickness = unsafe { ct_font.underline_thickness() } as f32;
            let cap_height = unsafe { ct_font.cap_height() } as f32;
            let x_height = unsafe { ct_font.x_height() } as f32;
            let ct_bbox = unsafe { ct_font.bounding_box() };

            // CTFont metrics at size 1000 — scale to units_per_em
            let scale = units_per_em as f32 / 1000.0;
            FontMetrics {
                units_per_em,
                ascent: ascent * scale,
                descent: -descent * scale, // CTFont descent is positive, FontMetrics expects negative
                line_gap: leading * scale,
                underline_position: underline_pos * scale,
                underline_thickness: underline_thickness * scale,
                cap_height: cap_height * scale,
                x_height: x_height * scale,
                bounding_box: Bounds {
                    origin: point(
                        ct_bbox.origin.x as f32 * scale,
                        ct_bbox.origin.y as f32 * scale,
                    ),
                    size: size(
                        ct_bbox.size.width as f32 * scale,
                        ct_bbox.size.height as f32 * scale,
                    ),
                },
            }
        } else {
            let swash_ref = loaded.swash_font_ref();
            let metrics = swash_ref.metrics(&[]);
            swash_metrics_to_font_metrics(&metrics)
        }
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        let lock = self.0.read();
        let loaded = &lock.fonts[font_id.0];
        if let Some(skrifa_ref) = loaded.skrifa_font_ref() {
            use skrifa::MetadataProvider;
            let glyph_metrics = skrifa_ref.glyph_metrics(
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::default(),
            );
            let skrifa_glyph_id = skrifa::GlyphId::new(glyph_id.0);
            if let Some(bounds) = glyph_metrics.bounds(skrifa_glyph_id) {
                return Ok(Bounds {
                    origin: point(bounds.x_min, bounds.y_min),
                    size: size(bounds.x_max - bounds.x_min, bounds.y_max - bounds.y_min),
                });
            }
        }
        // Fallback: use swash glyph metrics
        let swash_ref = loaded.swash_font_ref();
        let glyph_metrics = swash_ref.glyph_metrics(&[]);
        let advance = glyph_metrics.advance_width(glyph_id.0 as u16);
        Ok(Bounds {
            origin: point(0.0, 0.0),
            size: size(advance, 0.0),
        })
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        let lock = self.0.read();
        let loaded = &lock.fonts[font_id.0];
        if let Some(skrifa_ref) = loaded.skrifa_font_ref() {
            use skrifa::MetadataProvider;
            let glyph_metrics = skrifa_ref.glyph_metrics(
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::default(),
            );
            let skrifa_glyph_id = skrifa::GlyphId::new(glyph_id.0);
            if let Some(advance) = glyph_metrics.advance_width(skrifa_glyph_id) {
                return Ok(size(advance, 0.0));
            }
        }
        // Fallback to swash
        let swash_ref = loaded.swash_font_ref();
        let glyph_metrics = swash_ref.glyph_metrics(&[]);
        let advance = glyph_metrics.advance_width(glyph_id.0 as u16);
        Ok(size(advance, 0.0))
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        let lock = self.0.read();
        let loaded = &lock.fonts[font_id.0];
        let swash_ref = loaded.swash_font_ref();
        let glyph = swash_ref.charmap().map(ch);
        if glyph == 0 {
            None
        } else {
            Some(GlyphId(glyph as u32))
        }
    }

    fn is_emoji(&self, font_id: FontId) -> bool {
        self.0.read().is_emoji(font_id)
    }

    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        self.0.write().raster_bounds(params)
    }

    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        self.0.write().rasterize_glyph(params, raster_bounds)
    }

    fn layout_line(&self, text: &str, font_size: Pixels, font_runs: &[FontRun]) -> LineLayout {
        self.0.write().layout_line(text, font_size, font_runs)
    }

    fn recommended_rendering_mode(
        &self,
        _font_id: FontId,
        _font_size: Pixels,
    ) -> TextRenderingMode {
        TextRenderingMode::Grayscale
    }
}

impl MacTextSystemState {
    fn add_fonts(&mut self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        for bytes in fonts {
            let data: Vec<u8> = bytes.into_owned();
            let arc_data = Arc::new(data);

            // Register with fontique collection
            let blob = fontique::Blob::from(arc_data.as_ref().clone());
            self.collection.register_fonts(blob, None);

            // Also load each font face directly
            if let Some(font_data) = swash::FontDataRef::new(&arc_data) {
                for i in 0..font_data.len() {
                    if let Some(swash_ref) = font_data.get(i) {
                        self.register_font_from_data(arc_data.clone(), swash_ref.offset, i as u32, swash_ref.key)?;
                    }
                }
            } else {
                return Err(anyhow!("Could not load an embedded font."));
            }
        }
        Ok(())
    }

    /// Register a single font from raw data, creating the necessary CGFont and CTFont.
    fn register_font_from_data(
        &mut self,
        data: Arc<Vec<u8>>,
        offset: u32,
        index: u32,
        cache_key: CacheKey,
    ) -> Result<FontId> {
        // Create CGFont from data
        let cf_data = CFData::from_bytes(&data);
        let provider = CGDataProvider::with_cf_data(Some(&cf_data))
            .ok_or_else(|| anyhow!("Failed to create CGDataProvider for font"))?;
        let cg_font = CGFont::with_data_provider(&provider)
            .ok_or_else(|| anyhow!("Failed to create CGFont from data"))?;

        // Get postscript name
        let postscript_name = CGFont::post_script_name(Some(&cg_font))
            .map(|s| s.to_string())
            .unwrap_or_default();

        if postscript_name.is_empty() {
            return Err(anyhow!("Font has no postscript name"));
        }

        // Check if already registered
        if let Some(&existing_id) = self.font_ids_by_postscript_name.get(&postscript_name) {
            return Ok(existing_id);
        }

        let font_id = FontId(self.fonts.len());
        self.font_ids_by_postscript_name
            .insert(postscript_name.clone(), font_id);
        self.postscript_names_by_font_id
            .insert(font_id, postscript_name.clone());
        self.fonts.push(LoadedFont {
            data,
            offset,
            index,
            cache_key,
            cg_font,
            native_font: None,
        });
        Ok(font_id)
    }

    fn load_family(
        &mut self,
        name: &str,
        features: &FontFeatures,
        fallbacks: Option<&FontFallbacks>,
    ) -> Result<SmallVec<[FontId; 4]>> {
        let name = inazuma::font_name_with_fallbacks(name, ".AppleSystemUIFont");

        let mut font_ids = SmallVec::new();

        // Try fontique collection query first
        let mut found_via_fontique = false;
        // Collect matched fonts first, then release the query borrow before registering
        let matched_fonts: Vec<QueryFont> = {
            let mut query = self.collection.query(&mut self.source_cache);
            query.set_families([QueryFamily::Named(name)]);

            let mut fonts: Vec<QueryFont> = Vec::new();
            query.matches_with(|font| {
                fonts.push(font.clone());
                QueryStatus::Continue
            });
            fonts
        };

        for qfont in matched_fonts {
            let data = Arc::new(qfont.blob.as_ref().to_vec());
            if let Some(font_data) = swash::FontDataRef::new(&data) {
                if let Some(swash_ref) = font_data.get(qfont.index as usize) {
                    match self.register_font_with_features(
                        data.clone(),
                        swash_ref.offset,
                        qfont.index,
                        swash_ref.key,
                        features,
                        fallbacks,
                    ) {
                        Ok(font_id) => {
                            font_ids.push(font_id);
                            found_via_fontique = true;
                        }
                        Err(e) => {
                            log::warn!("Failed to register font from fontique: {}", e);
                        }
                    }
                }
            }
        }

        // Fallback: Try finding via CTFont name lookup
        if !found_via_fontique {
            let cf_name = CFString::from_str(name);
            let ct_font = unsafe {
                CTFont::with_name(&cf_name, 12.0, std::ptr::null())
            };
            // Get the font's CGFont
            let cg_font = unsafe { ct_font.graphics_font(std::ptr::null_mut()) };

            // Get the font data via table serialization
            if let Some(data) = self.extract_font_data_from_ct_font(&ct_font) {
                let arc_data = Arc::new(data);
                if let Some(font_data) = swash::FontDataRef::new(&arc_data) {
                    for i in 0..font_data.len() {
                        if let Some(swash_ref) = font_data.get(i) {
                            match self.register_font_with_features(
                                arc_data.clone(),
                                swash_ref.offset,
                                i as u32,
                                swash_ref.key,
                                features,
                                fallbacks,
                            ) {
                                Ok(font_id) => {
                                    font_ids.push(font_id);
                                }
                                Err(e) => {
                                    log::warn!("Failed to register font: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Last resort: register from the CGFont directly
            if font_ids.is_empty() {
                let postscript_name = CGFont::post_script_name(Some(&cg_font))
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                if !postscript_name.is_empty() {
                    if let Some(&existing) = self.font_ids_by_postscript_name.get(&postscript_name) {
                        font_ids.push(existing);
                    } else {
                        // Use url-based loading approach: get font URL from descriptor
                        let desc = unsafe { ct_font.font_descriptor() };
                        if let Some(url_attr) = unsafe { desc.attribute(objc2_core_text::kCTFontURLAttribute) } {
                            use objc2_core_foundation::CFURL;
                            if let Some(url) = url_attr.downcast_ref::<CFURL>() {
                                let path_str = url.path();
                                if let Some(path_cf) = path_str {
                                    let path = path_cf.to_string();
                                    if let Ok(data) = std::fs::read(&path) {
                                        let arc_data = Arc::new(data);
                                        if let Some(font_data) = swash::FontDataRef::new(&arc_data) {
                                            for i in 0..font_data.len() {
                                                if let Some(swash_ref) = font_data.get(i) {
                                                    // Check if this face matches our postscript name
                                                    let face_name = swash_ref
                                                        .localized_strings()
                                                        .find_by_id(swash::StringId::PostScript, None)
                                                        .map(|s| s.to_string());
                                                    if face_name.as_deref() == Some(&postscript_name) || font_data.len() == 1 {
                                                        match self.register_font_with_features(
                                                            arc_data.clone(),
                                                            swash_ref.offset,
                                                            i as u32,
                                                            swash_ref.key,
                                                            features,
                                                            fallbacks,
                                                        ) {
                                                            Ok(font_id) => {
                                                                font_ids.push(font_id);
                                                            }
                                                            Err(e) => {
                                                                log::warn!("Failed to register font from file: {}", e);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if font_ids.is_empty() {
            anyhow::bail!("Could not find font family '{}'", name);
        }

        Ok(font_ids)
    }

    /// Register a font with features and fallbacks applied via CoreText.
    fn register_font_with_features(
        &mut self,
        data: Arc<Vec<u8>>,
        offset: u32,
        index: u32,
        cache_key: CacheKey,
        features: &FontFeatures,
        fallbacks: Option<&FontFallbacks>,
    ) -> Result<FontId> {
        // Create CGFont from data
        let cf_data = CFData::from_bytes(&data);
        let provider = CGDataProvider::with_cf_data(Some(&cf_data))
            .ok_or_else(|| anyhow!("Failed to create CGDataProvider"))?;
        let cg_font = CGFont::with_data_provider(&provider)
            .ok_or_else(|| anyhow!("Failed to create CGFont"))?;

        // Create a CTFont to apply features and fallbacks
        let ct_font = unsafe {
            CTFont::with_graphics_font(&cg_font, 12.0, std::ptr::null(), None)
        };

        // Apply features and fallbacks
        let ct_font = apply_features_and_fallbacks(&ct_font, features, fallbacks)?;

        // Get the resulting CGFont
        let result_cg_font = unsafe { ct_font.graphics_font(std::ptr::null_mut()) };

        // Get postscript name
        let postscript_name = unsafe { ct_font.post_script_name() }.to_string();

        if postscript_name.is_empty() {
            return Err(anyhow!("Font has no postscript name"));
        }

        // Validate: Check that font has an 'm' glyph (required for text measurement)
        let swash_ref = swash::FontRef {
            data: &data,
            offset,
            key: cache_key,
        };
        let has_m_glyph = swash_ref.charmap().map('m') != 0;
        let is_segoe_fluent_icons = postscript_name == "SegoeFluentIcons";
        if !has_m_glyph && !is_segoe_fluent_icons {
            log::warn!(
                "font '{}' has no 'm' character and was not loaded",
                postscript_name
            );
            return Err(anyhow!(
                "font '{}' has no 'm' character",
                postscript_name
            ));
        }

        // Validate traits
        let traits = unsafe { ct_font.traits() };
        let has_valid_traits = unsafe {
            let symbolic = traits.value(kCTFontSymbolicTrait as *const _ as *const _);
            let width = traits.value(kCTFontWidthTrait as *const _ as *const _);
            let weight = traits.value(kCTFontWeightTrait as *const _ as *const _);
            let slant = traits.value(kCTFontSlantTrait as *const _ as *const _);
            !symbolic.is_null() && !width.is_null() && !weight.is_null() && !slant.is_null()
        };

        if !has_valid_traits {
            log::error!("Failed to read traits for font {:?}", postscript_name);
            return Err(anyhow!(
                "Failed to read traits for font '{}'",
                postscript_name
            ));
        }

        // Check if already registered
        if let Some(&existing_id) = self.font_ids_by_postscript_name.get(&postscript_name) {
            return Ok(existing_id);
        }

        let font_id = FontId(self.fonts.len());
        self.font_ids_by_postscript_name
            .insert(postscript_name.clone(), font_id);
        self.postscript_names_by_font_id
            .insert(font_id, postscript_name.clone());
        self.fonts.push(LoadedFont {
            data,
            offset,
            index,
            cache_key,
            cg_font: result_cg_font,
            native_font: None,
        });
        Ok(font_id)
    }

    /// Try to extract font data from a CTFont by reading its file from disk.
    fn extract_font_data_from_ct_font(&self, ct_font: &CTFont) -> Option<Vec<u8>> {
        let desc = unsafe { ct_font.font_descriptor() };
        let url_attr = unsafe { desc.attribute(objc2_core_text::kCTFontURLAttribute) }?;
        use objc2_core_foundation::CFURL;
        let url = url_attr.downcast_ref::<CFURL>()?;
        let path_cf = url.path()?;
        let path = path_cf.to_string();
        std::fs::read(&path).ok()
    }

    fn id_for_native_font(&mut self, requested_font: &CTFont) -> FontId {
        let postscript_name = unsafe { requested_font.post_script_name() }.to_string();
        if let Some(font_id) = self.font_ids_by_postscript_name.get(&postscript_name) {
            *font_id
        } else {
            // Try to load the font from its file
            if let Some(data) = self.extract_font_data_from_ct_font(requested_font) {
                let arc_data = Arc::new(data);
                // Collect offset/index/key from swash without holding a borrow on arc_data
                let face_info: Option<(u32, u32, CacheKey)> = {
                    if let Some(font_data) = swash::FontDataRef::new(&arc_data) {
                        let mut result = None;
                        for i in 0..font_data.len() {
                            if let Some(swash_ref) = font_data.get(i) {
                                let face_name = swash_ref
                                    .localized_strings()
                                    .find_by_id(swash::StringId::PostScript, None)
                                    .map(|s| s.to_string());
                                if face_name.as_deref() == Some(&postscript_name) || font_data.len() == 1 {
                                    result = Some((swash_ref.offset, i as u32, swash_ref.key));
                                    break;
                                }
                            } else {
                            }
                        }
                        result
                    } else {
                        None
                    }
                };
                if let Some((offset, index, cache_key)) = face_info {
                    let cg_font = unsafe { requested_font.graphics_font(std::ptr::null_mut()) };
                    let font_id = FontId(self.fonts.len());
                    self.font_ids_by_postscript_name
                        .insert(postscript_name.clone(), font_id);
                    self.postscript_names_by_font_id
                        .insert(font_id, postscript_name);
                    self.fonts.push(LoadedFont {
                        data: arc_data,
                        offset,
                        index,
                        cache_key,
                        cg_font,
                        native_font: None,
                    });
                    return font_id;
                }
            }

            // Absolute fallback: store the original CTFont so we can create sized copies
            // via copy_with_attributes, preserving the system font's identity and variable
            // font instance axes. Going through CGFont loses this for .SFNS instances.
            let cg_font = unsafe { requested_font.graphics_font(std::ptr::null_mut()) };
            let native_ct_font = unsafe {
                requested_font.copy_with_attributes(0.0, std::ptr::null(), None)
            };

            let font_id = FontId(self.fonts.len());
            self.font_ids_by_postscript_name
                .insert(postscript_name.clone(), font_id);
            self.postscript_names_by_font_id
                .insert(font_id, postscript_name.clone());

            // Try to get data via file URL
            let data = self.extract_font_data_from_ct_font(requested_font)
                .unwrap_or_default();
            let arc_data = Arc::new(data);
            let (offset, cache_key) = if let Some(font_data) = swash::FontDataRef::new(&arc_data) {
                if let Some(swash_ref) = font_data.get(0) {
                    (swash_ref.offset, swash_ref.key)
                } else {
                    (0, CacheKey::new())
                }
            } else {
                (0, CacheKey::new())
            };

            self.fonts.push(LoadedFont {
                data: arc_data,
                offset,
                index: 0,
                cache_key,
                cg_font,
                native_font: Some(native_ct_font),
            });
            font_id
        }
    }

    fn is_emoji(&self, font_id: FontId) -> bool {
        self.postscript_names_by_font_id
            .get(&font_id)
            .is_some_and(|postscript_name| {
                postscript_name == "AppleColorEmoji" || postscript_name == ".AppleColorEmojiUI"
            })
    }

    fn raster_bounds(&mut self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        let loaded = &self.fonts[params.font_id.0];

        if params.is_emoji || loaded.data.is_empty() {
            // For emoji and system fonts without accessible raw data (e.g. .SFNS),
            // use CoreText-based rasterization via CGBitmapContext + CTFontDrawGlyphs
            return self.emoji_raster_bounds(params);
        }

        // Use swash to compute raster bounds
        let swash_ref = loaded.swash_font_ref();
        let ppem = f32::from(params.font_size) * params.scale_factor;
        let mut scaler = self.scale_context.builder(swash_ref)
            .size(ppem)
            .build();

        let subpixel_shift = params
            .subpixel_variant
            .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);

        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(Vector::new(
            subpixel_shift.x / params.scale_factor,
            subpixel_shift.y / params.scale_factor,
        ))
        .render(&mut scaler, params.glyph_id.0 as u16);

        if let Some(image) = image {
            let p = image.placement;
            Ok(Bounds {
                origin: point(DevicePixels(p.left), DevicePixels(-p.top)),
                size: size(DevicePixels(p.width as i32), DevicePixels(p.height as i32)),
            })
        } else {
            Ok(Bounds {
                origin: point(DevicePixels(0), DevicePixels(0)),
                size: size(DevicePixels(0), DevicePixels(0)),
            })
        }
    }

    fn emoji_raster_bounds(&mut self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        let loaded = &self.fonts[params.font_id.0];
        let ct_font = loaded.ct_font_at_size(f64::from(f32::from(params.font_size)));

        // Use CoreText to get the bounding rect for the glyph
        let glyph = params.glyph_id.0 as CGGlyph;
        let mut bounding_rect = unsafe {
            ct_font.bounding_rects_for_glyphs(
                objc2_core_text::CTFontOrientation::Default,
                NonNull::from(&glyph),
                std::ptr::null_mut(),
                1,
            )
        };

        let scale = params.scale_factor as f64;
        let x = (bounding_rect.origin.x * scale).floor() as i32;
        let y = (bounding_rect.origin.y * scale).floor() as i32;
        let width = ((bounding_rect.origin.x + bounding_rect.size.width) * scale).ceil() as i32 - x;
        let height = ((bounding_rect.origin.y + bounding_rect.size.height) * scale).ceil() as i32 - y;

        // Add padding
        let pad = ((params.font_size.as_f32() * 0.03 * params.scale_factor).ceil() as i32).clamp(1, 5);

        Ok(Bounds {
            origin: point(DevicePixels(x - pad), DevicePixels(-y - height)),
            size: size(DevicePixels(width + pad), DevicePixels(height)),
        })
    }

    fn rasterize_glyph(
        &mut self,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        if glyph_bounds.size.width.0 == 0 || glyph_bounds.size.height.0 == 0 {
            anyhow::bail!("glyph bounds are empty");
        }

        // Add an extra pixel when the subpixel variant isn't zero.
        let mut bitmap_size = glyph_bounds.size;
        if params.subpixel_variant.x > 0 {
            bitmap_size.width += DevicePixels(1);
        }
        if params.subpixel_variant.y > 0 {
            bitmap_size.height += DevicePixels(1);
        }
        let bitmap_size = bitmap_size;

        if self.fonts[params.font_id.0].data.is_empty() {
            // System fonts without raw data: use CoreText grayscale rasterization
            return self.rasterize_coretext_grayscale(params, glyph_bounds, bitmap_size);
        }
        if params.is_emoji {
            return self.rasterize_emoji(params, glyph_bounds, bitmap_size);
        }

        // Use swash for non-emoji glyph rasterization
        let loaded = &self.fonts[params.font_id.0];
        let swash_ref = loaded.swash_font_ref();
        let ppem = f32::from(params.font_size) * params.scale_factor;
        let mut scaler = self.scale_context.builder(swash_ref)
            .size(ppem)
            .build();

        let subpixel_shift = params
            .subpixel_variant
            .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);

        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(Vector::new(
            subpixel_shift.x / params.scale_factor,
            subpixel_shift.y / params.scale_factor,
        ))
        .render(&mut scaler, params.glyph_id.0 as u16);

        if let Some(image) = image {
            let p = image.placement;
            let w = p.width as i32;
            let h = p.height as i32;
            log::debug!("swash rasterized glyph {}: {}x{}, {} bytes", params.glyph_id.0, w, h, image.data.len());
            Ok((size(DevicePixels(w), DevicePixels(h)), image.data))
        } else {
            log::warn!("swash returned None for glyph {} (font_id={}, ppem={}, font_data_len={})",
                params.glyph_id.0, params.font_id.0, ppem,
                self.fonts[params.font_id.0].data.len());
            // Return empty bitmap
            let bytes = vec![0u8; bitmap_size.width.0 as usize * bitmap_size.height.0 as usize];
            Ok((bitmap_size, bytes))
        }
    }

    fn rasterize_coretext_grayscale(
        &self,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
        bitmap_size: Size<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        let loaded = &self.fonts[params.font_id.0];

        let mut bytes = vec![0u8; bitmap_size.width.0 as usize * bitmap_size.height.0 as usize];
        let color_space = CGColorSpace::new_device_gray()
            .ok_or_else(|| anyhow!("Failed to create gray color space"))?;
        let cx = unsafe {
            CGBitmapContextCreate(
                bytes.as_mut_ptr() as *mut c_void,
                bitmap_size.width.0 as usize,
                bitmap_size.height.0 as usize,
                8,
                bitmap_size.width.0 as usize,
                Some(&color_space),
                CGImageAlphaInfo::Only.0,
            )
        }
        .ok_or_else(|| anyhow!("Failed to create grayscale bitmap context"))?;

        CGContext::translate_ctm(
            Some(&cx),
            -glyph_bounds.origin.x.0 as CGFloat,
            (glyph_bounds.origin.y.0 + glyph_bounds.size.height.0) as CGFloat,
        );
        CGContext::scale_ctm(
            Some(&cx),
            params.scale_factor as CGFloat,
            params.scale_factor as CGFloat,
        );

        let subpixel_shift = params
            .subpixel_variant
            .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);

        CGContext::set_text_drawing_mode(Some(&cx), CGTextDrawingMode::Fill);
        CGContext::set_allows_antialiasing(Some(&cx), true);
        CGContext::set_should_antialias(Some(&cx), true);
        CGContext::set_allows_font_subpixel_positioning(Some(&cx), true);
        CGContext::set_should_subpixel_position_fonts(Some(&cx), true);

        // In an alpha-only context, the gray fill color value doesn't matter —
        // what matters is the alpha value (second param). Set to match the old font-kit behavior.
        CGContext::set_gray_fill_color(Some(&cx), 0.0, 1.0);

        let ct_font = loaded.ct_font_at_size(f64::from(f32::from(params.font_size)));
        let mut glyph = params.glyph_id.0 as CGGlyph;
        let position = objc2_core_foundation::CGPoint::new(
            (subpixel_shift.x / params.scale_factor) as f64,
            (subpixel_shift.y / params.scale_factor) as f64,
        );
        unsafe {
            ct_font.draw_glyphs(
                NonNull::from(&mut glyph),
                NonNull::from(&position),
                1,
                &cx,
            );
        }

        Ok((bitmap_size, bytes))
    }

    fn rasterize_emoji(
        &self,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
        bitmap_size: Size<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        let loaded = &self.fonts[params.font_id.0];

        let mut bytes = vec![0u8; bitmap_size.width.0 as usize * 4 * bitmap_size.height.0 as usize];
        let color_space = CGColorSpace::new_device_rgb()
            .ok_or_else(|| anyhow!("Failed to create RGB color space"))?;
        let cx = unsafe {
            CGBitmapContextCreate(
                bytes.as_mut_ptr() as *mut c_void,
                bitmap_size.width.0 as usize,
                bitmap_size.height.0 as usize,
                8,
                bitmap_size.width.0 as usize * 4,
                Some(&color_space),
                CGImageAlphaInfo::PremultipliedLast.0,
            )
        }
        .ok_or_else(|| anyhow!("Failed to create bitmap context for emoji"))?;

        // Move origin to bottom left and account for scaling
        CGContext::translate_ctm(
            Some(&cx),
            -glyph_bounds.origin.x.0 as CGFloat,
            (glyph_bounds.origin.y.0 + glyph_bounds.size.height.0) as CGFloat,
        );
        CGContext::scale_ctm(
            Some(&cx),
            params.scale_factor as CGFloat,
            params.scale_factor as CGFloat,
        );

        let subpixel_shift = params
            .subpixel_variant
            .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);

        CGContext::set_text_drawing_mode(Some(&cx), CGTextDrawingMode::Fill);
        CGContext::set_allows_antialiasing(Some(&cx), true);
        CGContext::set_should_antialias(Some(&cx), true);
        CGContext::set_allows_font_subpixel_positioning(Some(&cx), true);
        CGContext::set_should_subpixel_position_fonts(Some(&cx), true);
        CGContext::set_allows_font_subpixel_quantization(Some(&cx), false);
        CGContext::set_should_subpixel_quantize_fonts(Some(&cx), false);

        // Create CTFont at the right size for drawing
        let ct_font = loaded.ct_font_at_size(f64::from(f32::from(params.font_size)));

        let mut glyph = params.glyph_id.0 as CGGlyph;
        let position = objc2_core_foundation::CGPoint::new(
            (subpixel_shift.x / params.scale_factor) as f64,
            (subpixel_shift.y / params.scale_factor) as f64,
        );
        unsafe {
            ct_font.draw_glyphs(
                NonNull::from(&mut glyph),
                NonNull::from(&position),
                1,
                &cx,
            );
        }

        // Convert from RGBA with premultiplied alpha to BGRA with straight alpha.
        for pixel in bytes.chunks_exact_mut(4) {
            swap_rgba_pa_to_bgra(pixel);
        }

        Ok((bitmap_size, bytes))
    }

    fn layout_line(&mut self, text: &str, font_size: Pixels, font_runs: &[FontRun]) -> LineLayout {
        // Construct the attributed string, converting UTF8 ranges to UTF16 ranges.
        let mut string =
            CFMutableAttributedString::new(None, 0).expect("failed to create attributed string");
        let mut max_ascent = 0.0f32;
        let mut max_descent = 0.0f32;

        {
            let mut text = text;
            let mut break_ligature = true;
            for run in font_runs {
                let text_run;
                (text_run, text) = text.split_at(run.len);

                let utf16_start = string.length();
                let cf_text_run = CFString::from_str(text_run);
                unsafe {
                    CFMutableAttributedString::replace_string(
                        Some(&string),
                        CFRange::new(utf16_start, 0),
                        Some(&cf_text_run),
                    );
                }
                let utf16_end = string.length();

                let length = utf16_end - utf16_start;
                let cf_range = CFRange::new(utf16_start, length);
                let loaded = &self.fonts[run.font_id.0];

                // Get metrics: use CTFont for system fonts with empty data (swash returns zeros),
                // otherwise use swash metrics directly.
                if loaded.data.is_empty() {
                    let ct_font = loaded.ct_font_at_size(f64::from(f32::from(font_size)));
                    max_ascent = max_ascent.max(unsafe { ct_font.ascent() } as f32);
                    max_descent = max_descent.max(unsafe { ct_font.descent() } as f32);
                } else {
                    let swash_ref = loaded.swash_font_ref();
                    let metrics = swash_ref.metrics(&[]);
                    let font_scale = f32::from(font_size) / metrics.units_per_em as f32;
                    max_ascent = max_ascent.max(metrics.ascent * font_scale);
                    max_descent = max_descent.max(-metrics.descent * font_scale);
                }

                let font_size_for_run = if break_ligature {
                    px(f32::from(font_size).next_up())
                } else {
                    font_size
                };

                // Create sized CTFont from stored CGFont
                let sized_font = loaded.ct_font_at_size(f64::from(f32::from(font_size_for_run)));

                unsafe {
                    CFMutableAttributedString::set_attribute(
                        Some(&string),
                        cf_range,
                        Some(kCTFontAttributeName),
                        Some(&*sized_font),
                    );
                }
                break_ligature = !break_ligature;
            }
        }

        // Retrieve the glyphs from the shaped line
        let line = unsafe { CTLine::with_attributed_string(&string) };
        let glyph_runs = unsafe { line.glyph_runs() };
        let run_count = glyph_runs.count();
        let mut runs = <Vec<ShapedRun>>::with_capacity(run_count as usize);
        let mut ix_converter = StringIndexConverter::new(text);

        for i in 0..run_count {
            let run_ptr = unsafe { glyph_runs.value_at_index(i) };
            if run_ptr.is_null() {
                continue;
            }
            let run: &CTRun = unsafe { &*(run_ptr as *const CTRun) };

            let attributes = unsafe { run.attributes() };
            let font_value = unsafe {
                attributes.value(kCTFontAttributeName as *const _ as *const _)
            };
            if font_value.is_null() {
                continue;
            }
            let run_font: &CTFont = unsafe { &*(font_value as *const CTFont) };
            let font_id = self.id_for_native_font(run_font);

            let glyph_count = unsafe { run.glyph_count() };
            if glyph_count <= 0 {
                continue;
            }
            let glyph_count_usize = glyph_count as usize;

            let glyphs_ptr = unsafe { run.glyphs_ptr() };
            let positions_ptr = unsafe { run.positions_ptr() };
            let indices_ptr = unsafe { run.string_indices_ptr() };

            let mut glyphs_buf: Vec<CGGlyph> = Vec::new();
            let mut positions_buf: Vec<objc2_core_foundation::CGPoint> = Vec::new();
            let mut indices_buf: Vec<CFIndex> = Vec::new();

            let glyphs_slice: &[CGGlyph];
            let positions_slice: &[objc2_core_foundation::CGPoint];
            let indices_slice: &[CFIndex];

            if !glyphs_ptr.is_null() {
                glyphs_slice = unsafe { std::slice::from_raw_parts(glyphs_ptr, glyph_count_usize) };
            } else {
                glyphs_buf.resize(glyph_count_usize, 0);
                unsafe {
                    run.glyphs(
                        CFRange::new(0, glyph_count),
                        NonNull::new_unchecked(glyphs_buf.as_mut_ptr()),
                    );
                }
                glyphs_slice = &glyphs_buf;
            }

            if !positions_ptr.is_null() {
                positions_slice = unsafe {
                    std::slice::from_raw_parts(positions_ptr, glyph_count_usize)
                };
            } else {
                positions_buf.resize(
                    glyph_count_usize,
                    objc2_core_foundation::CGPoint::new(0.0, 0.0),
                );
                unsafe {
                    run.positions(
                        CFRange::new(0, glyph_count),
                        NonNull::new_unchecked(positions_buf.as_mut_ptr()),
                    );
                }
                positions_slice = &positions_buf;
            }

            if !indices_ptr.is_null() {
                indices_slice =
                    unsafe { std::slice::from_raw_parts(indices_ptr, glyph_count_usize) };
            } else {
                indices_buf.resize(glyph_count_usize, 0);
                unsafe {
                    run.string_indices(
                        CFRange::new(0, glyph_count),
                        NonNull::new_unchecked(indices_buf.as_mut_ptr()),
                    );
                }
                indices_slice = &indices_buf;
            }

            let glyphs = match runs.last_mut() {
                Some(run) if run.font_id == font_id => &mut run.glyphs,
                _ => {
                    runs.push(ShapedRun {
                        font_id,
                        glyphs: Vec::with_capacity(glyph_count_usize),
                    });
                    &mut runs.last_mut().unwrap().glyphs
                }
            };

            for ((&glyph_id, position), &glyph_utf16_ix) in glyphs_slice
                .iter()
                .zip(positions_slice.iter())
                .zip(indices_slice.iter())
            {
                let glyph_utf16_ix = glyph_utf16_ix as usize;
                if ix_converter.utf16_ix > glyph_utf16_ix {
                    ix_converter = StringIndexConverter::new(text);
                }
                ix_converter.advance_to_utf16_ix(glyph_utf16_ix);
                glyphs.push(ShapedGlyph {
                    id: GlyphId(glyph_id as u32),
                    position: point(position.x as f32, position.y as f32).map(px),
                    index: ix_converter.utf8_ix,
                    is_emoji: self.is_emoji(font_id),
                });
            }
        }

        let mut ascent: CGFloat = 0.0;
        let mut descent: CGFloat = 0.0;
        let mut leading: CGFloat = 0.0;
        let width =
            unsafe { line.typographic_bounds(&mut ascent, &mut descent, &mut leading) };
        LineLayout {
            runs,
            font_size,
            width: (width as f32).into(),
            ascent: max_ascent.into(),
            descent: max_descent.into(),
            len: text.len(),
        }
    }
}

fn swash_metrics_to_font_metrics(metrics: &swash::Metrics) -> FontMetrics {
    FontMetrics {
        units_per_em: metrics.units_per_em as u32,
        ascent: metrics.ascent,
        descent: metrics.descent,
        line_gap: metrics.leading,
        underline_position: metrics.underline_offset,
        underline_thickness: metrics.stroke_size,
        cap_height: metrics.cap_height,
        x_height: metrics.x_height,
        bounding_box: Bounds {
            origin: point(0.0, metrics.descent),
            size: size(metrics.max_width, metrics.ascent - metrics.descent),
        },
    }
}

#[derive(Debug, Clone)]
struct StringIndexConverter<'a> {
    text: &'a str,
    utf8_ix: usize,
    utf16_ix: usize,
}

impl<'a> StringIndexConverter<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            utf8_ix: 0,
            utf16_ix: 0,
        }
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

#[cfg(test)]
mod tests {
    use super::MacTextSystem;
    use inazuma::{FontRun, GlyphId, PlatformTextSystem, font, px};

    #[test]
    fn test_layout_line_bom_char() {
        let fonts = MacTextSystem::new();
        let font_id = fonts.font_id(&font("Helvetica")).unwrap();
        let line = "\u{feff}";
        let mut style = FontRun {
            font_id,
            len: line.len(),
        };

        let layout = fonts.layout_line(line, px(16.), &[style]);
        assert_eq!(layout.len, line.len());
        assert!(layout.runs.is_empty());

        let line = "a\u{feff}b";
        style.len = line.len();
        let layout = fonts.layout_line(line, px(16.), &[style]);
        assert_eq!(layout.len, line.len());
        assert_eq!(layout.runs.len(), 1);
        assert_eq!(layout.runs[0].glyphs.len(), 2);
        assert_eq!(layout.runs[0].glyphs[0].id, GlyphId(68u32)); // a
        assert_eq!(layout.runs[0].glyphs[1].id, GlyphId(69u32)); // b

        let line = "\u{feff}ab";
        let font_runs = &[
            FontRun {
                len: "\u{feff}".len(),
                font_id,
            },
            FontRun {
                len: "ab".len(),
                font_id,
            },
        ];
        let layout = fonts.layout_line(line, px(16.), font_runs);
        assert_eq!(layout.len, line.len());
        assert_eq!(layout.runs.len(), 1);
        assert_eq!(layout.runs[0].glyphs.len(), 2);
        assert_eq!(layout.runs[0].glyphs[0].id, GlyphId(68u32)); // a
        assert_eq!(layout.runs[0].glyphs[1].id, GlyphId(69u32)); // b
    }

    #[test]
    fn test_layout_line_zwnj_insertion() {
        let fonts = MacTextSystem::new();
        let font_id = fonts.font_id(&font("Helvetica")).unwrap();

        let text = "hello world";
        let font_runs = &[
            FontRun { font_id, len: 5 },
            FontRun { font_id, len: 6 },
        ];

        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, text.len());

        for run in &layout.runs {
            for glyph in &run.glyphs {
                assert!(
                    glyph.index < text.len(),
                    "Glyph index {} is out of bounds for text length {}",
                    glyph.index,
                    text.len()
                );
            }
        }

        let font_id2 = fonts.font_id(&font("Times")).unwrap_or(font_id);
        let font_runs_different = &[
            FontRun { font_id, len: 5 },
            FontRun {
                font_id: font_id2,
                len: 6,
            },
        ];

        let layout2 = fonts.layout_line(text, px(16.), font_runs_different);
        assert_eq!(layout2.len, text.len());

        for run in &layout2.runs {
            for glyph in &run.glyphs {
                assert!(
                    glyph.index < text.len(),
                    "Glyph index {} is out of bounds for text length {}",
                    glyph.index,
                    text.len()
                );
            }
        }
    }

    #[test]
    fn test_layout_line_zwnj_edge_cases() {
        let fonts = MacTextSystem::new();
        let font_id = fonts.font_id(&font("Helvetica")).unwrap();

        let text = "hello";
        let font_runs = &[FontRun { font_id, len: 5 }];
        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, text.len());

        let text = "abc";
        let font_runs = &[
            FontRun { font_id, len: 1 },
            FontRun { font_id, len: 1 },
            FontRun { font_id, len: 1 },
        ];
        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, text.len());

        for run in &layout.runs {
            for glyph in &run.glyphs {
                assert!(
                    glyph.index < text.len(),
                    "Glyph index {} is out of bounds for text length {}",
                    glyph.index,
                    text.len()
                );
            }
        }

        let text = "";
        let font_runs = &[];
        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, 0);
        assert!(layout.runs.is_empty());
    }
}
