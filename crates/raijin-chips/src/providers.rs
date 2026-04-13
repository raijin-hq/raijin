// Tier 1: Core
mod username;
mod hostname;
mod directory;
mod time_chip;
mod shell;
mod git_branch;
mod git_status;

// Tier 2: Languages & Runtimes
mod nodejs;
// mod python; // TODO: cp from starship
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

// Tier 4: Build Tools & Package Manager
mod package;
mod cmake;
mod gradle;
mod maven;
mod meson;
mod buf;

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

// Tier 6: VCS Beyond Git
mod git_commit;
mod git_state;
mod fossil_branch;
mod hg_branch;
mod pijul_channel;

// Tier 7: Environment Managers
// mod conda; // TODO: cp from starship
// mod nix_shell; // TODO: cp from starship
// mod guix_shell; // TODO: cp from starship
// mod direnv; // TODO: cp from starship
mod container;
// mod singularity; // TODO: cp from starship
// mod spack; // TODO: cp from starship

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
    registry.register(nodejs::NodejsProvider);
    // registry.register(python::PythonProvider);
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

    // Tier 4: Build Tools
        registry.register(package::PackageProvider);
    registry.register(cmake::CmakeProvider);
    registry.register(gradle::GradleProvider);
    registry.register(maven::MavenProvider);
    registry.register(meson::MesonProvider);
    registry.register(buf::BufProvider);

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

    // Tier 6: VCS Beyond Git
    registry.register(git_commit::GitCommitProvider);
    registry.register(git_state::GitStateProvider);
    registry.register(fossil_branch::FossilBranchProvider);
    registry.register(hg_branch::HgBranchProvider);
    registry.register(pijul_channel::PijulChannelProvider);

    // Tier 7: Environment Managers
    // registry.register(conda::CondaProvider);
    // registry.register(nix_shell::NixShellProvider);
    // registry.register(guix_shell::GuixShellProvider);
    // registry.register(direnv::DirenvProvider);
    registry.register(container::ContainerProvider);
    // registry.register(singularity::SingularityProvider);
    // registry.register(spack::SpackProvider);
}
