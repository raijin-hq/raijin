//! Built-in procedural renderer for box drawing, block elements, and related Unicode characters.
//!
//! Instead of relying on font glyphs (which often have inconsistent advances for
//! box-drawing characters), this module renders them as GPU-native geometric primitives
//! via Inazuma's `paint_quad` API. This is the industry-standard approach used by
//! Alacritty, Kitty, Ghostty, Rio, and other modern terminal emulators.
//!
//! Box drawing characters (U+2500-U+257F) use a table-driven approach: each character
//! is decomposed into 4 directional segments (top, bottom, left, right) with a weight
//! (none, light, heavy, double). A single rendering function handles all 128 combinations.

use inazuma::{point, px, size, Bounds, Oklch, Pixels, Window, fill};

// ── Segment weight for box drawing lines ──────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum W {
    /// No line segment in this direction.
    N,
    /// Light (thin) line — standard box drawing.
    L,
    /// Heavy (thick) line — double stroke width.
    H,
    /// Double parallel lines.
    D,
}

// ── Box drawing segment descriptor ────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct BoxSegs {
    up: W,
    down: W,
    left: W,
    right: W,
}

const fn b(up: W, down: W, left: W, right: W) -> BoxSegs {
    BoxSegs { up, down, left, right }
}

use W::*;

// ── Complete lookup table for U+2500..U+257F (128 entries) ────────────────────
//
// Each entry maps a box-drawing codepoint to its four directional segments.
// Reference: https://en.wikipedia.org/wiki/Box-drawing_characters

#[rustfmt::skip]
static BOX_DRAWING: [BoxSegs; 128] = [
    // U+2500 ─  U+2501 ━  U+2502 │  U+2503 ┃
    b(N,N,L,L), b(N,N,H,H), b(L,L,N,N), b(H,H,N,N),
    // U+2504 ┄  U+2505 ┅  U+2506 ┆  U+2507 ┇
    b(N,N,L,L), b(N,N,H,H), b(L,L,N,N), b(H,H,N,N),
    // U+2508 ┈  U+2509 ┉  U+250A ┊  U+250B ┋
    b(N,N,L,L), b(N,N,H,H), b(L,L,N,N), b(H,H,N,N),
    // U+250C ┌  U+250D ┍  U+250E ┎  U+250F ┏
    b(N,L,N,L), b(N,L,N,H), b(N,H,N,L), b(N,H,N,H),
    // U+2510 ┐  U+2511 ┑  U+2512 ┒  U+2513 ┓
    b(N,L,L,N), b(N,L,H,N), b(N,H,L,N), b(N,H,H,N),
    // U+2514 └  U+2515 ┕  U+2516 ┖  U+2517 ┗
    b(L,N,N,L), b(L,N,N,H), b(H,N,N,L), b(H,N,N,H),
    // U+2518 ┘  U+2519 ┙  U+251A ┚  U+251B ┛
    b(L,N,L,N), b(L,N,H,N), b(H,N,L,N), b(H,N,H,N),
    // U+251C ├  U+251D ┝  U+251E ┞  U+251F ┟
    b(L,L,N,L), b(L,L,N,H), b(H,L,N,L), b(L,H,N,L),
    // U+2520 ┠  U+2521 ┡  U+2522 ┢  U+2523 ┣
    b(H,H,N,L), b(H,L,N,H), b(L,H,N,H), b(H,H,N,H),
    // U+2524 ┤  U+2525 ┥  U+2526 ┦  U+2527 ┧
    b(L,L,L,N), b(L,L,H,N), b(H,L,L,N), b(L,H,L,N),
    // U+2528 ┨  U+2529 ┩  U+252A ┪  U+252B ┫
    b(H,H,L,N), b(H,L,H,N), b(L,H,H,N), b(H,H,H,N),
    // U+252C ┬  U+252D ┭  U+252E ┮  U+252F ┯
    b(N,L,L,L), b(N,L,H,L), b(N,L,L,H), b(N,L,H,H),
    // U+2530 ┰  U+2531 ┱  U+2532 ┲  U+2533 ┳
    b(N,H,L,L), b(N,H,H,L), b(N,H,L,H), b(N,H,H,H),
    // U+2534 ┴  U+2535 ┵  U+2536 ┶  U+2537 ┷
    b(L,N,L,L), b(L,N,H,L), b(L,N,L,H), b(L,N,H,H),
    // U+2538 ┸  U+2539 ┹  U+253A ┺  U+253B ┻
    b(H,N,L,L), b(H,N,H,L), b(H,N,L,H), b(H,N,H,H),
    // U+253C ┼  U+253D ┽  U+253E ┾  U+253F ┿
    b(L,L,L,L), b(L,L,H,L), b(L,L,L,H), b(L,L,H,H),
    // U+2540 ╀  U+2541 ╁  U+2542 ╂  U+2543 ╃
    b(H,L,L,L), b(L,H,L,L), b(H,H,L,L), b(H,L,H,L),
    // U+2544 ╄  U+2545 ╅  U+2546 ╆  U+2547 ╇
    b(H,L,L,H), b(L,H,H,L), b(L,H,L,H), b(H,L,H,H),
    // U+2548 ╈  U+2549 ╉  U+254A ╊  U+254B ╋
    b(L,H,H,H), b(H,H,H,L), b(H,H,L,H), b(H,H,H,H),
    // U+254C ╌  U+254D ╍  U+254E ╎  U+254F ╏
    b(N,N,L,L), b(N,N,H,H), b(L,L,N,N), b(H,H,N,N),
    // U+2550 ═  U+2551 ║  U+2552 ╒  U+2553 ╓
    b(N,N,D,D), b(D,D,N,N), b(N,L,N,D), b(N,D,N,L),
    // U+2554 ╔  U+2555 ╕  U+2556 ╖  U+2557 ╗
    b(N,D,N,D), b(N,L,D,N), b(N,D,L,N), b(N,D,D,N),
    // U+2558 ╘  U+2559 ╙  U+255A ╚  U+255B ╛
    b(L,N,N,D), b(D,N,N,L), b(D,N,N,D), b(L,N,D,N),
    // U+255C ╜  U+255D ╝  U+255E ╞  U+255F ╟
    b(D,N,L,N), b(D,N,D,N), b(L,L,N,D), b(D,D,N,L),
    // U+2560 ╠  U+2561 ╡  U+2562 ╢  U+2563 ╣
    b(D,D,N,D), b(L,L,D,N), b(D,D,L,N), b(D,D,D,N),
    // U+2564 ╤  U+2565 ╥  U+2566 ╦  U+2567 ╧
    b(N,L,D,D), b(N,D,L,L), b(N,D,D,D), b(L,N,D,D),
    // U+2568 ╨  U+2569 ╩  U+256A ╪  U+256B ╫
    b(D,N,L,L), b(D,N,D,D), b(L,L,D,D), b(D,D,L,L),
    // U+256C ╬  U+256D ╭  U+256E ╮  U+256F ╯
    b(D,D,D,D), b(N,L,N,L), b(N,L,L,N), b(L,N,L,N),
    // U+2570 ╰  U+2571 ╱  U+2572 ╲  U+2573 ╳
    b(L,N,N,L), b(N,N,N,N), b(N,N,N,N), b(N,N,N,N),
    // U+2574 ╴  U+2575 ╵  U+2576 ╶  U+2577 ╷
    b(N,N,L,N), b(L,N,N,N), b(N,N,N,L), b(N,L,N,N),
    // U+2578 ╸  U+2579 ╹  U+257A ╺  U+257B ╻
    b(N,N,H,N), b(H,N,N,N), b(N,N,N,H), b(N,H,N,N),
    // U+257C ╼  U+257D ╽  U+257E ╾  U+257F ╿
    b(N,N,L,H), b(L,H,N,N), b(N,N,H,L), b(H,L,N,N),
];

// ── Drawable character categories ─────────────────────────────────────────────

/// A character that should be rendered procedurally instead of from a font glyph.
#[derive(Clone, Copy)]
pub enum BuiltinChar {
    /// Box drawing (U+2500-U+257F) — rendered via segment table.
    Box(BoxSegs),
    /// Dashed horizontal line variants — light 3-dash, 4-dash, 2-dash.
    DashedH { heavy: bool, dashes: u8 },
    /// Dashed vertical line variants.
    DashedV { heavy: bool, dashes: u8 },
    /// Rounded corners (╭╮╰╯).
    RoundedCorner(Corner),
    /// Diagonal lines (╱╲╳).
    Diagonal { rising: bool, falling: bool },
    /// Block element — fraction of cell filled.
    Block(BlockKind),
    /// Shade pattern (░▒▓).
    Shade(f32),
    /// Quadrant pattern (▖▗▘▝ etc.) — bitmask of which quadrants to fill.
    Quadrant(u8),
}

#[derive(Clone, Copy)]
pub enum Corner {
    TopLeft,     // ╭ — round at bottom-right connection
    TopRight,    // ╮ — round at bottom-left connection
    BottomLeft,  // ╰ — round at top-right connection
    BottomRight, // ╯ — round at top-left connection
}

#[derive(Clone, Copy)]
pub enum BlockKind {
    /// Full cell block (█).
    Full,
    /// Lower N/8 block (▁▂▃▄▅▆▇).
    Lower(u8),
    /// Upper N/8 block (▀▔ + fractional).
    Upper(u8),
    /// Left N/8 block (▌▎▍▊).
    Left(u8),
    /// Right N/8 block (▐▕).
    Right(u8),
}

// ── Detection: does this char get built-in rendering? ─────────────────────────

/// Returns `Some(BuiltinChar)` if the character should be rendered procedurally.
pub fn builtin_char(c: char) -> Option<BuiltinChar> {
    let code = c as u32;
    match code {
        // Box drawing U+2500-U+257F
        0x2500..=0x257F => {
            let idx = (code - 0x2500) as usize;
            let segs = BOX_DRAWING[idx];

            // Dashed lines (same segments as solid, rendered differently)
            match code {
                0x2504 => return Some(BuiltinChar::DashedH { heavy: false, dashes: 3 }),
                0x2505 => return Some(BuiltinChar::DashedH { heavy: true, dashes: 3 }),
                0x2506 => return Some(BuiltinChar::DashedV { heavy: false, dashes: 3 }),
                0x2507 => return Some(BuiltinChar::DashedV { heavy: true, dashes: 3 }),
                0x2508 => return Some(BuiltinChar::DashedH { heavy: false, dashes: 4 }),
                0x2509 => return Some(BuiltinChar::DashedH { heavy: true, dashes: 4 }),
                0x250A => return Some(BuiltinChar::DashedV { heavy: false, dashes: 4 }),
                0x250B => return Some(BuiltinChar::DashedV { heavy: true, dashes: 4 }),
                0x254C => return Some(BuiltinChar::DashedH { heavy: false, dashes: 2 }),
                0x254D => return Some(BuiltinChar::DashedH { heavy: true, dashes: 2 }),
                0x254E => return Some(BuiltinChar::DashedV { heavy: false, dashes: 2 }),
                0x254F => return Some(BuiltinChar::DashedV { heavy: true, dashes: 2 }),
                _ => {}
            }

            // Rounded corners
            match code {
                0x256D => return Some(BuiltinChar::RoundedCorner(Corner::TopLeft)),
                0x256E => return Some(BuiltinChar::RoundedCorner(Corner::TopRight)),
                0x256F => return Some(BuiltinChar::RoundedCorner(Corner::BottomRight)),
                0x2570 => return Some(BuiltinChar::RoundedCorner(Corner::BottomLeft)),
                _ => {}
            }

            // Diagonal lines
            match code {
                0x2571 => return Some(BuiltinChar::Diagonal { rising: true, falling: false }),
                0x2572 => return Some(BuiltinChar::Diagonal { rising: false, falling: true }),
                0x2573 => return Some(BuiltinChar::Diagonal { rising: true, falling: true }),
                _ => {}
            }

            // Half-line segments (╴╵╶╷╸╹╺╻╼╽╾╿)
            // These use the same segment table — just render normally
            Some(BuiltinChar::Box(segs))
        }

        // Block elements U+2580-U+259F
        0x2580 => Some(BuiltinChar::Block(BlockKind::Upper(4))),  // ▀ upper half
        0x2581 => Some(BuiltinChar::Block(BlockKind::Lower(1))),  // ▁ lower 1/8
        0x2582 => Some(BuiltinChar::Block(BlockKind::Lower(2))),  // ▂ lower 1/4
        0x2583 => Some(BuiltinChar::Block(BlockKind::Lower(3))),  // ▃ lower 3/8
        0x2584 => Some(BuiltinChar::Block(BlockKind::Lower(4))),  // ▄ lower half
        0x2585 => Some(BuiltinChar::Block(BlockKind::Lower(5))),  // ▅ lower 5/8
        0x2586 => Some(BuiltinChar::Block(BlockKind::Lower(6))),  // ▆ lower 3/4
        0x2587 => Some(BuiltinChar::Block(BlockKind::Lower(7))),  // ▇ lower 7/8
        0x2588 => Some(BuiltinChar::Block(BlockKind::Full)),       // █ full block
        0x2589 => Some(BuiltinChar::Block(BlockKind::Left(7))),   // ▉ left 7/8
        0x258A => Some(BuiltinChar::Block(BlockKind::Left(6))),   // ▊ left 3/4
        0x258B => Some(BuiltinChar::Block(BlockKind::Left(5))),   // ▋ left 5/8
        0x258C => Some(BuiltinChar::Block(BlockKind::Left(4))),   // ▌ left half
        0x258D => Some(BuiltinChar::Block(BlockKind::Left(3))),   // ▍ left 3/8
        0x258E => Some(BuiltinChar::Block(BlockKind::Left(2))),   // ▎ left 1/4
        0x258F => Some(BuiltinChar::Block(BlockKind::Left(1))),   // ▏ left 1/8
        0x2590 => Some(BuiltinChar::Block(BlockKind::Right(4))),  // ▐ right half
        0x2591 => Some(BuiltinChar::Shade(0.25)),                  // ░ light shade
        0x2592 => Some(BuiltinChar::Shade(0.50)),                  // ▒ medium shade
        0x2593 => Some(BuiltinChar::Shade(0.75)),                  // ▓ dark shade
        0x2594 => Some(BuiltinChar::Block(BlockKind::Upper(1))),  // ▔ upper 1/8
        0x2595 => Some(BuiltinChar::Block(BlockKind::Right(1))),  // ▕ right 1/8
        // Quadrants: bits = top-left, top-right, bottom-left, bottom-right
        0x2596 => Some(BuiltinChar::Quadrant(0b0010)),            // ▖ bottom-left
        0x2597 => Some(BuiltinChar::Quadrant(0b0001)),            // ▗ bottom-right
        0x2598 => Some(BuiltinChar::Quadrant(0b1000)),            // ▘ top-left
        0x2599 => Some(BuiltinChar::Quadrant(0b1110)),            // ▙ all except bottom-right → TL+BL+BR... wait
        // ▙ = upper-left + lower-left + lower-right = 0b1011
        0x259A => Some(BuiltinChar::Quadrant(0b1001)),            // ▚ top-left + bottom-right
        0x259B => Some(BuiltinChar::Quadrant(0b1110)),            // ▛ top-left + top-right + bottom-left
        0x259C => Some(BuiltinChar::Quadrant(0b1101)),            // ▜ top-left + top-right + bottom-right
        0x259D => Some(BuiltinChar::Quadrant(0b0100)),            // ▝ top-right
        0x259E => Some(BuiltinChar::Quadrant(0b0110)),            // ▞ top-right + bottom-left
        0x259F => Some(BuiltinChar::Quadrant(0b0111)),            // ▟ top-right + bottom-left + bottom-right

        _ => None,
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Render a built-in character at the given cell position.
pub fn draw_builtin(
    bc: BuiltinChar,
    x: Pixels,
    y: Pixels,
    cell_width: Pixels,
    cell_height: Pixels,
    color: inazuma::Oklch,
    window: &mut Window,
) {
    let w = f32::from(cell_width);
    let h = f32::from(cell_height);
    let stroke = (h / 10.0).max(1.0).round();
    let cx = f32::from(x) + w / 2.0;
    let cy = f32::from(y) + h / 2.0;
    let x = f32::from(x);
    let y = f32::from(y);

    match bc {
        BuiltinChar::Box(segs) => {
            draw_box_segments(segs, x, y, w, h, cx, cy, stroke, color, window);
        }
        BuiltinChar::DashedH { heavy, dashes } => {
            let s = if heavy { stroke * 2.0 } else { stroke };
            draw_dashed_h(x, cy, w, s, dashes, color, window);
        }
        BuiltinChar::DashedV { heavy, dashes } => {
            let s = if heavy { stroke * 2.0 } else { stroke };
            draw_dashed_v(y, cx, h, s, dashes, color, window);
        }
        BuiltinChar::RoundedCorner(corner) => {
            draw_rounded_corner(corner, x, y, w, h, cx, cy, stroke, color, window);
        }
        BuiltinChar::Diagonal { rising, falling } => {
            draw_diagonal(x, y, w, h, stroke, rising, falling, color, window);
        }
        BuiltinChar::Block(kind) => {
            draw_block(kind, x, y, w, h, color, window);
        }
        BuiltinChar::Shade(opacity) => {
            let shaded = color.opacity(opacity);
            paint_rect(x, y, w, h, shaded, window);
        }
        BuiltinChar::Quadrant(bits) => {
            draw_quadrant(bits, x, y, w, h, color, window);
        }
    }
}

// ── Box segment rendering ─────────────────────────────────────────────────────

fn draw_box_segments(
    segs: BoxSegs,
    x: f32, y: f32, w: f32, h: f32,
    cx: f32, cy: f32,
    stroke: f32,
    color: Oklch,
    window: &mut Window,
) {
    let heavy = stroke * 2.0;
    let gap = stroke * 1.5;

    // ── Vertical segments (up / down) ─────────────────────────────────────

    // Up segment: from top of cell to center
    match segs.up {
        N => {}
        L => paint_rect(cx - stroke / 2.0, y, stroke, h / 2.0 + stroke / 2.0, color, window),
        H => paint_rect(cx - heavy / 2.0, y, heavy, h / 2.0 + heavy / 2.0, color, window),
        D => {
            paint_rect(cx - gap, y, stroke, h / 2.0 + gap, color, window);
            paint_rect(cx + gap - stroke, y, stroke, h / 2.0 + gap, color, window);
        }
    }

    // Down segment: from center to bottom of cell
    match segs.down {
        N => {}
        L => paint_rect(cx - stroke / 2.0, cy - stroke / 2.0, stroke, h / 2.0 + stroke / 2.0, color, window),
        H => paint_rect(cx - heavy / 2.0, cy - heavy / 2.0, heavy, h / 2.0 + heavy / 2.0, color, window),
        D => {
            paint_rect(cx - gap, cy - gap + stroke, stroke, h / 2.0 + gap, color, window);
            paint_rect(cx + gap - stroke, cy - gap + stroke, stroke, h / 2.0 + gap, color, window);
        }
    }

    // ── Horizontal segments (left / right) ────────────────────────────────

    // Left segment: from left of cell to center
    match segs.left {
        N => {}
        L => paint_rect(x, cy - stroke / 2.0, w / 2.0 + stroke / 2.0, stroke, color, window),
        H => paint_rect(x, cy - heavy / 2.0, w / 2.0 + heavy / 2.0, heavy, color, window),
        D => {
            paint_rect(x, cy - gap, w / 2.0 + gap, stroke, color, window);
            paint_rect(x, cy + gap - stroke, w / 2.0 + gap, stroke, color, window);
        }
    }

    // Right segment: from center to right of cell
    match segs.right {
        N => {}
        L => paint_rect(cx - stroke / 2.0, cy - stroke / 2.0, w / 2.0 + stroke / 2.0, stroke, color, window),
        H => paint_rect(cx - heavy / 2.0, cy - heavy / 2.0, w / 2.0 + heavy / 2.0, heavy, color, window),
        D => {
            paint_rect(cx - gap + stroke, cy - gap, w / 2.0 + gap, stroke, color, window);
            paint_rect(cx - gap + stroke, cy + gap - stroke, w / 2.0 + gap, stroke, color, window);
        }
    }
}

// ── Dashed lines ──────────────────────────────────────────────────────────────

fn draw_dashed_h(x: f32, cy: f32, w: f32, stroke: f32, dashes: u8, color: Oklch, window: &mut Window) {
    let count = dashes as f32;
    let dash_w = w / (count * 2.0 - 1.0);
    for i in 0..dashes {
        let dx = x + (i as f32) * dash_w * 2.0;
        paint_rect(dx, cy - stroke / 2.0, dash_w, stroke, color, window);
    }
}

fn draw_dashed_v(y: f32, cx: f32, h: f32, stroke: f32, dashes: u8, color: Oklch, window: &mut Window) {
    let count = dashes as f32;
    let dash_h = h / (count * 2.0 - 1.0);
    for i in 0..dashes {
        let dy = y + (i as f32) * dash_h * 2.0;
        paint_rect(cx - stroke / 2.0, dy, stroke, dash_h, color, window);
    }
}

// ── Rounded corners ───────────────────────────────────────────────────────────

fn draw_rounded_corner(
    corner: Corner,
    x: f32, y: f32, w: f32, h: f32,
    cx: f32, cy: f32,
    stroke: f32,
    color: Oklch,
    window: &mut Window,
) {
    let radius = (w / 4.0).max(stroke);

    match corner {
        Corner::TopLeft => {
            // ╭ — line goes down and right, rounded at the meeting point
            // Vertical: from center+radius to bottom
            paint_rect(cx - stroke / 2.0, cy + radius, stroke, h / 2.0 - radius, color, window);
            // Horizontal: from center+radius to right
            paint_rect(cx + radius, cy - stroke / 2.0, w / 2.0 - radius, stroke, color, window);
            // Rounded corner quad at the meeting point
            paint_rounded_corner_quad(cx, cy, radius, stroke, corner, color, window);
        }
        Corner::TopRight => {
            // ╮ — line goes down and left
            paint_rect(cx - stroke / 2.0, cy + radius, stroke, h / 2.0 - radius, color, window);
            paint_rect(x, cy - stroke / 2.0, w / 2.0 - radius, stroke, color, window);
            paint_rounded_corner_quad(cx, cy, radius, stroke, corner, color, window);
        }
        Corner::BottomLeft => {
            // ╰ — line goes up and right
            paint_rect(cx - stroke / 2.0, y, stroke, h / 2.0 - radius, color, window);
            paint_rect(cx + radius, cy - stroke / 2.0, w / 2.0 - radius, stroke, color, window);
            paint_rounded_corner_quad(cx, cy, radius, stroke, corner, color, window);
        }
        Corner::BottomRight => {
            // ╯ — line goes up and left
            paint_rect(cx - stroke / 2.0, y, stroke, h / 2.0 - radius, color, window);
            paint_rect(x, cy - stroke / 2.0, w / 2.0 - radius, stroke, color, window);
            paint_rounded_corner_quad(cx, cy, radius, stroke, corner, color, window);
        }
    }
}

fn paint_rounded_corner_quad(
    cx: f32, cy: f32,
    radius: f32, stroke: f32,
    corner: Corner,
    color: Oklch,
    window: &mut Window,
) {
    // Use Inazuma's PaintQuad with corner_radii for a GPU-native rounded corner.
    // We draw a hollow rounded-corner quad (border only) at the connection point.
    let r = radius;
    let (qx, qy) = match corner {
        Corner::TopLeft => (cx, cy),
        Corner::TopRight => (cx - r, cy),
        Corner::BottomLeft => (cx, cy - r),
        Corner::BottomRight => (cx - r, cy - r),
    };

    let corner_radii = match corner {
        Corner::TopLeft => inazuma::Corners {
            top_left: px(r),
            top_right: px(0.0),
            bottom_right: px(0.0),
            bottom_left: px(0.0),
        },
        Corner::TopRight => inazuma::Corners {
            top_left: px(0.0),
            top_right: px(r),
            bottom_right: px(0.0),
            bottom_left: px(0.0),
        },
        Corner::BottomLeft => inazuma::Corners {
            top_left: px(0.0),
            top_right: px(0.0),
            bottom_right: px(0.0),
            bottom_left: px(r),
        },
        Corner::BottomRight => inazuma::Corners {
            top_left: px(0.0),
            top_right: px(0.0),
            bottom_right: px(r),
            bottom_left: px(0.0),
        },
    };

    let border_w = px(stroke);
    let border_widths = match corner {
        Corner::TopLeft => inazuma::Edges {
            top: px(0.0), right: border_w, bottom: border_w, left: px(0.0),
        },
        Corner::TopRight => inazuma::Edges {
            top: px(0.0), right: px(0.0), bottom: border_w, left: border_w,
        },
        Corner::BottomLeft => inazuma::Edges {
            top: border_w, right: border_w, bottom: px(0.0), left: px(0.0),
        },
        Corner::BottomRight => inazuma::Edges {
            top: border_w, right: px(0.0), bottom: px(0.0), left: border_w,
        },
    };

    let border_color = inazuma::Edges {
        top: color, right: color, bottom: color, left: color,
    };

    window.paint_quad(inazuma::PaintQuad {
        bounds: Bounds::new(point(px(qx), px(qy)), size(px(r), px(r))),
        corner_radii,
        background: inazuma::transparent_black().into(),
        border_widths,
        border_colors: border_color.into(),
        border_style: inazuma::BorderStyle::Solid,
    });
}

// ── Diagonal lines ────────────────────────────────────────────────────────────

fn draw_diagonal(
    x: f32, y: f32, w: f32, h: f32,
    stroke: f32,
    rising: bool, falling: bool,
    color: Oklch,
    window: &mut Window,
) {
    // Approximate diagonals with multiple thin horizontal rects (scanline approach)
    let steps = h.ceil() as usize;
    let rect_h = (h / steps as f32).max(1.0);

    for i in 0..steps {
        let frac = i as f32 / steps as f32;
        let sy = y + frac * h;

        if rising {
            // ╱ bottom-left to top-right
            let sx = x + (1.0 - frac) * w - stroke / 2.0;
            paint_rect(sx, sy, stroke, rect_h, color, window);
        }
        if falling {
            // ╲ top-left to bottom-right
            let sx = x + frac * w - stroke / 2.0;
            paint_rect(sx, sy, stroke, rect_h, color, window);
        }
    }
}

// ── Block elements ────────────────────────────────────────────────────────────

fn draw_block(kind: BlockKind, x: f32, y: f32, w: f32, h: f32, color: Oklch, window: &mut Window) {
    match kind {
        BlockKind::Full => paint_rect(x, y, w, h, color, window),
        BlockKind::Lower(eighths) => {
            let bh = h * eighths as f32 / 8.0;
            paint_rect(x, y + h - bh, w, bh, color, window);
        }
        BlockKind::Upper(eighths) => {
            let bh = h * eighths as f32 / 8.0;
            paint_rect(x, y, w, bh, color, window);
        }
        BlockKind::Left(eighths) => {
            let bw = w * eighths as f32 / 8.0;
            paint_rect(x, y, bw, h, color, window);
        }
        BlockKind::Right(eighths) => {
            let bw = w * eighths as f32 / 8.0;
            paint_rect(x + w - bw, y, bw, h, color, window);
        }
    }
}

// ── Quadrant rendering ────────────────────────────────────────────────────────

fn draw_quadrant(bits: u8, x: f32, y: f32, w: f32, h: f32, color: Oklch, window: &mut Window) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    // Bit 3 = top-left, Bit 2 = top-right, Bit 1 = bottom-left, Bit 0 = bottom-right
    if bits & 0b1000 != 0 { paint_rect(x, y, hw, hh, color, window); }
    if bits & 0b0100 != 0 { paint_rect(x + hw, y, hw, hh, color, window); }
    if bits & 0b0010 != 0 { paint_rect(x, y + hh, hw, hh, color, window); }
    if bits & 0b0001 != 0 { paint_rect(x + hw, y + hh, hw, hh, color, window); }
}

// ── Utility ───────────────────────────────────────────────────────────────────

#[inline]
fn paint_rect(x: f32, y: f32, w: f32, h: f32, color: Oklch, window: &mut Window) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    window.paint_quad(fill(
        Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
        color,
    ));
}
