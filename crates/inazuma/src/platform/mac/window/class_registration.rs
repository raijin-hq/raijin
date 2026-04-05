use super::*;
use objc2::{DefinedClass, define_class, sel, MainThreadOnly};
use objc2_app_kit::{
    NSDraggingDestination, NSPanel, NSTextInputClient, NSView,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowDelegate,
};
use objc2_foundation::{NSNotification, NSObjectProtocol};

// ─── Ivars ──────────────────────────────────────────────────────────────────

/// Shared ivar struct for GPUIWindow, GPUIPanel, and GPUIView.
/// Stores a raw pointer to the Arc<Mutex<MacWindowState>>.
#[derive(Default)]
pub(super) struct WindowIvars {
    pub(super) window_state: Cell<*const c_void>,
}

// SAFETY: The pointer is only accessed on the main thread.
unsafe impl Send for WindowIvars {}
unsafe impl Sync for WindowIvars {}

impl Drop for WindowIvars {
    fn drop(&mut self) {
        let raw = self.window_state.get();
        if !raw.is_null() {
            // Drop the Arc<Mutex<MacWindowState>> that was stored as a raw pointer
            unsafe {
                Arc::from_raw(raw as *const Mutex<MacWindowState>);
            }
        }
    }
}

// ─── GPUIWindow ─────────────────────────────────────────────────────────────

define_class!(
    #[unsafe(super(NSWindow))]
    #[name = "GPUIWindow"]
    #[ivars = WindowIvars]
    #[thread_kind = MainThreadOnly]
    pub(super) struct GPUIWindow;

    impl GPUIWindow {
        #[unsafe(method_id(initWithContentRect:styleMask:backing:defer:screen:))]
        fn init_with_content_rect(
            this: objc2::rc::Allocated<Self>,
            rect: NSRect,
            style_mask: objc2_app_kit::NSWindowStyleMask,
            backing: isize,
            defer: bool,
            screen: *mut AnyObject,
        ) -> Option<objc2::rc::Retained<Self>> {
            let this = this.set_ivars(WindowIvars::default());
            unsafe {
                msg_send![super(this),
                    initWithContentRect: rect,
                    styleMask: style_mask,
                    backing: backing,
                    defer: defer,
                    screen: screen
                ]
            }
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, _notification: &NSNotification) {
            callbacks::window_did_resize(self.ivars());
        }

        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn window_did_change_occlusion_state(&self, _notification: &NSNotification) {
            callbacks::window_did_change_occlusion_state(self.ivars());
        }

        #[unsafe(method(windowWillEnterFullScreen:))]
        fn window_will_enter_fullscreen(&self, _notification: &NSNotification) {
            callbacks::window_will_enter_fullscreen(self.ivars());
        }

        #[unsafe(method(windowWillExitFullScreen:))]
        fn window_will_exit_fullscreen(&self, _notification: &NSNotification) {
            callbacks::window_will_exit_fullscreen(self.ivars());
        }

        #[unsafe(method(windowDidMove:))]
        fn window_did_move(&self, _notification: &NSNotification) {
            callbacks::window_did_move(self.ivars());
        }

        #[unsafe(method(windowDidChangeScreen:))]
        fn window_did_change_screen(&self, _notification: &NSNotification) {
            callbacks::window_did_change_screen(self.ivars());
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSNotification) {
            callbacks::window_did_change_key_status(self.ivars(), sel!(windowDidBecomeKey:));
        }

        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _notification: &NSNotification) {
            callbacks::window_did_change_key_status(self.ivars(), sel!(windowDidResignKey:));
        }

        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _sender: &AnyObject) -> bool {
            callbacks::window_should_close(self.ivars())
        }

        #[unsafe(method(close))]
        fn close(&self) {
            callbacks::close_window(self.ivars(), self);
        }

        // Drag and drop
        #[unsafe(method(draggingEntered:))]
        fn dragging_entered(&self, sender: &AnyObject) -> NSDragOperation {
            callbacks_mouse::dragging_entered(self.ivars(), sender)
        }

        #[unsafe(method(draggingUpdated:))]
        fn dragging_updated(&self, sender: &AnyObject) -> NSDragOperation {
            callbacks_mouse::dragging_updated(self.ivars(), sender)
        }

        #[unsafe(method(draggingExited:))]
        fn dragging_exited(&self, _sender: Option<&AnyObject>) {
            callbacks_mouse::dragging_exited(self.ivars());
        }

        #[unsafe(method(performDragOperation:))]
        fn perform_drag_operation(&self, sender: &AnyObject) -> bool {
            callbacks_mouse::perform_drag_operation(self.ivars(), sender)
        }

        #[unsafe(method(concludeDragOperation:))]
        fn conclude_drag_operation(&self, _sender: Option<&AnyObject>) {
            callbacks_mouse::conclude_drag_operation(self.ivars());
        }

        // Tab management
        #[unsafe(method(addTitlebarAccessoryViewController:))]
        fn add_titlebar_accessory_view_controller(&self, view_controller: &AnyObject) {
            callbacks::add_titlebar_accessory_view_controller(self, view_controller);
        }

        #[unsafe(method(moveTabToNewWindow:))]
        fn move_tab_to_new_window(&self, _sender: Option<&AnyObject>) {
            callbacks::move_tab_to_new_window(self.ivars(), self);
        }

        #[unsafe(method(mergeAllWindows:))]
        fn merge_all_windows(&self, _sender: Option<&AnyObject>) {
            callbacks::merge_all_windows(self.ivars(), self);
        }

        #[unsafe(method(selectNextTab:))]
        fn select_next_tab(&self, _sender: Option<&AnyObject>) {
            callbacks::select_next_tab(self.ivars());
        }

        #[unsafe(method(selectPreviousTab:))]
        fn select_previous_tab(&self, _sender: Option<&AnyObject>) {
            callbacks::select_previous_tab(self.ivars());
        }

        #[unsafe(method(toggleTabBar:))]
        fn toggle_tab_bar(&self, _sender: Option<&AnyObject>) {
            callbacks::toggle_tab_bar(self.ivars(), self);
        }
    }

    unsafe impl NSObjectProtocol for GPUIWindow {}
    unsafe impl NSWindowDelegate for GPUIWindow {}
    unsafe impl NSDraggingDestination for GPUIWindow {}
);

// ─── GPUIPanel ──────────────────────────────────────────────────────────────

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "GPUIPanel"]
    #[ivars = WindowIvars]
    #[thread_kind = MainThreadOnly]
    pub(super) struct GPUIPanel;

    impl GPUIPanel {
        #[unsafe(method_id(initWithContentRect:styleMask:backing:defer:screen:))]
        fn init_with_content_rect(
            this: objc2::rc::Allocated<Self>,
            rect: NSRect,
            style_mask: objc2_app_kit::NSWindowStyleMask,
            backing: isize,
            defer: bool,
            screen: *mut AnyObject,
        ) -> Option<objc2::rc::Retained<Self>> {
            let this = this.set_ivars(WindowIvars::default());
            unsafe {
                msg_send![super(this),
                    initWithContentRect: rect,
                    styleMask: style_mask,
                    backing: backing,
                    defer: defer,
                    screen: screen
                ]
            }
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, _notification: &NSNotification) {
            callbacks::window_did_resize(self.ivars());
        }

        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn window_did_change_occlusion_state(&self, _notification: &NSNotification) {
            callbacks::window_did_change_occlusion_state(self.ivars());
        }

        #[unsafe(method(windowWillEnterFullScreen:))]
        fn window_will_enter_fullscreen(&self, _notification: &NSNotification) {
            callbacks::window_will_enter_fullscreen(self.ivars());
        }

        #[unsafe(method(windowWillExitFullScreen:))]
        fn window_will_exit_fullscreen(&self, _notification: &NSNotification) {
            callbacks::window_will_exit_fullscreen(self.ivars());
        }

        #[unsafe(method(windowDidMove:))]
        fn window_did_move(&self, _notification: &NSNotification) {
            callbacks::window_did_move(self.ivars());
        }

        #[unsafe(method(windowDidChangeScreen:))]
        fn window_did_change_screen(&self, _notification: &NSNotification) {
            callbacks::window_did_change_screen(self.ivars());
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSNotification) {
            callbacks::window_did_change_key_status(self.ivars(), sel!(windowDidBecomeKey:));
        }

        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _notification: &NSNotification) {
            callbacks::window_did_change_key_status(self.ivars(), sel!(windowDidResignKey:));
        }

        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _sender: &AnyObject) -> bool {
            callbacks::window_should_close(self.ivars())
        }

        #[unsafe(method(close))]
        fn close(&self) {
            callbacks::close_window(self.ivars(), self);
        }

        // Drag and drop
        #[unsafe(method(draggingEntered:))]
        fn dragging_entered(&self, sender: &AnyObject) -> NSDragOperation {
            callbacks_mouse::dragging_entered(self.ivars(), sender)
        }

        #[unsafe(method(draggingUpdated:))]
        fn dragging_updated(&self, sender: &AnyObject) -> NSDragOperation {
            callbacks_mouse::dragging_updated(self.ivars(), sender)
        }

        #[unsafe(method(draggingExited:))]
        fn dragging_exited(&self, _sender: Option<&AnyObject>) {
            callbacks_mouse::dragging_exited(self.ivars());
        }

        #[unsafe(method(performDragOperation:))]
        fn perform_drag_operation(&self, sender: &AnyObject) -> bool {
            callbacks_mouse::perform_drag_operation(self.ivars(), sender)
        }

        #[unsafe(method(concludeDragOperation:))]
        fn conclude_drag_operation(&self, _sender: Option<&AnyObject>) {
            callbacks_mouse::conclude_drag_operation(self.ivars());
        }

        // Tab management
        #[unsafe(method(addTitlebarAccessoryViewController:))]
        fn add_titlebar_accessory_view_controller(&self, view_controller: &AnyObject) {
            callbacks::add_titlebar_accessory_view_controller(self, view_controller);
        }

        #[unsafe(method(moveTabToNewWindow:))]
        fn move_tab_to_new_window(&self, _sender: Option<&AnyObject>) {
            callbacks::move_tab_to_new_window(self.ivars(), self);
        }

        #[unsafe(method(mergeAllWindows:))]
        fn merge_all_windows(&self, _sender: Option<&AnyObject>) {
            callbacks::merge_all_windows(self.ivars(), self);
        }

        #[unsafe(method(selectNextTab:))]
        fn select_next_tab(&self, _sender: Option<&AnyObject>) {
            callbacks::select_next_tab(self.ivars());
        }

        #[unsafe(method(selectPreviousTab:))]
        fn select_previous_tab(&self, _sender: Option<&AnyObject>) {
            callbacks::select_previous_tab(self.ivars());
        }

        #[unsafe(method(toggleTabBar:))]
        fn toggle_tab_bar(&self, _sender: Option<&AnyObject>) {
            callbacks::toggle_tab_bar(self.ivars(), self);
        }
    }

    unsafe impl NSObjectProtocol for GPUIPanel {}
    unsafe impl NSWindowDelegate for GPUIPanel {}
    unsafe impl NSDraggingDestination for GPUIPanel {}
);

// ─── GPUIView ───────────────────────────────────────────────────────────────

define_class!(
    #[unsafe(super(NSView))]
    #[name = "GPUIView"]
    #[ivars = WindowIvars]
    #[thread_kind = MainThreadOnly]
    pub(super) struct GPUIView;

    impl GPUIView {
        #[unsafe(method_id(initWithFrame:))]
        fn init_with_frame(
            this: objc2::rc::Allocated<Self>,
            frame: NSRect,
        ) -> Option<objc2::rc::Retained<Self>> {
            let this = this.set_ivars(WindowIvars::default());
            unsafe { msg_send![super(this), initWithFrame: frame] }
        }

        // Key events
        #[unsafe(method(performKeyEquivalent:))]
        fn perform_key_equivalent(&self, event: &objc2_app_kit::NSEvent) -> bool {
            callbacks::handle_key_equivalent(self.ivars(), self, event)
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_key_down(self.ivars(), self, event);
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_key_up(self.ivars(), self, event);
        }

        // Mouse events
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(otherMouseUp:))]
        fn other_mouse_up(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(pressureChangeWithEvent:))]
        fn pressure_change_with_event(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(magnifyWithEvent:))]
        fn magnify_with_event(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(swipeWithEvent:))]
        fn swipe_with_event(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &objc2_app_kit::NSEvent) {
            callbacks::handle_view_event(self.ivars(), self, event);
        }

        // Layer
        #[unsafe(method(makeBackingLayer))]
        fn make_backing_layer(&self) -> *mut AnyObject {
            callbacks::make_backing_layer(self.ivars())
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn view_did_change_backing_properties(&self) {
            callbacks::view_did_change_backing_properties(self.ivars());
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            callbacks::set_frame_size(self.ivars(), self, size);
        }

        #[unsafe(method(displayLayer:))]
        fn display_layer(&self, _layer: &AnyObject) {
            callbacks::display_layer(self.ivars());
        }

        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn view_did_change_effective_appearance(&self) {
            callbacks::view_did_change_effective_appearance(self.ivars());
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&objc2_app_kit::NSEvent>) -> bool {
            callbacks_mouse::accepts_first_mouse(self.ivars())
        }
    }

    unsafe impl NSObjectProtocol for GPUIView {}

    unsafe impl NSTextInputClient for GPUIView {
        #[unsafe(method(characterIndexForPoint:))]
        fn character_index_for_point(&self, point: NSPoint) -> u64 {
            callbacks_mouse::character_index_for_point(self.ivars(), point)
        }

        #[unsafe(method(validAttributesForMarkedText))]
        fn valid_attributes_for_marked_text(&self) -> *mut AnyObject {
            callbacks::valid_attributes_for_marked_text()
        }

        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            callbacks::has_marked_text(self.ivars())
        }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            callbacks::marked_range(self.ivars())
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            callbacks::selected_range(self.ivars())
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect_for_character_range(
            &self,
            range: NSRange,
            actual_range: *mut c_void,
        ) -> NSRect {
            callbacks::first_rect_for_character_range(self.ivars(), range, actual_range)
        }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, text: &AnyObject, replacement_range: NSRange) {
            callbacks::insert_text(self.ivars(), text, replacement_range);
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(
            &self,
            text: &AnyObject,
            selected_range: NSRange,
            replacement_range: NSRange,
        ) {
            callbacks::set_marked_text(self.ivars(), text, selected_range, replacement_range);
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            callbacks::unmark_text(self.ivars());
        }

        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        fn attributed_substring_for_proposed_range(
            &self,
            range: NSRange,
            actual_range: *mut c_void,
        ) -> *mut AnyObject {
            callbacks::attributed_substring_for_proposed_range(self.ivars(), range, actual_range)
        }

        // Suppress beep on keystrokes with modifier keys.
        #[unsafe(method(doCommandBySelector:))]
        fn do_command_by_selector(&self, _selector: objc2::runtime::Sel) {
            callbacks::do_command_by_selector(self.ivars());
        }
    }
);

// ─── BlurredView ────────────────────────────────────────────────────────────

define_class!(
    #[unsafe(super(NSVisualEffectView))]
    #[name = "BlurredView"]
    #[thread_kind = MainThreadOnly]
    pub(super) struct BlurredView;

    impl BlurredView {
        #[unsafe(method_id(initWithFrame:))]
        fn init_with_frame(this: objc2::rc::Allocated<Self>, frame: NSRect) -> Option<objc2::rc::Retained<Self>> {
            let this = this.set_ivars(());
            let this: Option<objc2::rc::Retained<Self>> = unsafe {
                objc2::msg_send![super(this), initWithFrame: frame]
            };
            if let Some(ref view) = this {
                view.setMaterial(NSVisualEffectMaterial::Selection);
                view.setState(NSVisualEffectState::Active);
            }
            this
        }

        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            callbacks_mouse::blurred_view_update_layer(self);
        }
    }

    unsafe impl NSObjectProtocol for BlurredView {}
);
