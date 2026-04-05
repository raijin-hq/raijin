use inazuma::WindowAppearance;
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAppearanceNameVibrantDark, NSAppearanceNameVibrantLight,
};

pub(crate) fn window_appearance_from_native(appearance: &NSAppearance) -> WindowAppearance {
    let name = appearance.name();
    unsafe {
        if *name == *NSAppearanceNameVibrantLight {
            WindowAppearance::VibrantLight
        } else if *name == *NSAppearanceNameVibrantDark {
            WindowAppearance::VibrantDark
        } else if *name == *NSAppearanceNameAqua {
            WindowAppearance::Light
        } else if *name == *NSAppearanceNameDarkAqua {
            WindowAppearance::Dark
        } else {
            log::warn!("unknown appearance: {:?}", name);
            WindowAppearance::Light
        }
    }
}
