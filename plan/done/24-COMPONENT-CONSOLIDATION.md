# Phase 24: Component Library Konsolidierung — inazuma-component → raijin-ui

## Problem

Zwei Component Libraries mit 12 doppelten Komponenten:

| Library | Dateien | Zeilen | Importiert von |
|---|---|---|---|
| `inazuma-component` (eigene erweiterte Library) | 250 | 69.617 | 19 Crates |
| `raijin-ui` (portiert aus Referenz-Codebase) | 104 | 25.016 | 71 Crates |

## Ziel

**`inazuma-component` wird komplett aufgelöst. Alles wird `raijin-ui`.** Eine einzige, ultimative Component Library.

Die 12 doppelten Komponenten werden zu ultimativen Versionen verschmolzen die das Beste aus beiden kombinieren.

`raijin-ui-input` (eigenständiger Crate, Editor-Wrapper) wird NICHT angefasst — der hat mit dieser Konsolidierung nichts zu tun.

## Doppelte Komponenten — verschmelzen (12 Stück)

Für jede: beide Versionen lesen, Features aus beiden kombinieren, ultimative Version in `raijin-ui` bauen.

| Komponente | inazuma-component | raijin-ui |
|---|---|---|
| avatar | 3 Dateien (erweitert) | 1 Datei |
| button | 5 Dateien (erweitert) | 4 Dateien |
| chip | 1 | 1 |
| divider | 1 | 1 |
| popover | 2 | 2 |
| tooltip | 2 | 1 |
| modal | 1 | 1 |
| label | 2 | 3 |
| list | 4 | 7 |
| tab | 8 (erweitert) | 3 |
| icon | 1 | 3 |
| toggle | 2 | 2 |
| scrollbar | 2 | 1 |

## Einzigartige Komponenten — nach raijin-ui verschieben (16+ Stück)

Aus `inazuma-component`: accordion, alert, animation, chart, checkbox, clipboard, collapsible, color_picker, description_list, dialog, form, group_box, hover_card, focus_trap, breadcrumb, badge

## Input System — nach raijin-ui als Submodul

Das Form Input System aus `inazuma-component` (10k Zeilen) wandert nach `raijin-ui::input`:
- Rope-basierter Mini-Editor
- Selection, IME, Auto-Pairs, Mask Patterns
- OTP Input, Number Input
- Cursor, Blink, Popovers
- Search

Das ist ein eigenständiges Input-System — NICHT dasselbe wie `raijin-ui-input` (der Editor-Wrapper). Zwei verschiedene Dinge:
- `raijin_ui::Input` = Leichtgewichtiger Form Input (Terminal Input Bar, Search Bars, Settings-Felder)
- `raijin_ui_input::InputField` = Voller Editor als Textfeld (Code-Editing, Agent-Prompts)

## Weitere Systeme aus inazuma-component → raijin-ui

- **Highlighter** (syntax highlighting, diagnostics) → `raijin-ui::highlighter`
- **Dock Primitives** → prüfen ob `raijin-workspace` das schon hat
- **TitleBar** → `raijin-ui::title_bar`
- **Component Registry** (visual testing) → `raijin-ui` oder eigener Test-Crate

## Ergebnis

Nach Phase 24:
- `raijin-ui` = Die einzige UI Component Library (alle Komponenten + Input System + Highlighter)
- `inazuma-component` = **Existiert nicht mehr**
- `raijin-ui-input` = Unverändert (eigenständiger Editor-Wrapper Crate)

## Prozess

### Phase 1: Verschmelzung der 12 Duplikate

Für jede doppelte Komponente:
1. Beide Versionen vollständig lesen
2. Feature-Matrix: was hat die eine, was die andere
3. Ultimative Version in `raijin-ui` schreiben die ALLES kombiniert
4. `cargo check` nach jeder Komponente

### Phase 2: Einzigartige Komponenten verschieben

16+ Komponenten von `inazuma-component` nach `raijin-ui`:
1. Dateien verschieben
2. In `raijin-ui/src/components.rs` registrieren
3. Imports aller Consumer anpassen

### Phase 3: Input System verschieben

Das 10k-Zeilen Input System nach `raijin-ui/src/input/`:
1. Alle Dateien verschieben
2. Interne Imports anpassen
3. Consumer (`terminal_pane.rs` etc.) auf `raijin_ui::input::Input` umstellen

### Phase 4: Highlighter + Rest verschieben

Highlighter, TitleBar, Dock, Component Registry nach `raijin-ui` oder in die richtigen Crates.

### Phase 5: inazuma-component entfernen

1. Alle Consumer umgestellt?
2. `cargo check` — kein Crate importiert mehr `inazuma-component`?
3. Crate aus Workspace entfernen
4. Verzeichnis löschen

### Phase 6: Validierung

```bash
# inazuma-component existiert nicht mehr:
ls crates/inazuma-component  # → Fehler

# Kein Code referenziert es mehr:
grep -r "inazuma.component\|inazuma_component" crates/*/Cargo.toml crates/*/src/
# → 0 Treffer

# Alles kompiliert:
cargo check
```
