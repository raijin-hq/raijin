// Tier 1: Core
mod username;
mod hostname;
mod directory;
mod time_chip;
mod shell;
mod git_branch;
mod git_status;

// Tier 2: Languages & Runtimes
pub(crate) mod cc;
mod c;
mod cobol;
mod cpp;
mod nodejs;
mod python;
mod rust_lang;
mod golang;
mod java;
mod ruby;
mod php;
mod swift;
mod deno;
mod bun;
mod dotnet;
mod elixir;
mod dart;
mod kotlin;
mod scala;
mod zig;
mod lua;
mod julia;
mod nim;
mod haskell;
mod erlang;
mod crystal;
mod ocaml;
mod perl;
mod vlang;
mod quarto;
mod raku;
mod red;
mod rlang;
mod solidity;
mod typst;
mod xmake;
mod mojo;
mod odin;
mod purescript;
mod elm;
mod fennel;
mod fortran;
mod gleam;
mod haxe;
mod daml;

// Tier 3: DevOps & Cloud
mod kubernetes;
mod docker_context;
mod aws;
mod gcloud;
mod azure;
mod terraform;
mod helm;
mod pulumi;
mod vagrant;
mod openstack;
mod nats;

// Tier 4: Build Tools & Package Manager
mod package;
mod cmake;
mod gradle;
mod maven;
mod meson;
mod buf;
mod mise;
mod pixi;
mod opa;

// Tier 5: Environment & System
pub mod battery;
mod memory_usage;
mod os_info;
mod localip;
mod cmd_duration;
mod jobs;
mod shlvl;
mod sudo;
mod status;
mod os;
mod netns;

// Tier 6: VCS Beyond Git
mod git_commit;
mod git_state;
mod fossil_branch;
mod fossil_metrics;
mod git_metrics;
mod hg_branch;
mod hg_state;
mod pijul_channel;

// Tier 7: Environment Managers
mod conda;
mod nix_shell;
mod guix_shell;
mod direnv;
mod container;
mod singularity;
mod spack;
mod vcsh;

use crate::registry::ChipRegistry;

/// Register all 69+ chip providers with the registry.
pub fn register_all(registry: &mut ChipRegistry) {
    // Tier 1: Core (always visible)
    registry.register(username::UsernameProvider);
    registry.register(hostname::HostnameProvider);
    registry.register(directory::DirectoryProvider);
    registry.register(time_chip::TimeProvider);
    registry.register(shell::ShellProvider);
    registry.register(git_branch::GitBranchProvider);
    registry.register(git_status::GitStatusProvider);

    // Tier 2: Languages & Runtimes
    registry.register(c::CProvider);
    registry.register(cobol::CobolProvider);
    registry.register(cpp::CppProvider);
    registry.register(nodejs::NodejsProvider);
    registry.register(python::PythonProvider);
    registry.register(rust_lang::RustProvider);
    registry.register(golang::GolangProvider);
    registry.register(java::JavaProvider);
    registry.register(ruby::RubyProvider);
    registry.register(php::PhpProvider);
    registry.register(swift::SwiftProvider);
    registry.register(deno::DenoProvider);
    registry.register(bun::BunProvider);
    registry.register(dotnet::DotnetProvider);
    registry.register(elixir::ElixirProvider);
    registry.register(dart::DartProvider);
    registry.register(kotlin::KotlinProvider);
    registry.register(scala::ScalaProvider);
    registry.register(zig::ZigProvider);
    registry.register(lua::LuaProvider);
    registry.register(julia::JuliaProvider);
    registry.register(nim::NimProvider);
    registry.register(haskell::HaskellProvider);
    registry.register(erlang::ErlangProvider);
    registry.register(crystal::CrystalProvider);
    registry.register(ocaml::OcamlProvider);
    registry.register(perl::PerlProvider);
    registry.register(vlang::VlangProvider);
    registry.register(quarto::QuartoProvider);
    registry.register(raku::RakuProvider);
    registry.register(red::RedProvider);
    registry.register(rlang::RlangProvider);
    registry.register(solidity::SolidityProvider);
    registry.register(typst::TypstProvider);
    registry.register(xmake::XmakeProvider);
    registry.register(mojo::MojoProvider);
    registry.register(odin::OdinProvider);
    registry.register(purescript::PurescriptProvider);
    registry.register(elm::ElmProvider);
    registry.register(fennel::FennelProvider);
    registry.register(fortran::FortranProvider);
    registry.register(gleam::GleamProvider);
    registry.register(haxe::HaxeProvider);
    registry.register(daml::DamlProvider);

    // Tier 3: DevOps & Cloud
    registry.register(kubernetes::KubernetesProvider);
    registry.register(docker_context::DockerContextProvider);
    registry.register(aws::AwsProvider);
    registry.register(gcloud::GcloudProvider);
    registry.register(azure::AzureProvider);
    registry.register(terraform::TerraformProvider);
    registry.register(helm::HelmProvider);
    registry.register(pulumi::PulumiProvider);
    registry.register(vagrant::VagrantProvider);
    registry.register(openstack::OpenstackProvider);
    registry.register(nats::NatsProvider);

    // Tier 4: Build Tools
        registry.register(package::PackageProvider);
    registry.register(cmake::CmakeProvider);
    registry.register(gradle::GradleProvider);
    registry.register(maven::MavenProvider);
    registry.register(meson::MesonProvider);
    registry.register(buf::BufProvider);
    registry.register(mise::MiseProvider);
    registry.register(pixi::PixiProvider);
    registry.register(opa::OpaProvider);

    // Tier 5: Environment & System
    registry.register(battery::BatteryProvider);
    registry.register(memory_usage::MemoryUsageProvider);
    registry.register(os_info::OsInfoProvider);
    registry.register(localip::LocalipProvider);
    registry.register(cmd_duration::CmdDurationProvider);
    registry.register(jobs::JobsProvider);
    registry.register(shlvl::ShlvlProvider);
    registry.register(sudo::SudoProvider);
    registry.register(status::StatusProvider);
    registry.register(os::OsProvider);
    registry.register(netns::NetnsProvider);

    // Tier 6: VCS Beyond Git
    registry.register(git_commit::GitCommitProvider);
    registry.register(git_state::GitStateProvider);
    registry.register(fossil_branch::FossilBranchProvider);
    registry.register(fossil_metrics::FossilMetricsProvider);
    registry.register(git_metrics::GitMetricsProvider);
    registry.register(hg_branch::HgBranchProvider);
    registry.register(hg_state::HgStateProvider);
    registry.register(pijul_channel::PijulChannelProvider);

    // Tier 7: Environment Managers
    registry.register(conda::CondaProvider);
    registry.register(nix_shell::NixShellProvider);
    registry.register(guix_shell::GuixShellProvider);
    registry.register(direnv::DirenvProvider);
    registry.register(container::ContainerProvider);
    registry.register(singularity::SingularityProvider);
    registry.register(spack::SpackProvider);
    registry.register(vcsh::VcshProvider);
}
