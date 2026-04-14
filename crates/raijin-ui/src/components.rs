// ── Existing raijin-ui components ─────────────────────────────────────────────
mod ai;
// app_shell moved to raijin-shell crate
mod avatar;
mod banner;
mod button;
mod callout;
mod chip;
mod collab;
mod context_menu;
mod count_badge;
mod data_table;
mod diff_stat;
mod disclosure;
mod divider;
mod dropdown_menu;
mod facepile;
mod gradient_fade;
mod group;
mod icon;
mod image;
mod indent_guides;
mod indicator;
mod keybinding;
mod keybinding_hint;
mod label;
mod list;
mod modal;
mod navigable;
mod popover;
mod popover_menu;
mod progress;
mod right_click_menu;
mod scrollbar;
pub mod select;
mod stack;
mod sticky_items;
mod tab;
mod toggle;
mod tooltip;
mod tree_view_item;

// ── Migrated from inazuma-component (L0) ─────────────────────────────────────
mod accordion;
mod alert;
mod badge;
mod breadcrumb;
mod chart;
mod checkbox;
mod clipboard;
mod color_picker;
mod description_list;
mod dialog;
mod dock;
mod erased_editor;
mod form;
mod group_box;
mod hover_card;
pub mod input;
mod kbd;
mod link;
mod menu;
mod modal_layer;
mod notification;
mod pagination;
mod plot;
mod radio;
mod rating;
mod resizable;
mod setting;
mod sheet;
mod sidebar;
mod skeleton;
mod slider;
mod spinner;
mod stepper;
mod switch;
mod table;
pub mod text;
mod tag;
pub mod time;
mod title_bar;
mod tree;

#[cfg(feature = "stories")]
mod stories;

/// Initialize all component keybindings and global state.
pub(crate) fn init(cx: &mut inazuma::App) {
    color_picker::init(cx);
    dialog::init(cx);
    dock::init(cx);
    input::init(cx);
    list::init(cx);
    menu::init(cx);
    popover::init(cx);
    select::init(cx);
    sheet::init(cx);
    table::init(cx);
    text::init(cx);
    time::init(cx);
    tree::init(cx);
}

// ── Re-exports: existing raijin-ui ───────────────────────────────────────────
pub use ai::*;
// AppShell re-exported from raijin-shell crate
pub use avatar::*;
pub use banner::*;
pub use button::*;
pub use callout::*;
pub use chip::*;
pub use collab::*;
pub use context_menu::*;
pub use count_badge::*;
pub use data_table::*;
pub use diff_stat::*;
pub use disclosure::*;
pub use divider::*;
pub use dropdown_menu::*;
pub use facepile::*;
pub use gradient_fade::*;
pub use group::*;
pub use icon::*;
pub use image::*;
pub use indent_guides::*;
pub use indicator::*;
pub use keybinding::*;
pub use keybinding_hint::*;
pub use label::*;
pub use list::*;
pub use modal::*;
pub use navigable::*;
pub use popover::*;
pub use popover_menu::*;
pub use progress::*;
pub use right_click_menu::*;
pub use scrollbar::*;
pub use select::*;
pub use stack::*;
pub use sticky_items::*;
pub use tab::*;
pub use toggle::*;
pub use tooltip::*;
pub use tree_view_item::*;

// ── Re-exports: migrated from inazuma-component ──────────────────────────────
pub use accordion::*;
pub use alert::*;
pub use badge::*;
pub use breadcrumb::*;
pub use chart::*;
pub use checkbox::*;
pub use clipboard::*;
pub use color_picker::*;
pub use description_list::*;
pub use dialog::*;
pub use dock::*;
pub use erased_editor::*;
pub use form::*;
pub use group_box::*;
pub use hover_card::*;
pub use input::*;
pub use kbd::*;
pub use link::*;
pub use menu::*;
pub use modal_layer::*;
pub use notification::*;
pub use pagination::*;
pub use plot::{
    IntoPlot, Plot, StrokeStyle, origin_point, polygon,
    AXIS_GAP, AxisText, PlotAxis, Grid, PlotLabel,
};
pub use radio::*;
pub use rating::*;
pub use resizable::*;
pub use setting::*;
pub use sheet::*;
pub use sidebar::*;
pub use skeleton::*;
pub use slider::*;
pub use spinner::*;
pub use stepper::*;
pub use switch::*;
pub use table::*;
pub use text::*;
pub use tag::*;
pub use time::*;
pub use title_bar::*;
pub use tree::*;

#[cfg(feature = "stories")]
pub use stories::*;
