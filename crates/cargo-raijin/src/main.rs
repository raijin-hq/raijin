mod build;
mod bundle;
mod dev;
mod icon;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Cargo subcommand wrapper — `cargo raijin <command>`
#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    /// Raijin development tools
    Raijin(RaijinCli),
}

#[derive(Parser)]
#[command(version, about = "Raijin development tools")]
struct RaijinCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in development mode with hot-reload
    Dev {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build the application bundle
    Build {
        /// Build in debug mode (default is release)
        #[arg(long)]
        debug: bool,
    },
    /// Compile .icon to Assets.car via actool
    Icon {
        /// Path to .icon file (default: crates/raijin-app/assets/rajin.icon)
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    let Cargo::Raijin(cli) = Cargo::parse();
    let workspace_root = find_workspace_root()?;

    match cli.command {
        Commands::Dev { release } => dev::run(&workspace_root, release),
        Commands::Build { debug } => build::run(&workspace_root, !debug),
        Commands::Icon { path } => icon::run(&workspace_root, path.as_deref()),
    }
}

/// Walk up from CWD to find the workspace root (Cargo.toml with [workspace]).
fn find_workspace_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir().context("failed to get current directory")?;

    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)
                .with_context(|| format!("failed to read {}", cargo_toml.display()))?;
            if content.contains("[workspace]") {
                return Ok(dir);
            }
        }

        if !dir.pop() {
            bail!("could not find workspace root (no Cargo.toml with [workspace] found)");
        }
    }
}
