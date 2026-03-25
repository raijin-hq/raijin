# Phase 3: Design System — Von "funktional" zu "Warp-Level" (Woche 6–9)

> **Ziel:** Das Ding muss GEIL aussehen

---

## 3.1 — Farb-System & Theming

- [ ] Design-Token-System definieren (TOML):

```toml
[colors]
bg_primary = "#1a1b26"
bg_secondary = "#1e1f2b"
bg_tertiary = "#252736"
bg_block = "#1c1d28"
bg_block_hover = "#22233a"
accent = "#7dcfff"
accent_secondary = "#bb9af7"
text_primary = "#c0caf5"
text_secondary = "#565f89"
text_tertiary = "#3b4261"
border = "#292e42"
success = "#9ece6a"
warning = "#e0af68"
error = "#f7768e"
```

- [ ] Theme-Loader der TOML-Files liest und in GPUI-Styles übersetzt
- [ ] Theme Library: Min. 5 Themes (Dark, Light, Dracula, Nord, Gruvbox)
- [ ] GUI Theme Builder: Accent Color + Background Image → Palette generieren
- [ ] Accent-Color konfigurierbar
- [ ] Tab-Farben pro Tab konfigurierbar (6 Farben wie Warp)
- [ ] Transparenter Background mit Opacity-Slider

---

## 3.2 — Typographie-Hierarchie

- [ ] Font-Stack: Terminal (monospace) + UI (proportional)
- [ ] Größen-Skala: 11px → 12px → 13px → 14px → 16px
- [ ] Font konfigurierbar in Settings (Type + Size)

---

## 3.3 — Visual Layering

- [ ] 3+ Hintergrund-Ebenen mit subtilen Borders
- [ ] Komponenten-Design: Tabs, Sidebar, Blocks, Scrollbar

---

## 3.4 — Animationen

- [ ] Hover: 150ms ease-out
- [ ] Tab-Switch: Accent slide
- [ ] Block-Expand/Collapse
- [ ] Cursor-Blink: Smooth opacity

---

## Milestone

✅ App sieht visuell auf Warp-Niveau aus
✅ Mindestens 5 Themes verfügbar
✅ Animationen fühlen sich smooth und polished an
