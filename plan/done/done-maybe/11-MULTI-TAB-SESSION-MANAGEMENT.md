# Multi-Tab & Session Management

## Ziel

Warp-style Multi-Tab System: mehrere Terminal-Sessions in Tabs, Tabs öffnen/schließen, Keyboard-Shortcuts, Settings als temporärer Tab, Launch Configurations.

## Features

### Tab Management
- **Mehrere Terminal-Tabs** — jeder Tab hat eigene Terminal-Session (eigenes PTY, eigenen CWD)
- **Neuer Tab (+)** — Button rechts neben Tabs, öffnet neuen Terminal-Tab
- **Tab schließen (Cmd+W)** — schließt aktiven Tab, killt PTY
- **Tab X-Button on hover** — Tabs zeigen ein X-Icon beim Hovern mit der Maus (wie Warp)
- **Tab wechseln (Cmd+Shift+Tab / Cmd+1-9)** — zwischen Tabs wechseln
- **Tab-Reihenfolge** — Drag & Drop zum Umsortieren (später)
- **Feste Tab-Breite** — wie Warp, alle Tabs gleich breit, Text overflow hidden

### Neuer Tab CWD
- **Einstellbar** in Settings: "Current Session" oder "Home"
- Default: CWD vom aktuell aktiven Tab übernehmen
- Konfigurierbar via `config.toml`:
  ```toml
  [general]
  new_tab_directory = "current_session"  # "current_session" | "home" | "/custom/path"
  ```

### Settings Tab
- **Temporär** — Settings-Tab erscheint nur wenn geöffnet (Cmd+,), verschwindet wenn geschlossen
- **Kein Terminal** — Settings-Tab hat kein PTY, nur die Settings-View
- **Schließbar** wie jeder andere Tab (Cmd+W oder X-Button)

### Plus-Button (+) mit Dropdown
- Wie Warp: `+` Button + Dropdown-Pfeil daneben
- Dropdown-Menü:
  - New Terminal Tab (Shift+Cmd+T)
  - Restore Closed Tab (Shift+Cmd+T)
  - Separator
  - Zsh / Bash / Fish (verfügbare Shells)
  - Separator
  - Launch Configurations...

### Launch Configurations (später)
- YAML-Dateien in `~/.config/raijin/launch/`
- Wie Warp: https://docs.warp.dev/terminal/sessions/launch-configurations
- Pro Config: Name, CWD, Shell, Commands to run, Layout

## Architektur

### Workspace Refactor
- `Workspace` wird zum Tab-Manager
- Jeder Tab ist ein `TabSession`:
  ```rust
  enum TabContent {
      Terminal(TerminalSession),
      Settings,
  }

  struct TabSession {
      id: TabId,
      title: String,
      content: TabContent,
  }

  struct TerminalSession {
      terminal: Terminal,
      block_manager: BlockManager,
      shell_context: ShellContext,
      input_state: Entity<InputState>,
      show_terminal: bool,
      interactive_mode: bool,
  }
  ```
- `Workspace` hat `Vec<TabSession>` + `active_tab_index`

### Keyboard Shortcuts
| Shortcut | Action |
|----------|--------|
| Cmd+T | Neuer Terminal-Tab |
| Cmd+W | Aktiven Tab schließen |
| Cmd+, | Settings-Tab öffnen |
| Cmd+1-9 | Zu Tab 1-9 wechseln |
| Cmd+Shift+] | Nächster Tab |
| Cmd+Shift+[ | Vorheriger Tab |

### CWD Detection für neue Tabs
- `proc_pidinfo` (macOS) um CWD des Shell-Foreground-Prozesses zu lesen
- Wie Alacritty: `crates/raijin-terminal/src/macos_proc.rs`

## Referenz
- [Warp Launch Configurations](https://docs.warp.dev/terminal/sessions/launch-configurations)
- [Warp Tabs Behavior](https://docs.warp.dev/terminal/appearance/tabs-behavior)
