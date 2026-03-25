#!/usr/bin/env zsh
# Raijin (雷神) Shell Integration for Zsh
# Sends OSC 133 markers for block boundary detection.
# Loaded automatically via ZDOTDIR injection when Raijin spawns a PTY.

# Guard against double-sourcing
[[ -n "$_RAIJIN_HOOKED" ]] && return
_RAIJIN_HOOKED=1

# State machine: 0=idle, 1=command executing (OSC 133;C sent, D not yet)
_raijin_state=0

# Open fd to TTY directly so marks work even when stdout is redirected
if ! exec {_raijin_fd}>/dev/tty 2>/dev/null; then
    _raijin_fd=1
fi

_raijin_precmd() {
    local ret=$?

    # Close previous command block with exit code
    if [[ $_raijin_state == 1 ]]; then
        builtin printf '\e]133;D;%d\a' "$ret" >&$_raijin_fd
    fi

    # Mark prompt start
    builtin printf '\e]133;A\a' >&$_raijin_fd

    _raijin_state=0
}

_raijin_preexec() {
    # Mark prompt end / input region start
    builtin printf '\e]133;B\a' >&$_raijin_fd

    # Mark command execution start / output region start
    builtin printf '\e]133;C\a' >&$_raijin_fd

    _raijin_state=1
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd _raijin_precmd
add-zsh-hook preexec _raijin_preexec

# Raijin Mode: suppress shell prompt (replaced by Raijin's context chips)
if [[ "$RAIJIN_MODE" == "raijin" ]]; then
    PROMPT=''
    RPROMPT=''
    # Prevent Starship from overriding our empty prompt
    export STARSHIP_SHELL=''
    export STARSHIP_SESSION_KEY='disabled'
fi
