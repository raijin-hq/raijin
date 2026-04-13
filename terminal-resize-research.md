# Terminal Resize Architecture Research

## Executive Summary

**The resize pattern is: Call `set_size()` in `prepaint()` with bounds from the layout system.** This solves the chicken-and-egg problem because:

1. **Layout phase** (Taffy): Computes element dimensions but doesn't make them available
2. **Prepaint phase** ✅ **RESIZE HAPPENS HERE**: Bounds are NOW available as a parameter
3. **Paint phase**: Grid renders with correct dimensions

The PTY resize happens as a side effect of the terminal state update, not in a separate message.

---

## 1. Reference Implementation (GPUI Framework) — PRODUCTION PATTERN

**File:** `crates/terminal_view/src/terminal_element.rs` (lines 863-991)

### Lifecycle: request_layout → prepaint → paint

```rust
// PREPAINT: bounds are NOW available as parameter
fn prepaint(
    &mut self,
    global_id: Option<&GlobalElementId>,
    inspector_id: Option<&gpui::InspectorElementId>,
    bounds: Bounds<Pixels>,  // ← Layout bounds, only available HERE
    _: &mut Self::RequestLayoutState,
    window: &mut Window,
    cx: &mut App,
) -> Self::PrepaintState {
    // ... calculate cell_width, line_height from bounds ...
    let mut size = bounds.size;
    size.width -= gutter;

    if size.width < cell_width * 2.0 {
        size.width = cell_width * 2.0;  // Min bounds check
    }

    // Calculate terminal dimensions from bounds
    let dimensions = TerminalBounds::new(
        px(line_height),
        cell_width,
        Bounds { origin: bounds.origin, size }
    );

    // ✅ RESIZE HAPPENS IN PREPAINT
    self.terminal.update(cx, |terminal, cx| {
        terminal.set_size(dimensions);  // Line 991
        terminal.sync(window, cx);
        // ... rest of prepaint work ...
    });
}
```

### Key Points:
- **Bounds passed to prepaint**: `Bounds<Pixels>` parameter contains the element's computed layout size
- **No frame delay**: Resize happens in the same frame as layout
- **Conditional resize**: Terminal checks `if self.last_content.terminal_bounds != new_bounds` before queueing resize event
- **Side effect in update**: The resize is a side effect of calling `terminal.update(cx, |terminal, cx| {...})`

---

## 2. Terminal State Machine (Reference) — How Resize is Processed

**File:** `crates/terminal/src/terminal.rs` (lines 1438-1043)

### set_size() → InternalEvent::Resize → Process on sync

```rust
// UI thread: prepaint calls this
pub fn set_size(&mut self, new_bounds: TerminalBounds) {
    if self.last_content.terminal_bounds != new_bounds {
        self.events.push_back(InternalEvent::Resize(new_bounds))  // Queue event
    }
}

// Background thread: processes queued events
match &InternalEvent::Resize(mut new_bounds) {
    // 1. Sanity check bounds
    new_bounds.bounds.size.height =
        cmp::max(new_bounds.line_height, new_bounds.height());
    new_bounds.bounds.size.width =
        cmp::max(new_bounds.cell_width, new_bounds.width());

    self.last_content.terminal_bounds = new_bounds;

    // 2. Send SIGWINCH to PTY
    if let TerminalType::Pty { pty_tx, .. } = &self.terminal_type {
        pty_tx.0.send(Msg::Resize(new_bounds.into())).ok();
    }

    // 3. Resize alacritty grid
    term.resize(new_bounds);

    // 4. Signal for re-layout if needed (matches recalculation)
    if !self.matches.is_empty() {
        cx.emit(Event::Wakeup);
    }
}
```

### Key Pattern:
- `set_size()` is **idempotent**: checks if size changed before queueing
- Event is queued, not processed immediately
- PTY gets SIGWINCH asynchronously
- Grid resizes happen atomically with lock held

---

## 3. Alacritty Terminal Grid — Resize Implementation

**File:** `alacritty_terminal/src/term/mod.rs` (lines 655-694)

```rust
pub fn resize<S: Dimensions>(&mut self, size: S) {
    let old_cols = self.columns();
    let old_lines = self.screen_lines();

    let num_cols = size.columns();
    let num_lines = size.screen_lines();

    // Early exit: no-op if size unchanged
    if old_cols == num_cols && old_lines == num_lines {
        debug!("Term::resize dimensions unchanged");
        return;
    }

    debug!("New num_cols is {num_cols} and num_lines is {num_lines}");

    // Move vi mode cursor with content
    let history_size = self.history_size();
    let mut delta = num_lines as i32 - old_lines as i32;
    let min_delta = cmp::min(0, num_lines as i32 - self.grid.cursor.point.line.0 - 1);
    delta = cmp::min(cmp::max(delta, min_delta), history_size as i32);
    self.vi_mode_cursor.point.line += delta;

    // Resize both active and inactive grids
    let is_alt = self.mode.contains(TermMode::ALT_SCREEN);
    self.grid.resize(!is_alt, num_lines, num_cols);
    self.inactive_grid.resize(is_alt, num_lines, num_cols);

    // Invalidate selections and tabs if dimensions changed
    if old_cols != num_cols {
        self.selection = None;
        self.tabs.resize(num_cols);
    }
    // ... cursor clamping logic ...
}
```

### Key Points:
- **Dual grid**: Manages both active screen and alt-screen independently
- **Content preservation**: Moves cursor and preserves selection during resize
- **Tab recalculation**: Tabs are reset when columns change
- **Early exit**: Returns immediately if dimensions unchanged

---

## 4. Ghostty's Approach — Concurrent Resize Pattern

**File:** `src/termio/Termio.zig` (lines 479-518)

Ghostty separates PTY resize, grid resize, and renderer notification:

```zig
pub fn resize(
    self: *Termio,
    td: *ThreadData,
    size: renderer.Size,
) !void {
    self.size = size;
    const grid_size = size.grid();

    // Step 1: Resize PTY immediately
    try self.backend.resize(grid_size, size.terminal());

    // Step 2: Lock and resize grid + renderer state
    {
        self.renderer_state.mutex.lock();
        defer self.renderer_state.mutex.unlock();

        try self.terminal.resize(
            self.alloc,
            grid_size.columns,
            grid_size.rows,
        );

        // Update pixel sizes
        self.terminal.width_px = grid_size.columns * self.size.cell.width;
        self.terminal.height_px = grid_size.rows * self.size.cell.height;

        // Disable synchronized output for immediate visual update
        self.terminal.modes.set(.synchronized_output, false);

        // Send in-band size report if enabled
        if (self.terminal.modes.get(.in_band_size_reports)) {
            try self.sizeReportLocked(td, .mode_2048);
        }
    }

    // Step 3: Notify renderer to update GPU and re-render
    _ = self.renderer_mailbox.push(.{ .resize = size }, .{ .forever = {} });
    self.renderer_wakeup.notify() catch {};
}
```

### Key Differences from Reference:
- **Explicit locking scope**: Minimizes critical section
- **Synchronized output disabled**: Forces immediate rendering
- **In-band reporting**: Can send CSI 9 size reports to shell
- **Renderer notification**: Mailbox pattern for GPU update

---

## 5. Inazuma Element Lifecycle

**Files:**
- Reference GPUI: `crates/gpui/src/element.rs`
- Inazuma (our fork): `crates/inazuma/src/element.rs`

### Phase Order: request_layout → prepaint → paint

```
Frame N:
┌─────────────────────────────────────────────┐
│ 1. request_layout(cx)                       │
│    └─ Returns LayoutId from Taffy           │
│    └─ Bounds NOT available yet              │
└─────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────┐
│ 2. Taffy Layout Computation                 │
│    └─ Solves constraints                    │
│    └─ Bounds NOW computed                   │
└─────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────┐
│ 3. prepaint(bounds, cx)  ✅ RESIZE HERE     │
│    └─ bounds: Bounds<Pixels> available      │
│    └─ Safe to trigger state changes         │
│    └─ Returns PrepaintState                 │
└─────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────┐
│ 4. paint(bounds, cx)                        │
│    └─ Render with prepaint state            │
└─────────────────────────────────────────────┘
```

### Signatures:

```rust
pub trait Element {
    type RequestLayoutState: 'static;
    type PrepaintState: 'static;

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState);

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,      // ← ONLY available in prepaint
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState;

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    );
}
```

### Key: prepaint() is the right place to:
- Calculate derived dimensions from bounds
- Update state with new sizes
- Trigger side effects (resize PTY, grid changes)
- Prepare rendering state

---

## 6. Current State in Raijin

**Files:**
- `crates/raijin-terminal/src/terminal.rs` (line 69): `set_size()` defined but never called
- `crates/raijin-app/src/terminal/grid_element.rs` (lines 106-116): `request_layout()` is minimal
- `crates/raijin-app/src/terminal/grid_element.rs` (lines 118-248): `prepaint()` receives bounds but doesn't use them for resize

### The Problem:

```rust
// Terminal::set_size exists but is never called
pub fn set_size(&self, rows: u16, cols: u16) {
    if rows == 0 || cols == 0 { return; }

    let mut term = self.term.lock();
    let current_rows = term.screen_lines() as u16;
    let current_cols = term.columns() as u16;

    if rows == current_rows && cols == current_cols {
        return;  // Early exit: idempotent
    }

    let dims = TermDimensions { ... };
    term.resize(dims);
    drop(term);
    let _ = pty::resize_pty(self.pty_master.as_ref(), rows, cols);
}

// TerminalGridElement::prepaint receives bounds but ignores them
fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,  // ← Available but unused for resize
    _layout: &mut Self::RequestLayoutState,
    window: &mut Window,
    _cx: &mut App,
) -> Self::PrepaintState {
    // Only uses bounds for rendering calculation, not PTY resize
    let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);

    // ... no call to terminal.set_size() ...
}
```

### Solution: Call set_size() in prepaint()

The grid_element needs to:
1. Calculate rows/cols from `bounds` and font metrics
2. Call `self.terminal.set_size(rows, cols)` before rendering

---

## 7. One-Frame Delay Pattern

**Is it acceptable?** Generally, NO for interactive terminals.

### Why the reference codebase does it in the same frame:
- User expectation: resize window → immediate text reflow
- Text wrapping depends on column count
- Delaying creates visual glitch

### Ghostty's approach:
- Synchronous in the render thread
- Disables synchronized output to force immediate display
- No frame delay

### Raijin's options:
1. **Prepaint resize** (recommended, like reference): Same frame, clean
2. **Deferred resize** (one-frame delay): Causes text wrap artifacts, bad UX

---

## 8. Critical Observation: Reference Workspace Pattern

Key architectural decision: **Workspace doesn't manage terminal resize directly.**

Instead:
- TerminalView/TerminalElement owns the Terminal
- Each TerminalElement calls `terminal.set_size()` in its own prepaint
- This scales: multiple terminals, each managing its bounds independently

**Implication for Raijin:**
- If Workspace contains multiple terminals, each grid_element resizes independently
- No need for workspace-level resize coordination
- Each block can have a different size (not recommended UI, but architecture supports it)

---

## 9. The Frame Synchronization Question

**Q: Do bounds always correspond to a layout frame?**

Yes:
- `prepaint()` is called after Taffy completes layout
- Bounds reflect the last layout constraint resolution
- Window resize events trigger layout invalidation → new frame → prepaint with new bounds

**Window resize flow:**
```
Window::handle_resize_event()
  → Layout::invalidate()
  → Next frame render:
      request_layout() + Taffy
      prepaint(new_bounds)
      paint()
```

---

## 10. Specific Code References

### Reference Terminal: Resize in Prepaint

```
File: crates/terminal_view/src/terminal_element.rs
Lines: 863-1012
Pattern:
  - Line 867: bounds: Bounds<Pixels> parameter
  - Line 966-982: Calculate dimensions from bounds
  - Line 991: terminal.set_size(dimensions)
  - Line 992: terminal.sync(window, cx)
```

### Alacritty: Grid Resize

```
File: alacritty_terminal/src/term/mod.rs
Lines: 655-694
Pattern:
  - Line 662-664: Idempotent check
  - Line 677-678: Resize both grids
  - Line 681-690: Invalidate selections/tabs if cols changed
```

### GPUI Element Lifecycle

```
File: crates/gpui/src/element.rs
Lines: 73-104
Pattern:
  - Line 73-79: request_layout(id, window, cx) → LayoutId
  - Line 83-91: prepaint(bounds, cx) → PrepaintState
  - Line 95-104: paint(bounds, cx)
```

---

## Recommendation for Raijin

### Immediate Fix (Minimal, 1 frame):

In `crates/raijin-app/src/terminal/grid_element.rs`:

```rust
fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _layout: &mut Self::RequestLayoutState,
    window: &mut Window,
    _cx: &mut App,
) -> Self::PrepaintState {
    let font_size = px(self.font_size);
    let font_id = window.text_system().resolve_font(&self.font);
    let cell_width = window
        .text_system()
        .advance(font_id, font_size, 'M')
        .unwrap_or_default()
        .width;
    let ascent = window.text_system().ascent(font_id, font_size);
    let descent = window.text_system().descent(font_id, font_size);
    let cell_height = ascent + descent.abs() + px(CELL_PADDING);

    // Calculate rows/cols from bounds
    let available_width = bounds.size.width;
    let available_height = bounds.size.height;

    let cols = (available_width / cell_width).max(2.0) as u16;  // Min 2 cols
    let rows = (available_height / cell_height).max(1.0) as u16;  // Min 1 row

    // ✅ RESIZE PTY HERE
    self.grid_handle.set_size(rows, cols);

    // ... rest of prepaint logic unchanged ...
}
```

### Why This Works:

1. **Bounds are fresh**: From Taffy layout this frame
2. **Idempotent**: `set_size()` checks and early-exits if unchanged
3. **No frame delay**: Resize and render happen same frame
4. **Follows reference pattern**: Production-tested approach

---

## Relevant Architecture Decisions

| Decision | Raijin | Reference | Ghostty |
|----------|--------|-----|---------|
| Where resize happens | ❌ Nowhere | ✅ prepaint() | ✅ sync() method |
| Frame delay | ✅ (bug) | ❌ Same frame | ❌ Same frame |
| Idempotent check | ✅ set_size() has it | ✅ In state.set_size() | ✅ Implicit |
| PTY notification | ✅ SIGWINCH | ✅ via Msg::Resize | ✅ Direct ioctl |
| Grid type | alacritty grid | alacritty grid | Custom grid |
| Lock strategy | FairMutex | Arc + Entity | Render thread lock |

---

## Summary Table

| System | Lifecycle | Resize Trigger | Timing | Idempotent |
|--------|-----------|----------------|--------|-----------|
| **Reference** | request_layout → Taffy → prepaint → paint | prepaint(bounds) | Same frame | Yes (state.set_size checks) |
| **Alacritty** | Event loop driven | window resize event | Event frame | Yes (grid.resize checks) |
| **Ghostty** | Renderer driven | resize() method | Sync frame | Implicit (always resize) |
| **Raijin (current)** | request_layout → prepaint → paint | ❌ NEVER | N/A | set_size() ready but unused |

**Key insight:** Raijin has 90% of the solution already. Just need to call `set_size()` in `prepaint()` with bounds-derived dimensions.
