//! raijin-chips — Modular chip provider system for the Raijin terminal.
//!
//! **Feature Crate** — contains Detection + Gathering + Rendering + Icons.
//! Provides a standard detection and gathering pipeline for context chips,
//! plus a rendering layer that produces ready-to-use Inazuma elements.
//!
//! # Architecture
//!
//! - `provider.rs` — `ChipProvider` trait (Send+Sync, data only) + `ChipOutput`
//! - `context.rs` — `ChipContext`, `DirContents`, `DetectionCache`
//! - `command.rs` — Timeout-protected command execution
//! - `registry.rs` — `ChipRegistry` with renderer map + `render_all()`
//! - `render.rs` — Standard chip renderer + theme color mapping
//! - `icons.rs` — Provider icon string → `IconName` mapping
//! - `providers/` — 69+ individual provider implementations
//!

pub mod command;
pub mod context;
pub mod icons;
pub mod provider;
pub mod providers;
pub mod registry;
pub mod render;

pub use command::{exec_cmd, CommandOutput, DEFAULT_COMMAND_TIMEOUT};
pub use context::{
    ChipContext, DetectionCache, DirContents, collect_chip_env_vars,
    DEFAULT_SCAN_TIMEOUT,
};
pub use icons::icon_name_from_str;
pub use provider::{ChipId, ChipOutput, ChipProvider, ChipSegment, parse_version_number};
pub use registry::{ChipRegistry, ChipRenderFn};
pub use render::{render_standard_chip, chip_theme_color};
