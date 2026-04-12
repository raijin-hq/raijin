use crate::utils::actions::{
    Cancel, SelectDown, SelectFirst, SelectLast, SelectNextColumn, SelectPageDown,
    SelectPageUp, SelectPrevColumn, SelectUp,
};
use inazuma::{App, KeyBinding};

const CONTEXT: &str = "DataTable";
pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectPrevColumn, Some(CONTEXT)),
        KeyBinding::new("right", SelectNextColumn, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", SelectPageDown, Some(CONTEXT)),
        KeyBinding::new("tab", SelectNextColumn, Some(CONTEXT)),
        KeyBinding::new("shift-tab", SelectPrevColumn, Some(CONTEXT)),
    ]);
}
