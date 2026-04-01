use super::*;

pub(super) struct FontInfo {
    pub(super) font_family_h: HSTRING,
    pub(super) font_face: IDWriteFontFace3,
    pub(super) features: IDWriteTypography,
    pub(super) fallbacks: Option<IDWriteFontFallback>,
    pub(super) font_collection: IDWriteFontCollection1,
}

pub(crate) struct DirectWriteTextSystem {
    pub(super) components: DirectWriteComponents,
    pub(super) state: RwLock<DirectWriteState>,
}

pub(super) struct DirectWriteComponents {
    pub(super) locale: HSTRING,
    pub(super) factory: IDWriteFactory5,
    pub(super) in_memory_loader: IDWriteInMemoryFontFileLoader,
    pub(super) builder: IDWriteFontSetBuilder1,
    pub(super) text_renderer: TextRendererWrapper,
    pub(super) system_ui_font_name: SharedString,
    pub(super) system_subpixel_rendering: bool,
}

impl Drop for DirectWriteComponents {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .factory
                .UnregisterFontFileLoader(&self.in_memory_loader);
        }
    }
}

pub(super) struct GPUState {
    pub(super) device: ID3D11Device,
    pub(super) device_context: ID3D11DeviceContext,
    pub(super) sampler: Option<ID3D11SamplerState>,
    pub(super) blend_state: ID3D11BlendState,
    pub(super) vertex_shader: ID3D11VertexShader,
    pub(super) pixel_shader: ID3D11PixelShader,
}

pub(super) struct DirectWriteState {
    pub(super) gpu_state: GPUState,
    pub(super) system_font_collection: IDWriteFontCollection1,
    pub(super) custom_font_collection: IDWriteFontCollection1,
    pub(super) fonts: Vec<FontInfo>,
    pub(super) font_to_font_id: HashMap<Font, FontId>,
    pub(super) font_info_cache: HashMap<usize, FontId>,
    pub(super) layout_line_scratch: Vec<u16>,
}

