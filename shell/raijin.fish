#!/usr/bin/env fish
# Raijin (雷神) Shell Integration for Fish
# Sends OSC 133 markers for block boundary detection and
# OSC 7777 metadata (JSON, hex-encoded) for context chips.

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

    # --- Gather metadata ---
    set -l _cwd (pwd)
    set -l _user $USER
    set -l _host (hostname -s 2>/dev/null; or echo unknown)
    set -l _shell "fish"
    set -l _git_branch ""
    set -l _git_dirty "false"

    if git rev-parse --git-dir >/dev/null 2>&1
        set _git_branch (git rev-parse --abbrev-ref HEAD 2>/dev/null)
        if test -n "$_git_branch"; and not git diff --quiet HEAD 2>/dev/null
            set _git_dirty "true"
        end
    end

    # Build JSON
    set -l _json '{'
    set _json "$_json\"cwd\":\"$_cwd\","
    set _json "$_json\"username\":\"$_user\","
    set _json "$_json\"hostname\":\"$_host\","
    set _json "$_json\"shell\":\"$_shell\""
    if test -n "$_git_branch"
        set _json "$_json,\"git_branch\":\"$_git_branch\""
        set _json "$_json,\"git_dirty\":$_git_dirty"
    end
    if test $_raijin_state -eq 1
        set _json "$_json,\"last_exit_code\":$ret"
    end
    set _json "$_json}"

    # Hex-encode JSON to prevent escape sequence breakage
    set -l _hex (printf '%s' "$_json" | xxd -p | string join '')

    # Send metadata via custom OSC 7777
    printf '\e]7777;raijin-precmd;%s\a' "$_hex"

    printf '\e]133;A\a'
    set -g _raijin_state 0
end

function _raijin_preexec --on-event fish_preexec
    printf '\e]133;B\a'
    printf '\e]133;C\a'
    set -g _raijin_state 1
end

# Raijin Mode: no shell-side prompt suppression needed.
# The Raijin renderer hides prompt rows on the Rust side.
