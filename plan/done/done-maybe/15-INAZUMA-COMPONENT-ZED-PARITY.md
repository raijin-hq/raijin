# Phase: Inazuma-Component Zed Parity — Port Missing Zed UI Components

## Context

Inazuma-component (geforkt von gpui-component) hat ~240 Dateien und ist breiter als Zed's `ui` Crate (~87 Dateien). Aber Zed hat Komponenten und Infrastruktur die uns fehlen. Dieses Plan portiert alles was Zed hat und wir nicht.

**Quelle:** `.reference/zed/crates/ui/src/`
**Ziel:** `crates/inazuma-component/ui/src/`

---

## Tier 1: Modal System (KRITISCH — wird jetzt in Phase 4 gebaut)

| Zed | Inazuma-Component | Status |
|-----|-------------------|--------|
| `ModalView` Trait | Fehlt | 🔴 Portieren |
| `ManagedView` (auto-impl) | Fehlt | 🔴 Portieren |
| `DismissEvent` / `DismissDecision` | Fehlt | 🔴 Portieren |
| `ModalLayer` Entity (toggle, hide, focus mgmt) | Fehlt | 🔴 Portieren |
| `Modal` UI Component (Header/Footer/Sections) | Fehlt (Dialog existiert, aber anderes Pattern) | 🔴 Portieren |
| `AlertModal` (einfaches Confirmation-Modal) | `AlertDialog` existiert, aber andere API | 🟡 Evaluieren |

---

## Tier 2: Core Trait System

| Zed Trait | Beschreibung | Inazuma-Component | Status |
|-----------|-------------|-------------------|--------|
| `Clickable` | Standardisiertes Click-Handling | Teilweise in `Selectable` | 🟡 Prüfen |
| `Disableable` | Enable/Disable State | Teilweise in einzelnen Components | 🟡 Prüfen |
| `Toggleable` / `ToggleState` | Toggle/Radio State | Fehlt als generisches Trait | 🔴 Portieren |
| `AnimationExt` | Animation Behavior | Eigene Implementierung vorhanden | 🟢 Vorhanden |
| `FixedPositioning` | Fixed Layout | Fehlt | 🔴 Portieren |
| `VisibleOnHover` | Visibility Trait | Fehlt | 🔴 Portieren |
| `Navigable` | Keyboard Navigation | Fehlt | 🔴 Portieren |

---

## Tier 3: Style System

| Zed | Beschreibung | Inazuma-Component | Status |
|-----|-------------|-------------------|--------|
| `Elevation` | Shadow/Z-Index Levels (1-3) | Eigene Shadow-Utilities | 🟡 Prüfen |
| `Severity` | Error/Warning/Info/Success | Notification hat Severity | 🟡 Erweitern |
| `Platform` | macOS/Linux/Windows Styles | Fehlt als System | 🔴 Portieren |
| `Units` | px/rem/vh/vw Konstanten | Teilweise | 🟡 Prüfen |
| `Typography` | Font System | Eigene Implementierung | 🟢 Vorhanden |
| `Spacing` | Gap/Padding Definitionen | Via Tailwind-Style | 🟢 Vorhanden |
| `Appearance` | Visual Styles | Theme-System vorhanden | 🟢 Vorhanden |

---

## Tier 4: Missing UI Components

### Button Varianten
| Zed | Beschreibung | Status |
|-----|-------------|--------|
| `button_like.rs` | Button-styled Non-Button | 🔴 Portieren |
| `button_link.rs` | Link als Button | 🔴 Portieren |
| `copy_button.rs` | Copy-to-Clipboard Button | 🔴 Portieren |
| `split_button.rs` | Button mit Dropdown | 🔴 Portieren |
| `toggle_button.rs` | Radio-style Toggle | 🔴 Portieren |

### Label Varianten
| Zed | Beschreibung | Status |
|-----|-------------|--------|
| `highlighted_label.rs` | Text mit Highlighting | 🔴 Portieren |
| `label_like.rs` | Label-styled Text | 🔴 Portieren |
| `loading_label.rs` | Label mit Loading-Indicator | 🔴 Portieren |
| `spinner_label.rs` | Label mit Spinner | 🔴 Portieren |

### Visual Components
| Zed | Beschreibung | Priorität für Raijin | Status |
|-----|-------------|---------------------|--------|
| `disclosure.rs` | Expand/Collapse Control | Hoch (File Browser, Settings) | 🔴 Portieren |
| `tree_view_item.rs` | Tree mit Disclosure | Hoch (File Browser) | 🔴 Portieren |
| `keybinding.rs` | Keyboard-Shortcut Anzeige | Hoch (Terminal UX) | 🔴 Portieren |
| `keybinding_hint.rs` | Help-Text für Shortcuts | Hoch (Terminal UX) | 🔴 Portieren |
| `indent_guides.rs` | Editor Indent-Lines | Mittel (Code Editor) | 🔴 Portieren |
| `gradient_fade.rs` | Fade-Effekt | Niedrig | 🔴 Portieren |
| `facepile.rs` | Stacked Avatars | Niedrig (Collab) | 🔴 Portieren |
| `sticky_items.rs` | Sticky Positioning | Mittel | 🔴 Portieren |

### Badge/Indicator
| Zed | Beschreibung | Status |
|-----|-------------|--------|
| `count_badge.rs` | Zahlen-Badge | 🔴 Portieren |
| `diff_stat.rs` | Diff-Statistiken | 🔴 Portieren |
| `indicator.rs` | Status-Dot | 🔴 Portieren |

### List Varianten
| Zed | Beschreibung | Status |
|-----|-------------|--------|
| `list_bullet_item.rs` | Aufzählungs-Items | 🔴 Portieren |
| `list_header.rs` | Section Header | 🔴 Portieren |
| `list_sub_header.rs` | Subsection Header | 🔴 Portieren |
| `list_separator.rs` | Visueller Separator | 🔴 Portieren |

### Menu
| Zed | Beschreibung | Status |
|-----|-------------|--------|
| `right_click_menu.rs` | Kontext-spezifische Menüs | 🔴 Portieren |

---

## Tier 5: Accessibility & Utilities

| Zed | Beschreibung | Status |
|-----|-------------|--------|
| `apca_contrast.rs` | APCA Kontrast-Algorithmus | 🔴 Portieren |
| `color_contrast.rs` | Farbkontrast-Prüfung | 🔴 Portieren |
| `format_distance.rs` | Relative Zeit-Formatierung | 🔴 Portieren |
| `corner_solver.rs` | Corner-Radius Berechnung | 🔴 Portieren |
| `search_input.rs` | Such-Input mit Icons | 🔴 Portieren |

---

## Nicht Portieren (Zed-spezifisch)

| Zed | Grund |
|-----|-------|
| `ai.rs` + AI Components | Zed-spezifisches AI-Feature |
| `collab.rs` + Collab Components | Zed-spezifisches Collaboration |
| `announcement_toast.rs` | Zed-Marketing |
| `stories.rs` | Zed-internes Preview-System |

---

## Implementierungs-Reihenfolge

1. **Jetzt (Phase 4):** Modal System (ModalView, ModalLayer, Modal UI)
2. **Phase 5:** Core Traits (Toggleable, Navigable, VisibleOnHover)
3. **Phase 6:** High-Value Components (Disclosure, TreeView, Keybinding, Labels)
4. **Phase 7:** Style System (Elevation, Severity, Platform)
5. **Phase 8:** Rest (Badges, Indicators, List Varianten, Accessibility)

---

## Zählung

| Kategorie | Anzahl |
|-----------|--------|
| **Kritisch (Modal System)** | 6 Items |
| **Core Traits** | 5 Items |
| **Style System** | 3 Items |
| **UI Components** | 22 Items |
| **Accessibility/Utils** | 5 Items |
| **Total zu portieren** | ~41 Items |
| **Nicht portieren (Zed-spezifisch)** | 7 Items |
