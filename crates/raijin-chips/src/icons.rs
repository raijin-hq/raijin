use raijin_ui::IconName;

/// Map a provider's icon string to an IconName enum variant.
///
/// Uses dedicated Simple Icons (si_*) for languages and DevOps tools,
/// Lucide icons for core chips, and generic fallbacks where needed.
pub fn icon_name_from_str(name: &str) -> Option<IconName> {
    match name {
        // Tier 1: Core — Lucide icons
        "Folder" => Some(IconName::Folder),
        "CountdownTimer" | "Timer" => Some(IconName::CountdownTimer),
        "GitBranch" => Some(IconName::GitBranch),
        "FileDiff" => Some(IconName::FileDiff),
        "GitCommit" => Some(IconName::GitCommit),
        "GitMerge" => Some(IconName::GitMergeConflict),

        // Tier 2: Languages — Simple Icons
        "Hexagon" | "Nodejs" => Some(IconName::SiNodejs),
        "Snake" | "Python" => Some(IconName::SiPython),
        "Cog" | "Rust" => Some(IconName::SiRust),
        "Go" => Some(IconName::SiGo),
        "Coffee" | "Java" => Some(IconName::SiJava),
        "Gem" | "Ruby" => Some(IconName::SiRuby),
        "Php" => Some(IconName::SiPhp),
        "Bird" | "Swift" => Some(IconName::SiSwift),
        "Deno" => Some(IconName::SiDeno),
        "Bun" => Some(IconName::SiBun),
        "Dotnet" => Some(IconName::SiDotnet),
        "Droplet" | "Elixir" => Some(IconName::SiElixir),
        "Target" | "Dart" => Some(IconName::SiDart),
        "Kotlin" => Some(IconName::SiKotlin),
        "Scala" => Some(IconName::SiScala),
        "Zap" | "Zig" => Some(IconName::SiZig),
        "Moon" | "Lua" => Some(IconName::SiLua),
        "Julia" => Some(IconName::SiJulia),
        "Crown" | "Nim" => Some(IconName::SiNim),
        "Haskell" => Some(IconName::SiHaskell),
        "Erlang" => Some(IconName::SiErlang),
        "Crystal" => Some(IconName::SiCrystal),
        "Ocaml" => Some(IconName::SiOcaml),
        "Perl" => Some(IconName::SiPerl),
        "V" | "Vlang" => Some(IconName::SiV),

        // New languages
        "C" => Some(IconName::SiC),
        "Cpp" | "C++" => Some(IconName::SiCpp),
        "Cobol" => Some(IconName::SiCobol),
        "Daml" => Some(IconName::SiDaml),
        "Elm" => Some(IconName::SiElm),
        "Fennel" => Some(IconName::SiFennel),
        "Fortran" => Some(IconName::SiFortran),
        "Gleam" => Some(IconName::SiGleam),
        "Haxe" => Some(IconName::SiHaxe),
        "Mojo" => Some(IconName::SiMojo),
        "Odin" => Some(IconName::SiOdin),
        "Opa" => Some(IconName::SiOpa),
        "Purescript" => Some(IconName::SiPurescript),
        "Quarto" => Some(IconName::SiQuarto),
        "Raku" => Some(IconName::SiRaku),
        "Red" => Some(IconName::SiRed),
        "Rlang" | "R" => Some(IconName::SiRlang),
        "Solidity" => Some(IconName::SiSolidity),
        "Typst" => Some(IconName::SiTypst),

        // Tier 3: DevOps & Cloud — Simple Icons
        "Ship" | "Kubernetes" => Some(IconName::SiKubernetes),
        "Container" | "Docker" => Some(IconName::SiDocker),
        "Cloud" | "Aws" => Some(IconName::SiAmazonaws),
        "Gcloud" | "GoogleCloud" => Some(IconName::SiGooglecloud),
        "Azure" => Some(IconName::SiMicrosoftazure),
        "Terraform" => Some(IconName::SiTerraform),
        "Anchor" | "Helm" => Some(IconName::SiHelm),
        "Pulumi" => Some(IconName::SiPulumi),
        "Vagrant" => Some(IconName::SiVagrant),
        "Openstack" => Some(IconName::SiOpenstack),

        // Tier 4: Build Tools
        "Layers" => Some(IconName::Blocks),
        "Package" => Some(IconName::Box),
        "Triangle" | "Cmake" => Some(IconName::SiCmake),
        "Gradle" => Some(IconName::SiGradle),
        "Maven" => Some(IconName::SiApachemaven),
        "Meson" => Some(IconName::SiMeson),

        // Tier 5: System — Lucide fallbacks
        "Battery" | "BatteryCharging" => Some(IconName::Power),
        "MemoryStick" => Some(IconName::Server),
        "Monitor" => Some(IconName::Screen),
        "Network" => Some(IconName::SignalHigh),
        "CornerDownRight" => Some(IconName::ArrowDownRight),
        "Shield" => Some(IconName::LockOutlined),
        "AlertCircle" => Some(IconName::Warning),
        "Settings" => Some(IconName::Settings),

        // Tier 7: Env Managers — Simple Icons where available
        "Snowflake" | "Nix" => Some(IconName::SiNixos),
        "Anaconda" | "Conda" => Some(IconName::SiAnaconda),
        "Box" => Some(IconName::Box),

        // Generic fallback
        "Code" => Some(IconName::Code),

        _ => None,
    }
}
