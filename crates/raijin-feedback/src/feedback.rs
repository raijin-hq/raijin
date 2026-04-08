use inazuma::{App, ClipboardItem, PromptLevel, actions};
use raijin_system_specs::{CopySystemSpecsIntoClipboard, SystemSpecs};
use inazuma_util::ResultExt;
use raijin_workspace::Workspace;
use raijin_actions::feedback::{EmailRaijin, FileBugReport, RequestFeature};

actions!(
    raijin,
    [
        /// Opens the Raijin repository on GitHub.
        OpenRaijinRepo,
    ]
);

const RAIJIN_REPO_URL: &str = "https://github.com/raijin-hq/raijin";

const REQUEST_FEATURE_URL: &str = "https://github.com/raijin-hq/raijin/discussions/new/choose";

fn file_bug_report_url(specs: &SystemSpecs) -> String {
    format!(
        concat!(
            "https://github.com/raijin-hq/raijin/issues/new",
            "?",
            "template=10_bug_report.yml",
            "&",
            "environment={}"
        ),
        urlencoding::encode(&specs.to_string())
    )
}

fn email_raijin_url(specs: &SystemSpecs) -> String {
    format!(
        concat!("mailto:hi@raijin.dev", "?", "body={}"),
        email_body(specs)
    )
}

fn email_body(specs: &SystemSpecs) -> String {
    let body = format!("\n\nSystem Information:\n\n{}", specs);
    urlencoding::encode(&body).to_string()
}

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace
            .register_action(|_, _: &CopySystemSpecsIntoClipboard, window, cx| {
                let specs = SystemSpecs::new(window, cx);

                cx.spawn_in(window, async move |_, cx| {
                    let specs = specs.await.to_string();

                    cx.update(|_, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(specs.clone()))
                    })
                    .log_err();

                    cx.prompt(
                        PromptLevel::Info,
                        "Copied into clipboard",
                        Some(&specs),
                        &["OK"],
                    )
                    .await
                })
                .detach();
            })
            .register_action(|_, _: &RequestFeature, _, cx| {
                cx.open_url(REQUEST_FEATURE_URL);
            })
            .register_action(move |_, _: &FileBugReport, window, cx| {
                let specs = SystemSpecs::new(window, cx);
                cx.spawn_in(window, async move |_, cx| {
                    let specs = specs.await;
                    cx.update(|_, cx| {
                        cx.open_url(&file_bug_report_url(&specs));
                    })
                    .log_err();
                })
                .detach();
            })
            .register_action(move |_, _: &EmailRaijin, window, cx| {
                let specs = SystemSpecs::new(window, cx);
                cx.spawn_in(window, async move |_, cx| {
                    let specs = specs.await;
                    cx.update(|_, cx| {
                        cx.open_url(&email_raijin_url(&specs));
                    })
                    .log_err();
                })
                .detach();
            })
            .register_action(move |_, _: &OpenRaijinRepo, _, cx| {
                cx.open_url(RAIJIN_REPO_URL);
            });
    })
    .detach();
}
