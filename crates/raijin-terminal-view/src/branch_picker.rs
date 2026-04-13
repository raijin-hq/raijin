use core::cmp;
use std::sync::Arc;

use inazuma::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Subscription, Task, Window, rems,
};
use inazuma_fuzzy::{StringMatch, StringMatchCandidate};
use inazuma_picker::{Picker, PickerDelegate};
use inazuma_util::ResultExt;
use raijin_ui::{
    Color, HighlightedLabel, Icon, IconName, IconSize, ListItem, ListItemSpacing, prelude::*,
};
use raijin_workspace::ModalView;

use crate::terminal_pane::PendingBranchSwitch;

/// Modal picker for switching git branches.
///
/// Warp-style branch selector with fuzzy search, keyboard navigation,
/// and accent-colored current branch highlighting.
pub struct BranchPicker {
    picker: Entity<Picker<BranchPickerDelegate>>,
    _subscription: Subscription,
}

impl BranchPicker {
    pub fn new(
        branches: Vec<String>,
        current_branch: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let delegate = BranchPickerDelegate::new(branches, current_branch);
        let picker = cx.new(|cx| Picker::uniform_list(delegate, window, cx));
        let _subscription = cx.subscribe(&picker, |_, _, _, cx| cx.emit(DismissEvent));
        Self {
            picker,
            _subscription,
        }
    }
}

impl ModalView for BranchPicker {}
impl EventEmitter<DismissEvent> for BranchPicker {}

impl Focusable for BranchPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for BranchPicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(rems(22.))
            .child(self.picker.clone())
            .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                this.picker.update(cx, |this, cx| {
                    this.cancel(&Default::default(), window, cx);
                })
            }))
    }
}

pub struct BranchPickerDelegate {
    branches: Vec<String>,
    current_branch: String,
    matches: Vec<StringMatch>,
    selected_index: usize,
}

impl BranchPickerDelegate {
    fn new(branches: Vec<String>, current_branch: String) -> Self {
        Self {
            branches,
            current_branch,
            matches: Vec::new(),
            selected_index: 0,
        }
    }
}

impl PickerDelegate for BranchPickerDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, _cx: &mut App) -> Arc<str> {
        Arc::from("Search branches...")
    }

    fn match_count(&self) -> usize {
        self.matches.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(
        &mut self,
        ix: usize,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn update_matches(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        cx.spawn_in(window, async move |picker, cx| {
            let candidates = picker.read_with(cx, |picker, _| {
                picker
                    .delegate
                    .branches
                    .iter()
                    .enumerate()
                    .map(|(ix, branch)| StringMatchCandidate::new(ix, branch))
                    .collect::<Vec<StringMatchCandidate>>()
            });
            let Some(candidates) = candidates.log_err() else {
                return;
            };
            let matches: Vec<StringMatch> = if query.is_empty() {
                candidates
                    .into_iter()
                    .enumerate()
                    .map(|(index, candidate)| StringMatch {
                        candidate_id: index,
                        string: candidate.string,
                        positions: Vec::new(),
                        score: 0.0,
                    })
                    .collect()
            } else {
                inazuma_fuzzy::match_strings(
                    &candidates,
                    &query,
                    true,
                    true,
                    10000,
                    &Default::default(),
                    cx.background_executor().clone(),
                )
                .await
            };
            picker
                .update(cx, |picker, _| {
                    let delegate = &mut picker.delegate;
                    delegate.matches = matches;
                    if delegate.matches.is_empty() {
                        delegate.selected_index = 0;
                    } else {
                        delegate.selected_index =
                            cmp::min(delegate.selected_index, delegate.matches.len() - 1);
                    }
                })
                .log_err();
        })
    }

    fn confirm(&mut self, _: bool, _window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(hit) = self.matches.get(self.selected_index) else {
            return;
        };
        let branch = self.branches[hit.candidate_id].clone();

        if branch != self.current_branch {
            cx.global_mut::<PendingBranchSwitch>().0 = Some(branch);
        }
        cx.emit(DismissEvent);
    }

    fn dismissed(&mut self, _: &mut Window, cx: &mut Context<Picker<Self>>) {
        cx.emit(DismissEvent);
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let hit = self.matches.get(ix)?;
        let branch_name = &hit.string;
        let is_current = *branch_name == self.current_branch;
        let accent = cx.theme().colors().accent;

        let highlights: Vec<_> = hit
            .positions
            .iter()
            .filter(|&&index| index < 60)
            .copied()
            .collect();

        Some(
            ListItem::new(format!("branch-{ix}"))
                .inset(true)
                .spacing(ListItemSpacing::Dense)
                .toggle_state(selected)
                .start_slot(
                    Icon::new(IconName::GitBranch)
                        .size(IconSize::Small)
                        .when(is_current, |icon| icon.color(Color::Custom(accent))),
                )
                .child(
                    HighlightedLabel::new(
                        inazuma_util::truncate_and_trailoff(branch_name, 60),
                        highlights,
                    )
                    .when(is_current, |label| label.color(Color::Accent)),
                )
                .end_slot::<Icon>(if is_current {
                    Some(
                        Icon::new(IconName::Check)
                            .size(IconSize::Small)
                            .color(Color::Custom(accent)),
                    )
                } else {
                    None
                }),
        )
    }
}

/// List local git branches by running `git branch --list`.
pub fn list_git_branches(cwd: &str) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["branch", "--list", "--no-color"])
        .current_dir(cwd)
        .output()
        .ok();

    let Some(output) = output else {
        return Vec::new();
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim_start_matches(['*', ' ']).trim().to_string())
        .filter(|b| !b.is_empty())
        .collect()
}
