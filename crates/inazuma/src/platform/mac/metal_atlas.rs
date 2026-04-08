use anyhow::{Context as _, Result};
use inazuma_collections::FxHashMap;
use derive_more::{Deref, DerefMut};
use etagere::BucketedAtlasAllocator;
use inazuma::{
    AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTextureList, AtlasTile, Bounds, DevicePixels,
    PlatformAtlas, Point, Size,
};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::*;
use parking_lot::Mutex;
use std::borrow::Cow;
use std::ffi::c_void;
use std::ptr::NonNull;

pub(crate) struct MetalAtlas(Mutex<MetalAtlasState>);

impl MetalAtlas {
    pub(crate) fn new(device: Retained<ProtocolObject<dyn MTLDevice>>, is_apple_gpu: bool) -> Self {
        MetalAtlas(Mutex::new(MetalAtlasState {
            device: AssertSend(device),
            is_apple_gpu,
            monochrome_textures: Default::default(),
            polychrome_textures: Default::default(),
            tiles_by_key: Default::default(),
        }))
    }

    pub(crate) fn metal_texture(
        &self,
        id: AtlasTextureId,
    ) -> Retained<ProtocolObject<dyn MTLTexture>> {
        self.0.lock().texture(id).metal_texture.0.clone()
    }
}

struct MetalAtlasState {
    device: AssertSend<Retained<ProtocolObject<dyn MTLDevice>>>,
    is_apple_gpu: bool,
    monochrome_textures: AtlasTextureList<MetalAtlasTexture>,
    polychrome_textures: AtlasTextureList<MetalAtlasTexture>,
    tiles_by_key: FxHashMap<AtlasKey, AtlasTile>,
}

impl PlatformAtlas for MetalAtlas {
    fn get_or_insert_with<'a>(
        &self,
        key: &AtlasKey,
        build: &mut dyn FnMut() -> Result<Option<(Size<DevicePixels>, Cow<'a, [u8]>)>>,
    ) -> Result<Option<AtlasTile>> {
        let mut lock = self.0.lock();
        if let Some(tile) = lock.tiles_by_key.get(key) {
            Ok(Some(tile.clone()))
        } else {
            let Some((size, bytes)) = build()? else {
                return Ok(None);
            };
            let tile = lock
                .allocate(size, key.texture_kind())
                .context("failed to allocate")?;
            let texture = lock.texture(tile.texture_id);
            texture.upload(tile.bounds, &bytes);
            lock.tiles_by_key.insert(key.clone(), tile.clone());
            Ok(Some(tile))
        }
    }

    fn remove(&self, key: &AtlasKey) {
        let mut lock = self.0.lock();
        let Some(id) = lock.tiles_by_key.get(key).map(|v| v.texture_id) else {
            return;
        };

        let textures = match id.kind {
            AtlasTextureKind::Monochrome => &mut lock.monochrome_textures,
            AtlasTextureKind::Polychrome => &mut lock.polychrome_textures,
            AtlasTextureKind::Subpixel => unreachable!(),
        };

        let Some(texture_slot) = textures
            .textures
            .iter_mut()
            .find(|texture| texture.as_ref().is_some_and(|v| v.id == id))
        else {
            return;
        };

        if let Some(mut texture) = texture_slot.take() {
            texture.decrement_ref_count();

            if texture.is_unreferenced() {
                textures.free_list.push(id.index as usize);
                lock.tiles_by_key.remove(key);
            } else {
                *texture_slot = Some(texture);
            }
        }
    }
}

impl MetalAtlasState {
    fn allocate(
        &mut self,
        size: Size<DevicePixels>,
        texture_kind: AtlasTextureKind,
    ) -> Option<AtlasTile> {
        {
            let textures = match texture_kind {
                AtlasTextureKind::Monochrome => &mut self.monochrome_textures,
                AtlasTextureKind::Polychrome => &mut self.polychrome_textures,
                AtlasTextureKind::Subpixel => unreachable!(),
            };

            if let Some(tile) = textures
                .iter_mut()
                .rev()
                .find_map(|texture| texture.allocate(size))
            {
                return Some(tile);
            }
        }

        let texture = self.push_texture(size, texture_kind);
        texture.allocate(size)
    }

    fn push_texture(
        &mut self,
        min_size: Size<DevicePixels>,
        kind: AtlasTextureKind,
    ) -> &mut MetalAtlasTexture {
        const DEFAULT_ATLAS_SIZE: Size<DevicePixels> = Size {
            width: DevicePixels(1024),
            height: DevicePixels(1024),
        };
        // Max texture size on all modern Apple GPUs. Anything bigger than that crashes in validateWithDevice.
        const MAX_ATLAS_SIZE: Size<DevicePixels> = Size {
            width: DevicePixels(16384),
            height: DevicePixels(16384),
        };
        let size = min_size.min(&MAX_ATLAS_SIZE).max(&DEFAULT_ATLAS_SIZE);
        let texture_descriptor = MTLTextureDescriptor::new();
        unsafe {
            texture_descriptor.setWidth(usize::from(size.width));
            texture_descriptor.setHeight(usize::from(size.height));
        }
        let pixel_format;
        let usage;
        match kind {
            AtlasTextureKind::Monochrome => {
                pixel_format = MTLPixelFormat::A8Unorm;
                usage = MTLTextureUsage::ShaderRead;
            }
            AtlasTextureKind::Polychrome => {
                pixel_format = MTLPixelFormat::BGRA8Unorm;
                usage = MTLTextureUsage::ShaderRead;
            }
            AtlasTextureKind::Subpixel => unreachable!(),
        }
        texture_descriptor.setPixelFormat(pixel_format);
        texture_descriptor.setUsage(usage);
        // Shared memory mode can be used only on Apple GPU families
        // https://developer.apple.com/documentation/metal/mtlresourceoptions/storagemodeshared
        texture_descriptor.setStorageMode(if self.is_apple_gpu {
            MTLStorageMode::Shared
        } else {
            MTLStorageMode::Managed
        });
        let metal_texture = self.device
            .newTextureWithDescriptor(&texture_descriptor)
            .unwrap();

        let texture_list = match kind {
            AtlasTextureKind::Monochrome => &mut self.monochrome_textures,
            AtlasTextureKind::Polychrome => &mut self.polychrome_textures,
            AtlasTextureKind::Subpixel => unreachable!(),
        };

        let index = texture_list.free_list.pop();

        let atlas_texture = MetalAtlasTexture {
            id: AtlasTextureId {
                index: index.unwrap_or(texture_list.textures.len()) as u32,
                kind,
            },
            allocator: etagere::BucketedAtlasAllocator::new(size_to_etagere(size)),
            metal_texture: AssertSend(metal_texture),
            live_atlas_keys: 0,
        };

        if let Some(ix) = index {
            texture_list.textures[ix] = Some(atlas_texture);
            texture_list.textures.get_mut(ix)
        } else {
            texture_list.textures.push(Some(atlas_texture));
            texture_list.textures.last_mut()
        }
        .unwrap()
        .as_mut()
        .unwrap()
    }

    fn texture(&self, id: AtlasTextureId) -> &MetalAtlasTexture {
        let textures = match id.kind {
            AtlasTextureKind::Monochrome => &self.monochrome_textures,
            AtlasTextureKind::Polychrome => &self.polychrome_textures,
            AtlasTextureKind::Subpixel => unreachable!(),
        };
        textures[id.index as usize].as_ref().unwrap()
    }
}

struct MetalAtlasTexture {
    id: AtlasTextureId,
    allocator: BucketedAtlasAllocator,
    metal_texture: AssertSend<Retained<ProtocolObject<dyn MTLTexture>>>,
    live_atlas_keys: u32,
}

impl MetalAtlasTexture {
    fn allocate(&mut self, size: Size<DevicePixels>) -> Option<AtlasTile> {
        let allocation = self.allocator.allocate(size_to_etagere(size))?;
        let tile = AtlasTile {
            texture_id: self.id,
            tile_id: allocation.id.into(),
            bounds: Bounds {
                origin: point_from_etagere(allocation.rectangle.min),
                size,
            },
            padding: 0,
        };
        self.live_atlas_keys += 1;
        Some(tile)
    }

    fn upload(&self, bounds: Bounds<DevicePixels>, bytes: &[u8]) {
        let region = MTLRegion {
            origin: MTLOrigin {
                x: usize::from(bounds.origin.x),
                y: usize::from(bounds.origin.y),
                z: 0,
            },
            size: MTLSize {
                width: usize::from(bounds.size.width),
                height: usize::from(bounds.size.height),
                depth: 1,
            },
        };
        let bytes_per_row =
            bounds.size.width.to_bytes(self.bytes_per_pixel()) as usize;
        unsafe {
            self.metal_texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                region,
                0,
                NonNull::new_unchecked(bytes.as_ptr() as *mut c_void),
                bytes_per_row,
            );
        }
    }

    fn bytes_per_pixel(&self) -> u8 {
        match self.metal_texture.pixelFormat() {
            MTLPixelFormat::A8Unorm | MTLPixelFormat::R8Unorm => 1,
            MTLPixelFormat::RGBA8Unorm | MTLPixelFormat::BGRA8Unorm => 4,
            _ => unimplemented!(),
        }
    }

    fn decrement_ref_count(&mut self) {
        self.live_atlas_keys -= 1;
    }

    fn is_unreferenced(&mut self) -> bool {
        self.live_atlas_keys == 0
    }
}

fn size_to_etagere(size: Size<DevicePixels>) -> etagere::Size {
    etagere::Size::new(size.width.into(), size.height.into())
}

fn point_from_etagere(value: etagere::Point) -> Point<DevicePixels> {
    Point {
        x: DevicePixels::from(value.x),
        y: DevicePixels::from(value.y),
    }
}

#[derive(Deref, DerefMut)]
struct AssertSend<T>(T);

unsafe impl<T> Send for AssertSend<T> {}
