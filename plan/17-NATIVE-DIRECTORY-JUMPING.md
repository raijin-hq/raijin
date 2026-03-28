# Native Directory-Jumping (zoxide-Equivalent)

## Ziel

Raijin bekommt native Frecency-basierte Directory-Navigation — das gleiche was
zoxide macht, aber ohne externes Tool. Der User tippt `cd pr` und bekommt sofort
ein Frecency-sortiertes Dropdown mit `/home/nyxb/dev/raijin/projects` ganz oben.

Zero Config. Kein `eval "$(zoxide init zsh)"`. Funktioniert ab dem ersten Start,
cross-shell, UI-integriert.

## Was wir bereits haben

| Baustein | Wo | Status |
|----------|-----|--------|
| CWD bei jedem Command | OSC 7777 → `ShellMetadataPayload.cwd` | ✅ Fertig |
| Metadata-Empfang | `workspace.rs` Zeile 187-199 | ✅ Fertig |
| ShellContext mit CWD | `raijin-shell/src/context.rs` | ✅ Fertig |
| Frecency-Algorithmus | `command_history.rs` Zeile 371-400 | ✅ Fertig (für Commands) |
| HistoryEntry mit cwd-Feld | `command_history.rs` Zeile 14-21 | ✅ Struct existiert |
| CompletionKind::Folder | `raijin-completions/src/matcher.rs` | ✅ Fertig |
| CompletionCandidate | `raijin-completions/src/matcher.rs` | ✅ Fertig |
| Folder-Completion für Specs | `shell_completion.rs` Zeile 147+ | ✅ Fertig |
| Input-Bar mit Dropdown | `workspace.rs` | ✅ Fertig |

**Es fehlt nur:** Eine `DirectoryHistory` die CWD-Besuche trackt, Frecency
berechnet, persistiert, und als Completion-Source bereitstellt.

## Architektur

```
OSC 7777 (precmd)
    │
    ▼
workspace.rs: ShellMetadataPayload.cwd
    │
    ├──▶ ShellContext (Display: Tab-Label, Chips)     [existiert]
    │
    └──▶ DirectoryHistory.record_visit(cwd)           [NEU]
             │
             ├── Frecency-DB updaten (rank + timestamp)
             ├── Decay wenn total_rank > MAX_AGE
             └── Persist nach ~/.config/raijin/directory_history.db
                      │
                      ▼
         ShellCompletionProvider.complete_directory()   [NEU]
             │
             └──▶ CompletionCandidate { kind: Folder, sort_priority: frecency }
                      │
                      ▼
                 Input-Bar Dropdown                    [existiert]
```

### Wo lebt der Code

```
raijin-app/src/directory_history.rs    ← NEU (~200 Zeilen)
```

Gleiche Ebene wie `command_history.rs`. Gehört in `raijin-app` weil:
- OSC 7777 kommt in `workspace.rs` an (raijin-app)
- Completions werden in `workspace.rs` zusammengebaut (raijin-app)
- Persistenz geht nach `~/.config/raijin/` (raijin-settings Pfade)
- `raijin-term` weiß nichts von UI, Completions, oder Persistenz — richtig so

## Datenstruktur

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Ein Verzeichnis-Eintrag in der Frecency-Datenbank.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectoryEntry {
    /// Absoluter Pfad
    pub path: String,
    /// Kumulativer Besuchs-Rank (+1.0 pro Besuch)
    pub rank: f64,
    /// Unix-Timestamp des letzten Besuchs (Sekunden)
    pub last_accessed: u64,
}

/// Die Frecency-Datenbank für Verzeichnisse.
pub struct DirectoryHistory {
    /// Alle bekannten Verzeichnisse, indexiert nach Pfad
    entries: HashMap<String, DirectoryEntry>,
    /// Schwellwert für Decay (wie zoxide: 10000.0)
    max_age: f64,
    /// Pfad zur persistenten DB-Datei
    db_path: PathBuf,
    /// Dirty-Flag: true wenn seit letztem Save etwas geändert wurde
    dirty: bool,
}
```

## Frecency-Algorithmus

Exakt zoxide's bewährter Algorithmus:

```rust
impl DirectoryEntry {
    /// Berechnet den Frecency-Score basierend auf Rank und Alter.
    ///
    /// Zeitfenster-Gewichtung (identisch mit zoxide):
    /// - < 1 Stunde:  rank × 4.0
    /// - < 1 Tag:     rank × 2.0
    /// - < 1 Woche:   rank × 0.5
    /// - >= 1 Woche:  rank × 0.25
    pub fn score(&self, now: u64) -> f64 {
        let age_secs = now.saturating_sub(self.last_accessed);
        let weight = if age_secs < 3600 {
            4.0
        } else if age_secs < 86400 {
            2.0
        } else if age_secs < 604800 {
            0.5
        } else {
            0.25
        };
        self.rank * weight
    }
}
```

## Kernmethoden

```rust
impl DirectoryHistory {
    /// Lade DB von Disk oder erstelle leere DB.
    pub fn load_or_create(db_path: PathBuf) -> Self { ... }

    /// Verzeichnis-Besuch aufzeichnen. Aufgerufen bei jedem OSC 7777.
    ///
    /// - Existierender Eintrag: rank += 1.0, last_accessed = now
    /// - Neuer Eintrag: rank = 1.0, last_accessed = now
    /// - Decay anwenden wenn total_rank > max_age
    pub fn record_visit(&mut self, path: &str) {
        let now = current_unix_timestamp();

        let entry = self.entries
            .entry(path.to_string())
            .or_insert_with(|| DirectoryEntry {
                path: path.to_string(),
                rank: 0.0,
                last_accessed: now,
            });

        entry.rank += 1.0;
        entry.last_accessed = now;
        self.dirty = true;

        // Decay wenn nötig
        let total_rank: f64 = self.entries.values().map(|e| e.rank).sum();
        if total_rank > self.max_age {
            self.apply_decay(total_rank);
        }
    }

    /// zoxide-Decay: rank *= 0.9 * max_age / total_rank
    /// Einträge mit rank < 1.0 nach Decay werden entfernt.
    fn apply_decay(&mut self, total_rank: f64) {
        let factor = 0.9 * self.max_age / total_rank;
        self.entries.retain(|_, entry| {
            entry.rank *= factor;
            entry.rank >= 1.0
        });
    }

    /// Frecency-sortierte Suche. Keywords werden right-to-left gegen
    /// Pfad-Segmente gematcht (wie zoxide).
    ///
    /// Beispiele:
    ///   "proj"      → matcht /home/nyxb/dev/projects
    ///   "dev proj"  → matcht /home/nyxb/dev/projects (beide Keywords in Order)
    ///   "raj src"   → matcht /home/nyxb/dev/raijin/src
    pub fn query(&self, keywords: &[&str]) -> Vec<&DirectoryEntry> {
        let now = current_unix_timestamp();
        let mut matches: Vec<_> = self.entries.values()
            .filter(|entry| matches_keywords(&entry.path, keywords))
            .collect();

        matches.sort_by(|a, b| {
            b.score(now)
                .partial_cmp(&a.score(now))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matches
    }

    /// Speichere DB auf Disk (nur wenn dirty).
    /// Atomarer Write: tmpfile → fsync → rename.
    pub fn save_if_dirty(&mut self) -> std::io::Result<()> { ... }
}
```

## Matching-Algorithmus

zoxide's Right-to-Left Substring Matching:

```rust
/// Matcht Keywords gegen einen Pfad. Alle Keywords müssen in Reihenfolge
/// (links nach rechts) im Pfad vorkommen. Case-insensitive.
///
/// Regeln (identisch mit zoxide):
/// 1. Keywords werden nacheinander von links nach rechts gesucht
/// 2. Jedes Keyword muss nach dem vorherigen Match gefunden werden
/// 3. Case-insensitive mit ASCII-Fast-Path
/// 4. Leere Keywords matchen alles
fn matches_keywords(path: &str, keywords: &[&str]) -> bool {
    if keywords.is_empty() {
        return true;
    }

    let path_lower = path.to_ascii_lowercase();
    let mut search_from = 0;

    for keyword in keywords {
        let keyword_lower = keyword.to_ascii_lowercase();
        match path_lower[search_from..].find(&keyword_lower) {
            Some(pos) => {
                search_from += pos + keyword_lower.len();
            }
            None => return false,
        }
    }

    true
}
```

## Persistenz

```rust
/// DB-Format: JSON Lines (einfach, human-readable, debuggbar).
/// Pfad: ~/.config/raijin/directory_history.json
///
/// Warum JSON statt Bincode:
/// - DB ist klein (typisch < 500 Einträge, < 50 KB)
/// - Debuggbar wenn User Probleme hat
/// - Keine Versionskompatibilitätsprobleme bei Updates
/// - Parse-Zeit irrelevant (einmal beim Start, < 1ms)

/// Atomarer Write:
/// 1. Write to ~/.config/raijin/directory_history.json.tmp
/// 2. fsync
/// 3. rename → directory_history.json
///
/// Save-Frequenz: Alle 30 Sekunden wenn dirty, plus beim Quit.
```

## Integration in workspace.rs

```rust
// workspace.rs — wo OSC 7777 Metadata ankommt (Zeile 187-199)

// BESTEHEND:
self.shell_context.update_from_metadata(&payload);
self.completion_provider.set_cwd(&payload.cwd);

// NEU (1 Zeile):
self.directory_history.record_visit(&payload.cwd);
```

## Integration in Completions

```rust
// In ShellCompletionProvider oder als eigener Provider:

/// Wenn der User "cd " oder "z " tippt, Frecency-Directories als
/// Completions liefern, ZUSÄTZLICH zu den normalen Folder-Completions.
pub fn complete_directory_jump(
    &self,
    keywords: &[&str],
    directory_history: &DirectoryHistory,
) -> Vec<CompletionCandidate> {
    directory_history
        .query(keywords)
        .into_iter()
        .enumerate()
        .map(|(i, entry)| CompletionCandidate {
            text: entry.path.clone(),
            display: shorten_path(&entry.path),  // ~/dev/raijin statt /home/nyxb/dev/raijin
            description: Some(format!("Score: {:.0}", entry.score(now))),
            kind: CompletionKind::Folder,
            sort_priority: i as u32,  // Bereits nach Frecency sortiert
        })
        .take(10)  // Max 10 Vorschläge
        .collect()
}
```

## Trigger-Erkennung

Wann werden Frecency-Directories angeboten statt normaler Folder-Completion:

```rust
// In der Completion-Logik:
match input.split_whitespace().collect::<Vec<_>>().as_slice() {
    // "cd" mit beliebig vielen Keywords → Frecency-Lookup
    ["cd", keywords @ ..] => complete_directory_jump(keywords, &self.dir_history),

    // "z" als Alias (optional, für zoxide-User die es gewohnt sind)
    ["z", keywords @ ..] => complete_directory_jump(keywords, &self.dir_history),

    // "pushd" auch
    ["pushd", keywords @ ..] => complete_directory_jump(keywords, &self.dir_history),

    // Alles andere → normale Completion-Pipeline
    _ => self.complete_normal(input),
}
```

## Optionale Erweiterungen (Phase 2+)

### 1. Keyboard-Shortcut für Directory-Picker
Ctrl+G (wie "Go") öffnet ein fzf-ähnliches Fuzzy-Finder-Overlay mit allen
Frecency-Directories. Input filtern, Enter zum Springen.

### 2. Recent Projects in Welcome-Screen
Die Top-10 Frecency-Directories als "Recent Projects" auf einem
Welcome-Screen oder in der Tab-Bar.

### 3. zoxide-DB Import
Für User die von zoxide kommen: beim ersten Start prüfen ob
`~/.local/share/zoxide/db.zo` existiert, Einträge importieren.
Format: Bincode v3 (4 Bytes Version-Header, dann `Vec<Dir>`).

### 4. Directory-Aware Command Suggestions
Wenn der User in `/home/nyxb/dev/raijin/` ist, Commands die er dort
häufig ausführt höher ranken (`cargo build` > `npm install`).
Die CWD-Felder in HistoryEntry ermöglichen das bereits.

## Implementierungsreihenfolge

| Schritt | Aufwand | Was |
|---------|---------|-----|
| 1. `DirectoryHistory` struct + Frecency | ~80 Zeilen, 2h | Kernlogik |
| 2. `record_visit()` + Decay | ~40 Zeilen, 1h | zoxide-Algorithmus |
| 3. `query()` + Matching | ~30 Zeilen, 1h | Right-to-left Substring |
| 4. JSON Persistenz (load/save) | ~30 Zeilen, 1h | Atomarer Write |
| 5. workspace.rs Hook (1 Zeile) | 1 Zeile, 5min | OSC 7777 → record_visit |
| 6. Completion-Integration | ~20 Zeilen, 1h | cd/z/pushd Trigger |
| **Gesamt** | **~200 Zeilen, 6h** | |

## Warum das besser ist als zoxide

| | zoxide | Raijin nativ |
|---|--------|-------------|
| Setup | `eval "$(zoxide init zsh)"` in .zshrc | Nichts — funktioniert sofort |
| Cross-Shell | Separate init pro Shell | Eine DB, automatisch cross-shell |
| UI | Text-Output im Terminal | Dropdown in der Input-Bar |
| Dependency | Externes Binary (Rust, ~10 MB) | Eingebaut (~200 Zeilen) |
| CWD-Tracking | Shell-Hook (`chpwd`) | OSC 7777 (bereits vorhanden) |
| Kompatibilität | Kann mit Terminal kollidieren | Ist das Terminal |
| Erweiterbar | Nicht ohne Fork | Directory-Picker, Recent Projects, CWD-aware Suggestions |
