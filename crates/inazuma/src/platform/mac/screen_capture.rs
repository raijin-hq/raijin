use anyhow::{Result, anyhow};
use block2::RcBlock;
use collections::HashMap;
use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayCopyDisplayMode, CGDisplayMode,
};
use futures::channel::oneshot;
use inazuma::{
    DevicePixels, ForegroundExecutor, ScreenCaptureFrame, ScreenCaptureSource, ScreenCaptureStream,
    SharedString, SourceMetadata, size,
};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, DefinedClass, define_class, msg_send};
use objc2_core_media::CMSampleBuffer;
use objc2_foundation::{NSError, NSObject, NSObjectProtocol};
use objc2_screen_capture_kit::{
    SCContentFilter, SCDisplay, SCShareableContent, SCStream, SCStreamConfiguration,
    SCStreamDelegate, SCStreamOutput, SCStreamOutputType, SCWindow,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

#[derive(Clone)]
pub struct MacScreenCaptureSource {
    sc_display: Retained<SCDisplay>,
    meta: Option<ScreenMeta>,
}

pub struct MacScreenCaptureStream {
    sc_stream: Retained<SCStream>,
    sc_stream_output: Retained<ProtocolObject<dyn SCStreamOutput>>,
    meta: SourceMetadata,
}

impl ScreenCaptureSource for MacScreenCaptureSource {
    fn metadata(&self) -> Result<SourceMetadata> {
        let (display_id, size) = {
            let display_id = unsafe { self.sc_display.displayID() };
            let display_mode = CGDisplayCopyDisplayMode(display_id);
            let width = CGDisplayMode::pixel_width(display_mode.as_deref());
            let height = CGDisplayMode::pixel_height(display_mode.as_deref());
            // display_mode is CFRetained — automatically released on drop

            (
                display_id,
                size(DevicePixels(width as i32), DevicePixels(height as i32)),
            )
        };
        let (label, is_main) = self
            .meta
            .clone()
            .map(|meta| (meta.label, meta.is_main))
            .unzip();

        Ok(SourceMetadata {
            id: display_id as u64,
            label,
            is_main,
            resolution: size,
        })
    }

    fn stream(
        &self,
        _foreground_executor: &ForegroundExecutor,
        frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
    ) -> oneshot::Receiver<Result<Box<dyn ScreenCaptureStream>>> {
        unsafe {
            let excluded_windows =
                objc2_foundation::NSArray::<SCWindow>::new();
            let filter = SCContentFilter::initWithDisplay_excludingWindows(
                SCContentFilter::alloc(),
                &self.sc_display,
                &excluded_windows,
            );

            let configuration =
                SCStreamConfiguration::init(SCStreamConfiguration::alloc());
            configuration.setScalesToFit(true);
            // 'BGRA' pixel format = 0x42475241
            configuration.setPixelFormat(0x42475241);

            let meta = self.metadata().unwrap();
            configuration.setWidth(meta.resolution.width.0 as usize);
            configuration.setHeight(meta.resolution.height.0 as usize);

            let delegate = InazumaStreamDelegate::create();
            let delegate_proto: &ProtocolObject<dyn SCStreamDelegate> =
                ProtocolObject::from_ref(&*delegate);

            let stream = SCStream::initWithFilter_configuration_delegate(
                SCStream::alloc(),
                &filter,
                &configuration,
                Some(delegate_proto),
            );

            let output = InazumaStreamOutput::create(frame_callback);
            let output_proto: Retained<ProtocolObject<dyn SCStreamOutput>> =
                ProtocolObject::from_retained(output.clone());

            let (tx, rx) = oneshot::channel();

            match stream.addStreamOutput_type_sampleHandlerQueue_error(
                &output_proto,
                SCStreamOutputType::Screen,
                None,
            ) {
                Ok(()) => {}
                Err(err) => {
                    let message = err.localizedDescription();
                    tx.send(Err(anyhow!(
                        "failed to add stream output: {}",
                        message
                    )))
                    .ok();
                    return rx;
                }
            }

            let tx = Rc::new(RefCell::new(Some(tx)));
            let stream_clone = stream.clone();
            let output_proto_clone = output_proto.clone();
            let meta_clone = meta.clone();

            let handler = RcBlock::new(move |error: *mut NSError| {
                let result = if error.is_null() {
                    let stream_obj = MacScreenCaptureStream {
                        sc_stream: stream_clone.clone(),
                        sc_stream_output: output_proto_clone.clone(),
                        meta: meta_clone.clone(),
                    };
                    Ok(Box::new(stream_obj) as Box<dyn ScreenCaptureStream>)
                } else {
                    let err = &*error;
                    let message = err.localizedDescription();
                    Err(anyhow!(
                        "failed to start screen capture stream: {}",
                        message
                    ))
                };
                if let Some(tx) = tx.borrow_mut().take() {
                    tx.send(result).ok();
                }
            });

            stream.startCaptureWithCompletionHandler(Some(&handler));
            rx
        }
    }
}

impl ScreenCaptureStream for MacScreenCaptureStream {
    fn metadata(&self) -> Result<SourceMetadata> {
        Ok(self.meta.clone())
    }
}

impl Drop for MacScreenCaptureStream {
    fn drop(&mut self) {
        unsafe {
            if let Err(err) = self.sc_stream.removeStreamOutput_type_error(
                &self.sc_stream_output,
                SCStreamOutputType::Screen,
            ) {
                let message = err.localizedDescription();
                log::error!("failed to remove stream output: {message}");
            }

            let handler = RcBlock::new(move |error: *mut NSError| {
                if !error.is_null() {
                    let err = &*error;
                    let message = err.localizedDescription();
                    log::error!("failed to stop screen capture stream: {message}");
                }
            });
            self.sc_stream
                .stopCaptureWithCompletionHandler(Some(&handler));
        }
    }
}

#[derive(Clone)]
struct ScreenMeta {
    label: SharedString,
    is_main: bool,
}

/// Build a map from display ID to human-readable screen label using NSScreen.
fn screen_id_to_human_label() -> HashMap<CGDirectDisplayID, ScreenMeta> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;
    use objc2::rc::Retained;
    use objc2_foundation::{NSDictionary, NSNumber, NSString};

    unsafe {
        let mtm = MainThreadMarker::new_unchecked();
        let screens = NSScreen::screens(mtm);
        let mut map = HashMap::default();
        let screen_number_key = NSString::from_str("NSScreenNumber");

        for (i, screen) in screens.iter().enumerate() {
            let device_desc = screen.deviceDescription();
            let Some(screen_number) =
                NSDictionary::objectForKey(&device_desc, &screen_number_key)
            else {
                continue;
            };
            let screen_number: Retained<NSNumber> = Retained::cast(screen_number);
            let screen_id = screen_number.unsignedIntegerValue() as CGDirectDisplayID;

            let name = screen.localizedName();
            {
                let rust_str = name.to_string();
                map.insert(
                    screen_id,
                    ScreenMeta {
                        label: rust_str.into(),
                        is_main: i == 0,
                    },
                );
            }
        }
        map
    }
}

pub(crate) fn get_sources() -> oneshot::Receiver<Result<Vec<Rc<dyn ScreenCaptureSource>>>> {
    unsafe {
        let (tx, rx) = oneshot::channel();
        let tx = Rc::new(RefCell::new(Some(tx)));
        let screen_id_to_label = screen_id_to_human_label();

        let block = RcBlock::new(
            move |shareable_content: *mut SCShareableContent, error: *mut NSError| {
                let Some(tx) = tx.borrow_mut().take() else {
                    return;
                };

                let result = if error.is_null() {
                    let content = &*shareable_content;
                    let displays = content.displays();
                    let mut result = Vec::new();
                    for display in displays.iter() {
                        let display_id = display.displayID();
                        let meta = screen_id_to_label.get(&display_id).cloned();
                        let source = MacScreenCaptureSource {
                            sc_display: display.clone(),
                            meta,
                        };
                        result.push(Rc::new(source) as Rc<dyn ScreenCaptureSource>);
                    }
                    Ok(result)
                } else {
                    let err = &*error;
                    let message = err.localizedDescription();
                    Err(anyhow!("Screen share failed: {}", message))
                };
                tx.send(result).ok();
            },
        );

        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(
            true,
            true,
            &block,
        );
        rx
    }
}

// ---------------------------------------------------------------------------
// Custom ObjC classes for stream delegate and output using define_class!
// ---------------------------------------------------------------------------

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "InazumaStreamDelegate"]
    #[ivars = ()]
    struct InazumaStreamDelegate;

    unsafe impl NSObjectProtocol for InazumaStreamDelegate {}

    unsafe impl SCStreamDelegate for InazumaStreamDelegate {
        #[unsafe(method(stream:didStopWithError:))]
        fn _stream_did_stop_with_error(&self, _stream: &SCStream, _error: &NSError) {}

        #[unsafe(method(outputVideoEffectDidStartForStream:))]
        fn _output_video_effect_did_start(&self, _stream: &SCStream) {}

        #[unsafe(method(outputVideoEffectDidStopForStream:))]
        fn _output_video_effect_did_stop(&self, _stream: &SCStream) {}
    }
);

impl InazumaStreamDelegate {
    fn create() -> Retained<Self> {
        let alloc = Self::alloc().set_ivars(());
        unsafe { msg_send![super(alloc), init] }
    }
}

/// Ivars for InazumaStreamOutput — stores the frame callback as a raw pointer.
#[derive(Default)]
struct StreamOutputIvars {
    frame_callback: std::cell::Cell<*mut c_void>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "InazumaStreamOutput"]
    #[ivars = StreamOutputIvars]
    struct InazumaStreamOutput;

    unsafe impl NSObjectProtocol for InazumaStreamOutput {}

    unsafe impl SCStreamOutput for InazumaStreamOutput {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        fn _stream_did_output_sample_buffer(
            &self,
            _stream: &SCStream,
            sample_buffer: &CMSampleBuffer,
            output_type: SCStreamOutputType,
        ) {
            if output_type != SCStreamOutputType::Screen {
                return;
            }

            unsafe {
                if let Some(image_buffer) = sample_buffer.image_buffer() {
                    let callback_ptr = self.ivars().frame_callback.get();
                    if !callback_ptr.is_null() {
                        let callback =
                            &*(callback_ptr as *const Box<dyn Fn(ScreenCaptureFrame) + Send>);
                        callback(ScreenCaptureFrame(image_buffer));
                    }
                }
            }
        }
    }
);

impl InazumaStreamOutput {
    fn create(
        callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
    ) -> Retained<Self> {
        let callback_ptr = Box::into_raw(Box::new(callback)) as *mut c_void;
        let alloc = Self::alloc();
        let this = alloc.set_ivars(StreamOutputIvars {
            frame_callback: std::cell::Cell::new(callback_ptr),
        });
        unsafe { msg_send![super(this), init] }
    }
}

impl Drop for InazumaStreamOutput {
    fn drop(&mut self) {
        let callback_ptr = self.ivars().frame_callback.get();
        if !callback_ptr.is_null() {
            unsafe {
                let _ =
                    Box::from_raw(callback_ptr as *mut Box<dyn Fn(ScreenCaptureFrame) + Send>);
            }
        }
    }
}
