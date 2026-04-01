use super::*;

pub fn mode_refresh_rate(mode: &randr::ModeInfo) -> Duration {
    if mode.dot_clock == 0 || mode.htotal == 0 || mode.vtotal == 0 {
        return Duration::from_millis(16);
    }

    let millihertz = mode.dot_clock as u64 * 1_000 / (mode.htotal as u64 * mode.vtotal as u64);
    let micros = 1_000_000_000 / millihertz;
    log::info!("Refreshing every {}ms", micros / 1_000);
    Duration::from_micros(micros)
}

pub(super) fn fp3232_to_f32(value: xinput::Fp3232) -> f32 {
    value.integral as f32 + value.frac as f32 / u32::MAX as f32
}

pub(super) fn detect_compositor_gpu(
    xcb_connection: &XCBConnection,
    screen: &xproto::Screen,
) -> Option<CompositorGpuHint> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::MetadataExt;

    xcb_connection
        .extension_information(dri3::X11_EXTENSION_NAME)
        .ok()??;

    let reply = dri3::open(xcb_connection, screen.root, 0)
        .ok()?
        .reply()
        .ok()?;
    let fd = reply.device_fd;

    let path = format!("/proc/self/fd/{}", fd.as_raw_fd());
    let metadata = std::fs::metadata(&path).ok()?;

    crate::platform::linux::compositor_gpu_hint_from_dev_t(metadata.rdev())
}

pub(super) fn check_compositor_present(xcb_connection: &XCBConnection, root: xproto::Window) -> bool {
    // Method 1: Check for _NET_WM_CM_S{root}
    let atom_name = format!("_NET_WM_CM_S{}", root);
    let atom1 = get_reply(
        || format!("Failed to intern {atom_name}"),
        xcb_connection.intern_atom(false, atom_name.as_bytes()),
    );
    let method1 = match atom1.log_with_level(Level::Debug) {
        Some(reply) if reply.atom != x11rb::NONE => {
            let atom = reply.atom;
            get_reply(
                || format!("Failed to get {atom_name} owner"),
                xcb_connection.get_selection_owner(atom),
            )
            .map(|reply| reply.owner != 0)
            .log_with_level(Level::Debug)
            .unwrap_or(false)
        }
        _ => false,
    };

    // Method 2: Check for _NET_WM_CM_OWNER
    let atom_name = "_NET_WM_CM_OWNER";
    let atom2 = get_reply(
        || format!("Failed to intern {atom_name}"),
        xcb_connection.intern_atom(false, atom_name.as_bytes()),
    );
    let method2 = match atom2.log_with_level(Level::Debug) {
        Some(reply) if reply.atom != x11rb::NONE => {
            let atom = reply.atom;
            get_reply(
                || format!("Failed to get {atom_name}"),
                xcb_connection.get_property(false, root, atom, xproto::AtomEnum::WINDOW, 0, 1),
            )
            .map(|reply| reply.value_len > 0)
            .unwrap_or(false)
        }
        _ => return false,
    };

    // Method 3: Check for _NET_SUPPORTING_WM_CHECK
    let atom_name = "_NET_SUPPORTING_WM_CHECK";
    let atom3 = get_reply(
        || format!("Failed to intern {atom_name}"),
        xcb_connection.intern_atom(false, atom_name.as_bytes()),
    );
    let method3 = match atom3.log_with_level(Level::Debug) {
        Some(reply) if reply.atom != x11rb::NONE => {
            let atom = reply.atom;
            get_reply(
                || format!("Failed to get {atom_name}"),
                xcb_connection.get_property(false, root, atom, xproto::AtomEnum::WINDOW, 0, 1),
            )
            .map(|reply| reply.value_len > 0)
            .unwrap_or(false)
        }
        _ => return false,
    };

    log::debug!(
        "Compositor detection: _NET_WM_CM_S?={}, _NET_WM_CM_OWNER={}, _NET_SUPPORTING_WM_CHECK={}",
        method1,
        method2,
        method3
    );

    method1 || method2 || method3
}

pub(super) fn check_gtk_frame_extents_supported(
    xcb_connection: &XCBConnection,
    atoms: &XcbAtoms,
    root: xproto::Window,
) -> bool {
    let Some(supported_atoms) = get_reply(
        || "Failed to get _NET_SUPPORTED",
        xcb_connection.get_property(
            false,
            root,
            atoms._NET_SUPPORTED,
            xproto::AtomEnum::ATOM,
            0,
            1024,
        ),
    )
    .log_with_level(Level::Debug) else {
        return false;
    };

    let supported_atom_ids: Vec<u32> = supported_atoms
        .value
        .chunks_exact(4)
        .filter_map(|chunk| chunk.try_into().ok().map(u32::from_ne_bytes))
        .collect();

    supported_atom_ids.contains(&atoms._GTK_FRAME_EXTENTS)
}

pub(super) fn xdnd_is_atom_supported(atom: u32, atoms: &XcbAtoms) -> bool {
    atom == atoms.TEXT
        || atom == atoms.STRING
        || atom == atoms.UTF8_STRING
        || atom == atoms.TEXT_PLAIN
        || atom == atoms.TEXT_PLAIN_UTF8
        || atom == atoms.TextUriList
}

pub(super) fn xdnd_get_supported_atom(
    xcb_connection: &XCBConnection,
    supported_atoms: &XcbAtoms,
    target: xproto::Window,
) -> u32 {
    if let Some(reply) = get_reply(
        || "Failed to get XDnD supported atoms",
        xcb_connection.get_property(
            false,
            target,
            supported_atoms.XdndTypeList,
            AtomEnum::ANY,
            0,
            1024,
        ),
    )
    .log_with_level(Level::Warn)
        && let Some(atoms) = reply.value32()
    {
        for atom in atoms {
            if xdnd_is_atom_supported(atom, supported_atoms) {
                return atom;
            }
        }
    }
    0
}

pub(super) fn xdnd_send_finished(
    xcb_connection: &XCBConnection,
    atoms: &XcbAtoms,
    source: xproto::Window,
    target: xproto::Window,
) {
    let message = ClientMessageEvent {
        format: 32,
        window: target,
        type_: atoms.XdndFinished,
        data: ClientMessageData::from([source, 1, atoms.XdndActionCopy, 0, 0]),
        sequence: 0,
        response_type: xproto::CLIENT_MESSAGE_EVENT,
    };
    check_reply(
        || "Failed to send XDnD finished event",
        xcb_connection.send_event(false, target, EventMask::default(), message),
    )
    .log_err();
    xcb_connection.flush().log_err();
}

pub(super) fn xdnd_send_status(
    xcb_connection: &XCBConnection,
    atoms: &XcbAtoms,
    source: xproto::Window,
    target: xproto::Window,
    action: u32,
) {
    let message = ClientMessageEvent {
        format: 32,
        window: target,
        type_: atoms.XdndStatus,
        data: ClientMessageData::from([source, 1, 0, 0, action]),
        sequence: 0,
        response_type: xproto::CLIENT_MESSAGE_EVENT,
    };
    check_reply(
        || "Failed to send XDnD status event",
        xcb_connection.send_event(false, target, EventMask::default(), message),
    )
    .log_err();
    xcb_connection.flush().log_err();
}

/// Recomputes `pointer_device_states` by querying all pointer devices.
/// When a device is present in `scroll_values_to_preserve`, its value for `ScrollAxisState.scroll_value` is used.
pub(super) fn current_pointer_device_states(
    xcb_connection: &XCBConnection,
    scroll_values_to_preserve: &BTreeMap<xinput::DeviceId, PointerDeviceState>,
) -> Option<BTreeMap<xinput::DeviceId, PointerDeviceState>> {
    let devices_query_result = get_reply(
        || "Failed to query XInput devices",
        xcb_connection.xinput_xi_query_device(XINPUT_ALL_DEVICES),
    )
    .log_err()?;

    let mut pointer_device_states = BTreeMap::new();
    pointer_device_states.extend(
        devices_query_result
            .infos
            .iter()
            .filter(|info| is_pointer_device(info.type_))
            .filter_map(|info| {
                let scroll_data = info
                    .classes
                    .iter()
                    .filter_map(|class| class.data.as_scroll())
                    .copied()
                    .rev()
                    .collect::<Vec<_>>();
                let old_state = scroll_values_to_preserve.get(&info.deviceid);
                let old_horizontal = old_state.map(|state| &state.horizontal);
                let old_vertical = old_state.map(|state| &state.vertical);
                let horizontal = scroll_data
                    .iter()
                    .find(|data| data.scroll_type == xinput::ScrollType::HORIZONTAL)
                    .map(|data| scroll_data_to_axis_state(data, old_horizontal));
                let vertical = scroll_data
                    .iter()
                    .find(|data| data.scroll_type == xinput::ScrollType::VERTICAL)
                    .map(|data| scroll_data_to_axis_state(data, old_vertical));
                if horizontal.is_none() && vertical.is_none() {
                    None
                } else {
                    Some((
                        info.deviceid,
                        PointerDeviceState {
                            horizontal: horizontal.unwrap_or_else(Default::default),
                            vertical: vertical.unwrap_or_else(Default::default),
                        },
                    ))
                }
            }),
    );
    if pointer_device_states.is_empty() {
        log::error!("Found no xinput mouse pointers.");
    }
    Some(pointer_device_states)
}

/// Returns true if the device is a pointer device. Does not include pointer device groups.
pub(super) fn is_pointer_device(type_: xinput::DeviceType) -> bool {
    type_ == xinput::DeviceType::SLAVE_POINTER
}

pub(super) fn scroll_data_to_axis_state(
    data: &xinput::DeviceClassDataScroll,
    old_axis_state_with_valid_scroll_value: Option<&ScrollAxisState>,
) -> ScrollAxisState {
    ScrollAxisState {
        valuator_number: Some(data.number),
        multiplier: SCROLL_LINES / fp3232_to_f32(data.increment),
        scroll_value: old_axis_state_with_valid_scroll_value.and_then(|state| state.scroll_value),
    }
}

pub(super) fn reset_all_pointer_device_scroll_positions(
    pointer_device_states: &mut BTreeMap<xinput::DeviceId, PointerDeviceState>,
) {
    pointer_device_states
        .iter_mut()
        .for_each(|(_, device_state)| reset_pointer_device_scroll_positions(device_state));
}

pub(super) fn reset_pointer_device_scroll_positions(pointer: &mut PointerDeviceState) {
    pointer.horizontal.scroll_value = None;
    pointer.vertical.scroll_value = None;
}

/// Returns the scroll delta for a smooth scrolling motion event, or `None` if no scroll data is present.
pub(super) fn get_scroll_delta_and_update_state(
    pointer: &mut PointerDeviceState,
    event: &xinput::MotionEvent,
) -> Option<Point<f32>> {
    let delta_x = get_axis_scroll_delta_and_update_state(event, &mut pointer.horizontal);
    let delta_y = get_axis_scroll_delta_and_update_state(event, &mut pointer.vertical);
    if delta_x.is_some() || delta_y.is_some() {
        Some(Point::new(delta_x.unwrap_or(0.0), delta_y.unwrap_or(0.0)))
    } else {
        None
    }
}

pub(super) fn get_axis_scroll_delta_and_update_state(
    event: &xinput::MotionEvent,
    axis: &mut ScrollAxisState,
) -> Option<f32> {
    let axis_index = get_valuator_axis_index(&event.valuator_mask, axis.valuator_number?)?;
    if let Some(axis_value) = event.axisvalues.get(axis_index) {
        let new_scroll = fp3232_to_f32(*axis_value);
        let delta_scroll = axis
            .scroll_value
            .map(|old_scroll| (old_scroll - new_scroll) * axis.multiplier);
        axis.scroll_value = Some(new_scroll);
        delta_scroll
    } else {
        log::error!("Encountered invalid XInput valuator_mask, scrolling may not work properly.");
        None
    }
}

pub(super) fn make_scroll_wheel_event(
    position: Point<Pixels>,
    scroll_delta: Point<f32>,
    modifiers: Modifiers,
) -> inazuma::ScrollWheelEvent {
    // When shift is held down, vertical scrolling turns into horizontal scrolling.
    let delta = if modifiers.shift {
        Point {
            x: scroll_delta.y,
            y: 0.0,
        }
    } else {
        scroll_delta
    };
    inazuma::ScrollWheelEvent {
        position,
        delta: ScrollDelta::Lines(delta),
        modifiers,
        touch_phase: TouchPhase::default(),
    }
}

pub(super) fn create_invisible_cursor(
    connection: &XCBConnection,
) -> anyhow::Result<crate::platform::linux::x11::client::xproto::Cursor> {
    let empty_pixmap = connection.generate_id()?;
    let root = connection.setup().roots[0].root;
    connection.create_pixmap(1, empty_pixmap, root, 1, 1)?;

    let cursor = connection.generate_id()?;
    connection.create_cursor(cursor, empty_pixmap, empty_pixmap, 0, 0, 0, 0, 0, 0, 0, 0)?;

    connection.free_pixmap(empty_pixmap)?;

    xcb_flush(connection);
    Ok(cursor)
}

pub(super) enum DpiMode {
    Randr,
    Scale(f32),
    NotSet,
}

pub(super) fn get_scale_factor(
    connection: &XCBConnection,
    resource_database: &Database,
    screen_index: usize,
) -> f32 {
    let env_dpi = std::env::var(GPUI_X11_SCALE_FACTOR_ENV)
        .ok()
        .map(|var| {
            if var.to_lowercase() == "randr" {
                DpiMode::Randr
            } else if let Ok(scale) = var.parse::<f32>() {
                if valid_scale_factor(scale) {
                    DpiMode::Scale(scale)
                } else {
                    panic!(
                        "`{}` must be a positive normal number or `randr`. Got `{}`",
                        GPUI_X11_SCALE_FACTOR_ENV, var
                    );
                }
            } else if var.is_empty() {
                DpiMode::NotSet
            } else {
                panic!(
                    "`{}` must be a positive number or `randr`. Got `{}`",
                    GPUI_X11_SCALE_FACTOR_ENV, var
                );
            }
        })
        .unwrap_or(DpiMode::NotSet);

    match env_dpi {
        DpiMode::Scale(scale) => {
            log::info!(
                "Using scale factor from {}: {}",
                GPUI_X11_SCALE_FACTOR_ENV,
                scale
            );
            return scale;
        }
        DpiMode::Randr => {
            if let Some(scale) = get_randr_scale_factor(connection, screen_index) {
                log::info!(
                    "Using RandR scale factor from {}=randr: {}",
                    GPUI_X11_SCALE_FACTOR_ENV,
                    scale
                );
                return scale;
            }
            log::warn!("Failed to calculate RandR scale factor, falling back to default");
            return 1.0;
        }
        DpiMode::NotSet => {}
    }

    // TODO: Use scale factor from XSettings here

    if let Some(dpi) = resource_database
        .get_value::<f32>("Xft.dpi", "Xft.dpi")
        .ok()
        .flatten()
    {
        let scale = dpi / 96.0; // base dpi
        log::info!("Using scale factor from Xft.dpi: {}", scale);
        return scale;
    }

    if let Some(scale) = get_randr_scale_factor(connection, screen_index) {
        log::info!("Using RandR scale factor: {}", scale);
        return scale;
    }

    log::info!("Using default scale factor: 1.0");
    1.0
}

pub(super) fn get_randr_scale_factor(connection: &XCBConnection, screen_index: usize) -> Option<f32> {
    let root = connection.setup().roots.get(screen_index)?.root;

    let version_cookie = connection.randr_query_version(1, 6).ok()?;
    let version_reply = version_cookie.reply().ok()?;
    if version_reply.major_version < 1
        || (version_reply.major_version == 1 && version_reply.minor_version < 5)
    {
        return legacy_get_randr_scale_factor(connection, root); // for randr <1.5
    }

    let monitors_cookie = connection.randr_get_monitors(root, true).ok()?; // true for active only
    let monitors_reply = monitors_cookie.reply().ok()?;

    let mut fallback_scale: Option<f32> = None;
    for monitor in monitors_reply.monitors {
        if monitor.width_in_millimeters == 0 || monitor.height_in_millimeters == 0 {
            continue;
        }
        let scale_factor = get_dpi_factor(
            (monitor.width as u32, monitor.height as u32),
            (
                monitor.width_in_millimeters as u64,
                monitor.height_in_millimeters as u64,
            ),
        );
        if monitor.primary {
            return Some(scale_factor);
        } else if fallback_scale.is_none() {
            fallback_scale = Some(scale_factor);
        }
    }

    fallback_scale
}

pub(super) fn legacy_get_randr_scale_factor(connection: &XCBConnection, root: u32) -> Option<f32> {
    let primary_cookie = connection.randr_get_output_primary(root).ok()?;
    let primary_reply = primary_cookie.reply().ok()?;
    let primary_output = primary_reply.output;

    let primary_output_cookie = connection
        .randr_get_output_info(primary_output, x11rb::CURRENT_TIME)
        .ok()?;
    let primary_output_info = primary_output_cookie.reply().ok()?;

    // try primary
    if primary_output_info.connection == randr::Connection::CONNECTED
        && primary_output_info.mm_width > 0
        && primary_output_info.mm_height > 0
        && primary_output_info.crtc != 0
    {
        let crtc_cookie = connection
            .randr_get_crtc_info(primary_output_info.crtc, x11rb::CURRENT_TIME)
            .ok()?;
        let crtc_info = crtc_cookie.reply().ok()?;

        if crtc_info.width > 0 && crtc_info.height > 0 {
            let scale_factor = get_dpi_factor(
                (crtc_info.width as u32, crtc_info.height as u32),
                (
                    primary_output_info.mm_width as u64,
                    primary_output_info.mm_height as u64,
                ),
            );
            return Some(scale_factor);
        }
    }

    // fallback: full scan
    let resources_cookie = connection.randr_get_screen_resources_current(root).ok()?;
    let screen_resources = resources_cookie.reply().ok()?;

    let mut crtc_cookies = Vec::with_capacity(screen_resources.crtcs.len());
    for &crtc in &screen_resources.crtcs {
        if let Ok(cookie) = connection.randr_get_crtc_info(crtc, x11rb::CURRENT_TIME) {
            crtc_cookies.push((crtc, cookie));
        }
    }

    let mut crtc_infos: HashMap<randr::Crtc, randr::GetCrtcInfoReply> = HashMap::default();
    let mut valid_outputs: HashSet<randr::Output> = HashSet::new();
    for (crtc, cookie) in crtc_cookies {
        if let Ok(reply) = cookie.reply()
            && reply.width > 0
            && reply.height > 0
            && !reply.outputs.is_empty()
        {
            crtc_infos.insert(crtc, reply.clone());
            valid_outputs.extend(&reply.outputs);
        }
    }

    if valid_outputs.is_empty() {
        return None;
    }

    let mut output_cookies = Vec::with_capacity(valid_outputs.len());
    for &output in &valid_outputs {
        if let Ok(cookie) = connection.randr_get_output_info(output, x11rb::CURRENT_TIME) {
            output_cookies.push((output, cookie));
        }
    }
    let mut output_infos: HashMap<randr::Output, randr::GetOutputInfoReply> = HashMap::default();
    for (output, cookie) in output_cookies {
        if let Ok(reply) = cookie.reply() {
            output_infos.insert(output, reply);
        }
    }

    let mut fallback_scale: Option<f32> = None;
    for crtc_info in crtc_infos.values() {
        for &output in &crtc_info.outputs {
            if let Some(output_info) = output_infos.get(&output) {
                if output_info.connection != randr::Connection::CONNECTED {
                    continue;
                }

                if output_info.mm_width == 0 || output_info.mm_height == 0 {
                    continue;
                }

                let scale_factor = get_dpi_factor(
                    (crtc_info.width as u32, crtc_info.height as u32),
                    (output_info.mm_width as u64, output_info.mm_height as u64),
                );

                if output != primary_output && fallback_scale.is_none() {
                    fallback_scale = Some(scale_factor);
                }
            }
        }
    }

    fallback_scale
}

pub(super) fn get_dpi_factor((width_px, height_px): (u32, u32), (width_mm, height_mm): (u64, u64)) -> f32 {
    let ppmm = ((width_px as f64 * height_px as f64) / (width_mm as f64 * height_mm as f64)).sqrt(); // pixels per mm

    const MM_PER_INCH: f64 = 25.4;
    const BASE_DPI: f64 = 96.0;
    const QUANTIZE_STEP: f64 = 12.0; // e.g. 1.25 = 15/12, 1.5 = 18/12, 1.75 = 21/12, 2.0 = 24/12
    const MIN_SCALE: f64 = 1.0;
    const MAX_SCALE: f64 = 20.0;

    let dpi_factor =
        ((ppmm * (QUANTIZE_STEP * MM_PER_INCH / BASE_DPI)).round() / QUANTIZE_STEP).max(MIN_SCALE);

    let validated_factor = if dpi_factor <= MAX_SCALE {
        dpi_factor
    } else {
        MIN_SCALE
    };

    if valid_scale_factor(validated_factor as f32) {
        validated_factor as f32
    } else {
        log::warn!(
            "Calculated DPI factor {} is invalid, using 1.0",
            validated_factor
        );
        1.0
    }
}

#[inline]
pub(super) fn valid_scale_factor(scale_factor: f32) -> bool {
    scale_factor.is_sign_positive() && scale_factor.is_normal()
}

#[inline]
pub(super) fn update_xkb_mask_from_event_state(xkb: &mut xkbc::State, event_state: xproto::KeyButMask) {
    let depressed_mods = event_state.remove((ModMask::LOCK | ModMask::M2).bits());
    let latched_mods = xkb.serialize_mods(xkbc::STATE_MODS_LATCHED);
    let locked_mods = xkb.serialize_mods(xkbc::STATE_MODS_LOCKED);
    let locked_layout = xkb.serialize_layout(xkbc::STATE_LAYOUT_LOCKED);
    xkb.update_mask(
        depressed_mods.into(),
        latched_mods,
        locked_mods,
        0,
        0,
        locked_layout,
    );
}
