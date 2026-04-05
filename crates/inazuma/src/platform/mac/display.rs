use anyhow::Result;
use inazuma::{Bounds, DisplayId, Pixels, PlatformDisplay, point, px, size};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_core_foundation::CFUUID;
use objc2_core_graphics::{CGDirectDisplayID, CGDisplayBounds, CGError, CGGetActiveDisplayList};
use objc2_foundation::{NSNumber, NSString};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct MacDisplay(pub(crate) CGDirectDisplayID);

unsafe impl Send for MacDisplay {}

impl MacDisplay {
    /// Get the screen with the given [`DisplayId`].
    pub fn find_by_id(id: DisplayId) -> Option<Self> {
        Self::all().find(|screen| screen.id() == id)
    }

    /// Get the primary screen - the one with the menu bar, and whose bottom left
    /// corner is at the origin of the AppKit coordinate system.
    pub fn primary() -> Self {
        // Instead of iterating through all active systems displays via `all()` we use the first
        // NSScreen and gets its CGDirectDisplayID, because we can't be sure that `CGGetActiveDisplayList`
        // will always return a list of active displays (machine might be sleeping).
        //
        // The following is what Chromium does too:
        //
        // https://chromium.googlesource.com/chromium/src/+/66.0.3359.158/ui/display/mac/screen_mac.mm#56
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let screens = NSScreen::screens(mtm);
            let screen = screens.objectAtIndex(0);
            let device_description = screen.deviceDescription();
            let screen_number_key = NSString::from_str("NSScreenNumber");
            let screen_number: *const AnyObject =
                msg_send![&device_description, objectForKey: &*screen_number_key];
            let screen_number: &NSNumber = &*(screen_number as *const NSNumber);
            let screen_id = screen_number.unsignedIntegerValue() as CGDirectDisplayID;
            Self(screen_id)
        }
    }

    /// Obtains an iterator over all currently active system displays.
    pub fn all() -> impl Iterator<Item = Self> {
        unsafe {
            // We're assuming there aren't more than 32 displays connected to the system.
            let mut displays = Vec::with_capacity(32);
            let mut display_count = 0;
            let result = CGGetActiveDisplayList(
                displays.capacity() as u32,
                displays.as_mut_ptr(),
                &mut display_count,
            );

            if result == CGError::Success {
                displays.set_len(display_count as usize);
                displays.into_iter().map(MacDisplay)
            } else {
                panic!("Failed to get active display list. Result: {}", result.0);
            }
        }
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: CGDirectDisplayID) -> *const CFUUID;
}

impl PlatformDisplay for MacDisplay {
    fn id(&self) -> DisplayId {
        DisplayId::new(self.0)
    }

    fn uuid(&self) -> Result<Uuid> {
        let cfuuid = unsafe { CGDisplayCreateUUIDFromDisplayID(self.0 as CGDirectDisplayID) };
        anyhow::ensure!(
            !cfuuid.is_null(),
            "AppKit returned a null from CGDisplayCreateUUIDFromDisplayID"
        );

        let cfuuid = unsafe { &*cfuuid };
        let bytes = cfuuid.uuid_bytes();
        Ok(Uuid::from_bytes([
            bytes.byte0,
            bytes.byte1,
            bytes.byte2,
            bytes.byte3,
            bytes.byte4,
            bytes.byte5,
            bytes.byte6,
            bytes.byte7,
            bytes.byte8,
            bytes.byte9,
            bytes.byte10,
            bytes.byte11,
            bytes.byte12,
            bytes.byte13,
            bytes.byte14,
            bytes.byte15,
        ]))
    }

    fn bounds(&self) -> Bounds<Pixels> {
        // CGDisplayBounds is in "global display" coordinates, where 0 is
        // the top left of the primary display.
        let bounds = CGDisplayBounds(self.0);

        Bounds {
            origin: Default::default(),
            size: size(px(bounds.size.width as f32), px(bounds.size.height as f32)),
        }
    }

    fn visible_bounds(&self) -> Bounds<Pixels> {
        let Some(screen) = self.get_nsscreen() else {
            return self.bounds();
        };

        let screen_frame = screen.frame();
        let visible_frame = screen.visibleFrame();

        // Convert from bottom-left origin (AppKit) to top-left origin
        let origin_y =
            screen_frame.size.height - visible_frame.origin.y - visible_frame.size.height
                + screen_frame.origin.y;

        Bounds {
            origin: point(
                px(visible_frame.origin.x as f32 - screen_frame.origin.x as f32),
                px(origin_y as f32),
            ),
            size: size(
                px(visible_frame.size.width as f32),
                px(visible_frame.size.height as f32),
            ),
        }
    }
}

impl MacDisplay {
    /// Find the NSScreen corresponding to this display
    fn get_nsscreen(&self) -> Option<Retained<NSScreen>> {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let screens = NSScreen::screens(mtm);
            let screen_number_key = NSString::from_str("NSScreenNumber");

            for screen in screens.iter() {
                let device_description = screen.deviceDescription();
                let screen_number: *const AnyObject =
                    msg_send![&device_description, objectForKey: &*screen_number_key];
                if screen_number.is_null() {
                    continue;
                }
                let screen_number: &NSNumber = &*(screen_number as *const NSNumber);
                let screen_id = screen_number.unsignedIntegerValue() as CGDirectDisplayID;
                if screen_id == self.0 {
                    return Some(screen.clone());
                }
            }
            None
        }
    }
}
