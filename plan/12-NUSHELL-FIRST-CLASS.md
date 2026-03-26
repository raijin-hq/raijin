# Plan 12: Nushell als First-Class Shell

> **Status:** ⬜ Planned
> **Erstellt:** 26. März 2026
> **Abhängigkeiten:** Phase 2A (Shell Integration) ✅, Phase 2B (Block Rendering) ✅
> **Referenz:** Ghostty (MIT, github.com/ghostty-org/ghostty), Nushell 0.111.0

---

## Motivation

Warp hat nach 3+ Jahren Nushell immer noch nicht implementiert (warpdotdev/Warp#2038). Ghostty hat als erster Terminal eine Nu-Integration shipped — aber nur sudo/ssh-Wrapper, **keine Block-UX**. Raijin kann der erste Warp-style Block-Terminal mit nativem Nu-Support sein.

Die Nu-Community besteht aus Power-Usern, die aktiv nach besseren Terminals suchen. Viele wechseln bereits von Warp zu Ghostty nur wegen Nu-Support.

---

## Architektur-Analyse: Warum Nu einfacher ist als gedacht

### OSC 133 funktioniert nativ

**Nushell emittiert OSC 133 Marker von sich aus** — seit reedline#1019 (Feb 2026). Anders als bei zsh/bash/fish brauchen wir **keine Hook-Injection** via ZDOTDIR oder --rcfile:

| Marker | Quelle | Status in Nu |
|--------|--------|--------------|
| `133;A` (PromptStart) | reedline nativ | ✅ Funktioniert |
| `133;B` (InputStart) | reedline nativ | ✅ Funktioniert |
| `133;C` (CommandStart) | Nushell engine | ✅ Funktioniert |
| `133;D;N` (CommandEnd) | Nushell engine | ✅ Funktioniert |
| `133;P;k=X` (PromptKind) | reedline nativ | ✅ Neu (i/c/s/r) |

Das bedeutet: **Raijins `OscScanner` + `BlockManager` funktionieren mit Nu ohne jede Änderung.** Blocks, Exit-Codes, Prompt-Detection — alles out-of-the-box.

### Was NICHT nativ funktioniert

| Feature | Problem | Lösung |
|---------|---------|--------|
| OSC 7777 Metadata | Nu hat keine precmd-Hook wie zsh | `raijin.nu` Hook-Script mit `pre_prompt` Hook |
| Prompt-Suppression | Nu's Prompt ist anders aufgebaut | Funktioniert trotzdem — OSC 133 A→C Region wird hidden |
| Command-Text im Block-Header | `pending_command` kommt vom Input-Bar | Funktioniert, da Shell-agnostisch |
| Shell-Duration | Braucht precmd/preexec Timing | `pre_prompt` + `pre_execution` Hooks in `raijin.nu` |

---

## Phase 1: Basic Nu Support (Low Effort, High Impact)

### 1.1 Shell-Detection in `pty.rs`

`spawn_pty()` erkennt Nu anhand des Shell-Pfads und wählt die richtige Injection-Strategie:

```rust
// In inject_shell_hooks():
"nu" => {
    // Nu emittiert OSC 133 nativ — keine ZDOTDIR-Tricks nötig.
    // Wir nutzen XDG_DATA_DIRS für Vendor-Autoload (wie Ghostty)
    // plus --execute für den initialen Import.
    let hooks_dir = hooks_dir.join("nushell");
    if hooks_dir.join("raijin.nu").exists() {
        // Prepend to XDG_DATA_DIRS for vendor autoload
        let xdg = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
        let raijin_xdg = format!("{}:{}", hooks_dir.parent().unwrap().display(), xdg);
        cmd.env("XDG_DATA_DIRS", &raijin_xdg);
        cmd.args(["-e", "use raijin *"]);
    }
}
```

### 1.2 Shell-Hook: `shell/nushell/vendor/autoload/raijin.nu`

Minimaler Hook — **kein OSC 133** (das macht Nu nativ), nur Raijin-spezifisches:

```nu
# Raijin Terminal — Nushell Integration
# OSC 133 (block boundaries) is handled natively by Nushell/reedline.
# This script adds Raijin-specific features on top.

# --- Feature Gating ---
let features = ($env.RAIJIN_SHELL_FEATURES? | default "metadata,sudo" | split row ",")

# --- OSC 7777 Metadata (CWD, git branch, user) ---
if "metadata" in $features {
    $env.config.hooks.pre_prompt = (
        $env.config.hooks.pre_prompt | default [] | append {||
            # Collect shell context as JSON, hex-encode, send via OSC 7777
            let meta = {
                cwd: ($env.PWD),
                username: (whoami | str trim),
                shell: "nu",
                shell_version: (version | get version),
                last_duration_ms: ($env.CMD_DURATION_MS? | default 0),
            }
            let hex = ($meta | to json -r | encode hex)
            print -n $"\e]7777;raijin-precmd;($hex)\a"
        }
    )
}

# --- TERMINFO-aware sudo (borrowed from Ghostty's approach) ---
if "sudo" in $features {
    def --env raijin-sudo [...args: string] {
        if ("-e" in $args) or ("--edit" in $args) {
            ^sudo ...$args
        } else {
            ^sudo $"TERMINFO=($env.TERMINFO? | default '')" ...$args
        }
    }
    alias sudo = raijin-sudo
}
```

### 1.3 OscScanner: OSC 133 P (PromptKind) erweitern

Nu sendet zusätzlich `133;P;k=i` (initial), `k=c` (continuation), `k=s` (secondary), `k=r` (right). Unterstützung hinzufügen:

```rust
// Neuer ShellMarker-Variant:
PromptKind { kind: PromptKindType },

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKindType {
    Initial,       // k=i — Primary prompt
    Continuation,  // k=c — Multiline continuation
    Secondary,     // k=s — Secondary prompt
    Right,         // k=r — Right-aligned prompt
}
```

Das ermöglicht Raijin, Multiline-Eingaben korrekt als einen Block zu erkennen.

### 1.4 Feature-Gating via Environment

Wie Ghostty: `RAIJIN_SHELL_FEATURES` als komma-separierte Feature-Flags:

| Feature | Default | Beschreibung |
|---------|---------|--------------|
| `metadata` | ✅ on | OSC 7777 Precmd-Metadata senden |
| `sudo` | ✅ on | TERMINFO-aware sudo Wrapper |
| `structured` | ❌ off | Structured Output Rendering (Phase 2) |

---

## Phase 2: Structured Output Rendering (Mittelfristig)

### Das Problem

Wenn Nu `ls` ausführt, passiert intern:
1. `ls` gibt `PipelineData::ListStream` zurück (typisierte `Value::Record` pro Zeile)
2. Der `table`-Befehl rendert das als ANSI-formatierten Text
3. Der Text geht über stdout an das Terminal
4. **Die Typinformation ist verloren**

### Die Lösung: `display_output` Hook

Nu hat einen `display_output` Hook — er kontrolliert, wie Pipeline-Output gerendert wird. Statt der normalen `table`-Darstellung können wir:

1. **Structured Data via OSC senden** + normales Table-Rendering als Fallback
2. Raijin erkennt den OSC-Marker und rendert eine **interaktive Tabelle** im Block

```nu
# In raijin.nu (nur wenn "structured" Feature aktiv):
$env.config.hooks.display_output = {||
    let input = $in
    # Prüfe ob der Output tabellenförmig ist (List of Records)
    if ($input | describe | str starts-with "table") {
        # Sende structured data als OSC 7778 (Raijin-spezifisch)
        let json = ($input | to json -r)
        let hex = ($json | encode hex)
        print -n $"\e]7778;raijin-table;($hex)\a"
        # AUCH normal rendern (Fallback für nicht-Raijin Terminals)
        $input | table
    } else {
        # Normaler Output
        $input | table
    }
}
```

### Raijin-Side: Table Block Renderer

Neuer Block-Typ in `block.rs`:

```rust
pub enum BlockContent {
    /// Normal command output (rendered from terminal grid)
    Text,
    /// Structured table data from Nu (rendered as interactive table)
    StructuredTable {
        columns: Vec<String>,
        rows: Vec<Vec<serde_json::Value>>,
    },
}
```

Features der interaktiven Tabelle:
- **Spalten-Header klickbar** → Sortierung toggle (asc/desc)
- **Filter-Icon** → Inline-Filter pro Spalte
- **GPU-gerendert** via Inazuma — keine TUI-Library nötig
- **Copy** → Einzelne Zellen, Zeilen, oder gesamte Tabelle
- **Resize** → Spaltenbreiten per Drag anpassen

### OSC 7778 Parser

Neues OSC in `osc_parser.rs`:

```rust
// Neuer ShellMarker:
StructuredOutput { format: String, data: String },

// Parse:
fn parse_osc_7778(&self) -> Option<ShellMarker> {
    let prefix = b"7778;raijin-table;";
    // ... hex decode → JSON string
}
```

---

## Phase 3: Deep Integration (Langfristig)

### 3.1 Nu Plugin: `nu_plugin_raijin`

Ein Rust-Binary das als Nu-Plugin läuft und über Unix-Socket mit dem Terminal kommuniziert:

```
Nu Pipeline → nu_plugin_raijin → Unix Socket → Raijin Terminal
```

**Vorteile gegenüber display_output Hook:**
- Zugriff auf volle `Value`-Typen (nicht nur JSON-Serialisierung)
- Kann `CustomValue` implementieren (z.B. für lazy-loaded große Datasets)
- Bidirektionale Kommunikation (Terminal kann Daten vom Plugin anfordern)
- Kein Overhead bei nicht-tabellenförmigem Output

**Neues Crate:** `crates/raijin-nuplugin/`

### 3.2 Shell-Switcher im Tab-Header

```
┌─ Tab 1 (zsh) ─┬─ Tab 2 (nu 0.111) ─┬─ + ─┐
```

- Shell-Auswahl per Dropdown beim Tab erstellen
- Shell-Version im Tab-Header (wichtig für pre-1.0 Nu)
- Kein Terminal-Neustart bei Shell-Wechsel (neues Tab mit anderer Shell)

### 3.3 Nu-spezifische Block-Features

| Feature | Beschreibung |
|---------|-------------|
| **Table Mode** | Interaktive Tabelle statt Text für tabellenförmigen Output |
| **Error Highlighting** | Nu-Errors haben strukturierte Spans — präzises Underlining |
| **Pipeline Visualization** | Zeige den Pipeline-Flow visuell (Input → Filter → Output) |
| **Type Badges** | Zeige den Output-Typ im Block-Header (table, list, record, string) |
| **Explore Inline** | Nu's `explore` TUI direkt im Block einbetten |

---

## Dateistruktur (Ziel)

```
shell/
├── raijin.zsh                              # Bestehend
├── raijin.bash                             # Bestehend
├── raijin.fish                             # Bestehend
└── nushell/
    └── vendor/
        └── autoload/
            └── raijin.nu                   # Neu (Phase 1)

crates/
├── raijin-terminal/src/
│   ├── pty.rs                              # Erweitert: Nu-Detection + XDG injection
│   ├── osc_parser.rs                       # Erweitert: OSC 133;P + OSC 7778
│   └── block.rs                            # Erweitert: BlockContent::StructuredTable
└── raijin-nuplugin/                        # Neu (Phase 3)
    ├── Cargo.toml
    └── src/
        └── main.rs                         # nu_plugin_raijin Binary
```

---

## Implementierungs-Reihenfolge

| Schritt | Aufwand | Impact | Beschreibung |
|---------|---------|--------|--------------|
| 1. Nu-Detection in `pty.rs` | S | Hoch | Shell erkennen, richtige Injection wählen |
| 2. `raijin.nu` Hook-Script | S | Hoch | Metadata, sudo Wrapper |
| 3. Feature-Gating Env-Var | XS | Mittel | `RAIJIN_SHELL_FEATURES` |
| 4. OSC 133;P Parser | S | Mittel | Multiline-Erkennung |
| 5. Testen: Blocks mit Nu | M | Hoch | Validierung dass alles out-of-the-box funktioniert |
| 6. `display_output` Hook | M | Sehr Hoch | Structured Data via OSC 7778 |
| 7. Table Block Renderer | L | Sehr Hoch | GPU-gerenderte interaktive Tabellen |
| 8. `nu_plugin_raijin` | XL | Hoch | Deep Integration, bidirektional |
| 9. Shell-Switcher UI | M | Mittel | Tab-Header Dropdown |

**S** = Stunden, **M** = 1-2 Tage, **L** = 3-5 Tage, **XL** = 1-2 Wochen

---

## Ghostty-Referenz (was wir abgucken)

| Konzept | Ghostty | Raijin-Adaption |
|---------|---------|-----------------|
| XDG_DATA_DIRS Injection | `src/shell-integration/nushell/` | Gleicher Ansatz für `raijin.nu` |
| Feature-Gating | `GHOSTTY_SHELL_FEATURES` env var | `RAIJIN_SHELL_FEATURES` env var |
| sudo Wrapper | TERMINFO preservation | Gleich, in `raijin.nu` |
| OSC 133 Parsing | `semantic_prompt.zig`, row-level tracking | `osc_parser.rs`, BlockManager |
| **NICHT übernehmen** | Margin-Bars (farbige Streifen) | Wir haben Block-Headers — besser |
| **NICHT übernehmen** | Kein Structured Output | Wir machen das als Differentiator |

**Lizenz-Hinweis:** Ghosttys Nu-Integration ist MIT. Die bash/zsh-Scripts sind GPLv3 (Kitty-derived) — nicht kopieren! Unsere bestehenden zsh/bash/fish-Hooks sind eigenständig.

---

## Risiken

| Risiko | Mitigation |
|--------|-----------|
| Nu Breaking Changes (pre-1.0) | Feature-Gating, Pin auf getestete Nu-Versionen, CI mit nightly Nu |
| OSC 7778 Overhead bei großen Tabellen | Size-Limit (z.B. max 1000 Rows), danach Fallback auf Text |
| Plugin-Protokoll Instabilität | `nu_plugin_raijin` erst in Phase 3, wenn Plugin-Protocol-Decoupling (Nu#14126) gelöst |
| Display-Output Hook Konflikte | User-Config respektieren, nur aktivieren wenn explizit gewünscht |

---

*Plan 12 — Raijin (雷神) × Nushell*
*fish für Komfort. Nu für Power. Raijin rendert beides besser als jeder andere.*
