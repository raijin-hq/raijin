use super::*;

pub(super) fn accepts_first_mouse(ivars: &WindowIvars) -> bool {
    let window_state = ivars.get_state();
    let mut lock = window_state.as_ref().lock();
    lock.first_mouse = true;
    true
}

pub(super) fn character_index_for_point(ivars: &WindowIvars, position: NSPoint) -> u64 {
    let position = screen_point_to_gpui_point(ivars, position);
    with_input_handler(ivars, |input_handler| {
        input_handler.character_index_for_point(position)
    })
    .flatten()
    .map(|index| index as u64)
    .unwrap_or(usize::MAX as u64)
}

fn screen_point_to_gpui_point(ivars: &WindowIvars, position: NSPoint) -> Point<Pixels> {
    let frame = callbacks::get_frame(ivars);
    let window_x = position.x - frame.origin.x;
    let window_y = frame.size.height - (position.y - frame.origin.y);

    point(px(window_x as f32), px(window_y as f32))
}

pub(super) fn dragging_entered(ivars: &WindowIvars, dragging_info: &AnyObject) -> NSDragOperation {
    let window_state = ivars.get_state();
    let position = drag_event_position(&window_state, dragging_info);
    let paths = external_paths_from_event(dragging_info);
    if let Some(event) = paths.map(|paths| FileDropEvent::Entered { position, paths })
        && send_file_drop_event(window_state, event)
    {
        return NSDragOperationCopy;
    }
    NSDragOperationNone
}

pub(super) fn dragging_updated(ivars: &WindowIvars, dragging_info: &AnyObject) -> NSDragOperation {
    let window_state = ivars.get_state();
    let position = drag_event_position(&window_state, dragging_info);
    if send_file_drop_event(window_state, FileDropEvent::Pending { position }) {
        NSDragOperationCopy
    } else {
        NSDragOperationNone
    }
}

pub(super) fn dragging_exited(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    send_file_drop_event(window_state, FileDropEvent::Exited);
}

pub(super) fn perform_drag_operation(ivars: &WindowIvars, dragging_info: &AnyObject) -> bool {
    let window_state = ivars.get_state();
    let position = drag_event_position(&window_state, dragging_info);
    send_file_drop_event(window_state, FileDropEvent::Submit { position })
}

fn external_paths_from_event(dragging_info: &AnyObject) -> Option<ExternalPaths> {
    let mut paths = SmallVec::new();
    unsafe {
        let pasteboard: *mut AnyObject = msg_send![dragging_info, draggingPasteboard];
        #[allow(deprecated)]
        let filenames_type = objc2_app_kit::NSFilenamesPboardType;
        let filenames: *mut AnyObject =
            msg_send![pasteboard, propertyListForType: &**filenames_type];
        if filenames.is_null() {
            return None;
        }
        let count: usize = msg_send![filenames, count];
        for i in 0..count {
            let file: *mut AnyObject = msg_send![filenames, objectAtIndex: i];
            let ns_string: &objc2_foundation::NSString =
                &*(file as *const AnyObject as *const objc2_foundation::NSString);
            let path = ns_string.to_string();
            paths.push(PathBuf::from(path));
        }
    }
    Some(ExternalPaths(paths))
}

pub(super) fn conclude_drag_operation(ivars: &WindowIvars) {
    let window_state = ivars.get_state();
    send_file_drop_event(window_state, FileDropEvent::Exited);
}

/// Sends the specified FileDropEvent using `PlatformInput::FileDrop` to the window
/// state and updates the window state according to the event passed.
fn send_file_drop_event(
    window_state: Arc<Mutex<MacWindowState>>,
    file_drop_event: FileDropEvent,
) -> bool {
    let external_files_dragged = match file_drop_event {
        FileDropEvent::Entered { .. } => Some(true),
        FileDropEvent::Exited => Some(false),
        _ => None,
    };

    let mut lock = window_state.lock();
    if let Some(mut callback) = lock.event_callback.take() {
        drop(lock);
        callback(PlatformInput::FileDrop(file_drop_event));
        let mut lock = window_state.lock();
        lock.event_callback = Some(callback);
        if let Some(external_files_dragged) = external_files_dragged {
            lock.external_files_dragged = external_files_dragged;
        }
        true
    } else {
        false
    }
}

fn drag_event_position(
    window_state: &Mutex<MacWindowState>,
    dragging_info: &AnyObject,
) -> Point<Pixels> {
    let drag_location: NSPoint = unsafe { msg_send![dragging_info, draggingLocation] };
    convert_mouse_position(drag_location, window_state.lock().content_size().height)
}

pub(super) fn blurred_view_update_layer(this: &AnyObject) {
    unsafe {
        let _: () = msg_send![
            super(this, objc2_app_kit::NSVisualEffectView::class()),
            updateLayer
        ];
        let layer: *mut AnyObject = msg_send![this, layer];
        if !layer.is_null() {
            remove_layer_background(layer);
        }
    }
}

unsafe fn remove_layer_background(layer: *mut AnyObject) {
    unsafe {
        let _: () = msg_send![layer, setBackgroundColor: ptr::null_mut::<AnyObject>()];

        let class_name: *mut AnyObject = msg_send![layer, className];
        if !class_name.is_null() {
            let test_name = objc2_foundation::NSString::from_str("CAChameleonLayer");
            let is_equal: bool = msg_send![class_name, isEqualToString: &*test_name];
            if is_equal {
                // Remove the desktop tinting effect.
                let _: () = msg_send![layer, setHidden: true];
                return;
            }
        }

        let filters: *mut AnyObject = msg_send![layer, filters];
        if !filters.is_null() {
            // Remove the increased saturation.
            // The effect of a `CAFilter` or `CIFilter` is determined by its name, and the
            // `description` reflects its name and some parameters. Currently `NSVisualEffectView`
            // uses a `CAFilter` named "colorSaturate". If one day they switch to `CIFilter`, the
            // `description` will still contain "Saturat" ("... inputSaturation = ...").
            let test_string = objc2_foundation::NSString::from_str("Saturat");
            let count: usize = msg_send![filters, count];
            for i in 0..count {
                let item: *mut AnyObject = msg_send![filters, objectAtIndex: i];
                let description: *mut AnyObject = msg_send![item, description];
                let hit: bool = msg_send![description, containsString: &*test_string];
                if !hit {
                    continue;
                }

                let all_indices = NSRange {
                    location: 0,
                    length: count,
                };
                let indices: *mut AnyObject =
                    msg_send![objc2_foundation::NSMutableIndexSet::class(), indexSet];
                let _: () = msg_send![indices, addIndexesInRange: all_indices];
                let _: () = msg_send![indices, removeIndex: i];
                let filtered: *mut AnyObject = msg_send![filters, objectsAtIndexes: indices];
                let _: () = msg_send![layer, setFilters: filtered];
                break;
            }
        }

        let sublayers: *mut AnyObject = msg_send![layer, sublayers];
        if !sublayers.is_null() {
            let count: usize = msg_send![sublayers, count];
            for i in 0..count {
                let sublayer: *mut AnyObject = msg_send![sublayers, objectAtIndex: i];
                remove_layer_background(sublayer);
            }
        }
    }
}
