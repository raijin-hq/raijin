use inazuma::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla, IntoElement,
    KeyBinding, Render, Subscription, Window, hsla,
};

use crate::{
    Colorize,
    actions::Confirm,
    input::{InputEvent, InputState},
    slider::{SliderEvent, SliderState},
};

const CONTEXT: &'static str = "ColorPicker";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "enter",
        Confirm { secondary: false },
        Some(CONTEXT),
    )])
}

/// Events emitted by the [`ColorPicker`](super::ColorPicker).
#[derive(Clone)]
pub enum ColorPickerEvent {
    Change(Option<Hsla>),
}

pub(super) fn color_palettes() -> Vec<Vec<Hsla>> {
    use crate::theme::DEFAULT_COLORS;
    use itertools::Itertools as _;

    macro_rules! c {
        ($color:tt) => {
            DEFAULT_COLORS
                .$color
                .keys()
                .sorted()
                .map(|k| DEFAULT_COLORS.$color.get(k).map(|c| c.hsla).unwrap())
                .collect::<Vec<_>>()
        };
    }

    vec![
        c!(stone),
        c!(red),
        c!(orange),
        c!(yellow),
        c!(green),
        c!(cyan),
        c!(blue),
        c!(purple),
        c!(pink),
    ]
}

#[derive(Clone)]
pub(super) struct HslaSliders {
    pub(super) hue: Entity<SliderState>,
    pub(super) saturation: Entity<SliderState>,
    pub(super) lightness: Entity<SliderState>,
    pub(super) alpha: Entity<SliderState>,
}

impl HslaSliders {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            hue: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
            saturation: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
            lightness: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
            alpha: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
        }
    }

    pub(super) fn read(&self, cx: &App) -> Hsla {
        hsla(
            self.hue.read(cx).value().start(),
            self.saturation.read(cx).value().start(),
            self.lightness.read(cx).value().start(),
            self.alpha.read(cx).value().start(),
        )
    }

    pub(super) fn update(&self, new_color: Hsla, window: &mut Window, cx: &mut App) {
        self.hue.update(cx, |slider, cx| {
            slider.set_value(new_color.h, window, cx);
        });
        self.saturation.update(cx, |slider, cx| {
            slider.set_value(new_color.s, window, cx);
        });
        self.lightness.update(cx, |slider, cx| {
            slider.set_value(new_color.l, window, cx);
        });
        self.alpha.update(cx, |slider, cx| {
            slider.set_value(new_color.a, window, cx);
        });
    }
}

/// State of the [`ColorPicker`](super::ColorPicker).
pub struct ColorPickerState {
    pub(super) focus_handle: FocusHandle,
    pub(super) value: Option<Hsla>,
    pub(super) hovered_color: Option<Hsla>,
    pub(super) state: Entity<InputState>,
    pub(super) hsla_sliders: HslaSliders,
    pub(super) needs_slider_sync: bool,
    pub(super) suppress_input_change: bool,
    pub(super) active_tab: usize,
    pub(super) open: bool,
    _subscriptions: Vec<Subscription>,
}

impl ColorPickerState {
    /// Create a new [`ColorPickerState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            InputState::new(window, cx).pattern(regex::Regex::new(r"^#[0-9a-fA-F]{0,8}$").unwrap())
        });
        let hsla_sliders = HslaSliders::new(cx);

        let mut _subscriptions = vec![
            cx.subscribe_in(
                &state,
                window,
                |this, state, ev: &InputEvent, window, cx| match ev {
                    InputEvent::Change => {
                        if this.suppress_input_change {
                            return;
                        }
                        let value = state.read(cx).value();
                        if let Ok(color) = Hsla::parse_hex(value.as_str()) {
                            this.hovered_color = Some(color);
                            this.sync_sliders(Some(color), window, cx);
                        }
                    }
                    InputEvent::PressEnter { .. } => {
                        let val = this.state.read(cx).value();
                        if let Ok(color) = Hsla::parse_hex(&val) {
                            this.open = false;
                            this.update_value(Some(color), true, window, cx);
                        }
                    }
                    _ => {}
                },
            ),
            cx.subscribe_in(
                &hsla_sliders.hue,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsla_sliders.saturation,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsla_sliders.lightness,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsla_sliders.alpha,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            value: None,
            hovered_color: None,
            state,
            hsla_sliders,
            needs_slider_sync: false,
            suppress_input_change: false,
            active_tab: 0,
            open: false,
            _subscriptions,
        }
    }

    /// Set default color value.
    pub fn default_value(mut self, value: impl Into<Hsla>) -> Self {
        let value = value.into();
        self.value = Some(value);
        self.hovered_color = Some(value);
        self.needs_slider_sync = true;
        self
    }

    /// Set current color value.
    pub fn set_value(
        &mut self,
        value: impl Into<Hsla>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_value(Some(value.into()), false, window, cx)
    }

    /// Get current color value.
    pub fn value(&self) -> Option<Hsla> {
        self.value
    }

    pub(super) fn on_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    pub(super) fn update_value(
        &mut self,
        value: Option<Hsla>,
        emit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.needs_slider_sync = false;
        self.value = value;
        self.hovered_color = value;
        self.state.update(cx, |view, cx| {
            if let Some(value) = value {
                view.set_value(value.to_hex(), window, cx);
            } else {
                view.set_value("", window, cx);
            }
        });
        if emit {
            cx.emit(ColorPickerEvent::Change(value));
        }
        cx.notify();
    }

    fn update_value_from_slider(
        &mut self,
        value: Hsla,
        emit: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.needs_slider_sync = false;
        self.value = Some(value);
        self.hovered_color = Some(value);
        if emit {
            cx.emit(ColorPickerEvent::Change(Some(value)));
        }
        cx.notify();
    }

    fn sync_sliders(&mut self, color: Option<Hsla>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(color) = color {
            self.hsla_sliders.update(color, window, cx);
        }
    }
}

impl EventEmitter<ColorPickerEvent> for ColorPickerState {}

impl Render for ColorPickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.state.clone()
    }
}

impl Focusable for ColorPickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
