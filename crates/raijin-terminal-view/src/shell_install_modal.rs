/// Shell installation modal — themed UI for installing missing shells.
///
/// Uses raijin_shell::shell_install for platform detection and install commands.
/// Uses raijin-ui Modal components and theme tokens — no hardcoded colors.
use inazuma::{
    App, DismissEvent, EventEmitter, Focusable, FocusHandle, InteractiveElement,
    Window,
};
use raijin_shell::shell_install::{ShellInstallInfo, detect_available_installer};
use raijin_ui::{
    Button, ButtonVariants, Context, IntoElement,
    Modal, ModalFooter, ModalHeader, ParentElement, Render, Styled, StyledExt,
    v_flex,
};
use raijin_workspace::ModalView;

use crate::terminal_pane::PendingShellInstallName;

pub struct ShellInstallModal {
    shell: &'static ShellInstallInfo,
    install_command: Option<String>,
    pm_name: Option<&'static str>,
    terminal_handle: raijin_terminal::TerminalHandle,
    focus_handle: FocusHandle,
}

impl EventEmitter<DismissEvent> for ShellInstallModal {}

impl ModalView for ShellInstallModal {
    fn fade_out_background(&self) -> bool {
        true
    }
}

impl Focusable for ShellInstallModal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ShellInstallModal {
    pub fn new(
        shell: &'static ShellInstallInfo,
        terminal_handle: raijin_terminal::TerminalHandle,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let installer = detect_available_installer(shell);
        Self {
            shell,
            install_command: installer.map(|i| i.command.to_string()),
            pm_name: installer.map(|i| i.package_manager),
            terminal_handle,
            focus_handle: cx.focus_handle(),
        }
    }

    fn install(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref cmd) = self.install_command {
            cx.global_mut::<PendingShellInstallName>().0 =
                Some(self.shell.binary.to_string());
            {
                let mut term = self.terminal_handle.lock();
                term.set_pending_block_command(cmd.clone());
            }
            let mut bytes = cmd.as_bytes().to_vec();
            bytes.push(b'\r');
            self.terminal_handle.write(&bytes);
        } else {
            let _ = std::process::Command::new("open")
                .arg(self.shell.url)
                .spawn();
        }
        cx.emit(DismissEvent);
    }

    fn cancel(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl Render for ShellInstallModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let description = if let Some(pm) = self.pm_name {
            format!(
                "Raijin supports {} natively with intelligent completions. \
                 Install via {} to get started.",
                self.shell.name, pm,
            )
        } else {
            format!(
                "Raijin supports {} natively with intelligent completions. \
                 Visit the installation page to get started.",
                self.shell.name,
            )
        };

        let install_label = if self.install_command.is_some() {
            format!("Install via {}", self.pm_name.unwrap_or("package manager"))
        } else {
            "Open installation page".to_string()
        };

        v_flex()
            .key_context("ShellInstallModal")
            .track_focus(&self.focus_handle)
            .elevation_3(cx)
            .w_96()
            .overflow_hidden()
            .child(
                Modal::new("shell-install", None)
                    .header(
                        ModalHeader::new()
                            .headline(format!("{} is not installed", self.shell.name))
                            .description(description)
                            .show_dismiss_button(true),
                    )
                    .footer(
                        ModalFooter::new()
                            .end_slot(
                                raijin_ui::h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("cancel", "Cancel")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.cancel(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("install", install_label)
                                            .primary()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.install(window, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
    }
}
