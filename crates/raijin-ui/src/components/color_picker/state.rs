use inazuma::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    KeyBinding, Oklch, Render, Subscription, Window, hsla, oklch_to_hsla,
};

use crate::{
    actions::Confirm,
    InputEvent, InputState,
    SliderEvent, SliderState,
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
    Change(Option<Oklch>),
}

pub(super) fn color_palettes() -> Vec<Vec<Oklch>> {
    use crate::styles::color::DEFAULT_COLORS;
    use itertools::Itertools as _;

    macro_rules! c {
        ($color:tt) => {
            DEFAULT_COLORS
                .$color
                .keys()
                .sorted()
                .map(|k| DEFAULT_COLORS.$color.get(k).map(|c| c.color).unwrap())
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
pub(super) struct HslSliders {
    pub(super) hue: Entity<SliderState>,
    pub(super) saturation: Entity<SliderState>,
    pub(super) lightness: Entity<SliderState>,
    pub(super) alpha: Entity<SliderState>,
}

impl HslSliders {
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

    pub(super) fn read(&self, cx: &App) -> Oklch {
        hsla(
            self.hue.read(cx).value().start(),
            self.saturation.read(cx).value().start(),
            self.lightness.read(cx).value().start(),
            self.alpha.read(cx).value().start(),
        )
    }

    pub(super) fn update(&self, new_color: Oklch, window: &mut Window, cx: &mut App) {
        let (h, s, l, a) = oklch_to_hsla(new_color);
        self.hue.update(cx, |slider, cx| {
            slider.set_value(h, window, cx);
        });
        self.saturation.update(cx, |slider, cx| {
            slider.set_value(s, window, cx);
        });
        self.lightness.update(cx, |slider, cx| {
            slider.set_value(l, window, cx);
        });
        self.alpha.update(cx, |slider, cx| {
            slider.set_value(a, window, cx);
        });
    }
}

/// State of the [`ColorPicker`](super::ColorPicker).
pub struct ColorPickerState {
    pub(super) focus_handle: FocusHandle,
    pub(super) value: Option<Oklch>,
    pub(super) hovered_color: Option<Oklch>,
    pub(super) state: Entity<InputState>,
    pub(super) hsl_sliders: HslSliders,
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
        let hsl_sliders = HslSliders::new(cx);

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
                        if let Ok(color) = raijin_theme::parse_color(value.as_str()) {
                            this.hovered_color = Some(color);
                            this.sync_sliders(Some(color), window, cx);
                        }
                    }
                    InputEvent::PressEnter { .. } => {
                        let val = this.state.read(cx).value();
                        if let Ok(color) = raijin_theme::parse_color(&val) {
                            this.open = false;
                            this.update_value(Some(color), true, window, cx);
                        }
                    }
                    _ => {}
                },
            ),
            cx.subscribe_in(
                &hsl_sliders.hue,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsl_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsl_sliders.saturation,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsl_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsl_sliders.lightness,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsl_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsl_sliders.alpha,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsl_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            value: None,
            hovered_color: None,
            state,
            hsl_sliders,
            needs_slider_sync: false,
            suppress_input_change: false,
            active_tab: 0,
            open: false,
            _subscriptions,
        }
    }

    /// Set default color value.
    pub fn default_value(mut self, value: impl Into<Oklch>) -> Self {
        let value = value.into();
        self.value = Some(value);
        self.hovered_color = Some(value);
        self.needs_slider_sync = true;
        self
    }

    /// Set current color value.
    pub fn set_value(
        &mut self,
        value: impl Into<Oklch>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_value(Some(value.into()), false, window, cx)
    }

    /// Get current color value.
    pub fn value(&self) -> Option<Oklch> {
        self.value
    }

    pub(super) fn on_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    pub(super) fn update_value(
        &mut self,
        value: Option<Oklch>,
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
        value: Oklch,
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

    fn sync_sliders(&mut self, color: Option<Oklch>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(color) = color {
            self.hsl_sliders.update(color, window, cx);
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
