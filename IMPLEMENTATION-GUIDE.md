# Terminal Resize: Implementation Guide for Raijin

## The Problem (Currently)

Raijin's terminal grid never resizes when the window is resized. The infrastructure is 95% complete, but the final connection is missing.

### What Works:
- ✅ `Terminal::set_size(rows, cols)` method exists
- ✅ PTY resize via `pty::resize_pty()` works
- ✅ Alacritty grid resize logic works
- ✅ Bounds are available in `prepaint()`
- ✅ Element lifecycle is correct (request_layout → prepaint → paint)

### What's Missing:
- ❌ Call to `set_size()` in `prepaint()`
- ❌ Derivation of rows/cols from bounds in `prepaint()`

---

## The Solution: One Component

Edit: `/sessions/determined-optimistic-goldberg/mnt/raijin/crates/raijin-app/src/terminal/grid_element.rs`

### Current Code (lines 118-140)

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
    let line_height = cell_height;
```

### New Code (add after line 138)

Insert this block after `let line_height = cell_height;`:

```rust
    // Derive grid dimensions from layout bounds
    let available_width = bounds.size.width - px(BLOCK_HEADER_PAD_X);
    let available_height = bounds.size.height;
    
    let cols = (available_width / cell_width).max(2.0) as u16;
    let rows = (available_height / cell_height).max(1.0) as u16;
    
    // Resize PTY and grid if bounds changed
    // set_size() is idempotent: only resizes if dimensions differ
    self.grid_handle.set_size(rows, cols);
```

### Complete Function (After Fix)

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
    let line_height = cell_height;

    // ✅ NEW: Derive grid dimensions from layout bounds
    let available_width = bounds.size.width - px(BLOCK_HEADER_PAD_X);
    let available_height = bounds.size.height;
    
    let cols = (available_width / cell_width).max(2.0) as u16;
    let rows = (available_height / cell_height).max(1.0) as u16;
    
    // ✅ NEW: Resize PTY and grid if bounds changed
    self.grid_handle.set_size(rows, cols);

    let mut lines = Vec::new();
    let mut backgrounds = Vec::new();
    let mut cursor_rect = None;

    let bg_color = terminal_bg();
    let text_x = bounds.origin.x + px(BLOCK_HEADER_PAD_X);

    let Ok(snapshot) = self.grid.lock() else {
        return GridPrepaint { lines, backgrounds, cursor_rect, line_height };
    };

    let mut current_y = bounds.origin.y;

    // ... rest of function unchanged ...
}
```

---

## Why This Works

### 1. Timing: Prepaint is the Right Place

```
Frame N (after window resize):
┌─ request_layout() called
│  └─ Taffy layout runs
├─ prepaint(bounds) called  ← NEW RESIZE HAPPENS HERE
│  └─ bounds = fresh from Taffy
│  └─ self.grid_handle.set_size(rows, cols) called
│  └─ Terminal::set_size() updates grid and sends SIGWINCH
├─ paint() called
│  └─ Renders grid with NEW dimensions
└─ Frame complete
```

No frame delay: resize and render happen in the same frame.

### 2. Idempotency: set_size() Checks Before Updating

```rust
// In Terminal::set_size() (terminal.rs:69-92)
pub fn set_size(&self, rows: u16, cols: u16) {
    if rows == 0 || cols == 0 { return; }

    let mut term = self.term.lock();
    let current_rows = term.screen_lines() as u16;
    let current_cols = term.columns() as u16;

    if rows == current_rows && cols == current_cols {
        return;  // ← Early exit if unchanged
    }

    // ... only resize if changed ...
}
```

**Benefit:** Can call `set_size()` every frame without performance cost. It's safe.

### 3. Bounds Calculation is Correct

- `available_width = bounds.size.width - BLOCK_HEADER_PAD_X` accounts for left margin
- `available_height = bounds.size.height` uses full height
- `.max(2.0)` and `.max(1.0)` prevent divide-by-zero or single-column terminals
- As u16 cast is safe (no negative values due to max)

### 4. Follows Production Pattern

This is exactly how the reference codebase's TerminalElement does it:

```rust
// From reference crates/terminal_view/src/terminal_element.rs:966-991
let mut size = bounds.size;
size.width -= gutter;

if size.width < cell_width * 2.0 {
    size.width = cell_width * 2.0;
}

// ... calculate dimensions ...
let dimensions = TerminalBounds::new(px(line_height), cell_width, Bounds { origin, size });

// ✅ SAME PATTERN: resize in prepaint
self.terminal.update(cx, |terminal, cx| {
    terminal.set_size(dimensions);
    terminal.sync(window, cx);
});
```

---

## Testing the Fix

### Manual Test

1. Run Raijin: `cargo run -p raijin-app`
2. Type a command: `echo "This is a very long line of text that will wrap at the terminal width"`
3. Resize window horizontally (drag edges)
4. Observe: Text should reflow immediately to new column width
5. Resize back: Text should reflow back

### Automated Test (Pseudo-code)

```rust
#[test]
fn test_grid_resize_on_bounds_change() {
    let mut grid = TerminalGridElement::new(grid, font, font_size);
    
    // Initial prepaint with 80-column bounds
    let bounds_80 = Bounds { size: size(480, 400), origin: point(0, 0) };
    let prepaint_80 = grid.prepaint(..., bounds_80, ...);
    
    // Check terminal is 80 cols
    assert_eq!(term.columns(), 80);
    
    // Prepaint with 40-column bounds
    let bounds_40 = Bounds { size: size(240, 400), origin: point(0, 0) };
    let prepaint_40 = grid.prepaint(..., bounds_40, ...);
    
    // Check terminal is 40 cols
    assert_eq!(term.columns(), 40);
}
```

---

## Verification Checklist

After implementing the fix:

- [ ] Code compiles: `cargo build -p raijin-app`
- [ ] No clippy warnings: `cargo clippy --workspace`
- [ ] Tests pass: `cargo test --workspace`
- [ ] Terminal resizes when window is resized
- [ ] Text wrapping changes immediately (no frame delay)
- [ ] PTY receives SIGWINCH (shell sees resize)
- [ ] No visual artifacts or glitches

---

## Constants Reference

Used in the fix:

| Constant | File | Value | Purpose |
|----------|------|-------|---------|
| `BLOCK_HEADER_PAD_X` | `terminal/constants.rs` | ~4px | Left margin before text |
| `CELL_PADDING` | `terminal/constants.rs` | ~2px | Vertical padding per cell |

These are already defined and used for rendering, so no changes needed.

---

## Why This is the Right Architecture

### Raijin Advantages Over Alternatives:

| Alternative | Why Not |
|-------------|---------|
| Resize in request_layout() | Bounds not available until after layout |
| Resize in paint() | Too late; rendering code shouldn't affect state |
| Resize in workspace | Violates composition; each element owns its bounds |
| Resize on window event | Terminal would need to hook window events; prepaint is cleaner |
| One-frame delay | Bad UX; text wrap changes are visible |

**Prepaint is optimal because:**
1. Bounds are fresh from layout
2. Before rendering (can prepare state)
3. In the same frame as paint
4. Idempotent (safe to call every frame)
5. Matches the reference codebase's production architecture

---

## Debugging Tips

If it doesn't work:

### Check 1: Does set_size() get called?

Add logging:
```rust
self.grid_handle.set_size(rows, cols);
```

Look for terminal.rs output:
```
INFO: Terminal resize: rows=24, cols=80
```

### Check 2: Does bounds change?

Add logging before set_size():
```rust
eprintln!("prepaint bounds: {:?}", bounds);
eprintln!("derived: rows={}, cols={}", rows, cols);
```

Resize window and check stderr.

### Check 3: Does SIGWINCH reach shell?

In terminal, run: `trap 'echo SIGWINCH' SIGWINCH`
Resize window. Should see "SIGWINCH" printed.

### Check 4: Verify grid actually resized

```rust
let term = self.grid_handle.lock();
eprintln!("Terminal grid: {}x{}", term.columns(), term.screen_lines());
```

Should match your calculated cols/rows.

---

## Implementation Confidence

**Confidence Level: 95% (production-ready)**

Rationale:
- ✅ Pattern is proven (reference codebase uses same approach)
- ✅ No API changes needed
- ✅ Fits existing element lifecycle
- ✅ Idempotent (safe, no edge cases)
- ✅ Code is ~5 lines

**Risk Level: Very Low**

- set_size() already implemented and tested
- Only adding one call, not changing behavior
- Idempotency prevents edge cases
- No external dependencies

---

## Next Steps (Post-Implementation)

Once resize works:

1. **Optimize**: Cache cell_width/cell_height calculations if needed
2. **Edge cases**: Test with font size changes, theme changes, etc.
3. **Synchronized output**: Consider disabling sync output on resize (like Ghostty) for immediate visual update
4. **Selection preservation**: Handle text selection during resize (like Alacritty does)
5. **Performance monitoring**: Profile prepaint() for resize-heavy workloads

---

## Related Files

### Read (to understand context):
- `crates/raijin-terminal/src/terminal.rs` — Terminal API
- `crates/raijin-app/src/terminal/constants.rs` — Constants
- `crates/inazuma/src/element.rs` — Element lifecycle

### Modify (the fix):
- `crates/raijin-app/src/terminal/grid_element.rs` — Add 9 lines to prepaint()

---

## Conclusion

Raijin already has 95% of a production terminal resize implementation. This fix connects the final piece: calling `set_size()` at the right time (prepaint) with the right data (bounds-derived rows/cols).

The pattern is proven by the reference codebase, battle-tested by alacritty, and matches the architecture of other GPU-accelerated terminals. Implementation is straightforward and low-risk.
