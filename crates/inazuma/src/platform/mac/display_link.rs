use anyhow::Result;
use objc2_core_graphics::CGDirectDisplayID;
use dispatch2::{
    _dispatch_source_type_data_add, DispatchObject, DispatchQueue, DispatchRetained, DispatchSource,
};
use objc2_core_video::{CVDisplayLink, CVOptionFlags, CVReturn, CVTimeStamp, kCVReturnSuccess};
use std::ffi::c_void;
use std::ptr::NonNull;
use inazuma_util::ResultExt;

pub struct DisplayLink {
    display_link: Option<objc2_core_foundation::CFRetained<CVDisplayLink>>,
    frame_requests: DispatchRetained<DispatchSource>,
}

impl DisplayLink {
    pub fn new(
        display_id: CGDirectDisplayID,
        data: *mut c_void,
        callback: extern "C" fn(*mut c_void),
    ) -> Result<DisplayLink> {
        unsafe extern "C-unwind" fn display_link_callback(
            _display_link: NonNull<CVDisplayLink>,
            _current_time: NonNull<CVTimeStamp>,
            _output_time: NonNull<CVTimeStamp>,
            _flags_in: CVOptionFlags,
            _flags_out: NonNull<CVOptionFlags>,
            frame_requests: *mut c_void,
        ) -> CVReturn {
            unsafe {
                let frame_requests = &*(frame_requests as *const DispatchSource);
                frame_requests.merge_data(1);
                kCVReturnSuccess
            }
        }

        unsafe {
            let frame_requests = DispatchSource::new(
                &raw const _dispatch_source_type_data_add as *mut _,
                0,
                0,
                Some(DispatchQueue::main()),
            );
            frame_requests.set_context(data);
            frame_requests.set_event_handler_f(callback);
            frame_requests.resume();

            #[allow(deprecated)]
            let display_link = {
                let mut dl_ptr: *mut CVDisplayLink = std::ptr::null_mut();
                let code = CVDisplayLink::create_with_active_cg_displays(
                    NonNull::new_unchecked(&mut dl_ptr),
                );
                anyhow::ensure!(code == kCVReturnSuccess, "could not create display link, code: {}", code);
                let dl = objc2_core_foundation::CFRetained::from_raw(NonNull::new_unchecked(dl_ptr));

                let code = dl.set_output_callback(
                    Some(display_link_callback),
                    &*frame_requests as *const DispatchSource as *mut c_void,
                );
                anyhow::ensure!(code == kCVReturnSuccess, "could not set output callback, code: {}", code);

                let code = dl.set_current_cg_display(display_id);
                anyhow::ensure!(code == kCVReturnSuccess, "could not assign display to display link, code: {}", code);

                dl
            };

            Ok(Self {
                display_link: Some(display_link),
                frame_requests,
            })
        }
    }

    #[allow(deprecated)]
    pub fn start(&mut self) -> Result<()> {
        let code = self.display_link.as_ref().unwrap().start();
        anyhow::ensure!(code == kCVReturnSuccess, "could not start display link, code: {}", code);
        Ok(())
    }

    #[allow(deprecated)]
    pub fn stop(&mut self) -> Result<()> {
        let code = self.display_link.as_ref().unwrap().stop();
        anyhow::ensure!(code == kCVReturnSuccess, "could not stop display link, code: {}", code);
        Ok(())
    }
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        self.stop().log_err();
        // We see occasional segfaults on the CVDisplayLink thread.
        // Forget the display link to avoid releasing it while the background thread may still access it.
        std::mem::forget(self.display_link.take());
        self.frame_requests.cancel();
    }
}
