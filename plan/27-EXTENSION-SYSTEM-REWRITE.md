# Phase 27 — Extension System Rewrite

## Problem

The extension system was copied from Zed but never properly migrated:

1. **WIT namespace is `zed:extension`** — must be `raijin:extension`
2. **10 legacy API versions** (`since_v0_0_1` through `since_v0_8_0`) for backwards compatibility with Zed extensions we don't have
3. **wasmtime 33** — tree-sitter 0.26.8 pulls wasmtime 36 transitively, causing dual-version conflicts
4. **800+ compile errors** from the wasmtime 33→36 API changes (`async: true` removed from bindgen!, WASI trait restructuring)
5. **`zed::extension::*` module paths** throughout the generated bindings

## Decision

We have **zero published extensions** — no backwards compatibility needed. Clean rewrite with only the latest API.

## Scope

### Crates affected
- `raijin-extension-api` — WIT definitions (source of truth)
- `raijin-extension-host` — WASM runtime, WIT bindgen, extension lifecycle
- `raijin-extension-cli` — CLI for building extensions
- `raijin-extension` — Extension trait definitions

### Crates that depend on extension-host
- `raijin-extensions-ui`
- `raijin-language-models`
- `raijin-agent-ui`
- `raijin-remote-server`
- `raijin-recent-projects`
- `raijin-activity-indicator`

## Implementation

### Phase 1 — WIT Namespace Migration
- Rename all `.wit` files: `package zed:extension` → `package raijin:extension`
- Single API version directory (no `since_v*` versioning)
- Update all WIT interface/world names

### Phase 2 — wasmtime 36 Upgrade
- Remove `async: true` from `bindgen!` macros (no longer needed in wasmtime 36)
- Update WASI imports: `wasmtime_wasi::p2::IoView`, `WasiView`, `WasiCtx` API changes
- Fix `WasiCtxBuilder` usage for wasmtime 36
- Single `bindgen!` invocation for the one API version

### Phase 3 — Extension Host Simplification
- Delete all `since_v0_*` binding files (10 files)
- Single `wit.rs` with one bindgen! for the latest API
- Remove version-dispatch logic in `wasm_host.rs`
- Remove `ExtensionApiVersion` enum and version negotiation
- Clean up trait impls (no more version-specific `impl` blocks)

### Phase 4 — Extension CLI Update
- Update `raijin-extension-cli` to build against the single API
- Remove multi-version compilation support
- Fix wasmtime Engine type mismatch (now single version)

### Phase 5 — Downstream Crate Updates
- Update all 6 dependent crates for any API changes
- Verify `raijin-extensions-ui` still works
- Verify `raijin-language-models` extension loading

## Non-Goals
- Extension marketplace/distribution (separate future work)
- Extension signing/sandboxing improvements
- New extension capabilities (that's feature work, not migration)

## Verification
```bash
cargo check --workspace  # zero errors, zero warnings
cargo test -p raijin-extension-host
```
