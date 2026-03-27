# Rendering-Optimierung für Raijin

## Status Quo: Was Inazuma bereits hat

Inazuma (unser GPUI-Fork) hat bereits eine vollständige GPU-Rendering-Pipeline:

- **Glyph-Atlas** (MetalAtlas): Jeder Glyph wird einmal gerastert und in einer
  GPU-Textur gespeichert. Content-addressed via FxHashMap. Auto-grow von 1024²
  bis 16384². Monochrome (Text) und Polychrome (Emoji) getrennt.
- **Instanced Rendering**: Alle Sprites mit gleicher Textur werden in einem
  einzigen `draw_primitives_instanced()` Call gezeichnet. 6 Vertices × N Instanzen.
- **Scene Batching**: Primitives werden nach Draw-Order, Typ, und Textur-ID
  gruppiert um Pipeline-State-Wechsel zu minimieren.
- **LineLayout Cache**: Zwei-Frame FxHashMap Cache für shape_line() Ergebnisse.
  Cache-Key: text_hash + font_size + runs (ohne Farbe).
- **Ref-counted Tiles**: Unbenutzte Glyphs werden automatisch evicted.
- **Lazy Rasterization**: Glyphs werden erst gerastert wenn sie zum ersten Mal
  gemalt werden. CoreText (macOS) für die Rasterisierung.

Wir müssen KEINEN eigenen Glyph-Atlas, kein eigenes Instanced Rendering, und
keinen eigenen Text-Shape-Cache bauen. Das existiert alles.

## Das eigentliche Problem

Bei jedem Frame wird für JEDEN Block JEDE Zeile:
1. `shape_line()` aufgerufen (Glyph-Lookup, Font-Metrics, Kerning)
2. Jeder Glyph einzeln via `paint_glyph()` in die Scene eingefügt
3. Jede Hintergrund-Zelle via `paint_quad()` in die Scene eingefügt

Für fertige Blocks ist das 100% redundant — der Content ändert sich nie.

## Optimierungs-Architektur (4 Stufen)

### Stufe 1: Viewport-Culling (sofort, ~30 Zeilen)

Blocks die komplett außerhalb des sichtbaren Bereichs liegen werden übersprungen.

```rust
// Im Prepaint, vor dem Block-Loop:
let viewport_top = scroll_offset;
let viewport_bottom = scroll_offset + bounds.size.height;

for (i, block) in blocks.iter().enumerate() {
    let block_top = block_y_positions[i];
    let block_bottom = block_top + block_heights[i];

    // Komplett über dem Viewport → überspringen
    if block_bottom < viewport_top { continue; }
    // Komplett unter dem Viewport → restliche Blocks auch nicht sichtbar
    if block_top > viewport_bottom { break; }

    // Block ist (teilweise) sichtbar → rendern
    render_block(block, ...);
}
```

Erwarteter Gewinn: Bei 50+ Blocks sind typischerweise nur 2-3 sichtbar.
97% weniger Arbeit bei langem Scrollback.

### Stufe 2: Size-Change Guard (sofort, ~5 Zeilen)

set_size() nur aufrufen wenn sich die Größe tatsächlich geändert hat:

```rust
// Im Element oder Workspace:
if new_rows != self.last_rows || new_cols != self.last_cols {
    self.handle.set_size(new_rows, new_cols);
    self.last_rows = new_rows;
    self.last_cols = new_cols;
}
```

Verhindert dass block_router.resize() bei jedem Frame durch alle Blocks iteriert.

### Stufe 3: Block-Level Scene Cache (mittelfristig, ~150 Zeilen)

Fertige Blocks erzeugen bei jedem Frame die identische Scene (gleiche Sprites,
gleiche Quads, gleiche Positionen). Statt sie jedes Mal neu zu berechnen:

```rust
struct CachedBlockScene {
    /// Shaped lines — das Ergebnis von shape_line() pro Zeile
    shaped_lines: Vec<ShapedLine>,
    /// Background rects (ANSI bg colors)
    backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    /// Cache-Invalidierung
    valid_columns: usize,
    valid_font_version: u64,
    valid_theme_version: u64,
}

// Pro Block im TerminalElement:
block_scene_cache: FxHashMap<BlockId, CachedBlockScene>
```

Rendering-Flow für fertige Blocks:
1. Cache-Hit → ShapedLines und Backgrounds direkt aus Cache nehmen
2. Positionen neu berechnen (billig: nur Y-Offset basierend auf Scroll)
3. paint_glyph() mit gecachten ShapedLines aufrufen — Inazuma's Atlas
   cached die Glyphs bereits, also ist das nur "Sprite in Scene einfügen"

Cache-Miss (erster Render oder Invalidierung):
1. Normal shapen via shape_line()
2. Ergebnis in Cache speichern
3. Weiter wie bei Cache-Hit

Invalidierung:
- Font-Änderung → alle Caches invalidieren
- Theme-Änderung → nur Farben neu berechnen, Shapes bleiben
- Resize (Spalten) → alle Caches invalidieren (Reflow nötig)
- Scroll → KEINE Invalidierung (nur Y-Offset ändert sich)

Erwarteter Gewinn: 95%+ Reduktion der shape_line() Calls für fertige Blocks.
Bei 10 sichtbaren fertigen Blocks × 30 Zeilen = 300 shape_line() Calls
eliminiert, nur der aktive Block (1 Block × aktuelle Zeilen) wird neu geshaped.

### Stufe 4: Aktiver Block — Incremental Shaping (langfristig, ~100 Zeilen)

Auch der aktive Block ändert sich meist nur am Ende (neue Zeilen kommen dazu).
Statt alle Zeilen neu zu shapen:

```rust
struct ActiveBlockShapeState {
    /// Bereits geshapte Zeilen
    shaped_lines: Vec<ShapedLine>,
    /// Anzahl Zeilen beim letzten Shape-Durchlauf
    last_shaped_count: usize,
    /// Hash der letzten Zeile (für Änderungserkennung)
    last_line_hash: u64,
}
```

Pro Frame:
1. Neue Zeilen (index > last_shaped_count) → shapen und appenden
2. Letzte Zeile → Hash vergleichen, bei Änderung neu shapen
3. Alle anderen Zeilen → aus Cache

Das nutzt die Tatsache dass Terminal-Output append-only ist: neue Zeilen
kommen unten dazu, bestehende ändern sich nur selten (Cursor-Zeile).

Erwarteter Gewinn: Auch bei schnellem Output (100 Zeilen/Sekunde) werden
pro Frame nur 1-5 Zeilen neu geshaped statt alle.

### Stufe 5: shape_line_by_hash() statt shape_line() (parallel zu Stufe 3/4, ~20 Zeilen)

Inazuma's Text-System bietet eine schnellere API die wir aktuell nicht nutzen.
Statt:

```rust
// Aktuell in grid_element.rs — alloziert SharedString bei JEDEM Aufruf
let shaped = window.text_system().shape_line(
    SharedString::from(line_text),  // ← Allokation, auch bei Cache-Hit
    font_size,
    &runs,
    Some(cell_width),
);
```

Gibt es:

```rust
// Besser — String wird nur bei Cache-Miss materialisiert
let shaped = window.text_system().shape_line_by_hash(
    line_hash,           // u64 — billig zu berechnen
    line_text.len(),
    font_size,
    &runs,
    Some(cell_width),
    || SharedString::from(line_text),  // ← lazy, nur bei Cache-Miss
);
```

Die Hash-basierte Variante vermeidet die String-Allokation komplett bei
Cache-Hits. Für Terminal-Content (der sich zwischen Frames selten ändert)
bedeutet das: kein Heap-Alloc für 95%+ der Zeilen.

Inazuma's LineLayoutCache macht intern:
1. Hash im Current-Frame-Cache suchen → Hit → sofort zurück (keine Allokation)
2. Hash im Previous-Frame-Cache suchen → Hit → in Current kopieren, zurück
3. Miss → `materialize_text()` aufrufen → shapen → cachen

Das ist die gleiche Cache-Infrastruktur, nur mit einem schnelleren Einstiegspunkt.

## Zusammenfassung: Warum wir NICHTS an Inazuma ändern müssen

| Was | Wer macht es | Status |
|-----|-------------|--------|
| Glyph-Rasterisierung | MetalAtlas + CoreText | ✅ Fertig |
| Glyph-Caching | MetalAtlas (FxHashMap, ref-counted) | ✅ Fertig |
| Instanced Rendering | MetalRenderer (draw_primitives_instanced) | ✅ Fertig |
| Scene Batching | Scene::batches() (nach Typ + Textur) | ✅ Fertig |
| Text-Shape-Caching | LineLayoutCache (Zwei-Frame FxHashMap) | ✅ Fertig |
| Viewport-Culling | TerminalElement | ❌ Fehlt → Stufe 1 |
| Size-Change Guard | TerminalElement | ❌ Fehlt → Stufe 2 |
| Block-Level Scene Cache | TerminalElement | ❌ Fehlt → Stufe 3 |
| Incremental Shaping | TerminalElement | ❌ Fehlt → Stufe 4 |
| Hash-basiertes Shaping | grid_element.rs | ❌ Fehlt → Stufe 5 |

Alles was fehlt sind 4 Optimierungen auf APP-EBENE (in terminal_element.rs
bzw. block_element.rs). Keine Änderungen an Inazuma's Rendering-Pipeline.
Kein eigener Glyph-Atlas. Kein eigenes Instanced Rendering. Kein eigener
Hash-Algorithmus. Inazuma macht das alles schon — wir müssen nur aufhören,
bei jedem Frame die gleiche redundante Arbeit reinzuschieben.

## Implementierungsreihenfolge

| Stufe | Aufwand | Gewinn | Wann |
|-------|---------|--------|------|
| 1. Viewport-Culling | ~30 Zeilen, 1 Stunde | 90%+ weniger Arbeit bei Scrollback | Sofort |
| 2. Size-Change Guard | ~5 Zeilen, 10 Minuten | Kein Resize pro Frame | Sofort |
| 3. Block Scene Cache | ~150 Zeilen, 4 Stunden | 95%+ weniger shape_line() | Diese Woche |
| 4. Incremental Shaping | ~100 Zeilen, 3 Stunden | Schneller aktiver Block | Nach Stufe 3 |
| 5. shape_line_by_hash() | ~20 Zeilen, 30 Minuten | Keine String-Allokation bei Cache-Hit | Parallel zu 3/4 |

Stufe 1+2 zuerst — das sind zusammen 35 Zeilen Code und eliminieren die
schlimmsten Performance-Probleme. Dann testen. Stufe 5 kann parallel zu
Stufe 3/4 eingebaut werden (einfacher Austausch des API-Calls).
Wenn es immer noch zu langsam ist, Stufe 3+4 bauen.
