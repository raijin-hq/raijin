mod list_item;
mod searchable;
mod select;
mod state;
mod traits;

pub use searchable::{SearchableVec, SelectGroup};
pub use select::Select;
pub use state::{SelectEvent, SelectState};
pub use traits::{SelectDelegate, SelectItem};

pub(crate) fn init(cx: &mut inazuma::App) {
    use inazuma::KeyBinding;
    use crate::utils::actions::{Cancel, Confirm, SelectDown, SelectUp};

    const CONTEXT: &str = "Select";
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new(
            "secondary-enter",
            Confirm { secondary: true },
            Some(CONTEXT),
        ),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ])
}
