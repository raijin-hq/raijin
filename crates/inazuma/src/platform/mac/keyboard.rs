mod key_equivalents;
mod key_equivalents_ext;
mod layout;

use inazuma_collections::HashMap;
use std::ffi::c_void;

use inazuma::{KeybindingKeystroke, Keystroke, PlatformKeyboardLayout, PlatformKeyboardMapper};

use super::{
    TISCopyCurrentKeyboardLayoutInputSource, TISGetInputSourceProperty, kTISPropertyInputSourceID,
    kTISPropertyLocalizedName,
};

use key_equivalents::get_key_equivalents;
pub(crate) use layout::{MacKeyboardLayout, MacKeyboardMapper};
