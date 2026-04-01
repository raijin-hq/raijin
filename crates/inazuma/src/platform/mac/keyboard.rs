mod key_equivalents;
mod key_equivalents_ext;
mod layout;

use collections::HashMap;
use std::ffi::{CStr, c_void};

use objc::{msg_send, runtime::Object, sel, sel_impl};

use inazuma::{KeybindingKeystroke, Keystroke, PlatformKeyboardLayout, PlatformKeyboardMapper};

use super::{
    TISCopyCurrentKeyboardLayoutInputSource, TISGetInputSourceProperty, kTISPropertyInputSourceID,
    kTISPropertyLocalizedName,
};

use key_equivalents::get_key_equivalents;
pub(crate) use layout::{MacKeyboardLayout, MacKeyboardMapper};
