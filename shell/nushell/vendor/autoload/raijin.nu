# Raijin Terminal — Nushell Integration
#
# OSC 133 (block boundaries) is handled natively by Nushell's reedline.
# This script adds Raijin-specific features:
# - OSC 7777 metadata (CWD, git branch, username, shell info)
# - sudo wrapper preserving TERMINFO

let features = ($env.RAIJIN_SHELL_FEATURES? | default "metadata,sudo" | split row ",")

if "metadata" in $features {
    $env.config = ($env.config | upsert hooks {|config|
        let existing = ($config.hooks? | default {})
        let existing_pre_prompt = ($existing.pre_prompt? | default [])

        $existing | upsert pre_prompt ($existing_pre_prompt | append {||
            mut meta = {
                cwd: ($env.PWD),
                shell: "nu",
            }

            # Username
            $meta = ($meta | upsert username (whoami | str trim))

            # Git info (only if in a git repo)
            let git_check = (do { git rev-parse --git-dir } | complete)
            if $git_check.exit_code == 0 {
                let branch = (git rev-parse --abbrev-ref HEAD | str trim)
                let dirty = ((do { git diff --quiet HEAD } | complete).exit_code != 0)
                $meta = ($meta | upsert git_branch $branch | upsert git_dirty $dirty)
            }

            # Command duration from last command
            if ($env.CMD_DURATION_MS? | is-not-empty) {
                $meta = ($meta | upsert last_duration_ms ($env.CMD_DURATION_MS | into int))
            }

            let hex = ($meta | to json -r | encode hex)
            print -n $"\e]7777;raijin-precmd;($hex)\u{07}"
        })
    })
}

if "sudo" in $features {
    # Wrap sudo to preserve TERMINFO for proper terminal rendering
    def --wrapped raijin-sudo [...args: string] {
        if ("-e" in $args) or ("--edit" in $args) {
            ^sudo ...$args
        } else {
            let terminfo = ($env.TERMINFO? | default "")
            ^sudo $"TERMINFO=($terminfo)" ...$args
        }
    }
}
