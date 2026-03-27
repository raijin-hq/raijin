//! Live block rendering — the currently active command with streaming output.
//!
//! Shows the block header + live grid output with cursor.
//! This is used for the active block that is still receiving PTY output.

// The active block is rendered the same way as finished blocks via
// block_element::render_block() — the only difference is is_running=true
// which shows "running..." instead of duration, and the cursor is visible.
//
// No separate implementation needed — block_element handles both cases.
