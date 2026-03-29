#!/usr/bin/env zsh
# Raijin (雷神) Shell Integration for Zsh
# Sends OSC 133 markers for block boundary detection and
# OSC 7777 metadata (JSON, hex-encoded) for context chips.
# Loaded automatically via ZDOTDIR injection when Raijin spawns a PTY.

# Guard against double-sourcing
[[ -n "$_RAIJIN_HOOKED" ]] && return
_RAIJIN_HOOKED=1

# State machine: 0=idle, 1=command executing (OSC 133;C sent, D not yet)
_raijin_state=0

# Write OSC markers to stdout (fd 1) so they travel through the PTY in the
# same byte stream as command output. This guarantees correct ordering:
# CommandStart → output → CommandEnd.
# preexec/precmd always have the original stdout (not redirected by user
# commands), so fd 1 is safe here. Using /dev/tty opens a separate fd to
# the same PTY device, which can cause race conditions where markers
# arrive before command output.
_raijin_fd=1

_raijin_precmd() {
    local ret=$?

    # Close previous command block with exit code
    if [[ $_raijin_state == 1 ]]; then
        builtin printf '\e]133;D;%d\a' "$ret" >&$_raijin_fd
    fi

    # --- Gather metadata ---
    local _cwd="$PWD"
    local _user="$USER"
    local _host="${HOST:-$(hostname -s 2>/dev/null || echo unknown)}"
    local _shell="zsh"
    local _git_branch=""
    local _git_dirty="false"

    if git rev-parse --git-dir >/dev/null 2>&1; then
        _git_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
        if [[ -n "$_git_branch" ]] && ! git diff --quiet HEAD 2>/dev/null; then
            _git_dirty="true"
        fi
    fi

    # Build JSON — values are escaped to handle quotes/backslashes in paths
    local _json='{'
    _json+='"cwd":"'${_cwd//\\/\\\\}'",'
    _json+='"username":"'${_user//\\/\\\\}'",'
    _json+='"hostname":"'${_host//\\/\\\\}'",'
    _json+='"shell":"'$_shell'"'
    if [[ -n "$_git_branch" ]]; then
        _json+=',"git_branch":"'${_git_branch//\\/\\\\}'"'
        _json+=',"git_dirty":'$_git_dirty
    fi
    if [[ $_raijin_state == 1 ]]; then
        _json+=',"last_exit_code":'$ret
        # Calculate command duration in milliseconds
        if [[ -n "$_raijin_cmd_start" ]]; then
            local _dur_ms=$(( (EPOCHREALTIME - _raijin_cmd_start) * 1000 ))
            _dur_ms=${_dur_ms%.*}
            _json+=',"last_duration_ms":'${_dur_ms:-0}
            unset _raijin_cmd_start
        fi
    fi
    _json+='}'

    # Hex-encode JSON to prevent bytes like 0x9C (ST terminator in emoji)
    # from breaking the escape sequence — same strategy as Warp.
    local _hex
    _hex=$(builtin printf '%s' "$_json" | xxd -p | tr -d '\n')

    # Send metadata via custom OSC 7777
    builtin printf '\e]7777;raijin-precmd;%s\a' "$_hex" >&$_raijin_fd

    # Mark prompt start
    builtin printf '\e]133;A\a' >&$_raijin_fd

    _raijin_state=0
}

_raijin_preexec() {
    # Record command start time (milliseconds via zsh EPOCHREALTIME)
    zmodload -F zsh/datetime p:EPOCHREALTIME 2>/dev/null
    _raijin_cmd_start=$EPOCHREALTIME

    # Mark prompt end / input region start
    builtin printf '\e]133;B\a' >&$_raijin_fd

    # Mark command execution start / output region start
    builtin printf '\e]133;C\a' >&$_raijin_fd

    _raijin_state=1
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd _raijin_precmd
add-zsh-hook preexec _raijin_preexec

# Raijin Mode: no shell-side prompt suppression needed.
# The Raijin renderer hides prompt rows (between PromptStart and CommandStart)
# on the Rust side — shell-agnostic, works with any prompt system.
