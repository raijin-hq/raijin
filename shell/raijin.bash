#!/usr/bin/env bash
# Raijin (雷神) Shell Integration for Bash
# Sends OSC 133 markers for block boundary detection and
# OSC 7777 metadata (JSON, hex-encoded) for context chips.
# Loaded automatically via --rcfile when Raijin spawns a PTY.

# Guard against double-sourcing
[[ -n "$_RAIJIN_HOOKED" ]] && return
_RAIJIN_HOOKED=1

_raijin_state=0

# Open fd to TTY directly
if ! exec {_raijin_fd}>/dev/tty 2>/dev/null; then
    _raijin_fd=1
fi

_raijin_precmd() {
    local ret=$?

    if [[ $_raijin_state == 1 ]]; then
        builtin printf '\e]133;D;%d\a' "$ret" >&$_raijin_fd
    fi

    # --- Gather metadata ---
    local _cwd="$PWD"
    local _user="$USER"
    local _host="${HOSTNAME:-$(hostname -s 2>/dev/null || echo unknown)}"
    local _shell="bash"
    local _git_branch=""
    local _git_dirty="false"

    if git rev-parse --git-dir >/dev/null 2>&1; then
        _git_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
        if [[ -n "$_git_branch" ]] && ! git diff --quiet HEAD 2>/dev/null; then
            _git_dirty="true"
        fi
    fi

    # Build JSON — escape backslashes and quotes in values
    local _json='{'
    _json+='"cwd":"'"${_cwd//\\/\\\\}"'",'
    _json+='"username":"'"${_user//\\/\\\\}"'",'
    _json+='"hostname":"'"${_host//\\/\\\\}"'",'
    _json+='"shell":"'"$_shell"'"'
    if [[ -n "$_git_branch" ]]; then
        _json+=',"git_branch":"'"${_git_branch//\\/\\\\}"'"'
        _json+=',"git_dirty":'"$_git_dirty"
    fi
    if [[ $_raijin_state == 1 ]]; then
        _json+=',"last_exit_code":'"$ret"
        if [[ -n "$_raijin_cmd_start" ]]; then
            local _now_ms=$(date +%s%3N 2>/dev/null || echo 0)
            local _dur_ms=$(( _now_ms - _raijin_cmd_start ))
            _json+=',"last_duration_ms":'"${_dur_ms}"
            unset _raijin_cmd_start
        fi
    fi
    _json+='}'

    # Hex-encode JSON to prevent escape sequence breakage
    local _hex
    _hex=$(builtin printf '%s' "$_json" | xxd -p | tr -d '\n')

    # Send metadata via custom OSC 7777
    builtin printf '\e]7777;raijin-precmd;%s\a' "$_hex" >&$_raijin_fd

    builtin printf '\e]133;A\a' >&$_raijin_fd
    _raijin_state=0
}

_raijin_preexec() {
    # Avoid firing for PROMPT_COMMAND itself
    if [[ "$BASH_COMMAND" == "$PROMPT_COMMAND" ]]; then
        return
    fi

    _raijin_cmd_start=$(date +%s%3N 2>/dev/null || echo 0)

    builtin printf '\e]133;B\a' >&$_raijin_fd
    builtin printf '\e]133;C\a' >&$_raijin_fd
    _raijin_state=1
}

# Install precmd via PROMPT_COMMAND
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND='_raijin_precmd'
else
    PROMPT_COMMAND="_raijin_precmd;${PROMPT_COMMAND}"
fi

# Install preexec via DEBUG trap
trap '_raijin_preexec' DEBUG

# Source user's bashrc if it exists
if [[ -f "$HOME/.bashrc" ]]; then
    source "$HOME/.bashrc"
fi

# Raijin Mode: no shell-side prompt suppression needed.
# The Raijin renderer hides prompt rows on the Rust side.
