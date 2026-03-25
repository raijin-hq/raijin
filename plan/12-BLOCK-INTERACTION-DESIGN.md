# Block Interaction & Design

## Block-Header Design (wie Warp)

### Erfolgreicher Block (exit 0)
- Grüner linker Rand (3px)
- Header: `nyxb MacBook-Pro.fritz.box ~ 12:12 (0.026s)` — kompakt, eine Zeile
- Header BG: etwas dunkler als Terminal-BG
- Rechts im Header: Action-Icons (Clip, Download, Filter) — sichtbar on hover

### Fehlgeschlagener Block (exit ≠ 0)
- Roter linker Rand (3px)
- Header BG: dezent rötlich getönt
- Sonst identisches Layout

### Block-Selection
- Klick auf Block → grüne Highlight-Tint über gesamten Block
- Markierter Block zeigt Action-Icons rechts oben

### Block Context-Menu (drei Punkte / Rechtsklick)
- Copy (Cmd+C)
- Copy command (Shift+Cmd+C)
- Copy output (Alt+Shift+Cmd+C)
- Share block... (Shift+Cmd+S)
- Share session...
- ---
- Save as workflow (Cmd+S)
- Attach as agent context (Ctrl+Shift+Space)
- ---
- Copy prompt
- Copy working directory
- ---
- Find within block (Cmd+F)
- Toggle block filter (Alt+Shift+F)
- Toggle bookmark (Cmd+B)
- ---
- Scroll to top of block (Shift+Cmd+↑)
- Scroll to bottom of block (Shift+Cmd+↓)

### Action-Icons (rechts im Block-Header, on hover)
- 📎 Attach as agent context
- ⬇ Download/Save output
- 🔍 Filter block
- ⋮ More (Context-Menu)

## Prompt-Suppression in Raijin Mode

In Raijin Mode darf die Shell PS1-Prompt NICHT im Terminal-Output sichtbar sein.
Die Shell-Hooks müssen den Prompt komplett unterdrücken — nur die Raijin Context Chips
unten am Input zeigen Prompt-Info.

### Shell-Hook Check
- `shell/raijin.zsh` — PS1 muss leer/minimal sein in Raijin Mode
- `shell/raijin.bash` — PS1 suppression
- `shell/raijin.fish` — fish_prompt override

## Block-Header Info-Format
```
{username} {hostname} {cwd} {time} ({duration})
```
- Username: aus `$USER`
- Hostname: gekürzt (ohne `.local`)
- CWD: shortened (`~` statt `/Users/nyxb`)
- Time: HH:MM
- Duration: nur bei > 0.1s anzeigen
