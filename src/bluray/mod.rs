use clap::ValueEnum;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

const MOVIE_OBJECT_PATH: &str = "BDMV/MovieObject.bdmv";
const MOVIE_OBJECT_HEADER: &[u8] = b"MOBJ0200";

/// Blu-Ray media region codes
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Region {
    /// North America, South America, U.S. Territories, Japan, South Korea, Taiwan, and other areas of
    /// Southeast Asia.
    A,
    /// Europe, Africa, Middle East, Australia, and New Zealand.
    B,
    /// Asia (except for Japan, Korea, Taiwan, and other areas of Southeast Asia)
    C,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BluRay {
    path: PathBuf,
    pub movie_objects: Vec<MovieObject>,
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("IO error for {0}")]
    IoError(&'static str, #[source] std::io::Error),
    #[error("invalid MovieObject.bdmv: header too short")]
    NoMagicBytes,
    #[error("invalid MovieObject.bdmv header: {0:#04x?}")]
    BadMagicBytes([u8; 8]),
    #[error("invalid MovieObject.bdmv header: no extension start address")]
    NoExtensionStartAddress,
    #[error("invalid MovieObject.bdmv header: no reserved bytes")]
    NoReservedBytes,
    #[error("invalid MovieObject.bdmv header: no length for movie objects")]
    MovieObjectsNoLength,
    #[error("invalid MovieObject.bdmv header: no reserved bytes for movie objects")]
    MovieObjectsNoReservedBytes,
    #[error("invalid MovieObject.bdmv header: no count of movie objects")]
    MovieObjectsNoCount,
    #[error("invalid MovieObject.bdmv: movie object missing flags")]
    MovieObjectNoFlags,
    #[error("invalid MovieObject.bdmv: movie object missing navigation commands count")]
    NavigationCommandsNoCount,
    #[error("invalid MovieObject.bdmv: movie object #{0} navigation command #{1} truncated")]
    NavigationCommandTruncated(u16, u16),
    #[error(
        "invalid MovieObject.bdmv: movie object #{0} navigation command #{1} could not be decoded: {2:#04x?}"
    )]
    NavigationCommandInvalid(u16, u16, [u8; 12]),
    #[error("Invalid MovieObject.bdmv: movie object #{0} navigation command #{1} has bad operand count {2:#04x}")]
    NavigationCommandBadOperandCount(u16, u16, u8),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MovieObject {
    resume_intention: bool,
    menu_call_mask: bool,
    title_search_mask: bool,
    pub navigation_commands: Vec<NavigationCommand>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct NavigationCommand {
    pub command: Command,
    pub operand_count: OperandCount,
    pub destination: Operand,
    pub source: Operand,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Command {
    Branch(Branch),
    Compare(Compare),
    Set(Set),
}

#[derive(Debug)]
pub enum Branch {
    Nop,
    GoTo,
    Break,
    JumpObject,
    JumpTitle,
    CallObject,
    CallTitle,
    Resume,
    PlayList,
    PlayItem,
    PlayMark,
    Terminate,
    LinkItem,
    LinkMark,
}

#[derive(Debug)]
pub enum Compare {
    Bc,
    Eq,
    Ne,
    Ge,
    Gt,
    Le,
    Lt,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Set {
    Move,
    Swap,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Rnd,
    And,
    Or,
    Xor,
    Bitset,
    Bitclr,
    ShiftLeft,
    ShiftRight,
    SetStream,
    SetNVTimer,
    ButtonPage,
    EnableButton,
    DisableButton,
    SetSecondaryStream,
    PopupOff,
    StillOn,
    StillOff,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum OperandCount {
    None,
    DestinationOnly,
    DestinationAndSource,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Operand {
    Immediate(u32),
    /// A general-purpose register. Valid values are 0 to 4095, inclusive.
    ///
    /// 0-999: unreserved
    /// 1000-1999: current audio, subtitle, and chapter number for each playlist
    /// 2000-3999: current play time for resume feature for each playlist
    /// 4001: sound FX on or off
    /// 4002: reserved for future use
    /// 4003: 3D mode
    /// 4004: reserved for future use
    /// 4005: "top menu" pressed flag
    /// 4006-4090: unreserved
    /// 4091-4095: reserved for BDS Java code
    Gpr(u16),
    /// Player-specific registers. Valid values are 0 to 127, inclusive.
    /// 0: interactive graphics stream number (read/write)
    /// 1: primary audio stream number (read/write)
    /// 2: PG TextST and PiP PG TextST stream numbers (read/write)
    /// 3: angle number (read/write)
    /// 4: title number (read/write)
    /// 5: chapter number (read/write)
    /// 6: play list ID (read/write)
    /// 7: play item ID (read/write)
    /// 8: presentation time (read-only)
    /// 9: timer (read-only)
    /// 10: selected button ID (read/write)
    /// 11: menu page ID (read/write)
    /// 12: TextST user style number (read/write)
    /// 13: parental level (read-only)
    /// 14: secondary audio/video stream (read/write)
    /// 15: audio capability (read-only)
    /// 16: audio language (read-only)
    /// 17: PG and TextST language (read-only)
    /// 18: menu language (read-only)
    /// 19: Country read-only
    /// 20: Region read-only
    /// 21: to 28 Reserved
    /// 29: Video Capability read-only
    /// 30: TextST Capability read-only
    /// 31: Player Profile and Version read-only
    ///     0000 or 1000: no VFS capability nor BD-J network connectivity
    ///     0001: VFS capability but not BD-J network connectivity
    ///     0011: VFS capability and BD-J network connectivity
    /// 32: to 35 Reserved
    /// 36: Backup PSR4 (for resume) read/write
    /// 37: Backup PSR5 read/write
    /// 38: Backup PSR6 read/write
    /// 39: Backup PSR7 read/write
    /// 40: Backup PSR8 read-only
    /// 41: Reserved
    /// 42: Backup PSR10 read/write
    /// 43: Backup PSR11 read/write
    /// 44: Backup PSR12 read/write
    /// 45 to 47: Reserved
    /// 48 to 61: TextST capability for each language read-only
    /// 62 to 95: Reserved
    /// 96 to 111: Reserved for BD system
    /// 96: playlist indicator #1 (b31-b24), #2 (b23-b16), #3 (b15-b8), #4 (b7-b0)
    /// 97: playlist indicator #5 (b31-b24), #6 (b23-b16), reserved (b15-b0)
    Psr(u8),
    // TODO: It's possible if this isn't needed if OperandCount is observed during decoding.
    Unknown(u32),
}

impl Operand {
    fn new_register(num: u32) -> Operand {
        if (num & 0x80000000) != 0 {
            let num = num & !0x80000000;
            if num < 128 {
                Operand::Psr(num.try_into().unwrap())
            } else {
                Operand::Unknown(num)
            }
        } else if num < 4096 {
            Operand::Gpr(num.try_into().unwrap())
        } else {
            Operand::Unknown(num)
        }
    }
}

impl BluRay {
    pub fn open(path: &Path) -> Result<BluRay, OpenError> {
        let mut movie_object_file = File::open(path.join(MOVIE_OBJECT_PATH))
            .map_err(|e| OpenError::IoError(MOVIE_OBJECT_PATH, e))?;
        let mut contents = vec![];
        movie_object_file
            .read_to_end(&mut contents)
            .map_err(|e| OpenError::IoError(MOVIE_OBJECT_PATH, e))?;
        let contents = contents;
        // First 8 bytes are the magic signature.
        let (magic_bytes, remainder) = contents
            .split_first_chunk::<8>()
            .ok_or(OpenError::NoMagicBytes)?;
        if magic_bytes != MOVIE_OBJECT_HEADER {
            return Err(OpenError::BadMagicBytes(*magic_bytes));
        }
        // Next 4 bytes are the extension start address, which may be zero.
        let (_extension_start_address, remainder) = remainder
            .split_first_chunk::<4>()
            .ok_or(OpenError::NoExtensionStartAddress)?;
        // Next 28 bytes are reserved.
        let (_reserved, remainder) = remainder
            .split_first_chunk::<28>()
            .ok_or(OpenError::NoReservedBytes)?;
        let (movie_objects_length, remainder) = remainder
            .split_first_chunk::<4>()
            .ok_or(OpenError::MovieObjectsNoLength)?;
        let movie_objects_length = u32::from_be_bytes(*movie_objects_length);
        println!("movie objects length: {movie_objects_length} bytes");
        let (_reserved, remainder) = remainder
            .split_first_chunk::<4>()
            .ok_or(OpenError::MovieObjectsNoReservedBytes)?;
        let (movie_objects_count, remainder) = remainder
            .split_first_chunk::<2>()
            .ok_or(OpenError::MovieObjectsNoCount)?;
        let movie_objects_count = u16::from_be_bytes(*movie_objects_count);
        println!("movie objects count: {movie_objects_count}");
        let mut unparsed = remainder;
        let mut movie_objects = vec![];
        for i in 0..movie_objects_count {
            let (flags, remainder) = unparsed
                .split_first_chunk::<2>()
                .ok_or(OpenError::MovieObjectNoFlags)?;
            unparsed = remainder;
            let flags = u16::from_be_bytes(*flags);
            let resume_intention = (flags & (1 << 15)) != 0;
            let menu_call_mask = (flags & (1 << 14)) != 0;
            let title_search_mask = (flags & (1 << 13)) != 0;

            let (navigation_commands_count, remainder) = unparsed
                .split_first_chunk::<2>()
                .ok_or(OpenError::NavigationCommandsNoCount)?;
            unparsed = remainder;
            let navigation_commands_count = u16::from_be_bytes(*navigation_commands_count);
            println!("movie object #{i} navigation command count: {navigation_commands_count}");

            let mut navigation_commands = vec![];
            for j in 0..navigation_commands_count {
                // Each navigation command should be exactly 96 bits.
                let (bytes, remainder) = unparsed
                    .split_first_chunk::<12>()
                    .ok_or(OpenError::NavigationCommandTruncated(i, j))?;
                unparsed = remainder;

                let destination = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
                let source = u32::from_be_bytes(bytes[4..8].try_into().unwrap());

                let operand_count = (bytes[0] >> 5) & 0x7;
                let operand_count = match operand_count {
                    0 => Ok(OperandCount::None),
                    1 => Ok(OperandCount::DestinationOnly),
                    2 => Ok(OperandCount::DestinationAndSource),
                    _ => Err(OpenError::NavigationCommandBadOperandCount(
                        i,
                        j,
                        operand_count,
                    )),
                }?;
                let command_group = (bytes[0] >> 3) & 0x3;
                let command_sub_group = bytes[0] & 0x7;

                let destination_is_immediate_value = (bytes[1] & (1 << 7)) != 0;
                let source_is_immediate_value = (bytes[1] & (1 << 6)) != 0;
                let branch_option = bytes[1] & 0xf;

                let compare_option = bytes[2] & 0xf;

                let set_option = bytes[3] & 0x1f;

                let command = decode_command(
                    command_group,
                    command_sub_group,
                    branch_option,
                    compare_option,
                    set_option,
                )
                .ok_or(OpenError::NavigationCommandInvalid(i, j, *bytes))?;

                let destination = if destination_is_immediate_value {
                    Operand::Immediate(destination)
                } else {
                    Operand::new_register(destination)
                };

                let source = if source_is_immediate_value {
                    Operand::Immediate(source)
                } else {
                    Operand::new_register(source)
                };

                navigation_commands.push(NavigationCommand {
                    command,
                    operand_count,
                    destination,
                    source,
                });
            }

            movie_objects.push(MovieObject {
                resume_intention,
                menu_call_mask,
                title_search_mask,
                navigation_commands,
            });
        }
        Ok(BluRay {
            path: path.to_path_buf(),
            movie_objects,
        })
    }
}

fn decode_command(
    command_group: u8,
    command_sub_group: u8,
    branch_option: u8,
    compare_option: u8,
    set_option: u8,
) -> Option<Command> {
    Some(
        // Based on https://github.com/lw/BluRay/wiki/NavigationCommand and
        // https://forum.doom9.org/showthread.php?p=1423615
        match (
            command_group,
            command_sub_group,
            branch_option,
            compare_option,
            set_option,
        ) {
            (0, 0, 0, _, _) => Command::Branch(Branch::Nop),
            (0, 0, 1, _, _) => Command::Branch(Branch::GoTo),
            (0, 0, 2, _, _) => Command::Branch(Branch::Break),
            (0, 1, 0, _, _) => Command::Branch(Branch::JumpObject),
            (0, 1, 1, _, _) => Command::Branch(Branch::JumpTitle),
            (0, 1, 2, _, _) => Command::Branch(Branch::CallObject),
            (0, 1, 3, _, _) => Command::Branch(Branch::CallTitle),
            (0, 1, 4, _, _) => Command::Branch(Branch::Resume),
            (0, 2, 0, _, _) => Command::Branch(Branch::PlayList),
            (0, 2, 1, _, _) => Command::Branch(Branch::PlayItem),
            (0, 2, 2, _, _) => Command::Branch(Branch::PlayMark),
            (0, 2, 3, _, _) => Command::Branch(Branch::Terminate),
            (0, 2, 4, _, _) => Command::Branch(Branch::LinkItem),
            (0, 2, 5, _, _) => Command::Branch(Branch::LinkMark),
            (1, _, _, 1, _) => Command::Compare(Compare::Bc),
            (1, _, _, 2, _) => Command::Compare(Compare::Eq),
            (1, _, _, 3, _) => Command::Compare(Compare::Ne),
            (1, _, _, 4, _) => Command::Compare(Compare::Ge),
            (1, _, _, 5, _) => Command::Compare(Compare::Gt),
            (1, _, _, 6, _) => Command::Compare(Compare::Le),
            (1, _, _, 7, _) => Command::Compare(Compare::Lt),
            (2, 0, _, _, 0x1) => Command::Set(Set::Move),
            (2, 0, _, _, 0x2) => Command::Set(Set::Swap),
            (2, 0, _, _, 0x3) => Command::Set(Set::Add),
            (2, 0, _, _, 0x4) => Command::Set(Set::Sub),
            (2, 0, _, _, 0x5) => Command::Set(Set::Mul),
            (2, 0, _, _, 0x6) => Command::Set(Set::Div),
            (2, 0, _, _, 0x7) => Command::Set(Set::Mod),
            (2, 0, _, _, 0x8) => Command::Set(Set::Rnd),
            (2, 0, _, _, 0x9) => Command::Set(Set::And),
            (2, 0, _, _, 0xa) => Command::Set(Set::Or),
            (2, 0, _, _, 0xb) => Command::Set(Set::Xor),
            (2, 0, _, _, 0xc) => Command::Set(Set::Bitset),
            (2, 0, _, _, 0xd) => Command::Set(Set::Bitclr),
            (2, 0, _, _, 0xe) => Command::Set(Set::ShiftLeft),
            (2, 0, _, _, 0xf) => Command::Set(Set::ShiftRight),
            (2, 1, _, _, 0x1) => Command::Set(Set::SetStream),
            (2, 1, _, _, 0x2) => Command::Set(Set::SetNVTimer),
            (2, 1, _, _, 0x3) => Command::Set(Set::ButtonPage),
            (2, 1, _, _, 0x4) => Command::Set(Set::EnableButton),
            (2, 1, _, _, 0x5) => Command::Set(Set::DisableButton),
            (2, 1, _, _, 0x6) => Command::Set(Set::SetSecondaryStream),
            (2, 1, _, _, 0x7) => Command::Set(Set::PopupOff),
            (2, 1, _, _, 0x8) => Command::Set(Set::StillOn),
            (2, 1, _, _, 0x9) => Command::Set(Set::StillOff),
            _ => return None,
        },
    )
}
