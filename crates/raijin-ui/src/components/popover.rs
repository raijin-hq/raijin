use crate::prelude::*;
use crate::utils::actions::Cancel;
use crate::utils::anchored::anchored;
use crate::utils::popover_global_state::PopoverGlobalState;
use crate::ElementExt as _;
use inazuma::Anchor;
use crate::v_flex;
use inazuma::{
    AnyElement, App, Bounds, Context, Deferred, DismissEvent, Div, ElementId, EventEmitter,
    FocusHandle, Focusable, Half, InteractiveElement as _, IntoElement, KeyBinding, MouseButton,
    ParentElement, Pixels, Point, Render, RenderOnce, Stateful, StyleRefinement, Styled,
    Subscription, Window, deferred, div, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;
use std::rc::Rc;

// ── PopoverContainer (from raijin-ui) ────────────────────────────────────────

/// Y height added beyond the size of the contents.
pub const POPOVER_Y_PADDING: Pixels = px(8.);

/// A styled container for popover content with elevation and optional aside panel.
///
/// This is a layout wrapper — use [`InteractivePopover`] for trigger-based popovers
/// with state management, anchor positioning, and dismiss behavior.
#[derive(IntoElement)]
pub struct PopoverContainer {
    children: SmallVec<[AnyElement; 2]>,
    aside: Option<AnyElement>,
}

impl Default for PopoverContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl PopoverContainer {
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
            aside: None,
        }
    }

    pub fn aside(mut self, aside: impl IntoElement) -> Self
    where
        Self: Sized,
    {
        self.aside = Some(aside.into_element().into_any());
        self
    }
}

impl ParentElement for PopoverContainer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for PopoverContainer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .gap_1()
            .child(
                v_flex()
                    .elevation_2(cx)
                    .py(POPOVER_Y_PADDING / 2.)
                    .child(div().children(self.children)),
            )
            .when_some(self.aside, |this, aside| {
                this.child(
                    v_flex()
                        .elevation_2(cx)
                        .bg(cx.theme().colors().surface)
                        .px_1()
                        .child(aside),
                )
            })
    }
}

// Keep `Popover` as an alias for backward compatibility
pub type Popover = InteractivePopover;

// ── InteractivePopover (from inazuma-component) ──────────────────────────────

const CONTEXT: &str = "InteractivePopover";

pub(crate) fn init(cx: &mut App) {
    PopoverGlobalState::init(cx);
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))]);
}

/// A popover element with trigger, state management, anchor positioning,
/// focus tracking, overlay dismiss, and keyboard escape support.
#[derive(IntoElement)]
pub struct InteractivePopover {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    default_open: bool,
    open: Option<bool>,
    tracked_focus_handle: Option<FocusHandle>,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<
        Rc<
            dyn Fn(
                    &mut InteractivePopoverState,
                    &mut Window,
                    &mut Context<InteractivePopoverState>,
                ) -> AnyElement
                + 'static,
        >,
    >,
    children: Vec<AnyElement>,
    trigger_style: Option<StyleRefinement>,
    mouse_button: MouseButton,
    appearance: bool,
    overlay_closable: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl InteractivePopover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopLeft,
            trigger: None,
            trigger_style: None,
            content: None,
            tracked_focus_handle: None,
            children: vec![],
            mouse_button: MouseButton::Left,
            appearance: true,
            overlay_closable: true,
            default_open: false,
            open: None,
            on_open_change: None,
        }
    }

    /// Set the anchor position of the popover, default is `Anchor::TopLeft`.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the mouse button to trigger the popover, default is `MouseButton::Left`.
    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    /// Set the trigger element of the popover.
    pub fn trigger(mut self, trigger: impl IntoElement + 'static) -> Self {
        self.trigger = Some(Box::new(|_is_open, _, _| trigger.into_any_element()));
        self
    }

    /// Set the default open state of the popover, default is `false`.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Force set the open state of the popover (controlled mode).
    /// Must be used with `on_open_change` to handle state changes.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Callback when the open state changes.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = Some(style);
        self
    }

    /// Set whether clicking outside dismisses the popover, default is `true`.
    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    /// Set the content builder. Called on every render — avoid creating
    /// new entities inside the closure.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: Fn(
                &mut InteractivePopoverState,
                &mut Window,
                &mut Context<InteractivePopoverState>,
            ) -> E
            + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Set whether the popover has styled appearance (bg, border, shadow, padding).
    /// Default is `true`. When `false`, the popover is unstyled and click-out doesn't dismiss.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Bind a focus handle to receive focus when the popover opens.
    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }

    fn resolved_corner(anchor: Anchor, trigger_bounds: Bounds<Pixels>) -> Point<Pixels> {
        let offset = if anchor.is_center() {
            inazuma::point(trigger_bounds.size.width.half(), px(0.))
        } else {
            Point::default()
        };

        trigger_bounds.corner(anchor.swap_vertical().into())
            + offset
            + Point {
                x: px(0.),
                y: -trigger_bounds.size.height,
            }
    }

    pub(crate) fn render_popover<E>(
        anchor: Anchor,
        trigger_bounds: Bounds<Pixels>,
        content: E,
        _: &mut Window,
        _: &mut App,
    ) -> Deferred
    where
        E: IntoElement + 'static,
    {
        deferred(
            anchored()
                .snap_to_window_with_margin(px(8.))
                .anchor(anchor)
                .position(Self::resolved_corner(anchor, trigger_bounds))
                .child(div().relative().child(content)),
        )
        .with_priority(1)
    }

    pub(crate) fn render_popover_content(
        anchor: Anchor,
        appearance: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        v_flex()
            .id("content")
            .occlude()
            .tab_group()
            .when(appearance, |this| this.elevation_2(cx).p_3())
            .map(|this| match anchor {
                Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => this.top_1(),
                Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => this.bottom_1(),
            })
    }
}

impl ParentElement for InteractivePopover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InteractivePopover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

// ── InteractivePopoverState ──────────────────────────────────────────────────

pub struct InteractivePopoverState {
    focus_handle: FocusHandle,
    pub(crate) tracked_focus_handle: Option<FocusHandle>,
    trigger_bounds: Bounds<Pixels>,
    open: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    _dismiss_subscription: Option<Subscription>,
}

impl InteractivePopoverState {
    pub fn new(default_open: bool, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tracked_focus_handle: None,
            trigger_bounds: Bounds::default(),
            open: default_open,
            on_open_change: None,
            _dismiss_subscription: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.toggle_open(window, cx);
        }
    }

    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.toggle_open(window, cx);
        }
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.open = open;
        if self.open {
            PopoverGlobalState::global_mut(cx).register_deferred_popover(&self.focus_handle);
        } else {
            PopoverGlobalState::global_mut(cx).unregister_deferred_popover(&self.focus_handle);
        }
    }

    fn toggle_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_open(!self.open, cx);
        if self.open {
            let state = cx.entity();
            let focus_handle = if let Some(tracked_focus_handle) = self.tracked_focus_handle.clone()
            {
                tracked_focus_handle
            } else {
                self.focus_handle.clone()
            };
            focus_handle.focus(window, cx);

            self._dismiss_subscription = Some(window.subscribe(
                &cx.entity(),
                cx,
                move |_, _: &DismissEvent, window, cx| {
                    state.update(cx, |state, cx| {
                        state.dismiss(window, cx);
                    });
                    window.refresh();
                },
            ));
        } else {
            self._dismiss_subscription = None;
        }

        if let Some(callback) = self.on_open_change.as_ref() {
            callback(&self.open, window, cx);
        }
        cx.notify();
    }

    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }
}

impl Focusable for InteractivePopoverState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InteractivePopoverState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl EventEmitter<DismissEvent> for InteractivePopoverState {}

impl RenderOnce for InteractivePopover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let force_open = self.open;
        let default_open = self.default_open;
        let tracked_focus_handle = self.tracked_focus_handle.clone();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| {
            InteractivePopoverState::new(default_open, cx)
        });

        state.update(cx, |state, cx| {
            if let Some(tracked_focus_handle) = tracked_focus_handle {
                state.tracked_focus_handle = Some(tracked_focus_handle);
            }
            state.on_open_change = self.on_open_change.clone();
            if let Some(force_open) = force_open {
                state.set_open(force_open, cx);
            }
        });

        let open = state.read(cx).open;
        let focus_handle = state.read(cx).focus_handle.clone();
        let trigger_bounds = state.read(cx).trigger_bounds;

        let Some(trigger) = self.trigger else {
            return div().id("empty");
        };

        let parent_view_id = window.current_view();

        let el = div()
            .id(self.id)
            .child((trigger)(open, window, cx))
            .on_mouse_down(self.mouse_button, {
                let state = state.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    state.update(cx, |state, cx| {
                        state.set_open(open, cx);
                        state.toggle_open(window, cx);
                    });
                    cx.notify(parent_view_id);
                }
            })
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, _| {
                        state.trigger_bounds = bounds;
                    })
                }
            });

        if !open {
            return el;
        }

        let popover_content =
            Self::render_popover_content(self.anchor, self.appearance, window, cx)
                .track_focus(&focus_handle)
                .key_context(CONTEXT)
                .on_action(
                    window
                        .listener_for(&state, InteractivePopoverState::on_action_cancel),
                )
                .when_some(self.content, |this, content| {
                    this.child(state.update(cx, |state, cx| (content)(state, window, cx)))
                })
                .children(self.children)
                .when(self.overlay_closable, |this| {
                    this.on_mouse_down_out({
                        let state = state.clone();
                        move |_, window, cx| {
                            state.update(cx, |state, cx| {
                                state.dismiss(window, cx);
                            });
                            cx.notify(parent_view_id);
                        }
                    })
                })
                .refine_style(&self.style);

        el.child(Self::render_popover(
            self.anchor,
            trigger_bounds,
            popover_content,
            window,
            cx,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inazuma::MouseButton;

    #[test]
    fn test_interactive_popover_builder() {
        let popover = InteractivePopover::new("test")
            .anchor(Anchor::BottomCenter)
            .mouse_button(MouseButton::Right)
            .default_open(true)
            .appearance(false)
            .overlay_closable(false);

        assert_eq!(popover.anchor, Anchor::BottomCenter);
        assert_eq!(popover.mouse_button, MouseButton::Right);
        assert!(popover.default_open);
        assert!(!popover.appearance);
        assert!(!popover.overlay_closable);
    }

    #[test]
    fn test_resolved_corner_positions() {
        let bounds = Bounds {
            origin: Point {
                x: px(100.),
                y: px(100.),
            },
            size: inazuma::Size {
                width: px(200.),
                height: px(50.),
            },
        };

        let pos = InteractivePopover::resolved_corner(Anchor::TopLeft, bounds);
        assert_eq!(pos.x, px(100.));
        assert_eq!(pos.y, px(100.));

        let pos = InteractivePopover::resolved_corner(Anchor::TopCenter, bounds);
        assert_eq!(pos.x, px(200.));
        assert_eq!(pos.y, px(100.));

        let pos = InteractivePopover::resolved_corner(Anchor::BottomLeft, bounds);
        assert_eq!(pos.x, px(100.));
        assert_eq!(pos.y, px(50.));
    }
}
