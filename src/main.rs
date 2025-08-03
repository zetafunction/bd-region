mod bluray;

use clap::{Args, Parser, Subcommand};
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;

use crate::bluray::{BluRay, MovieObject, NavigationCommand, Operand, OperandCount, Region};

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
    /// What region to overwrite use of PSR 20 with.
    region: Region,
    #[arg(long, value_parser=parse_country)]
    /// What country to overwrite use of PSR 19 with. This should be an ISO 3166-1 alpha-2 code
    /// specified in uppercase letters, e.g. "US" or "JP".
    country: String,
    #[arg(long)]
    /// Any additional navigation commands to patch out with a nop. A location consists of a
    /// 0-based movie object index, a comma, and a 0-based navigation command index.
    nop_patch: Vec<NavigationCommandLocator>,
    /// Where to save the new MovieObject.bdmv file.
    output_path: PathBuf,
}

fn parse_country(s: &str) -> Result<String, String> {
    if s.len() == 2 && s.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(s.to_string())
    } else {
        Err("country must be an uppercase ISO 3166-1 alpha-2 code, e.g. 'US' or 'JP'".to_string())
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct NavigationCommandLocator {
    movie_object_index: u16,
    navigation_command_index: u16,
}

#[derive(Debug, Error)]
enum NavigationCommandLocatorParseError {
    #[error("missing comma")]
    MissingComma,
    #[error("invalid movie object index")]
    InvalidMovieObjectIndex(#[source] std::num::ParseIntError),
    #[error("invalid navigation command index")]
    InvalidNavigationCommandIndex(#[source] std::num::ParseIntError),
}

impl std::str::FromStr for NavigationCommandLocator {
    type Err = NavigationCommandLocatorParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first, second) = s.split_once(',').ok_or(Self::Err::MissingComma)?;
        let movie_object_index = first.parse().map_err(Self::Err::InvalidMovieObjectIndex)?;
        let navigation_command_index = second
            .parse()
            .map_err(Self::Err::InvalidNavigationCommandIndex)?;
        Ok(NavigationCommandLocator {
            movie_object_index,
            navigation_command_index,
        })
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let bluray = BluRay::open(&cli.path)?;

    match cli.command {
        Command::Dump => dump(bluray),
        Command::Test => test(bluray),
        Command::Remove(args) => args.exec(bluray)?,
    };
    Ok(())
}

fn dump(bluray: BluRay) {
    println!(
        "movie object header: {:02x?}",
        bluray.movie_object_file.header
    );
    println!(
        "movie objects byte size: {}",
        bluray.movie_object_file.movie_objects.byte_len
    );
    for (i, movie_object) in (0..).zip(bluray.movie_object_file.movie_objects.movie_objects.iter())
    {
        for (j, navigation_command) in (0..).zip(movie_object.navigation_commands.iter()) {
            println!("movie object #{i} navigation command #{j} {navigation_command:?}");
        }
    }
    println!(
        "movie object extension data: {:02x?}",
        bluray.movie_object_file.extension_data
    );
}

fn test(bluray: BluRay) {
    for (i, movie_object) in (0..).zip(bluray.movie_object_file.movie_objects.movie_objects.iter())
    {
        for (j, navigation_command) in (0..).zip(movie_object.navigation_commands.iter()) {
            match (
                &navigation_command.operand_count,
                &navigation_command.destination,
                &navigation_command.source,
            ) {
                (OperandCount::DestinationAndSource, _, &Operand::Psr(source))
                    if source == 19 || source == 20 =>
                {
                    println!("movie object #{i} navigation command #{j} {navigation_command:?}");
                }
                // PSR19 and PSR20 are read-only, so they should only appear as source operands.
                // Nonetheless, log out any other instance, even if it's unusual.
                (OperandCount::DestinationAndSource, &Operand::Psr(dest), _)
                    if dest == 19 || dest == 20 =>
                {
                    println!("UNEXPECTED: movie object #{i} navigation command #{j} {navigation_command:?}");
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

impl RemoveArgs {
    fn exec(self, mut bluray: BluRay) -> anyhow::Result<()> {
        let nop_patches: HashSet<_> = self.nop_patch.into_iter().collect();
        // TODO: A better design would avoid re-parsing this from the raw bytes.
        const NOP_COMMAND_BYTES: [u8; 12] = [0; 12];
        bluray.movie_object_file.movie_objects.movie_objects = (0..)
            .zip(bluray.movie_object_file.movie_objects.movie_objects)
            .map(
                |(
                    movie_object_index,
                    MovieObject {
                        header,
                        navigation_commands,
                    },
                )| {
                    let navigation_commands = (0..)
                        .zip(navigation_commands)
                        .map(|(navigation_command_index, command)| {
                            if nop_patches.contains(&NavigationCommandLocator {
                                movie_object_index,
                                navigation_command_index,
                            }) {
                                return NavigationCommand::from_bytes(&NOP_COMMAND_BYTES).unwrap();
                            }
                            // Both PSR19 (country) and PSR20 (region) are read-only, so no need to
                            // check the destination operand at all.
                            match (&command.operand_count, &command.source) {
                                (OperandCount::DestinationAndSource, &Operand::Psr(19)) => {
                                    let mut raw_bytes = command.raw_bytes;
                                    // Set the "source is immediate" flag
                                    raw_bytes[1] |= 1 << 6;
                                    raw_bytes[10..12].copy_from_slice(self.country.as_bytes());
                                    NavigationCommand::from_bytes(&raw_bytes).unwrap()
                                }
                                (OperandCount::DestinationAndSource, &Operand::Psr(20)) => {
                                    let mut raw_bytes = command.raw_bytes;
                                    // Set the "source is immediate" flag
                                    raw_bytes[1] |= 1 << 6;
                                    raw_bytes[8..12]
                                        .copy_from_slice(&(self.region as u32).to_be_bytes());
                                    NavigationCommand::from_bytes(&raw_bytes).unwrap()
                                }
                                _ => command,
                            }
                        })
                        .collect();
                    MovieObject {
                        header,
                        navigation_commands,
                    }
                },
            )
            .collect();

        let mut out = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.output_path)?;
        out.write_all(&bluray.movie_object_file.serialize())?;
        Ok(())
    }
}
