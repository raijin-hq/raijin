# Phase 23: Dependencies aufräumen — Workspace-weite Konsistenz

## Ziel

1. **Alle Dependencies im Root `Cargo.toml`** unter `[workspace.dependencies]`, sauber gruppiert und kommentiert
2. **Überall `.workspace = true`** in den Crate-Cargo.toml — keine hartcodierten Versionen in Crates
3. **Nur im äußersten Notfall** darf eine Dependency lokal mit Version stehen (z.B. Build-Dependencies wie `bindgen`, plattform-spezifische Nischen-Crates)
4. **Root Cargo.toml** in logische Sektionen mit Kommentaren gegliedert

## Ist-Zustand (Stand 2026-04-08)

- ~299 lokale Version-Dependencies in Crate-Cargo.toml die `.workspace = true` sein sollten
- Root Cargo.toml hat Dependencies teilweise unsortiert und ungruppiert
- `inazuma/Cargo.toml` hat ~50+ lokale Dependencies die nicht im Workspace sind (objc2, wayland, platform-spezifisch)
- Einige Crates nutzen Git-Deps direkt statt über Workspace

## Root Cargo.toml Sektionen (Ziel-Struktur)

```toml
[workspace.dependencies]

# ─── Raijin Crates ───────────────────────────────────────────
raijin-actions = { path = "crates/raijin-actions" }
raijin-agent = { path = "crates/raijin-agent" }
# ... alphabetisch sortiert

# ─── Inazuma Framework Crates ────────────────────────────────
inazuma = { path = "crates/inazuma" }
inazuma-clock = { path = "crates/inazuma-clock" }
# ... alphabetisch sortiert

# ─── Serialization ───────────────────────────────────────────
serde = { version = "1.0", features = ["derive", "rc"] }
serde_json = { version = "1.0", features = ["preserve_order", "raw_value"] }
toml = "0.8"
# ...

# ─── Async Runtime ──────────────────────────────────────────
futures = "0.3"
smol = "2"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
# ...

# ─── Database ───────────────────────────────────────────────
rusqlite = { version = "0.31", features = ["bundled", "blob"] }
# ...

# ─── Logging ────────────────────────────────────────────────
log = { version = "0.4", features = ["std"] }
tracing = "0.1"
# ...

# ─── macOS Platform (objc2 ecosystem) ───────────────────────
objc2 = "0.6"
objc2-foundation = "0.3"
objc2-app-kit = "0.3"
objc2-metal = "0.3"
objc2-core-foundation = { version = "0.3", features = [...] }
objc2-core-video = { version = "0.3", features = [...] }
objc2-core-graphics = { version = "0.3", features = [...] }
objc2-core-text = { version = "0.3", features = [...] }
objc2-core-media = "0.3"
objc2-screen-capture-kit = "0.3"
# ...

# ─── Linux Platform ────────────────────────────────────────
wayland-client = { version = "0.31", optional = true }
wayland-protocols = { version = "0.32", features = [...] }
# ...

# ─── HTTP / Networking ──────────────────────────────────────
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
http_client = ...
# ...

# ─── AI / ML ───────────────────────────────────────────────
tiktoken-rs = "0.6"
candle-core = "0.10"
# ...

# ─── Collab / WebRTC ───────────────────────────────────────
livekit = { version = "0.7", features = [...] }
libwebrtc = "0.3"
# ...

# ─── Text / Parsing ────────────────────────────────────────
tree-sitter = { version = "0.26", features = ["wasm"] }
pulldown-cmark = { version = "0.13", default-features = false }
regex = "1"
# ...

# ─── Crypto / Auth ─────────────────────────────────────────
jsonwebtoken = "10.0"
sha2 = "0.10"
# ...

# ─── Misc ──────────────────────────────────────────────────
anyhow = "1"
thiserror = "2"
itertools = "0.14"
rand = "0.9"
uuid = { version = "1", features = ["v4", "v5"] }
# ...
```

## Prozess

### Phase 1: Inventur

1. **Alle lokalen Dependencies finden:**
   ```bash
   grep -rn 'version = "' crates/*/Cargo.toml | grep -v "^\[package\]" | grep -v "workspace"
   ```

2. **Für jede lokale Dependency prüfen:**
   - Ist sie schon im Workspace? → In Crate auf `.workspace = true` ändern
   - Ist sie nicht im Workspace? → Im Workspace hinzufügen, dann in Crate `.workspace = true`
   - Ist sie ein Sonderfall (bindgen, build-dep, platform-exklusiv)? → Dokumentieren warum lokal

### Phase 2: Workspace Dependencies gruppieren

1. Root `Cargo.toml` komplett neu strukturieren mit den oben definierten Sektionen
2. Alphabetisch innerhalb jeder Sektion
3. Kommentar-Header für jede Sektion

### Phase 3: Crate-Cargo.toml bereinigen

Für jede `crates/*/Cargo.toml`:
1. Alle `dependency = "version"` durch `dependency.workspace = true` ersetzen
2. Wenn Features gebraucht werden: `dependency = { workspace = true, features = ["extra"] }`
3. `cargo check -p CRATE` nach jeder Änderung

### Phase 4: Spezialfälle dokumentieren

Jede Dependency die NICHT `.workspace = true` nutzt bekommt einen Kommentar warum:
```toml
# Nicht im Workspace: nur als build-dependency für FFI bindgen
bindgen = "0.71"
```

### Phase 5: Validierung

```bash
# Keine lokalen Versions mehr (außer dokumentierte Ausnahmen):
grep -rn 'version = "' crates/*/Cargo.toml | grep -v workspace | grep -v "# Nicht im Workspace"

# Workspace Cargo.toml ist sortiert und gruppiert:
# Manuell prüfen

# Alles kompiliert:
cargo check
```

## Bekannte Hauptverursacher

- **`inazuma/Cargo.toml`** — ~50+ lokale Dependencies (objc2, wayland, platform-spezifisch)
- **`inazuma-component/ui/Cargo.toml`** — diverse lokale Deps
- **`raijin-denoise/Cargo.toml`** — candle, rodio, rustfft lokal
- **`raijin-media/Cargo.toml`** — bindgen als build-dep (Ausnahme OK)
- **Diverse neue Crates** aus Phase 21 — manche haben noch lokale Deps von der Referenz-Codebase

## Regel für die Zukunft

In CLAUDE.md steht: Dependencies IMMER über Workspace. Wenn eine neue Dependency gebraucht wird:
1. Zuerst in Root `Cargo.toml` unter `[workspace.dependencies]` in der richtigen Sektion eintragen
2. Dann in der Crate-Cargo.toml mit `.workspace = true` referenzieren
3. Niemals eine Version direkt in eine Crate-Cargo.toml schreiben
