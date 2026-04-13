# Warp Feature-Analyse (Vollständig)

> Alle Features die Warp hat und die wir nachbauen oder verbessern müssen.
> Quelle: warp.dev/all-features, Changelog, Docs, GitHub Issues

---

## Kategorie 1: Terminal-Grundlagen

| Feature | Warp | Unser Ziel |
|---|---|---|
| GPU-beschleunigtes Rendering (Metal/Vulkan) | ✅ Custom Rust + Metal/wgpu | ✅ gpui-ce (Metal + wgpu) |
| Shell-Kompatibilität (zsh, bash, fish, pwsh) | ✅ | ✅ |
| Platform Support | macOS, Linux, Windows | macOS + Linux first, Windows später |
| Blocks (Command + Output gruppiert) | ✅ Killer Feature | ✅ Prio 1 |
| IDE-like Input Editor (Cursor, Selections, Multi-line) | ✅ | ✅ |
| Vim Keybindings im Input | ✅ | ✅ |
| Smart Completions (400+ CLI Tools) | ✅ | ✅ |
| Command Corrections (Typo-Fix Vorschläge) | ✅ | ✅ Phase 4 |
| Split Panes (horizontal/vertikal) | ✅ | ✅ |
| Tabs (mit farbigen Indikatoren) | ✅ | ✅ |
| Scrollback Buffer | ✅ | ✅ |
| Text Selection (Maus + Keyboard) | ✅ | ✅ |
| Copy/Paste | ✅ | ✅ |
| Backwards compatible (bestehende Shell-Configs) | ✅ | ✅ |

---

## Kategorie 2: Appearance & Customization

| Feature | Warp | Unser Ziel |
|---|---|---|
| Custom Themes (Library + GUI Builder) | ✅ Themes + Background Images | ✅ |
| Theme aus Background-Image generieren (Farbpalette) | ✅ | ✅ Nice-to-have |
| Accent Color konfigurierbar | ✅ | ✅ |
| Custom Prompt (Chips) + PS1 Support (Starship, P10k) | ✅ | ✅ |
| Input Position (Top/Bottom pinnable) | ✅ | ✅ |
| Transparenter Background (Opacity) | ✅ | ✅ |
| App-Zoom (CMD+/CMD-) | ✅ | ✅ |
| Tab-Bar Visibility (Always/Hover/Windowed) | ✅ | ✅ |
| Tab Close-Button Position (links/rechts) | ✅ | ✅ |
| Tab-Farben (pro Tab konfigurierbar) | ✅ | ✅ |
| Font konfigurierbar (Type + Size) | ✅ | ✅ |
| Syntax Highlighting im Terminal-Output | ✅ | ✅ |

---

## Kategorie 3: Agent Utility Bar

| Feature | Warp | Unser Ziel |
|---|---|---|
| Automatische Agent-Erkennung (claude, codex, gemini) | ✅ Erkennt CLI-Agent automatisch | ✅ Prio 1 |
| Kontextuelle Footer-Toolbar für Third-Party CLI Agents | ✅ Zeigt File Explorer, View Changes etc. | ✅ Prio 1 |
| File Explorer in Agent-Modus | ✅ Project Explorer Sidebar | ✅ |
| View Changes (Diff-View) in Agent-Modus | ✅ Inline Diff-Viewer | ✅ |
| Voice Input Icon | ✅ (Wispr Flow Integration) | ✅ Optional |
| Image Attachment Icon | ✅ Bilder an Agents senden | ✅ |
| Drag File-Paths aus Explorer in Agent-Commands | ✅ | ✅ |
| Agent Status Indikator (blocked, working, done) | ✅ | ✅ |
| Custom Wrapper-Commands als Agent erkennen | ❌ (Feature Request #8579) | ✅ Von Anfang an konfigurierbar |
| Desktop Notifications wenn Agent fertig | ✅ (via OSC escape sequences) | ✅ |

---

## Kategorie 4: Code Editor (Warp Code)

| Feature | Warp | Unser Ziel |
|---|---|---|
| Nativer File Editor (Tabs, Syntax Highlighting) | ✅ Warp Code | ✅ Phase 5 |
| Real-time Diff Tracking | ✅ | ✅ |
| Code Review Panel | ✅ Accept/Reject/Edit Diffs inline | ✅ |
| File Tree / Project Explorer | ✅ mit .gitignore awareness | ✅ |
| Go to Line (CTRL-G) | ✅ | ✅ |
| Open Files from Explorer in Editor | ✅ | ✅ |
| External Editor Integration (VS Code, Zed, Cursor) | ✅ Konfigurierbar | ✅ |
| Lightweight — kein vollständiger IDE-Ersatz | ✅ | ✅ |

---

## Kategorie 5: AI / Agent Mode

| Feature | Warp | Unser Ziel |
|---|---|---|
| AI Command Suggestions (Natural Language → Command) | ✅ # Prefix | ✅ |
| Chat mit AI (Seitenpanel) | ✅ Agent Mode Panel | ✅ |
| Pair Mode (AI assistiert neben dir) | ✅ | ✅ |
| Dispatch Mode (AI arbeitet autonom) | ✅ | ✅ Phase 6+ |
| /plan Command (AI erstellt Ausführungsplan) | ✅ | ✅ |
| Multi-Model Support (OpenAI, Anthropic, Google) | ✅ 20+ Modelle | ✅ BYOK (Bring Your Own Key) |
| @-Context (Files, Images, URLs, Conversations) | ✅ Universal Input | ✅ |
| Prompt Suggestions (Active AI) | ✅ Kontextuelle Vorschläge | ✅ |
| Next Command (Ghost-Command basierend auf History) | ✅ | ✅ |
| Error Explanation (Exit Code != 0 → AI erklärt) | ✅ | ✅ |
| AI Block-Inhalte kopierbar | ✅ | ✅ |
| Agent Thinking expanded lassen (Setting) | ✅ | ✅ |
| Secret Redaction (API Keys in AI-Context obscuren) | ✅ | ✅ |
| MCP Server Integration | ✅ Figma, Linear, Slack, Sentry etc. | ✅ |
| Auto-detect MCP Servers (claude/codex config files) | ✅ | ✅ |
| WARP.md / agents.md / claude.md Support | ✅ Rules für Agents | ✅ |
| Voice Input (Wispr Flow) | ✅ | ⬜ Nice-to-have |

---

## Kategorie 6: Warp Drive (Cloud Knowledge)

| Feature | Warp | Unser Ziel |
|---|---|---|
| Workflows (parametrisierte Commands speichern) | ✅ | ✅ Phase 7 |
| Notebooks (interaktive Runbooks) | ✅ | ✅ Phase 7 |
| Personal Drive (Cloud-basierte Wissensbibliothek) | ✅ | ✅ Phase 8+ |
| Team Drive (geteilte Workflows/Notebooks) | ✅ | ✅ Phase 8+ |
| Environment Variables (sync across sessions) | ✅ | ✅ |
| Rules (Agent-Verhaltens-Konfiguration) | ✅ | ✅ |
| MCP Server Configs (teilbar im Team) | ✅ | ✅ |
| Warp Drive on Web (Browser-Zugang) | ✅ | ⬜ Später |

---

## Kategorie 7: Oz Cloud Agents (Warp's Neueste Plattform)

| Feature | Warp | Unser Ziel |
|---|---|---|
| Cloud Agent Orchestration | ✅ Oz Platform | ⬜ Phase 9+ |
| Parallel Cloud Agents | ✅ Unlimited parallel | ⬜ |
| Triggers (Slack, Linear, GitHub, Cron, Webhooks) | ✅ | ⬜ |
| Agent Audit Trail | ✅ | ⬜ |
| CLI + API/SDK | ✅ oz CLI | ⬜ |
| Self-hosted Environments | ✅ | ⬜ |
| Full Terminal Use (PTY attach) | ✅ | ⬜ |
| Computer Use (GUI Sandbox) | ✅ | ⬜ |

---

## Kategorie 8: Collaboration

| Feature | Warp | Unser Ziel |
|---|---|---|
| Session Sharing (Real-time terminal sharing) | ✅ Beta | ⬜ Phase 8+ |
| Block Sharing (Permalink für Command+Output) | ✅ | ✅ Phase 7 |
| Shared Agent Sessions | ✅ | ⬜ Phase 8+ |

---

## Kategorie 9: Usability

| Feature | Warp | Unser Ziel |
|---|---|---|
| Command Palette (CMD+P) | ✅ | ✅ |
| Command Search (History + Drive) | ✅ | ✅ |
| Rich History (Exit Codes, Directory, Branch, Timestamps) | ✅ | ✅ |
| Markdown Viewer (mit ausführbaren Commands) | ✅ | ✅ |
| Launch Configurations (Window/Pane/Command Presets) | ✅ | ✅ Phase 7 |
| Quake Mode (Dedicated Hotkey Window) | ✅ | ✅ |
| Shell Selector (Dropdown für Shell-Wechsel) | ✅ | ✅ |
| Sticky Headers (Block-Header beim Scrollen) | ✅ | ✅ |
| Global Search (über Code, Terminal, Notebooks) | ✅ | ✅ Phase 6 |

---

## Kategorie 10: Privacy & Security

| Feature | Warp | Unser Ziel |
|---|---|---|
| Secret Redaction (API Keys obscuren) | ✅ | ✅ |
| Disable Telemetry | ✅ | ✅ Default: aus |
| Zero Data Retention (Enterprise) | ✅ | ✅ |
| Disable Active AI | ✅ | ✅ |
| SSO / SAML (Enterprise) | ✅ | ⬜ Phase 8+ |

---

## Kategorie 11: Integrations

| Feature | Warp | Unser Ziel |
|---|---|---|
| Raycast / Alfred Integration | ✅ | ✅ |
| External Editor öffnen (VS Code, Zed, Cursor) | ✅ | ✅ |
| Docker Extension | ✅ | ✅ Phase 7 |
| Figma MCP (Auto-Detect Figma References) | ✅ | ✅ via MCP |
| GitHub Actions Integration (Oz) | ✅ | ⬜ Phase 9+ |
