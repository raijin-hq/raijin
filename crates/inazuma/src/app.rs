// Pre-existing sub-modules
mod async_context;
mod context;
mod entity_map;
#[cfg(any(test, feature = "test-support"))]
mod headless_app_context;
#[cfg(any(test, feature = "test-support"))]
mod test_app;
#[cfg(any(test, feature = "test-support"))]
mod test_context;
#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
mod visual_test_context;

// New sub-modules
mod actions;
mod app_context_impl;
mod app_struct;
mod application;
mod effects;
mod globals;
mod observers;
mod platform;
mod types;
mod window_management;

// Re-exports from pre-existing sub-modules
pub use async_context::*;
pub use context::*;
pub use entity_map::*;
#[cfg(any(test, feature = "test-support"))]
pub use headless_app_context::*;
#[cfg(any(test, feature = "test-support"))]
pub use test_app::*;
#[cfg(any(test, feature = "test-support"))]
pub use test_context::*;
#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
pub use visual_test_context::*;

// Re-exports from new sub-modules
pub use app_struct::{App, SHUTDOWN_TIMEOUT};
pub use application::{Application, SystemWindowTab, SystemWindowTabController};
pub use types::{
    AnyDrag, AnyTooltip, AppCell, AppRef, AppRefMut, GpuiBorrow, KeystrokeEvent, QuitMode,
};

// pub(crate) re-exports needed by the rest of the crate
pub(crate) use types::{Effect, GlobalLease, GpuiMode, KeystrokeObserver};

// Re-imports into this module's namespace so sub-modules can use `super::Handler` etc.
use types::{
    Handler, Listener, NewEntityListener, NullHttpClient, QuitHandler, ReleaseListener,
    WindowClosedHandler,
};

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::{AppContext, TestAppContext};

    #[test]
    fn test_gpui_borrow() {
        let cx = TestAppContext::single();
        let observation_count = Rc::new(RefCell::new(0));

        let state = cx.update(|cx| {
            let state = cx.new(|_| false);
            cx.observe(&state, {
                let observation_count = observation_count.clone();
                move |_, _| {
                    let mut count = observation_count.borrow_mut();
                    *count += 1;
                }
            })
            .detach();

            state
        });

        cx.update(|cx| {
            // Calling this like this so that we don't clobber the borrow_mut above
            *std::borrow::BorrowMut::borrow_mut(&mut state.as_mut(cx)) = true;
        });

        cx.update(|cx| {
            state.write(cx, false);
        });

        assert_eq!(*observation_count.borrow(), 2);
    }
}
