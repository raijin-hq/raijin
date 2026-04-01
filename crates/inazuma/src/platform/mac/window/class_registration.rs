use super::*;

#[ctor]
unsafe fn build_classes() {
    unsafe {
        WINDOW_CLASS = build_window_class("GPUIWindow", class!(NSWindow));
        PANEL_CLASS = build_window_class("GPUIPanel", class!(NSPanel));
        VIEW_CLASS = {
            let mut decl = ClassDecl::new("GPUIView", class!(NSView)).unwrap();
            decl.add_ivar::<*mut c_void>(WINDOW_STATE_IVAR);
            unsafe {
                decl.add_method(sel!(dealloc), dealloc_view as extern "C" fn(&Object, Sel));

                decl.add_method(
                    sel!(performKeyEquivalent:),
                    handle_key_equivalent as extern "C" fn(&Object, Sel, id) -> BOOL,
                );
                decl.add_method(
                    sel!(keyDown:),
                    handle_key_down as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(keyUp:),
                    handle_key_up as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(mouseDown:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(mouseUp:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(rightMouseDown:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(rightMouseUp:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(otherMouseDown:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(otherMouseUp:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(mouseMoved:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(pressureChangeWithEvent:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(mouseExited:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(magnifyWithEvent:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(mouseDragged:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(scrollWheel:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(swipeWithEvent:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );
                decl.add_method(
                    sel!(flagsChanged:),
                    handle_view_event as extern "C" fn(&Object, Sel, id),
                );

                decl.add_method(
                    sel!(makeBackingLayer),
                    make_backing_layer as extern "C" fn(&Object, Sel) -> id,
                );

                decl.add_protocol(Protocol::get("CALayerDelegate").unwrap());
                decl.add_method(
                    sel!(viewDidChangeBackingProperties),
                    view_did_change_backing_properties as extern "C" fn(&Object, Sel),
                );
                decl.add_method(
                    sel!(setFrameSize:),
                    set_frame_size as extern "C" fn(&Object, Sel, NSSize),
                );
                decl.add_method(
                    sel!(displayLayer:),
                    display_layer as extern "C" fn(&Object, Sel, id),
                );

                decl.add_protocol(Protocol::get("NSTextInputClient").unwrap());
                decl.add_method(
                    sel!(validAttributesForMarkedText),
                    valid_attributes_for_marked_text as extern "C" fn(&Object, Sel) -> id,
                );
                decl.add_method(
                    sel!(hasMarkedText),
                    has_marked_text as extern "C" fn(&Object, Sel) -> BOOL,
                );
                decl.add_method(
                    sel!(markedRange),
                    marked_range as extern "C" fn(&Object, Sel) -> NSRange,
                );
                decl.add_method(
                    sel!(selectedRange),
                    selected_range as extern "C" fn(&Object, Sel) -> NSRange,
                );
                decl.add_method(
                    sel!(firstRectForCharacterRange:actualRange:),
                    first_rect_for_character_range
                        as extern "C" fn(&Object, Sel, NSRange, id) -> NSRect,
                );
                decl.add_method(
                    sel!(insertText:replacementRange:),
                    insert_text as extern "C" fn(&Object, Sel, id, NSRange),
                );
                decl.add_method(
                    sel!(setMarkedText:selectedRange:replacementRange:),
                    set_marked_text as extern "C" fn(&Object, Sel, id, NSRange, NSRange),
                );
                decl.add_method(sel!(unmarkText), unmark_text as extern "C" fn(&Object, Sel));
                decl.add_method(
                    sel!(attributedSubstringForProposedRange:actualRange:),
                    attributed_substring_for_proposed_range
                        as extern "C" fn(&Object, Sel, NSRange, *mut c_void) -> id,
                );
                decl.add_method(
                    sel!(viewDidChangeEffectiveAppearance),
                    view_did_change_effective_appearance as extern "C" fn(&Object, Sel),
                );

                // Suppress beep on keystrokes with modifier keys.
                decl.add_method(
                    sel!(doCommandBySelector:),
                    do_command_by_selector as extern "C" fn(&Object, Sel, Sel),
                );

                decl.add_method(
                    sel!(acceptsFirstMouse:),
                    accepts_first_mouse as extern "C" fn(&Object, Sel, id) -> BOOL,
                );

                decl.add_method(
                    sel!(characterIndexForPoint:),
                    character_index_for_point as extern "C" fn(&Object, Sel, NSPoint) -> u64,
                );
            }
            decl.register()
        };
        BLURRED_VIEW_CLASS = {
            let mut decl = ClassDecl::new("BlurredView", class!(NSVisualEffectView)).unwrap();
            unsafe {
                decl.add_method(
                    sel!(initWithFrame:),
                    blurred_view_init_with_frame as extern "C" fn(&Object, Sel, NSRect) -> id,
                );
                decl.add_method(
                    sel!(updateLayer),
                    blurred_view_update_layer as extern "C" fn(&Object, Sel),
                );
                decl.register()
            }
        };
    }
}

unsafe fn build_window_class(name: &'static str, superclass: &Class) -> *const Class {
    unsafe {
        let mut decl = ClassDecl::new(name, superclass).unwrap();
        decl.add_ivar::<*mut c_void>(WINDOW_STATE_IVAR);
        decl.add_method(sel!(dealloc), dealloc_window as extern "C" fn(&Object, Sel));

        decl.add_method(
            sel!(canBecomeMainWindow),
            yes as extern "C" fn(&Object, Sel) -> BOOL,
        );
        decl.add_method(
            sel!(canBecomeKeyWindow),
            yes as extern "C" fn(&Object, Sel) -> BOOL,
        );
        decl.add_method(
            sel!(windowDidResize:),
            window_did_resize as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowDidChangeOcclusionState:),
            window_did_change_occlusion_state as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowWillEnterFullScreen:),
            window_will_enter_fullscreen as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowWillExitFullScreen:),
            window_will_exit_fullscreen as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowDidMove:),
            window_did_move as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowDidChangeScreen:),
            window_did_change_screen as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowDidBecomeKey:),
            window_did_change_key_status as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowDidResignKey:),
            window_did_change_key_status as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(windowShouldClose:),
            window_should_close as extern "C" fn(&Object, Sel, id) -> BOOL,
        );

        decl.add_method(sel!(close), close_window as extern "C" fn(&Object, Sel));

        decl.add_method(
            sel!(draggingEntered:),
            dragging_entered as extern "C" fn(&Object, Sel, id) -> NSDragOperation,
        );
        decl.add_method(
            sel!(draggingUpdated:),
            dragging_updated as extern "C" fn(&Object, Sel, id) -> NSDragOperation,
        );
        decl.add_method(
            sel!(draggingExited:),
            dragging_exited as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(performDragOperation:),
            perform_drag_operation as extern "C" fn(&Object, Sel, id) -> BOOL,
        );
        decl.add_method(
            sel!(concludeDragOperation:),
            conclude_drag_operation as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(addTitlebarAccessoryViewController:),
            add_titlebar_accessory_view_controller as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(moveTabToNewWindow:),
            move_tab_to_new_window as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(mergeAllWindows:),
            merge_all_windows as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(selectNextTab:),
            select_next_tab as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(selectPreviousTab:),
            select_previous_tab as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(toggleTabBar:),
            toggle_tab_bar as extern "C" fn(&Object, Sel, id),
        );

        decl.register()
    }
}
