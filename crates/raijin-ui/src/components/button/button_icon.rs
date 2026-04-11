use crate::{CircularProgress, Icon, Sizable, Size, Spinner};
use inazuma::{App, IntoElement, RenderOnce, Window, prelude::FluentBuilder};
use inazuma_icons::IconName;

/// The icon element for a Button, supporting Icon, Spinner, or CircularProgress.
#[doc(hidden)]
#[derive(IntoElement)]
pub struct ButtonIcon {
    icon: ButtonIconVariant,
    loading_icon: Option<Icon>,
    loading: bool,
    size: Size,
}

impl From<Icon> for ButtonIcon {
    fn from(icon: Icon) -> Self {
        ButtonIcon::new(icon)
    }
}

impl From<IconName> for ButtonIcon {
    fn from(icon: IconName) -> Self {
        ButtonIcon::new(Icon::new(icon))
    }
}

impl From<Spinner> for ButtonIcon {
    fn from(spinner: Spinner) -> Self {
        ButtonIcon::new(spinner)
    }
}

impl ButtonIcon {
    /// Creates a new ButtonIcon with the given icon.
    pub fn new(icon: impl Into<ButtonIconVariant>) -> Self {
        Self {
            icon: icon.into(),
            loading_icon: None,
            loading: false,
            size: Size::Medium,
        }
    }

    pub(crate) fn loading_icon(mut self, icon: Option<Icon>) -> Self {
        self.loading_icon = icon;
        self
    }

    pub(crate) fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl Sizable for ButtonIcon {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// The underlying icon variant: an Icon, Spinner, or CircularProgress.
#[doc(hidden)]
#[derive(IntoElement)]
pub enum ButtonIconVariant {
    Icon(Icon),
    Spinner(Spinner),
    Progress(CircularProgress),
}

impl<T> From<T> for ButtonIconVariant
where
    T: Into<Icon>,
{
    fn from(icon: T) -> Self {
        Self::Icon(icon.into())
    }
}

impl From<Spinner> for ButtonIconVariant {
    fn from(spinner: Spinner) -> Self {
        Self::Spinner(spinner)
    }
}

impl From<CircularProgress> for ButtonIconVariant {
    fn from(progress: CircularProgress) -> Self {
        Self::Progress(progress)
    }
}

impl ButtonIconVariant {
    #[inline]
    pub(crate) fn is_spinner(&self) -> bool {
        matches!(self, Self::Spinner(_))
    }

    #[inline]
    pub(crate) fn is_progress(&self) -> bool {
        matches!(self, Self::Progress(_))
    }
}

impl Sizable for ButtonIconVariant {
    fn with_size(self, size: impl Into<Size>) -> Self {
        match self {
            Self::Icon(icon) => Self::Icon(icon.with_size(size)),
            Self::Spinner(spinner) => Self::Spinner(spinner.with_size(size)),
            Self::Progress(progress) => Self::Progress(progress.with_size(size)),
        }
    }
}

impl RenderOnce for ButtonIconVariant {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::Icon(icon) => icon.into_any_element(),
            Self::Spinner(spinner) => spinner.into_any_element(),
            Self::Progress(progress) => progress.into_any_element(),
        }
    }
}

impl RenderOnce for ButtonIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        if self.loading {
            if self.icon.is_spinner() || self.icon.is_progress() {
                self.icon.with_size(self.size).into_any_element()
            } else {
                Spinner::new()
                    .when_some(self.loading_icon, |this, icon| this.icon(icon))
                    .with_size(self.size)
                    .into_any_element()
            }
        } else {
            self.icon.with_size(self.size).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inazuma::test]
    fn test_button_icon_builder(_cx: &mut inazuma::TestAppContext) {
        let custom_icon = Icon::new(IconName::LoadCircle);
        let icon = ButtonIcon::new(IconName::Plus)
            .loading(true)
            .loading_icon(Some(custom_icon))
            .large();

        assert!(icon.loading);
        assert!(icon.loading_icon.is_some());
        assert_eq!(icon.size, Size::Large);
    }

    #[inazuma::test]
    fn test_button_icon_variant_types(_cx: &mut inazuma::TestAppContext) {
        let icon_variant = ButtonIconVariant::Icon(Icon::new(IconName::Plus));
        assert!(!icon_variant.is_spinner());
        assert!(!icon_variant.is_progress());

        let spinner_variant = ButtonIconVariant::Spinner(Spinner::new());
        assert!(spinner_variant.is_spinner());
        assert!(!spinner_variant.is_progress());

        let progress_variant =
            ButtonIconVariant::Progress(CircularProgress::new("test", 75.0));
        assert!(!progress_variant.is_spinner());
        assert!(progress_variant.is_progress());
    }
}
