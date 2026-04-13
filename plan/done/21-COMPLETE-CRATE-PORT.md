# Phase 21: Vollständige Portierung aller fehlenden Referenz-Crates

## Ziel

Alle 134 fehlenden Referenz-Crates nach Raijin portieren. Jeder Crate wird kopiert, umbenannt und angepasst — exakt wie bei den ~90 Crates die wir bereits portiert haben.

## Quelle

Alle Crates kommen aus: `/Users/nyxb/Projects/raijin/.reference/zed/crates/`

## Naming-Regeln

### Grundregel: inazuma vs raijin

**Inazuma (稲妻)** = Das UI-Framework. Alles was in der Referenz `gpui` heißt oder ein reiner Framework-Baustein ist (collections, text, rope, fuzzy, clock, menu, picker, settings, etc.) wird `inazuma-*`. Das sind generische, wiederverwendbare Komponenten die nichts mit der Raijin-App selbst zu tun haben.

**Raijin (雷神)** = Die App und alles App-spezifische. Alles was in der Referenz `zed` heißt oder ein Feature/Panel/Provider/Tool ist (editor, workspace, theme, terminal, vim, agent, collab, etc.) wird `raijin-*`.

**Faustregel:** Wenn es ohne Raijin als eigenständige Library existieren könnte → `inazuma`. Wenn es Raijin-spezifische Logik enthält → `raijin`.

### Crate-Verzeichnisse und Package-Names

| Referenz Prefix/Name | Raijin Prefix/Name |
|---|---|
| `gpui_*` | `inazuma-*` (z.B. `gpui_linux` → `inazuma-linux`) |
| `zed` | `raijin-app` (schon da) |
| `zed_*` | `raijin-*` (z.B. `zed_extension_api` → `raijin-extension-api`) |
| `zlog` | `raijin-log` (schon da) |
| `zlog_settings` | `raijin-log-settings` |
| `ztracing*` | `raijin-tracing*` (schon da) |
| Alle anderen | `raijin-*` (Unterstriche → Bindestriche im Verzeichnisnamen) |

**Beispiele:**
- `command_palette` → Verzeichnis `raijin-command-palette`, package name `raijin-command-palette`
- `gpui_linux` → Verzeichnis `inazuma-linux`, package name `inazuma-linux`
- `vim` → Verzeichnis `raijin-vim`, package name `raijin-vim`

### Cargo.toml Anpassungen

Für jede `Cargo.toml`:

1. **Package name** umbenennen:
   ```toml
   # Vorher (Referenz)
   name = "command_palette"
   # Nachher (Raijin)
   name = "raijin-command-palette"
   ```

2. **License** ändern:
   ```toml
   license = "GPL-3.0-or-later"
   ```

3. **Dependencies** umbenennen — JEDE Dependency die ein Referenz-Crate ist:
   ```toml
   # Vorher
   gpui.workspace = true
   theme.workspace = true
   settings.workspace = true
   workspace.workspace = true
   ui.workspace = true
   editor.workspace = true

   # Nachher
   inazuma.workspace = true
   raijin-theme.workspace = true
   inazuma-settings-framework.workspace = true
   raijin-workspace.workspace = true
   raijin-ui.workspace = true
   raijin-editor.workspace = true
   ```

4. **Vollständige Dependency-Mapping-Tabelle** (alle bekannten Renames):

   | Referenz Dependency | Raijin Dependency |
   |---|---|
   | `gpui` | `inazuma` |
   | `gpui_macros` | _(in inazuma/tooling/macros)_ |
   | `gpui_util` | `inazuma-gpui-util` |
   | `gpui_tokio` | `inazuma-tokio` |
   | `gpui_linux` | `inazuma-linux` |
   | `gpui_macos` | `inazuma-macos` |
   | `gpui_platform` | `inazuma-platform` |
   | `gpui_web` | `inazuma-web` |
   | `gpui_wgpu` | `inazuma-wgpu` |
   | `gpui_windows` | `inazuma-windows` |
   | `ui` | `raijin-ui` |
   | `ui_input` | `raijin-ui-input` |
   | `ui_macros` | `raijin-ui-macros` |
   | `ui_prompt` | `raijin-ui-prompt` |
   | `collections` | `inazuma-collections` |
   | `clock` | `inazuma-clock` |
   | `fuzzy` | `inazuma-fuzzy` |
   | `menu` | `inazuma-menu` |
   | `picker` | `inazuma-picker` |
   | `refineable` | `inazuma-refineable` |
   | `rope` | `inazuma-rope` |
   | `sum_tree` | `inazuma-sum_tree` |
   | `text` | `inazuma-text` |
   | `story` | `inazuma-story` |
   | `util` | `inazuma-util` |
   | `util_macros` | `inazuma-util-macros` |
   | `icons` | `inazuma-icons` |
   | `component` | `inazuma-component` |
   | `perf` | `inazuma-perf` |
   | `settings` | `inazuma-settings-framework` |
   | `settings_content` | `inazuma-settings-content` |
   | `settings_macros` | `inazuma-settings-macros` |
   | `settings_json` | `raijin-settings-json` |
   | `zed` | `raijin-app` |
   | `zed_actions` | `raijin-actions` |
   | `zed_env_vars` | `raijin-env-vars` |
   | `zed_extension_api` | `raijin-extension-api` |
   | `zlog` | `raijin-log` |
   | `zlog_settings` | `raijin-log-settings` |
   | `ztracing` | `raijin-tracing` |
   | `ztracing_macro` | `raijin-tracing-macro` |
   | `zeta_prompt` | `raijin-zeta-prompt` |
   | Alle anderen `xyz` | `raijin-xyz` (mit Bindestrichen) |

   **Beispiel:** `terminal_view` → `raijin-terminal-view`, `dap_adapters` → `raijin-dap-adapters`

### Source-Code Anpassungen

Für jede `.rs` Datei im `src/` Verzeichnis:

#### 1. `use`-Statement Renames

```rust
// Vorher (Referenz)
use gpui::{...};
use gpui_util::...;
use ui::{...};
use theme::{...};
use settings::{...};
use workspace::{...};
use editor::{...};
use collections::{...};
use fuzzy::{...};
use clock::{...};
use menu::{...};
use picker::{...};
use refineable::{...};
use rope::{...};
use sum_tree::{...};
use text::{...};
use util::{...};
use util_macros::{...};
use story::{...};
use zed_actions::{...};

// Nachher (Raijin)
use inazuma::{...};
use inazuma_gpui_util::...;
use raijin_ui::{...};
use raijin_theme::{...};
use inazuma_settings_framework::{...};
use raijin_workspace::{...};
use raijin_editor::{...};
use inazuma_collections::{...};
use inazuma_fuzzy::{...};
use inazuma_clock::{...};
use inazuma_menu::{...};
use inazuma_picker::{...};
use inazuma_refineable::{...};
use inazuma_rope::{...};
use inazuma_sum_tree::{...};
use inazuma_text::{...};
use inazuma_util::{...};
use inazuma_util_macros::{...};
use inazuma_story::{...};
use raijin_actions::{...};
```

Für alle anderen Referenz-Crates:
```rust
// use xyz::... → use raijin_xyz::...
// Bindestriche im Crate-Name werden Unterstriche im use-Statement
// z.B. raijin-terminal-view → use raijin_terminal_view::...
```

#### 2. Inline-Path Renames

Gleiche Regeln wie bei `use`-Statements, aber für qualifizierte Pfade im Code:
```rust
// gpui::red() → inazuma::red()
// theme::ActiveTheme → raijin_theme::ActiveTheme
// settings::Settings → inazuma_settings_framework::Settings
// workspace::Workspace → raijin_workspace::Workspace
```

#### 3. KRITISCH: NICHT umbenennen

Diese Submodule/Pfade dürfen NICHT umbenannt werden — sie sind lokale Module oder std-Library, nicht unsere Crates:

| Pattern | Warum nicht umbenennen |
|---|---|
| `std::collections::*` | Standard Library |
| `std::fs::*` | Standard Library |
| `std::task::*` | Standard Library |
| `futures::task::*` | futures Crate Submodul |
| `smol::fs::*` | smol Crate Submodul |
| `postage::watch::*` | postage Crate Submodul |
| `crate::client::*` | Lokales Submodul, nicht unser `raijin-client` |
| `crate::proto::*` | Lokales Submodul, nicht unser `raijin-proto` |
| `self::*` | Lokaler Pfad |
| `super::*` | Parent-Modul-Pfad |
| `inazuma_util::paths::*` | `paths` ist Submodul von inazuma-util, nicht raijin-paths |
| `inazuma_util::fs::*` | `fs` ist Submodul von inazuma-util, nicht raijin-fs |
| `raijin_rpc::proto::*` | `proto` ist Re-export in raijin-rpc, nicht raijin-proto |

**Regel:** Wenn etwas ein `::submodul` nach einem Crate-Namen ist, prüfen ob es ein lokales Submodul ist oder ein separater Crate. Im Zweifelsfall: den Referenz-Code lesen und schauen woher der Import kommt.

#### 4. Hsla → Oklch Konvertierung

Für Crates die Farben enthalten (Themes, UI, etc.):
- `Hsla` Typ → `Oklch` Typ
- `hsla()` Funktionsaufrufe → `oklch()` / `oklcha()`
- `gpui::Hsla` → `inazuma::Oklch`
- Farbwerte konvertieren mit: `node scripts/convert-hsla-to-oklch.mjs <datei> --inplace`

**Nicht alle Crates brauchen das** — nur solche die tatsächlich Farben definieren (UI-Crates, Theme-Crates). Die meisten Crates nutzen Farben nur als opaque `Oklch` Werte.

#### 5. GitHub-Fork-URLs

```toml
# Vorher
{ git = "https://github.com/raijin-hq/..." }

# Nachher
{ git = "https://github.com/raijin-hq/..." }
```

### Workspace Cargo.toml

Jeder neue Crate muss in `/Users/nyxb/Projects/raijin/Cargo.toml` eingetragen werden:

1. In `[workspace.members]`:
   ```toml
   "crates/raijin-command-palette",
   ```

2. In `[workspace.dependencies]`:
   ```toml
   raijin-command-palette = { path = "crates/raijin-command-palette" }
   ```

## Vollständige Liste: 134 fehlende Crates

### Kopier-Befehl pro Crate

```bash
# Template für jeden Crate:
cp -r .reference/zed/crates/CRATE_NAME crates/RAIJIN_NAME
```

### Die 134 Crates mit Referenz-Name → Raijin-Name

| # | Referenz Crate | Raijin Verzeichnis | Raijin Package Name |
|---|---|---|---|
| 1 | `acp_thread` | `raijin-acp-thread` | `raijin-acp-thread` |
| 2 | `acp_tools` | `raijin-acp-tools` | `raijin-acp-tools` |
| 3 | `action_log` | `raijin-action-log` | `raijin-action-log` |
| 4 | `activity_indicator` | `raijin-activity-indicator` | `raijin-activity-indicator` |
| 5 | `agent` | `raijin-agent` | `raijin-agent` |
| 6 | `agent_servers` | `raijin-agent-servers` | `raijin-agent-servers` |
| 7 | `agent_ui` | `raijin-agent-ui` | `raijin-agent-ui` |
| 8 | `ai_onboarding` | `raijin-ai-onboarding` | `raijin-ai-onboarding` |
| 9 | `assistant_slash_command` | `raijin-assistant-slash-command` | `raijin-assistant-slash-command` |
| 10 | `assistant_slash_commands` | `raijin-assistant-slash-commands` | `raijin-assistant-slash-commands` |
| 11 | `assistant_text_thread` | `raijin-assistant-text-thread` | `raijin-assistant-text-thread` |
| 12 | `audio` | `raijin-audio` | `raijin-audio` |
| 13 | `auto_update` | `raijin-auto-update` | `raijin-auto-update` |
| 14 | `auto_update_helper` | `raijin-auto-update-helper` | `raijin-auto-update-helper` |
| 15 | `auto_update_ui` | `raijin-auto-update-ui` | `raijin-auto-update-ui` |
| 16 | `aws_http_client` | `raijin-aws-http-client` | `raijin-aws-http-client` |
| 17 | `bedrock` | `raijin-bedrock` | `raijin-bedrock` |
| 18 | `call` | `raijin-call` | `raijin-call` |
| 19 | `channel` | `raijin-channel` | `raijin-channel` |
| 20 | `cli` | `raijin-cli` | `raijin-cli` |
| 21 | `codestral` | `raijin-codestral` | `raijin-codestral` |
| 22 | `collab` | `raijin-collab` | `raijin-collab` |
| 23 | `collab_ui` | `raijin-collab-ui` | `raijin-collab-ui` |
| 24 | `command_palette` | `raijin-command-palette` | `raijin-command-palette` |
| 25 | `command_palette_hooks` | `raijin-command-palette-hooks` | `raijin-command-palette-hooks` |
| 26 | `component_preview` | `raijin-component-preview` | `raijin-component-preview` |
| 27 | `copilot` | `raijin-copilot` | `raijin-copilot` |
| 28 | `copilot_chat` | `raijin-copilot-chat` | `raijin-copilot-chat` |
| 29 | `copilot_ui` | `raijin-copilot-ui` | `raijin-copilot-ui` |
| 30 | `crashes` | `raijin-crashes` | `raijin-crashes` |
| 31 | `csv_preview` | `raijin-csv-preview` | `raijin-csv-preview` |
| 32 | `dap_adapters` | `raijin-dap-adapters` | `raijin-dap-adapters` |
| 33 | `debug_adapter_extension` | `raijin-debug-adapter-extension` | `raijin-debug-adapter-extension` |
| 34 | `debugger_tools` | `raijin-debugger-tools` | `raijin-debugger-tools` |
| 35 | `debugger_ui` | `raijin-debugger-ui` | `raijin-debugger-ui` |
| 36 | `deepseek` | `raijin-deepseek` | `raijin-deepseek` |
| 37 | `denoise` | `raijin-denoise` | `raijin-denoise` |
| 38 | `dev_container` | `raijin-dev-container` | `raijin-dev-container` |
| 39 | `diagnostics` | `raijin-diagnostics` | `raijin-diagnostics` |
| 40 | `docs_preprocessor` | `raijin-docs-preprocessor` | `raijin-docs-preprocessor` |
| 41 | `edit_prediction` | `raijin-edit-prediction` | `raijin-edit-prediction` |
| 42 | `edit_prediction_cli` | `raijin-edit-prediction-cli` | `raijin-edit-prediction-cli` |
| 43 | `edit_prediction_context` | `raijin-edit-prediction-context` | `raijin-edit-prediction-context` |
| 44 | `edit_prediction_ui` | `raijin-edit-prediction-ui` | `raijin-edit-prediction-ui` |
| 45 | `encoding_selector` | `raijin-encoding-selector` | `raijin-encoding-selector` |
| 46 | `etw_tracing` | `raijin-etw-tracing` | `raijin-etw-tracing` |
| 47 | `eval` | `raijin-eval` | `raijin-eval` |
| 48 | `eval_cli` | `raijin-eval-cli` | `raijin-eval-cli` |
| 49 | `eval_utils` | `raijin-eval-utils` | `raijin-eval-utils` |
| 50 | `explorer_command_injector` | `raijin-explorer-command-injector` | `raijin-explorer-command-injector` |
| 51 | `extension_cli` | `raijin-extension-cli` | `raijin-extension-cli` |
| 52 | `extension_host` | `raijin-extension-host` | `raijin-extension-host` |
| 53 | `extensions_ui` | `raijin-extensions-ui` | `raijin-extensions-ui` |
| 54 | `feedback` | `raijin-feedback` | `raijin-feedback` |
| 55 | `file_finder` | `raijin-file-finder` | `raijin-file-finder` |
| 56 | `fs_benchmarks` | `raijin-fs-benchmarks` | `raijin-fs-benchmarks` |
| 57 | `git_graph` | `raijin-git-graph` | `raijin-git-graph` |
| 58 | `git_ui` | `raijin-git-ui` | `raijin-git-ui` |
| 59 | `go_to_line` | `raijin-go-to-line` | `raijin-go-to-line` |
| 60 | `google_ai` | `raijin-google-ai` | `raijin-google-ai` |
| 61 | `gpui_linux` | `inazuma-linux` | `inazuma-linux` | **⚠ NICHT KOPIEREN — aus inazuma extrahieren (siehe Fallstrick 5)** |
| 62 | `gpui_macos` | `inazuma-macos` | `inazuma-macos` | **⚠ NICHT KOPIEREN — aus inazuma extrahieren (siehe Fallstrick 5)** |
| 63 | `gpui_platform` | `inazuma-platform` | `inazuma-platform` | **⚠ NICHT KOPIEREN — aus inazuma extrahieren (siehe Fallstrick 5)** |
| 64 | `gpui_web` | `inazuma-web` | `inazuma-web` | **⚠ NICHT KOPIEREN — aus inazuma extrahieren (siehe Fallstrick 5)** |
| 65 | `gpui_wgpu` | `inazuma-wgpu` | `inazuma-wgpu` | **⚠ NICHT KOPIEREN — aus inazuma extrahieren (siehe Fallstrick 5)** |
| 66 | `gpui_windows` | `inazuma-windows` | `inazuma-windows` | **⚠ NICHT KOPIEREN — aus inazuma extrahieren (siehe Fallstrick 5)** |
| 67 | `html_to_markdown` | `raijin-html-to-markdown` | `raijin-html-to-markdown` |
| 68 | `image_viewer` | `raijin-image-viewer` | `raijin-image-viewer` |
| 69 | `inspector_ui` | `raijin-inspector-ui` | `raijin-inspector-ui` |
| 70 | `install_cli` | `raijin-install-cli` | `raijin-install-cli` |
| 71 | `journal` | `raijin-journal` | `raijin-journal` |
| 72 | `keymap_editor` | `raijin-keymap-editor` | `raijin-keymap-editor` |
| 73 | `language_extension` | `raijin-language-extension` | `raijin-language-extension` |
| 74 | `language_models` | `raijin-language-models` | `raijin-language-models` |
| 75 | `language_onboarding` | `raijin-language-onboarding` | `raijin-language-onboarding` |
| 76 | `language_selector` | `raijin-language-selector` | `raijin-language-selector` |
| 77 | `language_tools` | `raijin-language-tools` | `raijin-language-tools` |
| 78 | `line_ending_selector` | `raijin-line-ending-selector` | `raijin-line-ending-selector` |
| 79 | `livekit_api` | `raijin-livekit-api` | `raijin-livekit-api` |
| 80 | `livekit_client` | `raijin-livekit-client` | `raijin-livekit-client` |
| 81 | `lmstudio` | `raijin-lmstudio` | `raijin-lmstudio` |
| 82 | `markdown_preview` | `raijin-markdown-preview` | `raijin-markdown-preview` |
| 83 | `media` | `raijin-media` | `raijin-media` |
| 84 | `migrator` | `raijin-migrator` | `raijin-migrator` |
| 85 | `miniprofiler_ui` | `raijin-miniprofiler-ui` | `raijin-miniprofiler-ui` |
| 86 | `mistral` | `raijin-mistral` | `raijin-mistral` |
| 87 | `nc` | `raijin-nc` | `raijin-nc` |
| 88 | `notifications` | `raijin-notifications` | `raijin-notifications` |
| 89 | `ollama` | `raijin-ollama` | `raijin-ollama` |
| 90 | `onboarding` | `raijin-onboarding` | `raijin-onboarding` |
| 91 | `open_path_prompt` | `raijin-open-path-prompt` | `raijin-open-path-prompt` |
| 92 | `opencode` | `raijin-opencode` | `raijin-opencode` |
| 93 | `outline` | `raijin-outline` | `raijin-outline` |
| 94 | `outline_panel` | `raijin-outline-panel` | `raijin-outline-panel` |
| 95 | `panel` | `raijin-panel` | `raijin-panel` |
| 96 | `platform_title_bar` | `raijin-platform-title-bar` | `raijin-platform-title-bar` |
| 97 | `project_benchmarks` | `raijin-project-benchmarks` | `raijin-project-benchmarks` |
| 98 | `project_panel` | `raijin-project-panel` | `raijin-project-panel` |
| 99 | `project_symbols` | `raijin-project-symbols` | `raijin-project-symbols` |
| 100 | `prompt_store` | `raijin-prompt-store` | `raijin-prompt-store` |
| 101 | `recent_projects` | `raijin-recent-projects` | `raijin-recent-projects` |
| 102 | `remote_connection` | `raijin-remote-connection` | `raijin-remote-connection` |
| 103 | `remote_server` | `raijin-remote-server` | `raijin-remote-server` |
| 104 | `repl` | `raijin-repl` | `raijin-repl` |
| 105 | `reqwest_client` | `raijin-reqwest-client` | `raijin-reqwest-client` |
| 106 | `rules_library` | `raijin-rules-library` | `raijin-rules-library` |
| 107 | `scheduler` | `raijin-scheduler` | `raijin-scheduler` |
| 108 | `schema_generator` | `raijin-schema-generator` | `raijin-schema-generator` |
| 109 | `search` | `raijin-search` | `raijin-search` |
| 110 | `settings_profile_selector` | `raijin-settings-profile-selector` | `raijin-settings-profile-selector` |
| 111 | `settings_ui` | `raijin-settings-ui` | `raijin-settings-ui` |
| 112 | `shell_command_parser` | `raijin-shell-command-parser` | `raijin-shell-command-parser` |
| 113 | `sidebar` | `raijin-sidebar` | `raijin-sidebar` |
| 114 | `snippets_ui` | `raijin-snippets-ui` | `raijin-snippets-ui` |
| 115 | `storybook` | `raijin-storybook` | `raijin-storybook` |
| 116 | `streaming_diff` | `raijin-streaming-diff` | `raijin-streaming-diff` |
| 117 | `svg_preview` | `raijin-svg-preview` | `raijin-svg-preview` |
| 118 | `system_specs` | `raijin-system-specs` | `raijin-system-specs` |
| 119 | `tab_switcher` | `raijin-tab-switcher` | `raijin-tab-switcher` |
| 120 | `tasks_ui` | `raijin-tasks-ui` | `raijin-tasks-ui` |
| 121 | `terminal_view` | `raijin-terminal-view` | `raijin-terminal-view` | **⚠ NICHT KOPIEREN — wird in Phase 20 eigen gebaut (siehe Fallstrick 8)** |
| 122 | `time_format` | `raijin-time-format` | `raijin-time-format` |
| 123 | `title_bar` | `raijin-title-bar` | `raijin-title-bar` |
| 124 | `toolchain_selector` | `raijin-toolchain-selector` | `raijin-toolchain-selector` |
| 125 | `ui_prompt` | `raijin-ui-prompt` | `raijin-ui-prompt` |
| 126 | `vercel` | `raijin-vercel` | `raijin-vercel` |
| 127 | `vim` | `raijin-vim` | `raijin-vim` |
| 128 | `web_search` | `raijin-web-search` | `raijin-web-search` |
| 129 | `web_search_providers` | `raijin-web-search-providers` | `raijin-web-search-providers` |
| 130 | `which_key` | `raijin-which-key` | `raijin-which-key` |
| 131 | `worktree_benchmarks` | `raijin-worktree-benchmarks` | `raijin-worktree-benchmarks` |
| 132 | `x_ai` | `raijin-x-ai` | `raijin-x-ai` |
| 133 | `zed_extension_api` | `raijin-extension-api` | `raijin-extension-api` |
| 134 | `zlog_settings` | `raijin-log-settings` | `raijin-log-settings` |

## Prozess pro Crate

### Schritt 1: Kopieren
```bash
cp -r .reference/zed/crates/CRATE_NAME crates/RAIJIN_NAME
```

### Schritt 2: Cargo.toml anpassen
- Package name ändern
- License ändern
- Alle Dependencies nach Mapping-Tabelle umbenennen
- `edition.workspace = true` und `publish.workspace = true` beibehalten

### Schritt 3: Source-Code anpassen
Für jede `.rs` Datei:
1. `use gpui::` → `use inazuma::`
2. `use ui::` → `use raijin_ui::`
3. `use theme::` → `use raijin_theme::`
4. `use settings::` → `use inazuma_settings_framework::`
5. `use workspace::` → `use raijin_workspace::`
6. `use editor::` → `use raijin_editor::`
7. `use collections::` → `use inazuma_collections::`
8. Alle weiteren Crate-Imports nach Mapping-Tabelle
9. **NICHT** umbenennen: `std::*`, `crate::*`, `self::*`, `super::*`, lokale Submodule

### Schritt 4: Workspace Cargo.toml
- In `[workspace.members]` eintragen
- In `[workspace.dependencies]` eintragen

### Schritt 5: Build testen
```bash
cargo build -p RAIJIN_NAME
```

## Bekannte Fallstricke

### 1. Falsche Agent-Renames (aus vorherigen Sessions)

Diese Fehler sind bereits passiert und müssen beim Portieren vermieden werden:
- `std::collections` → ~~`std::inazuma_collections`~~ (FALSCH — `std::collections` bleibt)
- `futures::task` → ~~`futures::raijin_task`~~ (FALSCH — `futures::task` bleibt)
- `raijin_rpc::proto` → ~~`raijin_rpc::raijin_proto`~~ (FALSCH — `proto` ist Re-export)
- `inazuma_util::paths` → ~~`inazuma_util::raijin_paths`~~ (FALSCH — `paths` ist Submodul)
- `inazuma_util::fs` → ~~`inazuma_util::raijin_fs`~~ (FALSCH — `fs` ist Submodul)
- `smol::fs` → ~~`raijin_fs`~~ (FALSCH — `smol::fs` ist smol's async fs)
- `crate::client` → ~~`crate::raijin_client`~~ (FALSCH — lokales Submodul)

### 2. Double-Prefix Bugs
Wenn ein Name schon korrekt ist, NICHT nochmal umbenennen:
- ~~`inazuma_inazuma_text`~~ → `inazuma_text`
- ~~`raijin_raijin_proto`~~ → `raijin_proto`

### 3. Identifier-Corruption
Nicht blind suchen-und-ersetzen — Wörter wie `text`, `task`, `watch`, `collections` kommen auch in normalen Variablennamen und Kommentaren vor:
- `CharScopeContext` → ~~`CharScopeConinazuma_text`~~ (FALSCH)
- `task_context` → ~~`raijin_task_context`~~ (FALSCH)

**Regel:** Nur `use CRATE_NAME::` und `CRATE_NAME::` Pfade umbenennen, nicht Substrings in Identifiern.

### 4. Hsla → Oklch
Nur für Crates die Farbwerte definieren. Die meisten Crates nutzen `Oklch` nur als opaken Typ und brauchen keine Farbkonvertierung. Wenn doch:
```bash
node scripts/convert-hsla-to-oklch.mjs crates/RAIJIN_NAME/src/FILE.rs --inplace
```

### 5. Platform-Crates NICHT von der Referenz kopieren
Die 6 Platform-Crates (`gpui_linux`, `gpui_macos`, `gpui_platform`, `gpui_web`, `gpui_wgpu`, `gpui_windows`) dürfen **NICHT** einfach von der Referenz kopiert werden! Unser `inazuma` hat den Platform-Code schon drin UND wir haben die `objc2`-Migration durchgeführt (weg von `cocoa 0.26` + `objc 0.2`). Die Referenz nutzt noch die alte API. Kopieren würde unsere Migration zerstören.

**Stattdessen:** Diese 6 Crates müssen aus unserem bestehenden `inazuma/src/platform/` Code extrahiert werden. Das ist ein eigenes Refactoring (Platform-Split), nicht Teil des Kopier-Prozesses. Bis dahin: leere Placeholder-Crates erstellen die auf `inazuma` re-exportieren, oder als `[workspace.members]` weglassen.

### 6. `component` ≠ `inazuma-component`
Der Referenz-`component` Crate (534 Zeilen) ist NUR der Component Trait + Layout. Das haben wir bereits als `inazuma-component-registry`. Unser `inazuma-component` (250 Dateien, 70+ UI Components) hat kein Referenz-Äquivalent — es ist unsere eigene erweiterte Library basierend auf gpui-ce-component. `component_preview` von der Referenz referenziert deren kleinen `component`, nicht unsere große Library.

### 7. Theme-System Unterschiede
Unser `raijin-theme` hat eigene Architektur die von der Referenz abweicht:
- Gruppierte `StatusColors` (`StatusStyle { color, background, border }`) statt der flachen Referenz-Felder
- TOML-Loader statt JSON-Schema-System
- OKLCH Farbsystem statt HSLA
- `fallback_themes.rs` mit Raijin Dark/Light (nicht One Dark/Light)
- Block-Badge-Felder (`block_success_badge`, `block_error_badge`, `block_running_badge`)

Crates die auf Theme-Typen zugreifen müssen nach dem Kopieren auf unsere Typen angepasst werden (z.B. `status().error.color` statt `status().error`).

### 8. Terminal-Crates — NICHT von der Referenz kopieren
Der Referenz-`terminal` ist ein simpler Wrapper um `alacritty_terminal`. Unser Terminal-System ist fundamental anders — wir haben `alacritty_terminal` komplett geforked und umgebaut zu `raijin-term`:
- `raijin-term` — eigener Terminal-Emulations-Core (Fork von alacritty_terminal mit BlockGrid, eigenem VT State Machine, eigenem PTY-Abstraction)
- `raijin-terminal` — Block-System (BlockManager), OSC 133 Parser, Shell-Integration (ZDOTDIR Injection), PTY-Spawning mit Shell-Hooks
- `raijin-shell` — Shell-Context (CWD, Git Branch, User Info)
- `raijin-completions` — Spec-basierte CLI Completion Engine

**`terminal_view` von der Referenz NICHT kopieren.** Das Referenz-terminal_view ist für deren simplen Wrapper gebaut — passt nicht auf unser Block-System, Input Bar, Context Chips etc. Unser Terminal-View wird in Phase 20 (Workspace Integration) als eigenes Workspace Item gebaut, basierend auf unserem bestehenden `raijin-app/src/workspace.rs` + `terminal_element.rs`.

### 9. `extension_api` hat in der Referenz ein spezielles Build-System
`zed_extension_api` wird als WASM-API kompiliert. Beim Portieren die Build-Konfiguration beibehalten.

### 6. Platform-Crates (gpui_linux, gpui_macos, etc.)
Diese sind neu in der Referenz-Architektur — sie splitten Platform-spezifischen Code aus dem monolithischen `gpui` in separate Crates. Unsere `inazuma` hat diesen Split noch nicht. Diese Crates portieren, aber sie werden erst funktional wenn wir den Platform-Split in inazuma durchführen.

## Externe Dependencies

Neue externe Dependencies die mit den 134 Crates reinkommen und im Workspace `Cargo.toml` ergänzt werden müssen. Vor dem Portieren prüfen welche Versions die Referenz nutzt:

```bash
grep "DEPENDENCY_NAME" .reference/zed/Cargo.toml
```

Dann in unser `Cargo.toml` unter `[workspace.dependencies]` eintragen.

## Reihenfolge

**Alle 134 Crates müssen zu 100% portiert werden. Nichts wird übersprungen, nichts wird zurückgestellt. Alles ist gleich wichtig.**

Die Batches geben nur die Reihenfolge vor — Leaf-Crates (ohne interne Dependencies) zuerst, damit sie als Dependencies für spätere Crates verfügbar sind:

**Batch 1 — Keine internen Dependencies (Leaf-Crates):**
`time_format`, `shell_command_parser`, `streaming_diff`, `html_to_markdown`, `audio`, `media`, `nc`, `denoise`, `crashes`, `system_specs`, `reqwest_client`, `aws_http_client`, `scheduler`

**Batch 2 — Nur Framework-Dependencies (inazuma, theme, settings):**
`panel`, `sidebar`, `command_palette`, `command_palette_hooks`, `ui_prompt`, `platform_title_bar`, `title_bar`, `encoding_selector`, `line_ending_selector`, `go_to_line`, `which_key`, `tab_switcher`, `notifications`, `feedback`, `onboarding`, `settings_ui`, `settings_profile_selector`, `recent_projects`, `activity_indicator`

**Batch 3 — AI Provider (relativ unabhängig voneinander):**
`google_ai`, `ollama`, `mistral`, `lmstudio`, `deepseek`, `codestral`, `bedrock`, `vercel`, `x_ai`, `opencode`, `copilot`, `copilot_chat`, `copilot_ui`, `language_models`

**Batch 4 — Editor Features:**
`outline`, `outline_panel`, `diagnostics`, `search`, `file_finder`, `project_panel`, `project_symbols`, `vim`, `repl`, `snippets_ui`, `csv_preview`, `svg_preview`, `image_viewer`, `markdown_preview`, `journal`, `keymap_editor`, `toolchain_selector`, `language_selector`, `language_extension`, `language_tools`, `language_onboarding`

**Batch 5 — AI/Agent:**
`acp_thread`, `acp_tools`, `prompt_store`, `rules_library`, `web_search`, `web_search_providers`, `assistant_slash_command`, `assistant_slash_commands`, `assistant_text_thread`, `agent_servers`, `agent`, `agent_ui`, `ai_onboarding`, `action_log`

**Batch 6 — Debug/DAP:**
`dap_adapters`, `debug_adapter_extension`, `debugger_tools`, `debugger_ui`

**Batch 7 — Collab:**
`livekit_api`, `livekit_client`, `call`, `channel`, `collab`, `collab_ui`

**Batch 8 — Extension System:**
`extension_cli`, `extension_host`, `extensions_ui`, `zed_extension_api`

**Batch 9 — Edit Prediction:**
`edit_prediction`, `edit_prediction_cli`, `edit_prediction_context`, `edit_prediction_ui`

**Batch 10 — Remaining:**
`terminal_view`, `tasks_ui`, `remote_connection`, `remote_server`, `dev_container`, `auto_update`, `auto_update_helper`, `auto_update_ui`, `install_cli`, `cli`, `migrator`, `miniprofiler_ui`, `inspector_ui`, `component_preview`, `storybook`, `explorer_command_injector`, `etw_tracing`, `open_path_prompt`, `docs_preprocessor`, `schema_generator`, `eval`, `eval_cli`, `eval_utils`, `fs_benchmarks`, `project_benchmarks`, `worktree_benchmarks`, `zlog_settings`, `git_graph`, `git_ui`

**Batch 11 — Platform Split:**
`gpui_linux`, `gpui_macos`, `gpui_platform`, `gpui_web`, `gpui_wgpu`, `gpui_windows`
