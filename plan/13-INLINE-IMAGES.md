# Inline Images — Kitty/Sixel/iTerm2 Protokolle

## Ziel

Terminal-Output kann Inline-Bilder enthalten (z.B. `cat image.png`, `imgcat`, Plotly-Grafiken).
Raijin soll alle drei gängigen Protokolle unterstützen.

## Protokolle

### Kitty Graphics Protocol (Priorität 1)
- Das modernste und mächtigste Protokoll
- Chunked Base64 Transmission (große Bilder in Teilen)
- Formate: RGB24, RGBA32, PNG
- Transmission-Modi: direct, file, shared memory
- Virtual Placements: Unicode Placeholder U+10EEEE mit image_id in Foreground-Color
- Aktionen: transmit, display, query, delete, animate, frame, compose
- State Machine für incomplete/chunked Images

### Sixel Protocol (Priorität 2)
- Ältestes Protokoll, breite Kompatibilität
- Parser mit 1024 Farb-Registern
- Max 4096x4096 Pixel
- Basiert auf DCS (Device Control String)

### iTerm2 Image Protocol (Priorität 3)
- OSC 1337 basiert
- Base64-encoded Image Data
- Unterstützt width/height Resize-Parameter
- Einfachster Parser der drei

## Architektur

```
PTY Output
  → VTE Parser erkennt Graphics-Escape-Sequenzen
  → raijin-term: GraphicsHandler speichert Bilddaten
  → GraphicData { id, pixels, width, height, format }
  → FxHashMap<GraphicId, GraphicData> auf Term-Ebene
  → grid_snapshot.rs: Bild-Referenzen in Snapshots
  → grid_element.rs: GPU-Textur-Upload via Inazuma
  → paint: Bild als Quad rendern an der Zell-Position
```

## Vor der Implementierung

**Zwingend: Rio und WezTerm Repos analysieren**

Beide haben alle drei Protokolle bereits implementiert. Den Code dort studieren und die Patterns übernehmen:

### Rio (https://github.com/raphamorim/rio)
- `rio-backend/src/ansi/sixel.rs` — Sixel Parser
- `rio-backend/src/ansi/kitty_graphics_protocol.rs` — Kitty Protocol mit chunked Transmission
- `rio-backend/src/ansi/iterm2_image_protocol.rs` — iTerm2 Parser
- `GraphicData` Abstraktion mit `GraphicId`
- Wie sie Bilder in der Grid-Cell referenzieren
- GPU-Textur-Mapping

### WezTerm (https://github.com/wez/wezterm)
- Sehr ausgereifter Kitty-Support
- `term/src/terminalstate/kitty.rs` — State Machine
- `term/src/terminalstate/image.rs` — Image Storage
- `term/src/image.rs` — ImageData, Placement, Animation
- Wie sie Sixel und iTerm2 parsen
- Wie sie Bilder im Grid speichern und bei Resize handlen

### Analyse-Fragen
1. Wo werden die Escape-Sequenzen abgefangen (VTE Handler vs. eigener Parser)?
2. Wie werden Bilder im Grid gespeichert (pro Zelle oder separate Map)?
3. Wie funktioniert Resize/Reflow mit Bildern?
4. Wie werden Bilder bei Scrollback evicted (Memory-Management)?
5. Wie werden animierte Bilder (Kitty Animation) gehandelt?
6. Virtual Placements: Wie wird U+10EEEE decoded?

## Implementierungsreihenfolge

1. `GraphicData` + `GraphicStore` Abstraktion in raijin-term
2. Kitty Graphics Parser + State Machine (chunked transmission)
3. Grid-Integration: Bild-Referenzen in Zellen
4. Snapshot-Integration: Bild-Daten in BlockSnapshot
5. GPU-Rendering: Textur-Upload + Quad-Painting via Inazuma
6. Sixel Parser
7. iTerm2 Parser
8. Memory-Management: Eviction bei zu vielen Bildern
9. Animation-Support (Kitty Frames)
