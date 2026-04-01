use super::*;

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

            let id: *mut Object = TISGetInputSourceProperty(
                current_keyboard,
                kTISPropertyInputSourceID as *const c_void,
            );
            let id: *const std::os::raw::c_char = msg_send![id, UTF8String];
            let id = CStr::from_ptr(id).to_str().unwrap().to_string();

            let name: *mut Object = TISGetInputSourceProperty(
                current_keyboard,
                kTISPropertyLocalizedName as *const c_void,
            );
            let name: *const std::os::raw::c_char = msg_send![name, UTF8String];
            let name = CStr::from_ptr(name).to_str().unwrap().to_string();

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
