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
pub(crate) type PlatformScreenCaptureFrame = objc2_core_foundation::CFRetained<objc2_core_video::CVImageBuffer>;

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

/// A window control button type used in [`WindowButtonLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowButton {
    /// The minimize button
    Minimize,
    /// The maximize button
    Maximize,
    /// The close button
    Close,
}

impl WindowButton {
    /// Returns a stable element ID for rendering this button.
    pub fn id(&self) -> &'static str {
        match self {
            WindowButton::Minimize => "minimize",
            WindowButton::Maximize => "maximize",
            WindowButton::Close => "close",
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn index(&self) -> usize {
        match self {
            WindowButton::Minimize => 0,
            WindowButton::Maximize => 1,
            WindowButton::Close => 2,
        }
    }
}

/// Maximum number of [`WindowButton`]s per side in the titlebar.
pub const MAX_BUTTONS_PER_SIDE: usize = 3;

/// Describes which [`WindowButton`]s appear on each side of the titlebar.
///
/// On Linux, this is read from the desktop environment's configuration
/// (e.g. GNOME's `gtk-decoration-layout` gsetting) via [`WindowButtonLayout::parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowButtonLayout {
    /// Buttons on the left side of the titlebar.
    pub left: [Option<WindowButton>; MAX_BUTTONS_PER_SIDE],
    /// Buttons on the right side of the titlebar.
    pub right: [Option<WindowButton>; MAX_BUTTONS_PER_SIDE],
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
impl WindowButtonLayout {
    /// Returns the built-in fallback button layout for Linux titlebars.
    pub fn linux_default() -> Self {
        Self {
            left: [None; MAX_BUTTONS_PER_SIDE],
            right: [
                Some(WindowButton::Minimize),
                Some(WindowButton::Maximize),
                Some(WindowButton::Close),
            ],
        }
    }

    /// Parses a GNOME-style `button-layout` string (e.g. `"close,minimize:maximize"`).
    pub fn parse(layout_string: &str) -> anyhow::Result<Self> {
        fn parse_side(
            s: &str,
            seen_buttons: &mut [bool; MAX_BUTTONS_PER_SIDE],
            unrecognized: &mut Vec<String>,
        ) -> [Option<WindowButton>; MAX_BUTTONS_PER_SIDE] {
            let mut result = [None; MAX_BUTTONS_PER_SIDE];
            let mut i = 0;
            for name in s.split(',') {
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let button = match trimmed {
                    "minimize" => Some(WindowButton::Minimize),
                    "maximize" => Some(WindowButton::Maximize),
                    "close" => Some(WindowButton::Close),
                    other => {
                        unrecognized.push(other.to_string());
                        None
                    }
                };
                if let Some(button) = button {
                    if seen_buttons[button.index()] {
                        continue;
                    }
                    if let Some(slot) = result.get_mut(i) {
                        *slot = Some(button);
                        seen_buttons[button.index()] = true;
                        i += 1;
                    }
                }
            }
            result
        }

        let (left_str, right_str) = layout_string.split_once(':').unwrap_or(("", layout_string));
        let mut unrecognized = Vec::new();
        let mut seen_buttons = [false; MAX_BUTTONS_PER_SIDE];
        let layout = Self {
            left: parse_side(left_str, &mut seen_buttons, &mut unrecognized),
            right: parse_side(right_str, &mut seen_buttons, &mut unrecognized),
        };

        if !unrecognized.is_empty()
            && layout.left.iter().all(Option::is_none)
            && layout.right.iter().all(Option::is_none)
        {
            anyhow::bail!(
                "button layout string {:?} contains no valid buttons (unrecognized: {})",
                layout_string,
                unrecognized.join(", ")
            );
        }

        Ok(layout)
    }
}

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
