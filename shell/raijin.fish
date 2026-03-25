#!/usr/bin/env fish
# Raijin (雷神) Shell Integration for Fish
# Sends OSC 133 markers for block boundary detection.

if set -q _RAIJIN_HOOKED
    exit 0
end
set -g _RAIJIN_HOOKED 1

set -g _raijin_state 0

function _raijin_precmd --on-event fish_prompt
    set -l ret $status

    if test $_raijin_state -eq 1
        printf '\e]133;D;%d\a' $ret
    end

    printf '\e]133;A\a'
    set -g _raijin_state 0
end

function _raijin_preexec --on-event fish_preexec
    printf '\e]133;B\a'
    printf '\e]133;C\a'
    set -g _raijin_state 1
end

# Raijin Mode: suppress prompt
if test "$RAIJIN_MODE" = "raijin"
    function fish_prompt
        # Empty prompt — Raijin shows its own context chips
    end
    function fish_right_prompt
    end
    set -gx STARSHIP_SHELL ''
end
