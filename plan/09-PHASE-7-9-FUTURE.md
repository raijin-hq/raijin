# Phase 7–9: Drive, Polish, Distribution & Future

---

## Phase 7: Drive, Workflows, Notebooks (Woche 17–19)

### 7.1 — Workflows
- [ ] Parametrisierte Commands speichern (Name + Template + Params)
- [ ] AI Autofill für Workflow-Parameter
- [ ] Workflow Library (persönlich, später Team-shared)

### 7.2 — Notebooks
- [ ] Interaktive Runbooks: Markdown + ausführbare Command-Blocks
- [ ] Click-to-Run auf Code-Blocks
- [ ] Output wird inline unter dem Block angezeigt

### 7.3 — Launch Configurations
- [ ] Presets speichern: Window-Layout + Panes + Commands
- [ ] Beim Start laden: "Open last session" oder "Choose preset"

### 7.4 — Weitere Features
- [ ] Environment Variables sync across sessions
- [ ] Block Sharing: Permalinks generieren (Command + Output → URL)
- [ ] Docker Extension: Container-Liste, Logs, Exec in Container

---

## Phase 8: Polish, Performance & Distribution (Woche 19–22)

### 8.1 — Performance
- [ ] < 8ms Frame-Budget (120fps target)
- [ ] Startup < 200ms (lazy loading, minimal init path)
- [ ] Memory profiling: kein Leak bei langen Sessions
- [ ] GPU Memory: Texture-Atlas für Glyphen, nicht pro-Cell

### 8.2 — Settings GUI
- [ ] Vollständiges Settings-Panel (GPUI native)
- [ ] Kategorien: Appearance, Terminal, Editor, AI, Keybindings, Privacy
- [ ] Live-Preview bei Theme/Font Änderungen

### 8.3 — Keybindings
- [ ] Context-abhängig (Terminal vs Editor vs Explorer)
- [ ] Warp-kompatible Defaults
- [ ] Vollständig anpassbar (TOML Config)
- [ ] Keybinding Viewer im Settings-Panel

### 8.4 — Integrations
- [ ] Raycast / Alfred: "Open in Raijin" Action
- [ ] External Editor: "Open in VS Code/Zed/Cursor" konfigurierbar

### 8.5 — Distribution
- [ ] macOS: DMG Installer + Homebrew Cask (`brew install --cask raijin`)
- [ ] Linux: AppImage + .deb + .rpm
- [ ] Auto-Updater (Sparkle auf macOS, eigener Mechanismus auf Linux)

### 8.6 — Branding
- [ ] Logo & Icon (⚡ Raijin Thunder)
- [ ] Landing Page (raijin.dev oder ähnlich)
- [ ] README mit Screenshots, Features, Installation

---

## Phase 9+: Future / Differenzierung (Post-Launch)

- [ ] Windows Support (via wgpu — kein Metal, nur Vulkan/DX12)
- [ ] Session Sharing: Real-time Terminal-Sharing mit anderen Usern
- [ ] Team Drive: Geteilte Workflows, Notebooks, MCP Configs
- [ ] Cloud Agent Orchestration (eigene Oz-Alternative)
- [ ] SSO / SAML (Enterprise)
- [ ] Web-Version (WASM — Terminal im Browser)

---

## Milestone Phase 7
✅ Workflows können gespeichert und ausgeführt werden
✅ Notebooks rendern Markdown mit ausführbaren Commands

## Milestone Phase 8
✅ App startet in < 200ms
✅ Settings GUI funktioniert vollständig
✅ DMG + Homebrew Installation funktioniert

## Milestone Phase 9+
✅ Windows Build kompiliert und läuft
✅ Session Sharing zwischen zwei Usern funktioniert
