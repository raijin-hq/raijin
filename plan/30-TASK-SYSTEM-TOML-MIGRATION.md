# Phase 30: Task System — JSON → TOML Migration + VS Code Import Removal

> **STATUS: Eigenständiges Projekt, verifiziert April 2026.** Alle Behauptungen unten gegen den Code geprüft:
> - `crates/raijin-task/src/vscode_format.rs` und `vscode_debug_format.rs` existieren ✓
> - `crates/raijin-task/test_data/` enthält alle drei genannten JSON-Dateien ✓
> - `raijin_paths.rs` hat tatsächlich alle genannten Funktionen mit `tasks.json`/`debug.json`-Pfaden ✓
>
> **Verhältnis zu Plan 25:** Unabhängig. Nicht Teil der Crate-by-Crate Wiring. Kann parallel oder vor/nach Phase 25 laufen. Berührt aber `raijin-task` und `raijin-tasks-ui`, die Stationen in Plan 25 sind — wenn diese Stationen verdrahtet werden bevor Plan 30 läuft, müssen sie später nochmal angefasst werden für die Migration.
>
> **Empfehlung:** Plan 30 vor der raijin-task-Verdrahtung in Plan 25 ausführen, damit die Verdrahtung gleich auf TOML-Basis passiert.

---

## Context

Raijin nutzt TOML für alle Settings (`config.toml`, `default.toml`, `initial_user_settings.toml`). Das Task-System ist noch ein Überbleibsel aus der Referenz-Codebase das JSON nutzt (`tasks.json`, `debug.json`). Zusätzlich gibt es VS Code Import-Code (`vscode_format.rs`, `vscode_debug_format.rs`) der `.vscode/tasks.json` und `.vscode/launch.json` aus fremden Projekten importiert — ein Editor-Feature das Raijin als Terminal-Emulator nicht braucht.

**Ziel:** Alles auf TOML vereinheitlichen, VS Code Import komplett entfernen.

## Was bleibt JSON

- `default_semantic_token_rules.json` — LSP-Standard, wird mit `parse_json_with_comments` geparsed ✅ (bereits kopiert)
- VS Code test data in `raijin-task/test_data/` — wird mit dem VS Code Modul gelöscht

## Phase 1: VS Code Import entfernen

### 1.1 Module löschen
- `crates/raijin-task/src/vscode_format.rs` — komplett löschen
- `crates/raijin-task/src/vscode_debug_format.rs` — komplett löschen
- `crates/raijin-task/test_data/typescript.json` — löschen
- `crates/raijin-task/test_data/rust-analyzer.json` — löschen
- `crates/raijin-task/test_data/tasks-without-labels.json` — löschen

### 1.2 Exports entfernen
- `crates/raijin-task/src/task.rs`:
  - Zeile 31: `pub use vscode_debug_format::VsCodeDebugTaskFile;` — entfernen
  - Zeile 32: `pub use vscode_format::VsCodeTaskFile;` — entfernen
  - Entsprechende `mod` declarations entfernen

### 1.3 VS Code Pfade entfernen
- `crates/raijin-paths/src/raijin_paths.rs`:
  - `local_vscode_tasks_file_relative_path()` — entfernen
  - `local_vscode_launch_file_relative_path()` — entfernen
  - `local_vscode_folder_name()` — entfernen

### 1.4 VS Code Import-Logik in project_settings entfernen
- `crates/raijin-project/src/project_settings.rs`:
  - Imports von `VsCodeTaskFile`, `VsCodeDebugTaskFile`, `local_vscode_tasks_file_relative_path` entfernen
  - Code-Pfade die `.vscode/tasks.json` lesen und konvertieren — entfernen
  - Code-Pfade die `.vscode/launch.json` lesen und konvertieren — entfernen

### 1.5 TrackedFile::new_convertible entfernen
- `crates/raijin-task/src/static_source.rs`:
  - `new_convertible()` Methode — entfernen (war nur für VS Code JSON → Raijin Konvertierung)

## Phase 2: Task-Dateien JSON → TOML

### 2.1 Konstanten und Pfade
- `crates/raijin-task/src/task_template.rs`:
  - `TaskTemplates::FILE_NAME` von `"tasks.json"` → `"tasks.toml"`
- `crates/raijin-project/src/task_inventory.rs`:
  - `InventoryContents for TaskTemplate::GLOBAL_SOURCE_FILE` von `"tasks.json"` → `"tasks.toml"`
- `crates/raijin-paths/src/raijin_paths.rs`:
  - `tasks_file()`: `config_dir().join("tasks.json")` → `config_dir().join("tasks.toml")`
  - `local_tasks_file_relative_path()`: `.raijin/tasks.json` → `.raijin/tasks.toml`
  - `task_file_name()`: `"tasks.json"` → `"tasks.toml"`

### 2.2 Parser umstellen
- `crates/raijin-task/src/static_source.rs`:
  - `TrackedFile::new()`: `serde_json_lenient::from_str::<T>` → `toml::from_str::<T>`
  - `toml` Dependency zu `raijin-task/Cargo.toml` hinzufügen
  - `serde_json_lenient` Dependency entfernen (prüfen ob noch anderswo gebraucht)

### 2.3 project_settings Parser
- `crates/raijin-project/src/project_settings.rs`:
  - Raijin-eigene `.raijin/tasks.toml` mit `toml::from_str` parsen statt `parse_json_with_comments`

## Phase 3: Asset-Dateien erstellen

### 3.1 Fehlende Settings-Assets als TOML
- `assets/settings/initial_server_settings.toml` — Template für Server-Settings
- `assets/settings/initial_local_settings.toml` — Template für Projekt-Settings

### 3.2 Task-Templates als TOML
- `assets/settings/initial_tasks.toml` — Raijin Task-Template
- `assets/settings/initial_debug_tasks.toml` — Debug Task-Template
- `assets/settings/initial_local_debug_tasks.toml` — Lokale Debug Task-Template

### 3.3 Asset-Referenzen aktualisieren
- `crates/inazuma-settings-framework/src/settings.rs`:
  - `initial_server_settings_content()`: `"settings/initial_server_settings.json"` → `"settings/initial_server_settings.toml"`
  - `initial_project_settings_content()`: `"settings/initial_local_settings.json"` → `"settings/initial_local_settings.toml"`
  - `initial_tasks_content()`: `"settings/initial_tasks.json"` → `"settings/initial_tasks.toml"`
  - `initial_debug_tasks_content()`: `"settings/initial_debug_tasks.json"` → `"settings/initial_debug_tasks.toml"`
  - `initial_local_debug_tasks_content()`: `"settings/initial_local_debug_tasks.json"` → `"settings/initial_local_debug_tasks.toml"`

## Phase 4: Debug-Tasks JSON → TOML

### 4.1 Debug-Pfade
- `crates/raijin-paths/src/raijin_paths.rs`:
  - `debug_scenarios_file()`: `config_dir().join("debug.json")` → `config_dir().join("debug.toml")`
  - `local_debug_file_relative_path()`: `.raijin/debug.json` → `.raijin/debug.toml`
  - `debug_task_file_name()`: `"debug.json"` → `"debug.toml"`

### 4.2 Debug-Format Parser
- Debug-Task Parsing ebenfalls von JSON auf TOML umstellen (gleiche Pattern wie Phase 2)

### 4.3 Debugger UI
- `crates/raijin-debugger-ui/src/debugger_panel.rs`:
  - `initial_local_debug_tasks_content()` — schreibt Template in neue Datei, Pfad wird automatisch korrekt wenn Phase 3.3 gemacht ist

## Phase 5: Tests aktualisieren

- `crates/raijin-project/tests/integration/project_tests.rs`:
  - Alle `".zed/tasks.json"` Referenzen → `".raijin/tasks.toml"`
  - JSON inline-Strings → TOML inline-Strings
  - `"settings.json"` Referenzen → prüfen ob bereits TOML
- `crates/raijin-project/tests/integration/task_inventory.rs`:
  - `"global tasks.json"` String-Referenzen → `"global tasks.toml"`
- `crates/raijin-tasks-ui/src/tasks_ui.rs` und `modal.rs`:
  - Inline JSON task definitions → TOML
- `crates/raijin-agent-ui/src/message_editor.rs`:
  - `.zed/tasks.json` → `.raijin/tasks.toml`
- `crates/raijin-agent/src/tools/edit_file_tool.rs` und `streaming_edit_file_tool.rs`:
  - `.zed/tasks.json` Pfad-Referenzen aktualisieren

## Phase 6: Cleanup

- `serde_json_lenient` Dependency aus `raijin-task/Cargo.toml` entfernen (falls nicht mehr gebraucht)
- `raijin-task/test_data/` Ordner entfernen falls leer
- Grep nach verbleibenden `tasks.json` / `debug.json` Referenzen im gesamten Repo
- `cargo clippy --workspace` — sicherstellen keine Warnings

## TOML Format-Beispiele

### tasks.toml
```toml
[[tasks]]
label = "Example task"
command = "for i in {1..5}; do echo \"Hello $i/5\"; sleep 1; done"
use_new_terminal = false
allow_concurrent_runs = false
reveal = "always"
reveal_target = "dock"
hide = "never"
shell = "system"
show_summary = true
show_command = true
save = "all"

[tasks.env]
foo = "bar"
```

### debug.toml
```toml
[[tasks]]
label = "Debug active Python file"
adapter = "Debugpy"
program = "$RAIJIN_FILE"
request = "launch"
cwd = "$RAIJIN_WORKTREE_ROOT"
```

## Verifikation

```bash
cargo build                    # Kompiliert ohne Fehler
cargo raijin dev               # Startet ohne Panic
cargo test --workspace         # Alle Tests grün
cargo clippy --workspace       # Keine Warnings
grep -r "tasks\.json" crates/  # Keine Reste (außer Kommentare)
grep -r "vscode_format" crates/ # Komplett weg
```
