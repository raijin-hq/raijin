#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

mod bindings;

#[cfg(target_os = "macos")]
pub mod core_media {
    #![allow(non_snake_case)]

    pub use crate::bindings::{
        CMItemIndex, CMSampleTimingInfo, CMTime, CMTimeMake, CMVideoCodecType,
        kCMSampleAttachmentKey_NotSync, kCMTimeInvalid, kCMVideoCodecType_H264,
    };
    use anyhow::Result;
    use objc2_core_foundation::{CFArray, CFDictionary, CFString, CFType};
    use objc2_core_video::CVImageBuffer;
    use std::{ffi::c_void, ptr};

    /// Opaque CoreMedia sample buffer type.
    #[repr(C)]
    pub struct CMSampleBuffer {
        _data: [u8; 0],
    }

    pub type CMSampleBufferRef = *const CMSampleBuffer;

    impl CMSampleBuffer {
        pub unsafe fn from_ref(ptr: CMSampleBufferRef) -> &'static Self {
            &*ptr
        }

        pub fn attachments(&self) -> Vec<*const c_void> {
            unsafe {
                let attachments = CMSampleBufferGetSampleAttachmentsArray(self as *const _ as _, true);
                if attachments.is_null() {
                    return Vec::new();
                }
                let count = objc2_core_foundation::CFArrayGetCount(attachments as _);
                (0..count)
                    .map(|i| objc2_core_foundation::CFArrayGetValueAtIndex(attachments as _, i))
                    .collect()
            }
        }

        pub fn image_buffer_ref(&self) -> *const c_void {
            unsafe { CMSampleBufferGetImageBuffer(self as *const _ as _) as *const c_void }
        }

        pub fn sample_timing_info(&self, index: usize) -> Result<CMSampleTimingInfo> {
            unsafe {
                let mut timing_info = CMSampleTimingInfo {
                    duration: kCMTimeInvalid,
                    presentationTimeStamp: kCMTimeInvalid,
                    decodeTimeStamp: kCMTimeInvalid,
                };
                let result = CMSampleBufferGetSampleTimingInfo(
                    self as *const _ as _,
                    index as CMItemIndex,
                    &mut timing_info,
                );
                anyhow::ensure!(result == 0, "error getting sample timing info, code {result}");
                Ok(timing_info)
            }
        }

        pub fn format_description_ref(&self) -> CMFormatDescriptionRef {
            unsafe { CMSampleBufferGetFormatDescription(self as *const _ as _) }
        }

        pub fn data_buffer_ref(&self) -> CMBlockBufferRef {
            unsafe { CMSampleBufferGetDataBuffer(self as *const _ as _) }
        }
    }

    #[link(name = "CoreMedia", kind = "framework")]
    unsafe extern "C" {
        fn CMSampleBufferGetSampleAttachmentsArray(
            buffer: *const c_void,
            create_if_necessary: bool,
        ) -> *const c_void;
        fn CMSampleBufferGetImageBuffer(buffer: *const c_void) -> *const c_void;
        fn CMSampleBufferGetSampleTimingInfo(
            buffer: *const c_void,
            index: CMItemIndex,
            timing_info_out: *mut CMSampleTimingInfo,
        ) -> i32;
        fn CMSampleBufferGetFormatDescription(buffer: *const c_void) -> CMFormatDescriptionRef;
        fn CMSampleBufferGetDataBuffer(sample_buffer: *const c_void) -> CMBlockBufferRef;
    }

    /// Opaque CoreMedia format description type.
    #[repr(C)]
    pub struct CMFormatDescription {
        _data: [u8; 0],
    }

    pub type CMFormatDescriptionRef = *const CMFormatDescription;

    impl CMFormatDescription {
        pub fn h264_parameter_set_count(&self) -> usize {
            unsafe {
                let mut count = 0;
                let result = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
                    self as *const _ as _,
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    &mut count,
                    ptr::null_mut(),
                );
                assert_eq!(result, 0);
                count
            }
        }

        pub fn h264_parameter_set_at_index(&self, index: usize) -> Result<&[u8]> {
            unsafe {
                let mut bytes = ptr::null();
                let mut len = 0;
                let result = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
                    self as *const _ as _,
                    index,
                    &mut bytes,
                    &mut len,
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                anyhow::ensure!(result == 0, "error getting parameter set, code: {result}");
                Ok(std::slice::from_raw_parts(bytes, len))
            }
        }
    }

    #[link(name = "CoreMedia", kind = "framework")]
    unsafe extern "C" {
        fn CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
            video_desc: *const c_void,
            parameter_set_index: usize,
            parameter_set_pointer_out: *mut *const u8,
            parameter_set_size_out: *mut usize,
            parameter_set_count_out: *mut usize,
            nal_unit_header_length_out: *mut isize,
        ) -> i32;
    }

    /// Opaque CoreMedia block buffer type.
    #[repr(C)]
    pub struct CMBlockBuffer {
        _data: [u8; 0],
    }

    pub type CMBlockBufferRef = *const CMBlockBuffer;

    impl CMBlockBuffer {
        pub fn bytes(&self) -> &[u8] {
            unsafe {
                let mut bytes = ptr::null();
                let mut len = 0;
                let result = CMBlockBufferGetDataPointer(
                    self as *const _ as _,
                    0,
                    &mut 0,
                    &mut len,
                    &mut bytes,
                );
                assert!(result == 0, "could not get block buffer data");
                std::slice::from_raw_parts(bytes, len)
            }
        }
    }

    #[link(name = "CoreMedia", kind = "framework")]
    unsafe extern "C" {
        fn CMBlockBufferGetDataPointer(
            buffer: *const c_void,
            offset: usize,
            length_at_offset_out: *mut usize,
            total_length_out: *mut usize,
            data_pointer_out: *mut *const u8,
        ) -> i32;
    }
}

#[cfg(target_os = "macos")]
pub mod core_video {
    #![allow(non_snake_case)]

    use crate::bindings::{CVReturn, kCVReturnSuccess};
    pub use crate::bindings::{
        kCVPixelFormatType_32BGRA, kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
        kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, kCVPixelFormatType_420YpCbCr8Planar,
    };
    use anyhow::Result;
    use std::{ffi::c_void, ptr};

    /// Opaque CVMetalTextureCache type.
    #[repr(C)]
    pub struct CVMetalTextureCache {
        _data: [u8; 0],
    }

    pub type CVMetalTextureCacheRef = *const CVMetalTextureCache;

    impl CVMetalTextureCache {
        /// # Safety
        ///
        /// metal_device must be a valid MTLDevice pointer.
        pub unsafe fn new(metal_device: *mut c_void) -> Result<CVMetalTextureCacheRef> {
            let mut cache = ptr::null();
            let result = unsafe {
                CVMetalTextureCacheCreate(
                    ptr::null(),
                    ptr::null(),
                    metal_device,
                    ptr::null(),
                    &mut cache,
                )
            };
            anyhow::ensure!(
                result == kCVReturnSuccess,
                "could not create texture cache, code: {result}"
            );
            Ok(cache)
        }

        /// # Safety
        ///
        /// The arguments must be valid for CVMetalTextureCacheCreateTextureFromImage.
        pub unsafe fn create_texture_from_image(
            cache: CVMetalTextureCacheRef,
            source: *const c_void,
            texture_attributes: *const c_void,
            pixel_format: u64,
            width: usize,
            height: usize,
            plane_index: usize,
        ) -> Result<CVMetalTextureRef> {
            let mut texture = ptr::null();
            let result = unsafe {
                CVMetalTextureCacheCreateTextureFromImage(
                    ptr::null(),
                    cache,
                    source,
                    texture_attributes,
                    pixel_format,
                    width,
                    height,
                    plane_index,
                    &mut texture,
                )
            };
            anyhow::ensure!(
                result == kCVReturnSuccess,
                "could not create texture, code: {result}"
            );
            Ok(texture)
        }
    }

    #[link(name = "CoreVideo", kind = "framework")]
    unsafe extern "C" {
        fn CVMetalTextureCacheCreate(
            allocator: *const c_void,
            cache_attributes: *const c_void,
            metal_device: *const c_void,
            texture_attributes: *const c_void,
            cache_out: *mut CVMetalTextureCacheRef,
        ) -> CVReturn;
        fn CVMetalTextureCacheCreateTextureFromImage(
            allocator: *const c_void,
            texture_cache: CVMetalTextureCacheRef,
            source_image: *const c_void,
            texture_attributes: *const c_void,
            pixel_format: u64,
            width: usize,
            height: usize,
            plane_index: usize,
            texture_out: *mut CVMetalTextureRef,
        ) -> CVReturn;
    }

    /// Opaque CVMetalTexture type.
    #[repr(C)]
    pub struct CVMetalTexture {
        _data: [u8; 0],
    }

    pub type CVMetalTextureRef = *const CVMetalTexture;

    impl CVMetalTexture {
        /// Returns the underlying Metal texture pointer.
        pub unsafe fn as_texture_ptr(texture: CVMetalTextureRef) -> *mut c_void {
            CVMetalTextureGetTexture(texture)
        }
    }

    #[link(name = "CoreVideo", kind = "framework")]
    unsafe extern "C" {
        fn CVMetalTextureGetTexture(texture: CVMetalTextureRef) -> *mut c_void;
    }
}
