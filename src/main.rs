mod bluray;

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
/// Utility to test or remove region checks from Blu-Ray disc. Blu-Ray discs can perform region
/// checks in MovieObject.bdmv or in BD-J; this utility only handles the former.
struct Cli {
    /// Path to the disc, i.e. the directory that contains the top-level BDMV and CERTIFICATE
    /// directories.
    path: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Test if a disc is region-locked, and if so, to what region.
    Test,
    /// Remove region checks from a disc.
    Remove(RemoveArgs),
}

#[derive(Args)]
struct RemoveArgs {
    #[arg(long)]
    region: bluray::Region,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let _bluray = bluray::BluRay::open(&cli.path)?;
    Ok(())
}
