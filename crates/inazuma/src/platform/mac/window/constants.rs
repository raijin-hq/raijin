use super::*;

pub(super) const WINDOW_STATE_IVAR: &str = "windowState";

pub(super) static mut WINDOW_CLASS: *const Class = ptr::null();
pub(super) static mut PANEL_CLASS: *const Class = ptr::null();
pub(super) static mut VIEW_CLASS: *const Class = ptr::null();
pub(super) static mut BLURRED_VIEW_CLASS: *const Class = ptr::null();

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
#[allow(non_upper_case_globals)]
pub(super) const NSTrackingMouseEnteredAndExited: NSUInteger = 0x01;
#[allow(non_upper_case_globals)]
pub(super) const NSTrackingMouseMoved: NSUInteger = 0x02;
#[allow(non_upper_case_globals)]
pub(super) const NSTrackingActiveAlways: NSUInteger = 0x80;
#[allow(non_upper_case_globals)]
pub(super) const NSTrackingInVisibleRect: NSUInteger = 0x200;
#[allow(non_upper_case_globals)]
pub(super) const NSWindowAnimationBehaviorUtilityWindow: NSInteger = 4;
#[allow(non_upper_case_globals)]
pub(super) const NSViewLayerContentsRedrawDuringViewResize: NSInteger = 2;
// https://developer.apple.com/documentation/appkit/nsdragoperation
pub(super) type NSDragOperation = NSUInteger;
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
    pub(super) fn CGSMainConnectionID() -> id;
    pub(super) fn CGSSetWindowBackgroundBlurRadius(
        connection_id: id,
        window_id: NSInteger,
        radius: i64,
    ) -> i32;
}
