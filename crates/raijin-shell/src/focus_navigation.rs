use inazuma::{Context, Window};

use raijin_ui::utils::focus_trap::FocusTrapManager;
use super::shell::{AppShell, Tab, TabPrev};

impl AppShell {
    pub(super) fn on_action_tab(
        &mut self,
        _: &Tab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(container_focus_handle) = FocusTrapManager::find_active_trap(window, cx) {
            let before_focus = window.focused(cx);
            window.focus_next(cx);

            if !container_focus_handle.contains_focused(window, cx) {
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100;

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_next(cx);
                    attempts += 1;

                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        window.focus_next(cx);
    }

    pub(super) fn on_action_tab_prev(
        &mut self,
        _: &TabPrev,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(container_focus_handle) = FocusTrapManager::find_active_trap(window, cx) {
            let before_focus = window.focused(cx);
            window.focus_prev(cx);

            if !container_focus_handle.contains_focused(window, cx) {
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100;

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_prev(cx);
                    attempts += 1;

                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        window.focus_prev(cx);
    }
}
