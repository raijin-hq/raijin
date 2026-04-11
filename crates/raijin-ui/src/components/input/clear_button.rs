use inazuma::{App, Styled};

use crate::{
    ActiveTheme as _, Button, ButtonVariants as _, Icon, IconName, Sizable as _,
};

#[inline]
pub(crate) fn clear_button(cx: &App) -> Button {
    Button::with_id("clean")
        .icon(Icon::new(IconName::XCircle))
        .ghost()
        .xsmall()
        .tab_stop(false)
        .text_color(cx.theme().colors().muted_foreground)
}
