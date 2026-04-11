use inazuma::{App, Global, OwnedMenu};

/// Stores the application menus.
pub struct MenuGlobalState {
    app_menus: Vec<OwnedMenu>,
}

impl Global for MenuGlobalState {}

impl MenuGlobalState {
    pub fn init(cx: &mut App) {
        cx.set_global(Self {
            app_menus: Vec::new(),
        });
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn app_menus(&self) -> &[OwnedMenu] {
        &self.app_menus
    }

    pub fn set_app_menus(&mut self, menus: Vec<OwnedMenu>) {
        self.app_menus = menus;
    }
}
