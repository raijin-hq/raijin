use objc2::runtime::AnyObject;
use objc2_app_kit::NSWindowStyleMask;
use objc2_foundation::NSInteger;

#[allow(non_upper_case_globals)]
pub(super) const NSWindowStyleMaskNonactivatingPanel: NSWindowStyleMask =
    NSWindowStyleMask::from_bits_retain(1 << 7);

// WindowLevel const value ref: https://docs.rs/core-graphics2/0.4.1/src/core_graphics2/window_level.rs.html
#[allow(non_upper_case_globals)]
pub(super) const NSNormalWindowLevel: NSInteger = 0;
#[allow(non_upper_case_globals)]
pub(super) const NSFloatingWindowLevel: NSInteger = 3;
#[allow(non_upper_case_globals)]
pub(super) const NSPopUpWindowLevel: NSInteger = 101;

// https://developer.apple.com/documentation/appkit/nsdragoperation
pub(super) type NSDragOperation = usize;
#[allow(non_upper_case_globals)]
pub(super) const NSDragOperationNone: NSDragOperation = 0;
#[allow(non_upper_case_globals)]
pub(super) const NSDragOperationCopy: NSDragOperation = 1;

#[derive(PartialEq)]
pub enum UserTabbingPreference {
    Never,
    Always,
    InFullScreen,
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    // Widely used private APIs; Apple uses them for their Terminal.app.
    pub(super) fn CGSMainConnectionID() -> *mut AnyObject;
    pub(super) fn CGSSetWindowBackgroundBlurRadius(
        connection_id: *mut AnyObject,
        window_id: NSInteger,
        radius: i64,
    ) -> i32;
}
