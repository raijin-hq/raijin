# Phase 6: AI Integration + Agent Toolbar (Woche 14–17)

> **Ziel:** AI-unterstütztes Terminal mit Agent Toolbar

---

## 6.1 — Agent Detection & Utility Bar

- [ ] Prozess-Erkennung: Laufende Child-Prozesse scannen
  - Bekannte Agents: `claude`, `codex`, `gemini`, `aider`, `copilot`
  - Pattern-Matching auf Prozessname + Argumente
- [ ] Konfigurierbar: Custom Commands als Agent erkennen (TOML Config)
  ```toml
  [[agents]]
  name = "my-agent"
  pattern = "python agent.py*"
  icon = "robot"
  ```
- [ ] Footer-Toolbar einblenden wenn Agent erkannt:
  - File Explorer Button (öffnet Project Explorer)
  - View Changes Button (öffnet Diff-View)
  - Image Attachment Icon (Bild an Agent senden via stdin/pipe)
  - Voice Input Icon (optional, Wispr Flow Integration)
  - Agent Status Indikator (blocked/working/done)
- [ ] Desktop Notifications via OSC wenn Agent fertig
- [ ] Task-Name in Tab-Title (aus Agent-Output oder Command parsen)

---

## 6.2 — AI Command Suggestions

- [ ] `#` Prefix im Input → Natural Language → Command Translation
- [ ] Multi-Provider BYOK (Bring Your Own Key):
  - Anthropic (Claude)
  - OpenAI (GPT-4)
  - Google (Gemini)
  - Konfigurierbar: API Endpoint + Key + Model
- [ ] Streaming Response im Input-Bereich

---

## 6.3 — Agent Mode Panel

- [ ] Seitenpanel (rechts) für AI Chat
- [ ] Pair Mode: AI assistiert neben dir (Vorschläge, Erklärungen)
- [ ] @-Context: Files, Images, URLs, vorherige Conversations referenzieren
- [ ] /plan Command: AI erstellt Ausführungsplan aus Beschreibung

---

## 6.4 — Error Explanation + Next Command

- [ ] Bei Exit Code != 0: AI-gestützte Fehler-Erklärung anbieten
- [ ] Next Command Suggestions: Ghost-Command basierend auf History + Context
- [ ] Prompt Suggestions: Kontextuelle Vorschläge basierend auf CWD + recent commands

---

## 6.5 — MCP + Rules + Security

- [ ] MCP Server Integration (Model Context Protocol)
- [ ] Auto-detect MCP Servers aus claude/codex Config-Files
- [ ] Rules Support: `.raijin.md`, `agents.md`, `claude.md` lesen
- [ ] Secret Redaction: API Keys in AI-Context automatisch obscuren

---

## Milestone

✅ Agent Toolbar erscheint automatisch wenn claude/codex läuft
✅ `#` Prefix übersetzt Natural Language in Shell-Commands
✅ Error Explanation erklärt fehlgeschlagene Commands
✅ MCP Servers werden automatisch erkannt
