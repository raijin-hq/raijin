mod app_menu;
mod atlas;
mod clipboard;
mod image;
mod input_handler;
mod keyboard;
mod keystroke;
mod traits;
mod types;

#[cfg(target_os = "macos")]
pub(crate) mod mac;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(crate) mod linux;

#[cfg(target_os = "windows")]
pub(crate) mod windows;

#[cfg(any(target_os = "linux", target_os = "freebsd", target_family = "wasm"))]
pub(crate) mod wgpu;

#[cfg(target_family = "wasm")]
pub(crate) mod web;

#[cfg(all(target_os = "linux", feature = "wayland"))]
#[expect(missing_docs)]
pub mod layer_shell;

#[cfg(any(test, feature = "test-support"))]
mod test;

#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
mod visual_test;

#[cfg(all(
    feature = "screen-capture",
    any(target_os = "windows", target_os = "linux", target_os = "freebsd",)
))]
pub mod scap_screen_capture;

#[cfg(all(
    any(target_os = "windows", target_os = "linux"),
    feature = "screen-capture"
))]
pub(crate) type PlatformScreenCaptureFrame = scap::frame::Frame;
#[cfg(not(feature = "screen-capture"))]
pub(crate) type PlatformScreenCaptureFrame = ();
#[cfg(all(target_os = "macos", feature = "screen-capture"))]
pub(crate) type PlatformScreenCaptureFrame = core_video::image_buffer::CVImageBuffer;

// Re-exports — maintain the same external API
pub use app_menu::*;
pub use atlas::*;
pub use clipboard::*;
pub use image::*;
pub use input_handler::*;
pub use keyboard::*;
pub use keystroke::*;
pub use traits::*;
pub use types::*;

#[cfg(any(test, feature = "test-support"))]
pub(crate) use test::*;

#[cfg(any(test, feature = "test-support"))]
pub use test::{TestDispatcher, TestScreenCaptureSource, TestScreenCaptureStream};

#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
pub use visual_test::VisualTestPlatform;

#[cfg(target_os = "macos")]
pub use mac::MacPlatform;

#[cfg(target_os = "windows")]
pub use windows::WindowsPlatform;

use std::rc::Rc;

/// Returns the default [`Platform`] for the current OS.
pub fn current_platform(headless: bool) -> Rc<dyn Platform> {
    #[cfg(target_os = "macos")]
    {
        Rc::new(mac::MacPlatform::new(headless))
    }

    #[cfg(target_os = "windows")]
    {
        Rc::new(
            windows::WindowsPlatform::new(headless).expect("failed to initialize Windows platform"),
        )
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        linux::current_platform(headless)
    }

    #[cfg(target_family = "wasm")]
    {
        let _ = headless;
        Rc::new(web::WebPlatform::new(true))
    }
}

/// Returns a background executor for the current platform.
pub fn background_executor() -> crate::BackgroundExecutor {
    current_platform(true).background_executor()
}

/// Creates a new application with the current platform.
pub fn application() -> crate::Application {
    crate::Application::with_platform(current_platform(false))
}

/// Creates a new headless application.
pub fn headless() -> crate::Application {
    crate::Application::with_platform(current_platform(true))
}

/// Unlike `application`, this function returns a single-threaded web application.
#[cfg(target_family = "wasm")]
pub fn single_threaded_web() -> crate::Application {
    crate::Application::with_platform(Rc::new(web::WebPlatform::new(false)))
}

/// Initializes panic hooks and logging for the web platform.
/// Call this before running the application in a wasm_bindgen entrypoint.
#[cfg(target_family = "wasm")]
pub fn web_init() {
    console_error_panic_hook::set_once();
    web::init_logging();
}

/// Returns a new headless renderer for the current platform, if available.
#[cfg(feature = "test-support")]
pub fn current_headless_renderer() -> Option<Box<dyn crate::PlatformHeadlessRenderer>> {
    #[cfg(target_os = "macos")]
    {
        Some(Box::new(mac::metal_renderer::MetalHeadlessRenderer::new()))
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

// TODO(jk): return an enum instead of a string
/// Return which compositor we're guessing we'll use.
/// Does not attempt to connect to the given compositor.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[inline]
pub fn guess_compositor() -> &'static str {
    if std::env::var_os("ZED_HEADLESS").is_some() {
        return "Headless";
    }

    #[cfg(feature = "wayland")]
    let wayland_display = std::env::var_os("WAYLAND_DISPLAY");
    #[cfg(not(feature = "wayland"))]
    let wayland_display: Option<std::ffi::OsString> = None;

    #[cfg(feature = "x11")]
    let x11_display = std::env::var_os("DISPLAY");
    #[cfg(not(feature = "x11"))]
    let x11_display: Option<std::ffi::OsString> = None;

    let use_wayland = wayland_display.is_some_and(|display| !display.is_empty());
    let use_x11 = x11_display.is_some_and(|display| !display.is_empty());

    if use_wayland {
        "Wayland"
    } else if use_x11 {
        "X11"
    } else {
        "Headless"
    }
}
