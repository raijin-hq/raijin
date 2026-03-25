# PRECMD JSON Metadata Architecture (wie Warp)

## Ziel

Shell-Prompt wird NICHT im Terminal-Grid gerendert. Stattdessen sendet die Shell Metadata via custom Escape-Sequence an Raijin. Raijin rendert Prompt-Info als eigene UI-Elemente (Context Chips, Block-Header).

## Architektur (wie Warp)

```
Shell (zsh/bash/fish)
  │
  ├─ PRECMD Hook: sammelt Metadata (cwd, git branch, exit code, etc.)
  │
  ├─ Sendet JSON via custom OSC Escape-Sequence:
  │   \e]7777;raijin-precmd;{"cwd":"/Users/nyxb","git_branch":"main",...}\a
  │
  ├─ Setzt PS1="" (kein Prompt wird gerendert)
  │
  └─ User tippt Command → PREEXEC Hook sendet OSC 133;C
       │
       └─ Command-Output geht normal ins Terminal-Grid

Raijin PTY-Reader
  │
  ├─ Scannt für OSC 7777 (Raijin Metadata) + OSC 133 (Block Markers)
  │
  ├─ Parst JSON aus OSC 7777 → TerminalEvent::ShellMetadata { cwd, git, ... }
  │
  └─ Workspace empfängt Event → Updated Context Chips + Block-Header
```

## Custom Escape Sequence

Format: `OSC 7777 ; raijin-precmd ; <json> ST`
- `OSC` = `\e]` (ESC + ])
- `ST` = `\a` (BEL) oder `\e\\`
- JSON payload:

```json
{
  "cwd": "/Users/nyxb/Projects/raijin",
  "git_branch": "main",
  "git_dirty": true,
  "git_stats": { "files": 3, "insertions": 42, "deletions": 10 },
  "hostname": "MacBook-Pro.fritz.box",
  "username": "nyxb",
  "last_exit_code": 0,
  "last_duration_ms": 26,
  "shell": "zsh",
  "time": "12:12"
}
```

## Shell Hooks

### raijin.zsh
```zsh
_raijin_precmd() {
    local ret=$?

    # Close previous block
    if [[ $_raijin_state == 1 ]]; then
        builtin printf '\e]133;D;%d\a' "$ret" >&$_raijin_fd
    fi

    # Gather metadata
    local git_branch=""
    git_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

    local json='{'
    json+='"cwd":"'$(pwd)'",'
    json+='"username":"'$USER'",'
    json+='"hostname":"'$(hostname -s)'",'
    json+='"git_branch":"'$git_branch'",'
    json+='"last_exit_code":'$ret','
    json+='"shell":"zsh"'
    json+='}'

    # Send metadata via custom OSC
    builtin printf '\e]7777;raijin-precmd;%s\a' "$json" >&$_raijin_fd

    # Mark prompt start
    builtin printf '\e]133;A\a' >&$_raijin_fd

    _raijin_state=0
}
```

### PS1 Suppression
In Raijin Mode: `PROMPT=$'\e]133;B\a'`
- PS1 is literally just the OSC 133;B marker (InputStart)
- No visible text rendered
- Works with ANY shell prompt system (Starship, P10k, oh-my-zsh)
- The marker itself is an escape sequence — invisible in terminal

## Raijin Side Changes

### osc_parser.rs
- Add parsing for `OSC 7777 ; raijin-precmd ; <json>` sequences
- New event: `ShellMarker::Metadata(String)` containing raw JSON

### event.rs
- New variant: `TerminalEvent::ShellMetadata(ShellMetadataPayload)`

### ShellMetadataPayload (new struct)
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ShellMetadataPayload {
    pub cwd: String,
    pub username: Option<String>,
    pub hostname: Option<String>,
    pub git_branch: Option<String>,
    pub git_dirty: Option<bool>,
    pub last_exit_code: Option<i32>,
    pub last_duration_ms: Option<u64>,
    pub shell: Option<String>,
}
```

### workspace.rs
- On `ShellMetadata` event: update `ShellContext` dynamically
- Context Chips refresh with live data (CWD changes on cd, git branch changes, etc.)
- Block-Header gets metadata from the block's ShellMetadata snapshot

### ShellContext
- No longer gathered once at startup
- Updated on every PRECMD via ShellMetadata events
- Each Block stores a snapshot of ShellContext at creation time

## Benefits

1. **No PS1 hacks** — works with Starship, P10k, oh-my-zsh, custom PS1, anything
2. **Live-updating Context Chips** — CWD updates on cd, git branch on checkout
3. **Rich Block-Headers** — username, hostname, cwd, time, duration per block
4. **Shell-agnostic** — same OSC protocol works for zsh/bash/fish
5. **Professional** — same architecture as Warp

## Implementation Order

1. Shell hooks: add JSON metadata in PRECMD, set PS1 to OSC 133;B marker only
2. osc_parser.rs: parse OSC 7777 sequences
3. New struct: ShellMetadataPayload with serde Deserialize
4. event.rs: add ShellMetadata variant
5. terminal.rs: emit ShellMetadata events from PTY reader
6. workspace.rs: handle ShellMetadata → update ShellContext dynamically
7. Block-Header: render from metadata snapshot instead of static ShellContext
