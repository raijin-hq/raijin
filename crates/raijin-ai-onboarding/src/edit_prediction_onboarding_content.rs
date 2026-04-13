use std::sync::Arc;

use raijin_client::{Client, UserStore};
use raijin_cloud_api_types::Plan;
use inazuma::{Entity, IntoElement, ParentElement};
use raijin_ui::prelude::*;

use crate::RaijinAiOnboarding;

pub struct EditPredictionOnboarding {
    user_store: Entity<UserStore>,
    client: Arc<Client>,
    copilot_is_configured: bool,
    continue_with_raijin_ai: Arc<dyn Fn(&mut Window, &mut App)>,
    continue_with_copilot: Arc<dyn Fn(&mut Window, &mut App)>,
}

impl EditPredictionOnboarding {
    pub fn new(
        user_store: Entity<UserStore>,
        client: Arc<Client>,
        copilot_is_configured: bool,
        continue_with_raijin_ai: Arc<dyn Fn(&mut Window, &mut App)>,
        continue_with_copilot: Arc<dyn Fn(&mut Window, &mut App)>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            user_store,
            copilot_is_configured,
            client,
            continue_with_raijin_ai,
            continue_with_copilot,
        }
    }
}

impl Render for EditPredictionOnboarding {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_free_plan = self
            .user_store
            .read(cx)
            .plan()
            .is_some_and(|plan| plan == Plan::RaijinFree);

        let github_copilot = v_flex()
            .gap_1()
            .child(Label::new(if self.copilot_is_configured {
                "Alternatively, you can continue to use GitHub Copilot as that's already set up."
            } else {
                "Alternatively, you can use GitHub Copilot as your edit prediction provider."
            }))
            .child(
                Button::new(
                    "configure-copilot",
                    if self.copilot_is_configured {
                        "Use Copilot"
                    } else {
                        "Configure Copilot"
                    },
                )
                .full_width()
                .style(ButtonStyle::Outlined)
                .on_click({
                    let callback = self.continue_with_copilot.clone();
                    move |_, window, cx| callback(window, cx)
                }),
            );

        v_flex()
            .gap_2()
            .child(RaijinAiOnboarding::new(
                self.client.clone(),
                &self.user_store,
                self.continue_with_raijin_ai.clone(),
                cx,
            ))
            .when(is_free_plan, |this| {
                this.child(raijin_ui::Divider::horizontal()).child(github_copilot)
            })
    }
}
