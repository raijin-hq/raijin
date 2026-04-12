use crate::component_prelude::*;
use inazuma::{
    AnyElement, AnyView, ClickEvent, Context, Corner, Corners, CursorStyle, DefiniteLength, Edges,
    IntoElement, ParentElement, RenderOnce, Styled, Window,
};
use raijin_ui_macros::RegisterComponent;

use crate::{
    ButtonCommon, ButtonLike, ButtonSize, ButtonStyle, Icon, Label,
    PopupMenu, Selectable, Tooltip,
};
use crate::{
    Color, DynamicSpacing, ElevationIndex, IconName, IconSize, KeyBinding, KeybindingPosition, prelude::*,
};

use super::button_icon::ButtonIcon;
use super::button_variant::{ButtonRounded, ButtonVariant, ButtonVariants};

/// A versatile button element supporting labels, icons, loading states, outlines,
/// compact mode, rounded corners, border control, variants, sizes, dropdowns,
/// and keyboard shortcut display.
///
/// # Constructors
///
/// ```ignore
/// // Button with label
/// Button::new("save", "Save")
///
/// // Button with label and icon
/// Button::new("save", "Save").icon(IconName::Save)
///
/// // Icon-only button (no label)
/// Button::with_icon("menu", IconName::Menu)
///
/// // Button with no label, set later
/// Button::with_id("trigger").label("Trigger")
/// ```
///
/// # Variants
///
/// ```ignore
/// Button::new("action", "Delete").danger()
/// Button::new("action", "Submit").primary()
/// Button::new("action", "Cancel").ghost()
/// Button::new("action", "Learn more").link()
/// ```
///
/// # States
///
/// ```ignore
/// Button::new("btn", "Click").disabled(true)
/// Button::new("btn", "Click").selected(true)
/// Button::new("btn", "Loading...").loading(true)
/// ```
#[derive(IntoElement, Documented, RegisterComponent)]
pub struct Button {
    base: ButtonLike,
    label: SharedString,
    label_color: Option<Color>,
    label_size: Option<LabelSize>,
    selected_label: Option<SharedString>,
    selected_label_color: Option<Color>,
    start_icon: Option<Icon>,
    end_icon: Option<Icon>,
    icon: Option<ButtonIcon>,
    loading_icon: Option<Icon>,
    key_binding: Option<KeyBinding>,
    key_binding_position: KeybindingPosition,
    alpha: Option<f32>,
    truncate: bool,
    // Extended state
    pub(super) selected: bool,
    pub(super) compact: bool,
    pub(super) outline: bool,
    pub(super) loading: bool,
    pub(super) variant: Option<ButtonVariant>,
    pub(super) size: Option<Size>,
    pub(super) rounded: ButtonRounded,
    pub(super) border_corners: Option<Corners<bool>>,
    pub(super) border_edges: Option<Edges<bool>>,
    dropdown_menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    dropdown_anchor: Corner,
}

impl Button {
    /// Creates a new [`Button`] with a label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            base: ButtonLike::new(id),
            label: label.into(),
            label_color: None,
            label_size: None,
            selected_label: None,
            selected_label_color: None,
            start_icon: None,
            end_icon: None,
            icon: None,
            loading_icon: None,
            key_binding: None,
            key_binding_position: KeybindingPosition::default(),
            alpha: None,
            truncate: false,
            selected: false,
            compact: false,
            outline: false,
            loading: false,
            variant: None,
            size: None,
            rounded: ButtonRounded::default(),
            border_corners: None,
            border_edges: None,
            dropdown_menu: None,
            dropdown_anchor: Corner::TopRight,
        }
    }

    /// Creates a button without a label. Use `.label()` or `.icon()` to configure.
    pub fn with_id(id: impl Into<ElementId>) -> Self {
        Self::new(id, SharedString::default())
    }

    /// Creates an icon-only button (no label).
    pub fn with_icon(id: impl Into<ElementId>, icon: impl Into<ButtonIcon>) -> Self {
        let mut btn = Self::with_id(id);
        btn.icon = Some(icon.into());
        btn
    }

    // -----------------------------------------------------------------------
    // Label
    // -----------------------------------------------------------------------

    /// Sets or replaces the button label.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the color of the button's label.
    pub fn color(mut self, label_color: impl Into<Option<Color>>) -> Self {
        self.label_color = label_color.into();
        self
    }

    /// Sets the size of the button's label text.
    pub fn label_size(mut self, label_size: impl Into<Option<LabelSize>>) -> Self {
        self.label_size = label_size.into();
        self
    }

    /// Sets the label shown when the button is in a selected state.
    pub fn selected_label<L: Into<SharedString>>(mut self, label: impl Into<Option<L>>) -> Self {
        self.selected_label = label.into().map(Into::into);
        self
    }

    /// Sets the label color shown when the button is in a selected state.
    pub fn selected_label_color(mut self, color: impl Into<Option<Color>>) -> Self {
        self.selected_label_color = color.into();
        self
    }

    // -----------------------------------------------------------------------
    // Icons
    // -----------------------------------------------------------------------

    /// Sets the primary icon, displayed at the start of the button.
    ///
    /// This is the unified icon method — accepts `IconName`, `Icon`, `Spinner`,
    /// or any type convertible to `ButtonIcon`.
    pub fn icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the icon displayed when the button is in loading state.
    pub fn loading_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.loading_icon = icon.into();
        self
    }

    /// Sets an icon at the start (left) of the button label.
    pub fn start_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.start_icon = icon.into();
        self
    }

    /// Sets an icon at the end (right) of the button label.
    pub fn end_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.end_icon = icon.into();
        self
    }

    // -----------------------------------------------------------------------
    // Keyboard shortcut
    // -----------------------------------------------------------------------

    /// Display a keybinding hint on the button.
    pub fn key_binding(mut self, key_binding: impl Into<Option<KeyBinding>>) -> Self {
        self.key_binding = key_binding.into();
        self
    }

    /// Sets the position of the keybinding relative to the label.
    pub fn key_binding_position(mut self, position: KeybindingPosition) -> Self {
        self.key_binding_position = position;
        self
    }

    // -----------------------------------------------------------------------
    // Appearance modifiers
    // -----------------------------------------------------------------------

    /// Sets the alpha (opacity) of the label text.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = Some(alpha);
        self
    }

    /// Truncates overflowing labels with an ellipsis.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Enables compact mode — reduced padding for tight layouts.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Enables outline mode — renders with a visible border.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Sets the loading state. When loading, the icon is replaced with a spinner.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Sets whether the button participates in tab navigation.
    /// `tab_stop(false)` removes the button from the tab order.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        if !tab_stop {
            self.base = self.base.tab_index(-1_isize);
        }
        self
    }

    /// Sets the rounded corner style.
    pub fn rounded(mut self, rounded: impl Into<ButtonRounded>) -> Self {
        self.rounded = rounded.into();
        self
    }

    /// Controls which corners have visible border radius.
    pub fn border_corners(mut self, corners: Corners<bool>) -> Self {
        self.border_corners = Some(corners);
        self
    }

    /// Controls which edges have visible borders.
    pub fn border_edges(mut self, edges: Edges<bool>) -> Self {
        self.border_edges = Some(edges);
        self
    }

    /// Overrides the border color from the button style.
    pub fn border_color(mut self, color: impl Into<inazuma::Oklch>) -> Self {
        self.base = self.base.border_color(color);
        self
    }

    // -----------------------------------------------------------------------
    // Dropdown menu integration
    // -----------------------------------------------------------------------

    /// Attaches a dropdown menu to the button.
    pub fn dropdown_menu(
        mut self,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.dropdown_menu = Some(Box::new(menu));
        self
    }

    /// Attaches a dropdown menu with a custom anchor corner.
    pub fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Corner>,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.dropdown_anchor = anchor.into();
        self.dropdown_menu = Some(Box::new(menu));
        self
    }

    /// Sets the tooltip shown on hover.
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.base = self.base.tooltip(tooltip);
        self
    }

    /// Binds a click handler to this button.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_click(handler);
        self
    }

    /// Sets the cursor style when hovering over the button.
    pub fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.base = self.base.cursor_style(cursor_style);
        self
    }

    /// Sets the anchor corner for dropdown menu positioning.
    pub fn anchor(mut self, anchor: impl Into<Corner>) -> Self {
        self.dropdown_anchor = anchor.into();
        self
    }

    /// Adds a tooltip that also displays the keybinding for the given action.
    pub fn tooltip_with_action<A: inazuma::Action>(
        mut self,
        tooltip_text: impl Into<SharedString>,
        action: &A,
        focus: Option<&inazuma::FocusHandle>,
    ) -> Self {
        let text: SharedString = tooltip_text.into();
        let action = action.boxed_clone();
        let focus = focus.cloned();
        self.base = self.base.tooltip(move |_window, cx| {
            if let Some(ref focus) = focus {
                Tooltip::for_action_in(&text, &*action, focus, cx)
            } else {
                Tooltip::for_action(&text, &*action, cx)
            }
        });
        self
    }

    /// Adds a dropdown caret (chevron-down icon) at the end of the button.
    pub fn dropdown_caret(mut self, show: bool) -> Self {
        if show {
            self.end_icon = Some(
                Icon::new(IconName::ChevronDown)
                    .size(IconSize::XSmall)
                    .color(Color::Muted),
            );
        }
        self
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl Toggleable for Button {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.base = self.base.toggle_state(selected);
        self
    }
}

impl SelectableButton for Button {
    fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.selected_style(style);
        self
    }
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.base = self.base.toggle_state(selected);
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut inazuma::StyleRefinement {
        self.base.base.style()
    }
}

impl Clickable for Button {
    fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_click(handler);
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.base = self.base.cursor_style(cursor_style);
        self
    }
}

impl inazuma::InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut inazuma::Interactivity {
        self.base.base.interactivity()
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements)
    }
}

impl FixedWidth for Button {
    fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.width(width);
        self
    }

    fn full_width(mut self) -> Self {
        self.base = self.base.full_width();
        self
    }
}

impl ButtonCommon for Button {
    fn id(&self) -> &ElementId {
        self.base.id()
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.style(style);
        self
    }

    fn size(mut self, size: ButtonSize) -> Self {
        self.base = self.base.size(size);
        self
    }

    fn button_tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.base = self.base.tooltip(tooltip);
        self
    }

    fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.base = self.base.tab_index(tab_index);
        self
    }

    fn layer(mut self, elevation: ElevationIndex) -> Self {
        self.base = self.base.layer(elevation);
        self
    }

    fn track_focus(mut self, focus_handle: &inazuma::FocusHandle) -> Self {
        self.base = self.base.track_focus(focus_handle);
        self
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl RenderOnce for Button {
    #[allow(refining_impl_trait)]
    fn render(self, _window: &mut Window, cx: &mut App) -> ButtonLike {
        let is_disabled = self.base.disabled;
        let is_selected = self.selected;

        let label = self
            .selected_label
            .filter(|_| is_selected)
            .unwrap_or(self.label);

        let label_color = if is_disabled {
            Color::Disabled
        } else if is_selected {
            self.selected_label_color.unwrap_or(Color::Selected)
        } else {
            self.label_color.unwrap_or_default()
        };

        let has_label = !label.is_empty();

        // Apply variant to base style if set
        let mut base = if let Some(variant) = self.variant {
            self.base.style(variant)
        } else {
            self.base
        };

        // Apply compact size
        if self.compact {
            base = base.size(ButtonSize::Compact);
        }

        base.child(
            h_flex()
                .when(self.truncate, |this| this.min_w_0().overflow_hidden())
                .gap(DynamicSpacing::Base04.rems(cx))
                // Primary icon (new unified icon system)
                .when_some(self.icon, |this, icon| {
                    let icon = icon
                        .loading_icon(self.loading_icon)
                        .loading(self.loading);
                    let icon = if let Some(size) = self.size {
                        icon.with_size(size)
                    } else {
                        icon
                    };
                    this.child(icon)
                })
                // Start icon (original API)
                .when_some(self.start_icon, |this, icon| {
                    this.child(if is_disabled {
                        icon.color(Color::Disabled)
                    } else {
                        icon
                    })
                })
                .when(has_label, |this| {
                    this.child(
                        h_flex()
                            .when(self.truncate, |this| this.min_w_0().overflow_hidden())
                            .when(
                                self.key_binding_position == KeybindingPosition::Start,
                                |this| this.flex_row_reverse(),
                            )
                            .gap(DynamicSpacing::Base06.rems(cx))
                            .justify_between()
                            .child(
                                Label::new(label)
                                    .color(label_color)
                                    .size(self.label_size.unwrap_or_default())
                                    .when_some(self.alpha, |this, alpha| this.alpha(alpha))
                                    .when(self.truncate, |this| this.truncate()),
                            )
                            .children(self.key_binding),
                    )
                })
                // End icon (original API)
                .when_some(self.end_icon, |this, icon| {
                    this.child(if is_disabled {
                        icon.color(Color::Disabled)
                    } else {
                        icon
                    })
                }),
        )
    }
}

impl Component for Button {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn sort_name() -> &'static str {
        "ButtonA"
    }

    fn description() -> Option<&'static str> {
        Some("A button triggers an event or action.")
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Variants",
                        vec![
                            single_example(
                                "Default",
                                Button::new("default", "Default").into_any_element(),
                            ),
                            single_example(
                                "Primary",
                                Button::new("primary", "Primary")
                                    .primary()
                                    .into_any_element(),
                            ),
                            single_example(
                                "Secondary",
                                Button::new("secondary", "Secondary")
                                    .secondary()
                                    .into_any_element(),
                            ),
                            single_example(
                                "Danger",
                                Button::new("danger", "Danger")
                                    .danger()
                                    .into_any_element(),
                            ),
                            single_example(
                                "Ghost",
                                Button::new("ghost", "Ghost")
                                    .ghost()
                                    .into_any_element(),
                            ),
                            single_example(
                                "Link",
                                Button::new("link", "Link").link().into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "Tinted",
                        vec![
                            single_example(
                                "Info",
                                Button::new("info", "Info")
                                    .info()
                                    .into_any_element(),
                            ),
                            single_example(
                                "Warning",
                                Button::new("warning", "Warning")
                                    .warning()
                                    .into_any_element(),
                            ),
                            single_example(
                                "Success",
                                Button::new("success", "Success")
                                    .success()
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "States",
                        vec![
                            single_example(
                                "Default",
                                Button::new("state_default", "Default").into_any_element(),
                            ),
                            single_example(
                                "Disabled",
                                Button::new("disabled", "Disabled")
                                    .disabled(true)
                                    .into_any_element(),
                            ),
                            single_example(
                                "Selected",
                                Button::new("selected", "Selected")
                                    .selected(true)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "With Icons",
                        vec![
                            single_example(
                                "Start Icon",
                                Button::new("icon_start", "Start Icon")
                                    .start_icon(Icon::new(IconName::Check))
                                    .into_any_element(),
                            ),
                            single_example(
                                "End Icon",
                                Button::new("icon_end", "End Icon")
                                    .end_icon(Icon::new(IconName::Check))
                                    .into_any_element(),
                            ),
                            single_example(
                                "Icon Button",
                                Button::new("icon_btn", "Save")
                                    .icon(IconName::Check)
                                    .into_any_element(),
                            ),
                        ],
                    ),
                ])
                .into_any_element(),
        )
    }
}
