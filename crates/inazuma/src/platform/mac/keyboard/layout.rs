use super::*;
use objc2_foundation::NSString as Objc2NSString;

pub(crate) struct MacKeyboardLayout {
    id: String,
    name: String,
}

pub(crate) struct MacKeyboardMapper {
    key_equivalents: Option<HashMap<char, char>>,
}

impl PlatformKeyboardLayout for MacKeyboardLayout {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl PlatformKeyboardMapper for MacKeyboardMapper {
    fn map_key_equivalent(
        &self,
        mut keystroke: Keystroke,
        use_key_equivalents: bool,
    ) -> KeybindingKeystroke {
        if use_key_equivalents && let Some(key_equivalents) = &self.key_equivalents {
            if keystroke.key.chars().count() == 1
                && let Some(key) = key_equivalents.get(&keystroke.key.chars().next().unwrap())
            {
                keystroke.key = key.to_string();
            }
        }
        KeybindingKeystroke::from_keystroke(keystroke)
    }

    fn get_key_equivalents(&self) -> Option<&HashMap<char, char>> {
        self.key_equivalents.as_ref()
    }
}

impl MacKeyboardLayout {
    pub(crate) fn new() -> Self {
        unsafe {
            let current_keyboard = TISCopyCurrentKeyboardLayoutInputSource();

            let id_ptr = TISGetInputSourceProperty(
                current_keyboard,
                kTISPropertyInputSourceID as *const c_void,
            );
            let id_ns: &Objc2NSString = &*(id_ptr as *const Objc2NSString);
            let id = id_ns.to_string();

            let name_ptr = TISGetInputSourceProperty(
                current_keyboard,
                kTISPropertyLocalizedName as *const c_void,
            );
            let name_ns: &Objc2NSString = &*(name_ptr as *const Objc2NSString);
            let name = name_ns.to_string();

            Self { id, name }
        }
    }
}

impl MacKeyboardMapper {
    pub(crate) fn new(layout_id: &str) -> Self {
        let key_equivalents = get_key_equivalents(layout_id);

        Self { key_equivalents }
    }
}
