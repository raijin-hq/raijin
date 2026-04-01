use super::*;

pub(crate) const DISABLE_DIRECT_COMPOSITION: &str = "GPUI_DISABLE_DIRECT_COMPOSITION";
pub(super) const RENDER_TARGET_FORMAT: DXGI_FORMAT = DXGI_FORMAT_B8G8R8A8_UNORM;
// This configuration is used for MSAA rendering on paths only, and it's guaranteed to be supported by DirectX 11.
pub(super) const PATH_MULTISAMPLE_COUNT: u32 = 4;

pub(crate) struct FontInfo {
    pub gamma_ratios: [f32; 4],
    pub grayscale_enhanced_contrast: f32,
    pub subpixel_enhanced_contrast: f32,
}

pub(crate) struct DirectXRenderer {
    pub(super) hwnd: HWND,
    pub(super) atlas: Arc<DirectXAtlas>,
    pub(super) devices: Option<DirectXRendererDevices>,
    pub(super) resources: Option<DirectXResources>,
    pub(super) globals: DirectXGlobalElements,
    pub(super) pipelines: DirectXRenderPipelines,
    pub(super) direct_composition: Option<DirectComposition>,
    pub(super) font_info: &'static FontInfo,

    pub(super) width: u32,
    pub(super) height: u32,

    /// Whether we want to skip drwaing due to device lost events.
    ///
    /// In that case we want to discard the first frame that we draw as we got reset in the middle of a frame
    /// meaning we lost all the allocated gpu textures and scene resources.
    pub(super) skip_draws: bool,
}

/// Direct3D objects
#[derive(Clone)]
pub(crate) struct DirectXRendererDevices {
    pub(crate) adapter: IDXGIAdapter1,
    pub(crate) dxgi_factory: IDXGIFactory6,
    pub(crate) device: ID3D11Device,
    pub(crate) device_context: ID3D11DeviceContext,
    pub(super) dxgi_device: Option<IDXGIDevice>,
}

pub(super) struct DirectXResources {
    // Direct3D rendering objects
    pub(super) swap_chain: IDXGISwapChain1,
    pub(super) render_target: Option<ID3D11Texture2D>,
    pub(super) render_target_view: Option<ID3D11RenderTargetView>,

    // Path intermediate textures (with MSAA)
    pub(super) path_intermediate_texture: ID3D11Texture2D,
    pub(super) path_intermediate_srv: Option<ID3D11ShaderResourceView>,
    pub(super) path_intermediate_msaa_texture: ID3D11Texture2D,
    pub(super) path_intermediate_msaa_view: Option<ID3D11RenderTargetView>,

    // Cached viewport
    pub(super) viewport: D3D11_VIEWPORT,
}

pub(super) struct DirectXRenderPipelines {
    pub(super) shadow_pipeline: PipelineState<Shadow>,
    pub(super) quad_pipeline: PipelineState<Quad>,
    pub(super) path_rasterization_pipeline: PipelineState<PathRasterizationSprite>,
    pub(super) path_sprite_pipeline: PipelineState<PathSprite>,
    pub(super) underline_pipeline: PipelineState<Underline>,
    pub(super) mono_sprites: PipelineState<MonochromeSprite>,
    pub(super) subpixel_sprites: PipelineState<SubpixelSprite>,
    pub(super) poly_sprites: PipelineState<PolychromeSprite>,
}

pub(super) struct DirectXGlobalElements {
    pub(super) global_params_buffer: Option<ID3D11Buffer>,
    pub(super) sampler: Option<ID3D11SamplerState>,
}

pub(super) struct DirectComposition {
    pub(super) comp_device: IDCompositionDevice,
    pub(super) comp_target: IDCompositionTarget,
    pub(super) comp_visual: IDCompositionVisual,
}

impl DirectXRendererDevices {
    pub(crate) fn new(
        directx_devices: &DirectXDevices,
        disable_direct_composition: bool,
    ) -> Result<Self> {
        let DirectXDevices {
            adapter,
            dxgi_factory,
            device,
            device_context,
        } = directx_devices;
        let dxgi_device = if disable_direct_composition {
            None
        } else {
            Some(device.cast().context("Creating DXGI device")?)
        };

        Ok(Self {
            adapter: adapter.clone(),
            dxgi_factory: dxgi_factory.clone(),
            device: device.clone(),
            device_context: device_context.clone(),
            dxgi_device,
        })
    }
}

