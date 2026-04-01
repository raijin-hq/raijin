use super::*;

pub(crate) struct X11Window(pub X11WindowStatePtr);

impl Drop for X11Window {
    fn drop(&mut self) {
        let mut state = self.0.state.borrow_mut();

        if let Some(parent) = state.parent.as_ref() {
            parent.state.borrow_mut().children.remove(&self.0.x_window);
        }

        state.renderer.destroy();

        let destroy_x_window = maybe!({
            check_reply(
                || "X11 DestroyWindow failure.",
                self.0.xcb.destroy_window(self.0.x_window),
            )?;
            xcb_flush(&self.0.xcb);

            anyhow::Ok(())
        })
        .log_err();

        if destroy_x_window.is_some() {
            state.destroyed = true;

            let this_ptr = self.0.clone();
            let client_ptr = state.client.clone();
            state
                .executor
                .spawn(async move {
                    this_ptr.close();
                    client_ptr.drop_window(this_ptr.x_window);
                })
                .detach();
        }

        drop(state);
    }
}

enum WmHintPropertyState {
    // Remove = 0,
    // Add = 1,
    Toggle = 2,
}

impl X11Window {
    pub fn new(
        handle: AnyWindowHandle,
        client: X11ClientStatePtr,
        executor: ForegroundExecutor,
        gpu_context: crate::platform::wgpu::GpuContext,
        compositor_gpu: Option<CompositorGpuHint>,
        params: WindowParams,
        xcb: &Rc<XCBConnection>,
        client_side_decorations_supported: bool,
        x_main_screen_index: usize,
        x_window: xproto::Window,
        atoms: &XcbAtoms,
        scale_factor: f32,
        appearance: WindowAppearance,
        parent_window: Option<X11WindowStatePtr>,
    ) -> anyhow::Result<Self> {
        let ptr = X11WindowStatePtr {
            state: Rc::new(RefCell::new(X11WindowState::new(
                handle,
                client,
                executor,
                gpu_context,
                compositor_gpu,
                params,
                xcb,
                client_side_decorations_supported,
                x_main_screen_index,
                x_window,
                atoms,
                scale_factor,
                appearance,
                parent_window,
            )?)),
            callbacks: Rc::new(RefCell::new(Callbacks::default())),
            xcb: xcb.clone(),
            x_window,
        };

        let state = ptr.state.borrow_mut();
        ptr.set_wm_properties(state)?;

        Ok(Self(ptr))
    }

    fn set_wm_hints<C: Display + Send + Sync + 'static, F: FnOnce() -> C>(
        &self,
        failure_context: F,
        wm_hint_property_state: WmHintPropertyState,
        prop1: u32,
        prop2: u32,
    ) -> anyhow::Result<()> {
        let state = self.0.state.borrow();
        let message = ClientMessageEvent::new(
            32,
            self.0.x_window,
            state.atoms._NET_WM_STATE,
            [wm_hint_property_state as u32, prop1, prop2, 1, 0],
        );
        check_reply(
            failure_context,
            self.0.xcb.send_event(
                false,
                state.x_root_window,
                xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                message,
            ),
        )?;
        xcb_flush(&self.0.xcb);
        Ok(())
    }

    fn get_root_position(
        &self,
        position: Point<Pixels>,
    ) -> anyhow::Result<TranslateCoordinatesReply> {
        let state = self.0.state.borrow();
        get_reply(
            || "X11 TranslateCoordinates failed.",
            self.0.xcb.translate_coordinates(
                self.0.x_window,
                state.x_root_window,
                (f32::from(position.x) * state.scale_factor) as i16,
                (f32::from(position.y) * state.scale_factor) as i16,
            ),
        )
    }

    fn send_moveresize(&self, flag: u32) -> anyhow::Result<()> {
        let state = self.0.state.borrow();

        check_reply(
            || "X11 UngrabPointer before move/resize of window failed.",
            self.0.xcb.ungrab_pointer(x11rb::CURRENT_TIME),
        )?;

        let pointer = get_reply(
            || "X11 QueryPointer before move/resize of window failed.",
            self.0.xcb.query_pointer(self.0.x_window),
        )?;
        let message = ClientMessageEvent::new(
            32,
            self.0.x_window,
            state.atoms._NET_WM_MOVERESIZE,
            [
                pointer.root_x as u32,
                pointer.root_y as u32,
                flag,
                0, // Left mouse button
                0,
            ],
        );
        check_reply(
            || "X11 SendEvent to move/resize window failed.",
            self.0.xcb.send_event(
                false,
                state.x_root_window,
                xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                message,
            ),
        )?;

        xcb_flush(&self.0.xcb);
        Ok(())
    }
}

