use inazuma::{
    App, Corners, Edges, ParentElement, Pixels, StyleRefinement, Styled, Window, div,
    px,
};
use raijin_theme::ActiveTheme;
use crate::traits::styled_ext::StyledExt;

pub trait FocusableExt<T: ParentElement + Styled + Sized> {
    fn focus_ring(self, is_focused: bool, margins: Pixels, window: &Window, cx: &App) -> Self;
}

impl<T: ParentElement + Styled + Sized> FocusableExt<T> for T {
    fn focus_ring(mut self, is_focused: bool, margins: Pixels, window: &Window, cx: &App) -> Self {
        if !is_focused {
            return self;
        }

        const RING_BORDER_WIDTH: Pixels = px(1.5);
        let rem_size = window.rem_size();
        let style = self.style();

        let border_widths = Edges::<Pixels> {
            top: style
                .border_widths
                .top
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom: style
                .border_widths
                .bottom
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            left: style
                .border_widths
                .left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            right: style
                .border_widths
                .right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
        };

        let radius = Corners::<Pixels> {
            top_left: style
                .corner_radii
                .top_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            top_right: style
                .corner_radii
                .top_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_left: style
                .corner_radii
                .bottom_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_right: style
                .corner_radii
                .bottom_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
        }
        .map(|v| *v + RING_BORDER_WIDTH);

        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii.top_left = Some(radius.top_left.into());
        inner_style.corner_radii.top_right = Some(radius.top_right.into());
        inner_style.corner_radii.bottom_left = Some(radius.bottom_left.into());
        inner_style.corner_radii.bottom_right = Some(radius.bottom_right.into());

        let inset = RING_BORDER_WIDTH + margins;

        self.child(
            div()
                .flex_none()
                .absolute()
                .top(-(inset + border_widths.top))
                .left(-(inset + border_widths.left))
                .right(-(inset + border_widths.right))
                .bottom(-(inset + border_widths.bottom))
                .border(RING_BORDER_WIDTH)
                .border_color(cx.theme().colors().border_focused.opacity(0.2))
                .refine_style(&inner_style),
        )
    }
}
