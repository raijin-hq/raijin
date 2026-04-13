use std::collections::HashMap;

use inazuma::{AnyElement, App, Window};
use raijin_theme::{ActiveTheme, ChipColors};
use rayon::prelude::*;

use crate::context::ChipContext;
use crate::provider::{ChipId, ChipOutput, ChipProvider};
use crate::render::render_standard_chip;

/// Signature for custom chip render functions.
///
/// Providers with special UI (popovers, multi-color segments) register
/// a custom render function that replaces the default `render_standard_chip`.
pub type ChipRenderFn = fn(&ChipOutput, &ChipColors, &mut Window, &App) -> AnyElement;

/// Registry of chip providers and their optional custom renderers.
///
/// Manages all registered providers and produces ready-to-use `AnyElement`s.
/// Detection is O(1) per provider (uses pre-scanned DirContents).
/// Gathering runs in **parallel** via Rayon — each provider's
/// `gather()` executes on the thread pool, bounded by per-command timeout.
/// Rendering happens sequentially on the UI thread.
pub struct ChipRegistry {
    providers: Vec<Box<dyn ChipProvider>>,
    renderers: HashMap<ChipId, ChipRenderFn>,
}

impl ChipRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            renderers: HashMap::default(),
        }
    }

    /// Register a chip provider with the default renderer.
    pub fn register(&mut self, provider: impl ChipProvider + 'static) {
        self.providers.push(Box::new(provider));
    }

    /// Register a chip provider with a custom render function.
    pub fn register_with_renderer(
        &mut self,
        provider: impl ChipProvider + 'static,
        render: ChipRenderFn,
    ) {
        let id = provider.id();
        self.providers.push(Box::new(provider));
        self.renderers.insert(id, render);
    }

    /// Gather and render all available chips as ready-to-use elements.
    ///
    /// Two-phase approach:
    /// 1. **Gather phase (parallel)**: Filter available providers and run `gather()`
    ///    on Rayon's thread pool. Each command is timeout-protected (500ms default).
    ///    Parallel execution means 10 providers with 500ms timeout each still only
    ///    takes ~500ms total, not 5s.
    /// 2. **Render phase (sequential)**: Map gathered ChipOutputs to AnyElements
    ///    on the UI thread. Dispatches to custom renderers or `render_standard_chip()`.
    pub fn render_all(
        &self,
        ctx: &ChipContext,
        window: &mut Window,
        cx: &App,
    ) -> Vec<AnyElement> {
        let chip_colors = &cx.theme().colors().chip;

        // Phase 1: Parallel gather on Rayon thread pool.
        // Chips with empty labels are filtered out (command failed / version not found).
        // This matches pattern: module returns None → not displayed.
        let outputs: Vec<ChipOutput> = self
            .providers
            .par_iter()
            .filter(|p| p.is_available(ctx))
            .map(|p| p.gather(ctx))
            .filter(|o| !o.label.is_empty())
            .collect();

        // Phase 2: Sequential render on UI thread (needs &mut Window)
        outputs
            .iter()
            .map(|output| {
                if let Some(render_fn) = self.renderers.get(output.id) {
                    render_fn(output, chip_colors, window, cx)
                } else {
                    render_standard_chip(output, chip_colors, window, cx)
                }
            })
            .collect()
    }

    /// Reorder providers to match a layout specification.
    ///
    /// - IDs listed in `layout` appear in that order.
    /// - If `"*"` is in the layout, remaining providers are appended at that position.
    /// - If `"*"` is not present, providers not listed are hidden.
    pub fn apply_layout(&mut self, layout: &[String]) {
        let has_wildcard = layout.iter().any(|id| id == "*");

        let mut ordered: Vec<Box<dyn ChipProvider>> = Vec::with_capacity(self.providers.len());
        let mut remaining: Vec<Box<dyn ChipProvider>> = Vec::new();

        for id in layout {
            if id == "*" {
                continue;
            }
            if let Some(pos) = self.providers.iter().position(|p| p.id() == id.as_str()) {
                ordered.push(self.providers.remove(pos));
            }
        }

        remaining.append(&mut self.providers);

        if has_wildcard {
            let wildcard_pos = layout.iter().position(|id| id == "*").unwrap_or(ordered.len());
            let after_wildcard = ordered.split_off(wildcard_pos.min(ordered.len()));
            ordered.append(&mut remaining);
            ordered.extend(after_wildcard);
        }

        self.providers = ordered;
    }

    /// Set a custom renderer for an already-registered provider.
    pub fn set_renderer(&mut self, id: ChipId, render: ChipRenderFn) {
        self.renderers.insert(id, render);
    }

    /// Get the number of registered providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Create a registry with all providers (Tier 1–7) and custom renderers.
    pub fn with_all_providers() -> Self {
        let mut registry = Self::new();
        crate::providers::register_all(&mut registry);
        registry
    }
}
