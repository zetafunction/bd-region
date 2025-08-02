mod bluray;

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::bluray::{BluRay, Operand, OperandCount, Region};

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
    /// For debugging.
    Dump,
    /// Test if a disc is region or country locked.
    Test,
    /// Remove region checks from a disc.
    Remove(RemoveArgs),
}

#[derive(Args)]
struct RemoveArgs {
    #[arg(long)]
    region: Region,
    #[arg(long)]
    country: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let bluray = BluRay::open(&cli.path)?;

    match cli.command {
        Command::Dump => dump(bluray),
        Command::Test => test(bluray),
        Command::Remove(_remove_args) => (),
    };
    Ok(())
}

fn dump(bluray: BluRay) {
    for (i, movie_object) in (0..).zip(bluray.movie_objects.iter()) {
        for (j, navigation_command) in (0..).zip(movie_object.navigation_commands.iter()) {
            println!("movie object #{i} navigation command #{j} {navigation_command:?}");
        }
    }
}

fn test(bluray: BluRay) {
    for (i, movie_object) in (0..).zip(bluray.movie_objects.iter()) {
        for (j, navigation_command) in (0..).zip(movie_object.navigation_commands.iter()) {
            match (
                &navigation_command.operand_count,
                &navigation_command.destination,
                &navigation_command.source,
            ) {
                (OperandCount::DestinationOnly, &Operand::Psr(dest), _)
                    if dest == 19 || dest == 20 =>
                {
                    println!("movie object #{i} navigation command #{j} {navigation_command:?}");
                }
                (OperandCount::DestinationAndSource, &Operand::Psr(dest), _)
                    if dest == 19 || dest == 20 =>
                {
                    println!("movie object #{i} navigation command #{j} {navigation_command:?}");
                }
                (OperandCount::DestinationAndSource, _, &Operand::Psr(source))
                    if source == 19 || source == 20 =>
                {
                    println!("movie object #{i} navigation command #{j} {navigation_command:?}");
                }
                (_, &Operand::Psr(dest), _) if dest == 19 || dest == 20 => {
                    println!("UNEXPECTED: movie object #{i} navigation command #{j} {navigation_command:?}");
                }
                (_, &Operand::Psr(source), _) if source == 19 || source == 20 => {
                    println!("UNEXPECTED: movie object #{i} navigation command #{j} {navigation_command:?}");
                }
                (_, _, _) => continue,
            }
        }
    }
}
