#!/usr/bin/env bash
# Raijin (雷神) Shell Integration for Bash
# Sends OSC 133 markers for block boundary detection.
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

    builtin printf '\e]133;A\a' >&$_raijin_fd
    _raijin_state=0
}

_raijin_preexec() {
    # Avoid firing for PROMPT_COMMAND itself
    if [[ "$BASH_COMMAND" == "$PROMPT_COMMAND" ]]; then
        return
    fi

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

# Raijin Mode: suppress shell prompt
if [[ "$RAIJIN_MODE" == "raijin" ]]; then
    PS1=''
    PS2=''
    export STARSHIP_SHELL=''
    export STARSHIP_SESSION_KEY='disabled'
fi
