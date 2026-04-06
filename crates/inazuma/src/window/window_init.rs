use crate::{
    App, Bounds, DispatchTree,
    Pixels, ScaledPixels, Subscription, SystemWindowTabController,
    WindowOptions, WindowParams, px,
};
use anyhow::{Result};
use collections::FxHashSet;
use smallvec::SmallVec;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;

use super::*;

impl Window {
    pub(crate) fn new(
        handle: AnyWindowHandle,
        options: WindowOptions,
        cx: &mut App,
    ) -> Result<Self> {
        let WindowOptions {
            window_bounds,
            titlebar,
            focus,
            show,
            kind,
            is_movable,
            is_resizable,
            is_minimizable,
            display_id,
            window_background,
            app_id,
            window_min_size,
            window_decorations,
            colorspace,
            #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
            tabbing_identifier,
        } = options;

        let window_bounds = window_bounds.unwrap_or_else(|| default_bounds(display_id, cx));
        let mut platform_window = cx.platform.open_window(
            handle,
            WindowParams {
                bounds: window_bounds.get_bounds(),
                titlebar,
                kind,
                is_movable,
                is_resizable,
                is_minimizable,
                focus,
                show,
                display_id,
                window_min_size,
                colorspace,
                #[cfg(target_os = "macos")]
                tabbing_identifier,
            },
        )?;

        let tab_bar_visible = platform_window.tab_bar_visible();
        SystemWindowTabController::init_visible(cx, tab_bar_visible);
        if let Some(tabs) = platform_window.tabbed_windows() {
            SystemWindowTabController::add_tab(cx, handle.window_id(), tabs);
        }

        let display_id = platform_window.display().map(|display| display.id());
        let sprite_atlas = platform_window.sprite_atlas();
        let mouse_position = platform_window.mouse_position();
        let modifiers = platform_window.modifiers();
        let capslock = platform_window.capslock();
        let content_size = platform_window.content_size();
        let scale_factor = platform_window.scale_factor();
        let appearance = platform_window.appearance();
        let text_system = Arc::new(WindowTextSystem::new(cx.text_system().clone()));
        let invalidator = WindowInvalidator::new();
        let active = Rc::new(Cell::new(platform_window.is_active()));
        let hovered = Rc::new(Cell::new(platform_window.is_hovered()));
        let needs_present = Rc::new(Cell::new(false));
        let next_frame_callbacks: Rc<RefCell<Vec<FrameCallback>>> = Default::default();
        let input_rate_tracker = Rc::new(RefCell::new(InputRateTracker::default()));
        let last_frame_time = Rc::new(Cell::new(None));

        platform_window
            .request_decorations(window_decorations.unwrap_or(WindowDecorations::Server));
        platform_window.set_background_appearance(window_background);

        match window_bounds {
            WindowBounds::Fullscreen(_) => platform_window.toggle_fullscreen(),
            WindowBounds::Maximized(_) => platform_window.zoom(),
            WindowBounds::Windowed(_) => {}
        }

        platform_window.on_close(Box::new({
            let window_id = handle.window_id();
            let mut cx = cx.to_async();
            move || {
                let _ = handle.update(&mut cx, |_, window, _| window.remove_window());
                let _ = cx.update(|cx| {
                    SystemWindowTabController::remove_tab(cx, window_id);
                });
            }
        }));
        platform_window.on_request_frame(Box::new({
            let mut cx = cx.to_async();
            let invalidator = invalidator.clone();
            let active = active.clone();
            let needs_present = needs_present.clone();
            let next_frame_callbacks = next_frame_callbacks.clone();
            let input_rate_tracker = input_rate_tracker.clone();
            move |request_frame_options| {
                let thermal_state = handle
                    .update(&mut cx, |_, _, cx| cx.thermal_state())
                    .log_err();

                if thermal_state == Some(ThermalState::Serious)
                    || thermal_state == Some(ThermalState::Critical)
                {
                    let now = Instant::now();
                    let last_frame_time = last_frame_time.replace(Some(now));

                    if let Some(last_frame) = last_frame_time
                        && now.duration_since(last_frame) < Duration::from_micros(16667)
                    {
                        return;
                    }
                }

                let next_frame_callbacks = next_frame_callbacks.take();
                if !next_frame_callbacks.is_empty() {
                    handle
                        .update(&mut cx, |_, window, cx| {
                            for callback in next_frame_callbacks {
                                callback(window, cx);
                            }
                        })
                        .log_err();
                }

                // Keep presenting if input was recently arriving at a high rate (>= 60fps).
                // Once high-rate input is detected, we sustain presentation for 1 second
                // to prevent display underclocking during active input.
                let needs_present = request_frame_options.require_presentation
                    || needs_present.get()
                    || (active.get() && input_rate_tracker.borrow_mut().is_high_rate());

                if invalidator.is_dirty() || request_frame_options.force_render {
                    measure("frame duration", || {
                        handle
                            .update(&mut cx, |_, window, cx| {
                                let arena_clear_needed = window.draw(cx);
                                window.present();
                                arena_clear_needed.clear();
                            })
                            .log_err();
                    })
                } else if needs_present {
                    handle
                        .update(&mut cx, |_, window, _| window.present())
                        .log_err();
                }

                handle
                    .update(&mut cx, |_, window, _| {
                        window.complete_frame();
                    })
                    .log_err();
            }
        }));
        platform_window.on_resize(Box::new({
            let mut cx = cx.to_async();
            move |_, _| {
                handle
                    .update(&mut cx, |_, window, cx| window.bounds_changed(cx))
                    .log_err();
            }
        }));
        platform_window.on_moved(Box::new({
            let mut cx = cx.to_async();
            move || {
                handle
                    .update(&mut cx, |_, window, cx| window.bounds_changed(cx))
                    .log_err();
            }
        }));
        platform_window.on_appearance_changed(Box::new({
            let mut cx = cx.to_async();
            move || {
                handle
                    .update(&mut cx, |_, window, cx| window.appearance_changed(cx))
                    .log_err();
            }
        }));
        platform_window.on_active_status_change(Box::new({
            let mut cx = cx.to_async();
            move |active| {
                handle
                    .update(&mut cx, |_, window, cx| {
                        window.active.set(active);
                        window.modifiers = window.platform_window.modifiers();
                        window.capslock = window.platform_window.capslock();
                        window
                            .activation_observers
                            .clone()
                            .retain(&(), |callback| callback(window, cx));

                        window.bounds_changed(cx);
                        window.refresh();

                        SystemWindowTabController::update_last_active(cx, window.handle.id);
                    })
                    .log_err();
            }
        }));
        platform_window.on_hover_status_change(Box::new({
            let mut cx = cx.to_async();
            move |active| {
                handle
                    .update(&mut cx, |_, window, _| {
                        window.hovered.set(active);
                        window.refresh();
                    })
                    .log_err();
            }
        }));
        platform_window.on_input({
            let mut cx = cx.to_async();
            Box::new(move |event| {
                handle
                    .update(&mut cx, |_, window, cx| window.dispatch_event(event, cx))
                    .log_err()
                    .unwrap_or(DispatchEventResult::default())
            })
        });
        platform_window.on_hit_test_window_control({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, window, _cx| {
                        for (area, hitbox) in &window.rendered_frame.window_control_hitboxes {
                            if window.mouse_hit_test.ids.contains(&hitbox.id) {
                                return Some(*area);
                            }
                        }
                        None
                    })
                    .log_err()
                    .unwrap_or(None)
            })
        });
        platform_window.on_move_tab_to_new_window({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::move_tab_to_new_window(cx, handle.window_id());
                    })
                    .log_err();
            })
        });
        platform_window.on_merge_all_windows({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::merge_all_windows(cx, handle.window_id());
                    })
                    .log_err();
            })
        });
        platform_window.on_select_next_tab({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::select_next_tab(cx, handle.window_id());
                    })
                    .log_err();
            })
        });
        platform_window.on_select_previous_tab({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, _window, cx| {
                        SystemWindowTabController::select_previous_tab(cx, handle.window_id())
                    })
                    .log_err();
            })
        });
        platform_window.on_toggle_tab_bar({
            let mut cx = cx.to_async();
            Box::new(move || {
                handle
                    .update(&mut cx, |_, window, cx| {
                        let tab_bar_visible = window.platform_window.tab_bar_visible();
                        SystemWindowTabController::set_visible(cx, tab_bar_visible);
                    })
                    .log_err();
            })
        });

        if let Some(app_id) = app_id {
            platform_window.set_app_id(&app_id);
        }

        platform_window.map_window().unwrap();

        Ok(Window {
            handle,
            invalidator,
            removed: false,
            platform_window,
            display_id,
            sprite_atlas,
            text_system,
            text_rendering_mode: cx.text_rendering_mode.clone(),
            rem_size: px(16.),
            rem_size_override_stack: SmallVec::new(),
            viewport_size: content_size,
            layout_engine: Some(TaffyLayoutEngine::new()),
            root: None,
            element_id_stack: SmallVec::default(),
            text_style_stack: Vec::new(),
            rendered_entity_stack: Vec::new(),
            element_offset_stack: Vec::new(),
            content_mask_stack: Vec::new(),
            element_opacity: 1.0,
            requested_autoscroll: None,
            rendered_frame: Frame::new(DispatchTree::new(cx.keymap.clone(), cx.actions.clone())),
            next_frame: Frame::new(DispatchTree::new(cx.keymap.clone(), cx.actions.clone())),
            next_frame_callbacks,
            next_hitbox_id: HitboxId(0),
            next_tooltip_id: TooltipId::default(),
            tooltip_bounds: None,
            dirty_views: FxHashSet::default(),
            focus_listeners: SubscriberSet::new(),
            focus_lost_listeners: SubscriberSet::new(),
            default_prevented: true,
            mouse_position,
            mouse_hit_test: HitTest::default(),
            modifiers,
            capslock,
            scale_factor,
            bounds_observers: SubscriberSet::new(),
            appearance,
            appearance_observers: SubscriberSet::new(),
            active,
            hovered,
            needs_present,
            input_rate_tracker,
            last_input_modality: InputModality::Mouse,
            refreshing: false,
            activation_observers: SubscriberSet::new(),
            focus: None,
            focus_enabled: true,
            pending_input: None,
            pending_modifier: ModifierState::default(),
            pending_input_observers: SubscriberSet::new(),
            prompt: None,
            client_inset: None,
            image_cache_stack: Vec::new(),
            captured_hitbox: None,
            #[cfg(any(feature = "inspector", debug_assertions))]
            inspector: None,
        })
    }

    pub(crate) fn new_focus_listener(
        &self,
        value: AnyWindowFocusListener,
    ) -> (Subscription, impl FnOnce() + use<>) {
        self.focus_listeners.insert((), value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(missing_docs)]
pub struct DispatchEventResult {
    pub propagate: bool,
    pub default_prevented: bool,
}

/// Indicates which region of the window is visible. Content falling outside of this mask will not be
/// rendered. Currently, only rectangular content masks are supported, but we give the mask its own type
/// to leave room to support more complex shapes in the future.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct ContentMask<P: Clone + Debug + Default + PartialEq> {
    /// The bounds
    pub bounds: Bounds<P>,
}

impl ContentMask<Pixels> {
    /// Scale the content mask's pixel units by the given scaling factor.
    pub fn scale(&self, factor: f32) -> ContentMask<ScaledPixels> {
        ContentMask {
            bounds: self.bounds.scale(factor),
        }
    }

    /// Intersect the content mask with the given content mask.
    pub fn intersect(&self, other: &Self) -> Self {
        let bounds = self.bounds.intersect(&other.bounds);
        ContentMask { bounds }
    }
}
