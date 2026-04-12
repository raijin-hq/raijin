//! Convenience crate that re-exports Inazuma's platform traits and the
//! `current_platform` constructor so consumers don't need `#[cfg]` gating.

pub use inazuma::Platform;

use std::rc::Rc;

/// Returns a background executor for the current platform.
pub fn background_executor() -> inazuma::BackgroundExecutor {
    current_platform(true).background_executor()
}

/// Creates an Application with the platform for the current OS.
pub fn application() -> inazuma::Application {
    inazuma::Application::with_platform(current_platform(false))
}

/// Creates a headless Application (no window, for CLI/test use).
pub fn headless() -> inazuma::Application {
    inazuma::Application::with_platform(current_platform(true))
}

/// Returns the platform implementation for the current OS.
fn current_platform(headless: bool) -> Rc<dyn inazuma::Platform> {
    inazuma::current_platform(headless)
}
