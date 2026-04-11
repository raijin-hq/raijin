// Diagnostics module - works on all platforms (no tree-sitter dependency)
mod diagnostics;
mod rope_ext;

pub use diagnostics::*;
pub use rope_ext::{Rope, RopeExt, RopeLines};

// Native implementation with full tree-sitter support
#[cfg(not(target_family = "wasm"))]
mod highlight_styles;
#[cfg(not(target_family = "wasm"))]
mod highlighter;
#[cfg(not(target_family = "wasm"))]
mod languages;
#[cfg(not(target_family = "wasm"))]
mod registry;

#[cfg(not(target_family = "wasm"))]
pub use highlight_styles::unique_styles;
#[cfg(not(target_family = "wasm"))]
pub use highlighter::*;
#[cfg(not(target_family = "wasm"))]
pub use languages::*;
#[cfg(not(target_family = "wasm"))]
pub use registry::*;

// WASM stub implementation (no tree-sitter support)
#[cfg(target_family = "wasm")]
mod wasm_stub;
#[cfg(target_family = "wasm")]
pub use wasm_stub::*;
