# Phase: Native Context Chips — Starship-Replacement

## Context

Raijin zeigt aktuell 7 hardcodierte Chips: Username, Hostname, CWD, Zeit, Shell, Git Branch, Git Stats. Starship zeigt 99+ Module. Ziel: Raijin ersetzt Starship komplett mit nativen, konfigurierbaren Chips — ohne Nerd Fonts, mit Lucide SVG Icons.

**Quelle:** `.reference/starship/src/modules/` (99 Module)
**Architektur:** Konfigurierbares Chip-System in `raijin-app` mit Detection + Rendering Pipeline

---

## Architektur

```
┌─────────────────────────────────────────────────┐
│ raijin-settings (config.toml)                   │
│   [chips]                                       │
│   enabled = ["user", "host", "dir", "git", ...] │
│   [chips.kubernetes]                            │
│   disabled = false                              │
│   icon = "helm"  # Lucide icon override         │
└─────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────┐
│ ChipProvider (trait)                            │
│   fn detect(&self, ctx: &ChipContext) -> bool   │
│   fn gather(&self, ctx: &ChipContext) -> Chip   │
│   fn icon(&self) -> IconName                    │
│   fn default_color(&self) -> Hsla               │
└─────────────────────────────────────────────────┘
           │
    ┌──────┼──────┬──────────┬────────────┐
    ▼      ▼      ▼          ▼            ▼
  User   Host   Dir    GitBranch    Kubernetes
  Chip   Chip   Chip     Chip         Chip
                          ...99+ Providers
```

---

## Tier 1: Core Chips (bereits implementiert, refactoren zu Provider-System)

| Chip | Info | Detection | Icon (Lucide) | Status |
|------|------|-----------|---------------|--------|
| `username` | `$USER` | Immer | `User` | ✅ Vorhanden |
| `hostname` | Hostname | Immer | `Monitor` | ✅ Vorhanden |
| `directory` | CWD (~ abbreviated) | Immer | `Folder` | ✅ Vorhanden |
| `time` | Aktuelle Uhrzeit | Immer | `Clock` | ✅ Vorhanden |
| `shell` | Aktive Shell + Selector | Immer | — | ✅ Vorhanden |
| `git_branch` | Git Branch | `.git/` vorhanden | `GitBranch` | ✅ Vorhanden |
| `git_status` | Files changed, +/- | `.git/` vorhanden | `FileDiff` | ✅ Vorhanden |

---

## Tier 2: Sprachen & Runtimes (Starship-Parity, hohe Priorität)

| Chip | Info | Detection | Icon |
|------|------|-----------|------|
| `nodejs` | Node.js Version | `package.json`, `.nvmrc`, `node_modules/` | `Hexagon` (oder custom Node icon) |
| `python` | Python Version + venv | `.py`, `requirements.txt`, `pyproject.toml`, `.venv/` | `Snake` (custom) oder Emoji 🐍 |
| `rust` | Rust Version | `Cargo.toml`, `.rs` Dateien | `Cog` (custom) oder 🦀 |
| `golang` | Go Version | `go.mod`, `.go` Dateien | Custom Go icon |
| `java` | Java Version | `pom.xml`, `.java`, `build.gradle` | `Coffee` |
| `ruby` | Ruby Version | `Gemfile`, `.rb` Dateien | `Gem` |
| `php` | PHP Version | `composer.json`, `.php` | Custom |
| `swift` | Swift Version | `Package.swift`, `.swift` | `Bird` (custom) |
| `deno` | Deno Version | `deno.json`, `deno.lock` | Custom |
| `bun` | Bun Version | `bun.lockb`, `bunfig.toml` | Custom |
| `dotnet` | .NET Version | `.csproj`, `.fsproj`, `.sln` | Custom |
| `elixir` | Elixir Version | `mix.exs`, `.ex` | `Droplet` |
| `dart` | Dart Version | `pubspec.yaml`, `.dart` | `Target` |
| `kotlin` | Kotlin Version | `.kt`, `build.gradle.kts` | Custom |
| `scala` | Scala Version | `build.sbt`, `.scala` | Custom |
| `zig` | Zig Version | `build.zig`, `.zig` | `Zap` |
| `lua` | Lua Version | `.lua`, `init.lua` | `Moon` |
| `julia` | Julia Version | `Project.toml`, `.jl` | Custom |
| `nim` | Nim Version | `.nim`, `*.nimble` | `Crown` |
| `haskell` | Haskell/Stack Version | `.hs`, `stack.yaml`, `*.cabal` | Custom λ |
| `erlang` | Erlang Version | `rebar.config`, `.erl` | Custom |
| `crystal` | Crystal Version | `shard.yml`, `.cr` | Custom |
| `ocaml` | OCaml Version | `.ml`, `dune-project` | Custom 🐫 |
| `perl` | Perl Version | `.pl`, `Makefile.PL` | Custom |
| `vlang` | V Version | `v.mod`, `.v` | Custom |

---

## Tier 3: DevOps & Cloud (hohe Priorität für Profis)

| Chip | Info | Detection | Icon |
|------|------|-----------|------|
| `kubernetes` | Context + Namespace | `~/.kube/config`, `$KUBECONFIG` | `Ship` (oder ☸) |
| `docker_context` | Docker Context | `$DOCKER_CONTEXT`, `~/.docker/config.json` | `Container` |
| `aws` | AWS Profile + Region | `$AWS_PROFILE`, `~/.aws/config` | `Cloud` |
| `gcloud` | GCP Project + Region | `$CLOUDSDK_CONFIG`, `~/.config/gcloud/` | `Cloud` |
| `azure` | Azure Subscription | `~/.azure/` | `Cloud` |
| `terraform` | Terraform Workspace | `.terraform/`, `*.tf` | `Layers` |
| `helm` | Helm Version | `Chart.yaml` | `Anchor` |
| `pulumi` | Pulumi Stack | `Pulumi.yaml` | Custom |
| `vagrant` | Vagrant Environment | `Vagrantfile` | Custom |
| `openstack` | OpenStack Environment | `$OS_CLOUD` | Custom |

---

## Tier 4: Build Tools & Package Manager

| Chip | Info | Detection | Icon |
|------|------|-----------|------|
| `package` | Package Version | `package.json`, `Cargo.toml`, `pyproject.toml` | `Package` |
| `cmake` | CMake Version | `CMakeLists.txt` | `Triangle` |
| `gradle` | Gradle Version | `build.gradle`, `gradlew` | Custom |
| `maven` | Maven Version | `pom.xml` | Custom |
| `meson` | Meson Version | `meson.build` | `Settings` |
| `buf` | Buf Version | `buf.yaml` | Custom |

---

## Tier 5: Environment & System

| Chip | Info | Detection | Icon |
|------|------|-----------|------|
| `battery` | Batterie-Level | System API | `Battery` / `BatteryCharging` |
| `memory_usage` | RAM Nutzung | System API | `MemoryStick` |
| `os` | Betriebssystem | System API | `Apple` / `Penguin` / `Monitor` |
| `localip` | Lokale IP | Network Interface | `Network` |
| `cmd_duration` | Command-Dauer | Letzter Command | `Timer` |
| `jobs` | Background Jobs | Shell-Jobs | `Layers` |
| `shlvl` | Shell Nesting Level | `$SHLVL` | `CornerDownRight` |
| `sudo` | Sudo aktiv | `sudo -n true` | `Shield` |
| `status` | Exit Code | Letzter Command | `AlertCircle` |

---

## Tier 6: VCS (über Git hinaus)

| Chip | Info | Detection | Icon |
|------|------|-----------|------|
| `git_commit` | Commit Hash (detached HEAD) | `.git/HEAD` | `GitCommit` |
| `git_state` | Merge/Rebase/Cherry-Pick | `.git/MERGE_HEAD` etc. | `GitMerge` |
| `fossil_branch` | Fossil VCS Branch | `.fslckout` | Custom |
| `hg_branch` | Mercurial Branch | `.hg/` | Custom |
| `pijul_channel` | Pijul Channel | `.pijul/` | Custom |

---

## Tier 7: Environment Managers

| Chip | Info | Detection | Icon |
|------|------|-----------|------|
| `conda` | Conda Environment | `$CONDA_DEFAULT_ENV` | Custom |
| `nix_shell` | Nix Shell | `$IN_NIX_SHELL` | `Snowflake` |
| `guix_shell` | Guix Shell | `$GUIX_ENVIRONMENT` | Custom |
| `direnv` | direnv Status | `.envrc`, `$DIRENV_DIR` | Custom |
| `container` | Container Runtime | `$container`, `/.dockerenv` | `Box` |
| `singularity` | Singularity Container | `$SINGULARITY_NAME` | Custom |
| `spack` | Spack Environment | `$SPACK_ENV` | Custom |

---

## Icon-Strategie

| Quelle | Anzahl | Abdeckung |
|--------|--------|-----------|
| **Lucide** (bereits integriert) | ~1500 | ~60% der Module |
| **Simple Icons** (Markennamen: Node, Go, Docker, AWS...) | ~3000 | ~35% (Sprachen, Cloud) |
| **Custom SVG** | ~10 | ~5% (Raijin-spezifisch) |

Simple Icons (`simple-icons.org`) liefert die markenspezifischen Icons die Lucide nicht hat: Node.js, Python, Go, Docker, AWS, GCP, Kubernetes, Terraform, etc. MIT-lizenziert.

**Integration:** SVG-Dateien in `inazuma-component-assets` einbetten, `IconName` Enum erweitern.

---

## Konfigurations-API

```toml
# ~/.config/raijin/config.toml

[chips]
# Reihenfolge der Chips (links nach rechts)
layout = ["os", "username", "hostname", "directory", "git_branch", "git_status", "nodejs", "rust", "python", "kubernetes", "docker", "cmd_duration", "time", "shell"]

# Globale Einstellungen
[chips.defaults]
show_icon = true
show_label = true

# Pro-Chip Konfiguration
[chips.kubernetes]
disabled = false
color = "#326CE5"
# Nur bestimmte Contexts zeigen
contexts = ["production", "staging"]

[chips.nodejs]
disabled = false
# Version-Format: "major", "major.minor", "full"
version_format = "major"

[chips.python]
disabled = false
# Virtualenv-Name zeigen
show_virtualenv = true

[chips.aws]
disabled = false
# Credential Expiration warnen
warn_expiring = true
```

---

## Implementierungs-Reihenfolge

1. **ChipProvider Trait + Registry** — Abstraktes System für dynamische Chip-Provider
2. **Refactor bestehende Chips** — User, Host, Dir, Git als ChipProvider
3. **Detection Engine** — CWD scannen für Dateien/Ordner (cached, debounced)
4. **Sprachen-Chips (Tier 2)** — Node, Python, Rust, Go als erste
5. **DevOps-Chips (Tier 3)** — Kubernetes, Docker, AWS
6. **Konfigurations-System** — TOML Layout + Per-Chip Config
7. **Icon-Erweiterung** — Simple Icons integrieren
8. **Rest (Tier 4-7)** — Inkrementell hinzufügen

---

## Zählung

| Tier | Anzahl | Beschreibung |
|------|--------|-------------|
| Tier 1 (Core) | 7 | Bereits vorhanden, refactoren |
| Tier 2 (Sprachen) | 25 | Höchste Nutzer-Nachfrage |
| Tier 3 (DevOps) | 10 | Professionelle User |
| Tier 4 (Build) | 6 | Nische |
| Tier 5 (System) | 9 | Nützlich |
| Tier 6 (VCS) | 5 | Git-Alternativen |
| Tier 7 (Env) | 7 | Nische |
| **Total** | **69** | Starship-Parity + eigene |

---

*Wenn alle 69 Chip-Provider implementiert sind, braucht kein Raijin-User mehr Starship.*
