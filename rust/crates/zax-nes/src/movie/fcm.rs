//! The `.fcm` movie format (FCE Ultra 0.98.x binary movies, version 2), read from its public
//! documentation (`TASVideos`, "FCM"): no code of FCE Ultra or FCEUX is used. A movie is read
//! into an [`Fm2Movie`], so that it plays the same way as an `.fm2`.
//!
//! Header (little-endian): the signature `FCM\x1a`, the version (2), a flags byte (bit 1: the
//! movie starts from reset or power on, bit 2: PAL), the frame count at `0x0c`, the rerecord
//! count at `0x10`, the length of the controller data at `0x14`, the offset of the savestate at
//! `0x18`, the offset of the controller data at `0x1c`, the ROM's MD5 at `0x20`, the emulator
//! version at `0x30`, then the ROM name and the author info, both nul-terminated UTF-8.
//!
//! The controller data is a list of updates, each a byte and 0 to 3 bytes of delta (its bits
//! 6-5): the number of frames from this update to the next one. Bit 7 set: a command (bits 4-0:
//! 0 nothing, 1 reset, 2 power cycle, 7-8 VS. System, 24-26 Famicom Disk System); clear: the
//! button (bits 2-0: A, B, Select, Start, Up, Down, Left, Right) of the gamepad (bits 4-3) is
//! toggled. A movie always holds a savestate; one that starts from it rather than from reset or
//! power on is refused, as Zaxnes cannot read FCE Ultra's states.

use std::fmt;

use super::fm2::{Fm2Frame, Fm2Header, Fm2Movie, COMMAND_HARD_RESET, COMMAND_SOFT_RESET};

/// The signature that opens an `.fcm` file.
pub const SIGNATURE: &[u8; 4] = b"FCM\x1a";
/// The size of the fixed part of the header (up to the ROM name).
const HEADER_SIZE: usize = 0x34;
/// Flags bit: the movie starts from reset or power on.
const FLAG_FROM_RESET: u8 = 0x02;
/// Flags bit: PAL timing.
const FLAG_PAL: u8 = 0x04;
/// The command codes this reader replays (the others are the VS. System's and the FDS's).
const COMMAND_NOTHING: u8 = 0;
const COMMAND_RESET: u8 = 1;
const COMMAND_POWER: u8 = 2;

/// Why an `.fcm` file was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FcmError {
    /// Not an `.fcm` file (signature), or one shorter than its header.
    NotFcm,
    /// A version other than 2.
    Version(u32),
    /// The movie starts from its savestate, not from reset or power on.
    StartsFromSavestate,
    /// The controller data lies outside the file.
    Truncated,
    /// A command of the Famicom Disk System or the VS. System, at an update.
    UnsupportedCommand {
        /// The command code.
        code: u8,
        /// The frame of the update.
        frame: usize,
    },
    /// A button of the third or fourth gamepad (Four Score).
    FourScore,
}

impl fmt::Display for FcmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFcm => f.write_str("not an fcm movie"),
            Self::Version(version) => write!(f, "fcm movie of version {version}, only 2 is read"),
            Self::StartsFromSavestate => f.write_str(
                "the movie starts from a savestate (of FCE Ultra), which is not supported",
            ),
            Self::Truncated => f.write_str("the controller data goes past the end of the file"),
            Self::UnsupportedCommand { code, frame } => write!(
                f,
                "frame {frame}: Famicom Disk System or VS. System command {code} is not supported"
            ),
            Self::FourScore => f.write_str("unsupported input device: Four Score"),
        }
    }
}

impl std::error::Error for FcmError {}

/// One update of the controller data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Update {
    /// A command code.
    Command(u8),
    /// The button `button` (0 A ... 7 Right) of gamepad `pad` (0 to 3) is toggled.
    Button { pad: u8, button: u8 },
}

/// Reads an `.fcm` file into an [`Fm2Movie`]: one frame per frame of the movie, from power on.
///
/// # Errors
/// The file is not a movie this reader replays (see [`FcmError`]).
pub fn parse(file: &[u8]) -> Result<Fm2Movie, FcmError> {
    if file.len() < HEADER_SIZE || &file[..4] != SIGNATURE {
        return Err(FcmError::NotFcm);
    }
    let word = |offset: usize| {
        u32::from_le_bytes([
            file[offset],
            file[offset + 1],
            file[offset + 2],
            file[offset + 3],
        ])
    };
    let version = word(0x04);
    if version != 2 {
        return Err(FcmError::Version(version));
    }
    let flags = file[0x08];
    let frame_count = word(0x0c) as usize;
    let rerecords = word(0x10);
    let data_length = word(0x14) as usize;
    let data_offset = word(0x1c) as usize;
    let mut rom_md5 = [0u8; 16];
    rom_md5.copy_from_slice(&file[0x20..0x30]);
    let emu_version = word(0x30);
    let (rom_name, rest) = nul_string(&file[HEADER_SIZE..]);
    let (author, _) = nul_string(rest);
    let data = data_offset
        .checked_add(data_length)
        .and_then(|end| file.get(data_offset..end))
        .ok_or(FcmError::Truncated)?;
    let updates = read_updates(data)?;
    let first_command = updates.iter().find_map(|(update, _)| match update {
        Update::Command(COMMAND_NOTHING) => None,
        other => Some(*other),
    });
    let from_power = flags & FLAG_FROM_RESET != 0
        || matches!(
            first_command,
            Some(Update::Command(COMMAND_RESET | COMMAND_POWER))
        );
    if !from_power {
        return Err(FcmError::StartsFromSavestate);
    }
    let frames = play(&updates, frame_count)?;
    let header = Fm2Header {
        version: 3,
        emu_version: Some(emu_version),
        rerecord_count: Some(u64::from(rerecords)),
        pal: flags & FLAG_PAL != 0,
        rom_filename: (!rom_name.is_empty()).then_some(rom_name),
        rom_md5: (rom_md5 != [0; 16]).then_some(rom_md5),
        guid: None,
        comments: if author.is_empty() {
            Vec::new()
        } else {
            vec![format!("author {author}")]
        },
        subtitles: Vec::new(),
    };
    Ok(Fm2Movie { header, frames })
}

/// A nul-terminated UTF-8 string at the start of `bytes`, and what follows its nul.
fn nul_string(bytes: &[u8]) -> (String, &[u8]) {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let text = String::from_utf8_lossy(&bytes[..end]).into_owned();
    (text, bytes.get(end + 1..).unwrap_or(&[]))
}

/// The updates of the controller data, each with its delta (the frames to the next update).
fn read_updates(data: &[u8]) -> Result<Vec<(Update, usize)>, FcmError> {
    let mut updates = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let byte = data[at];
        at += 1;
        let size = usize::from((byte >> 5) & 0x03);
        let delta_bytes = data.get(at..at + size).ok_or(FcmError::Truncated)?;
        at += size;
        let delta = delta_bytes
            .iter()
            .rev()
            .fold(0usize, |value, &b| (value << 8) | usize::from(b));
        let update = if byte & 0x80 != 0 {
            Update::Command(byte & 0x1f)
        } else {
            Update::Button {
                pad: (byte >> 3) & 0x03,
                button: byte & 0x07,
            }
        };
        updates.push((update, delta));
    }
    Ok(updates)
}

/// Plays the updates into frames: an update applies to the frame the movie is at, then the
/// movie moves on by its delta. `frame_count` (the header's) is the length: missing frames hold
/// the last inputs, frames past it are dropped.
fn play(updates: &[(Update, usize)], frame_count: usize) -> Result<Vec<Fm2Frame>, FcmError> {
    let mut frames: Vec<Fm2Frame> = Vec::with_capacity(frame_count);
    let mut held = 0u16;
    let mut commands = 0u8;
    for &(update, delta) in updates {
        let frame = frames.len();
        match update {
            Update::Command(COMMAND_NOTHING) => {}
            Update::Command(COMMAND_RESET) => {
                // A reset on the first frame is how a movie says "from reset": it starts from
                // power on, like the `.fm2` hard reset of the first frame.
                commands |= if frame == 0 {
                    COMMAND_HARD_RESET
                } else {
                    COMMAND_SOFT_RESET
                };
            }
            Update::Command(COMMAND_POWER) => commands |= COMMAND_HARD_RESET,
            Update::Command(code) => return Err(FcmError::UnsupportedCommand { code, frame }),
            Update::Button { pad, button } => {
                if pad > 1 {
                    return Err(FcmError::FourScore);
                }
                held ^= 1 << (u16::from(pad) * 8 + u16::from(button));
            }
        }
        for _ in 0..delta {
            if frames.len() >= frame_count {
                break;
            }
            frames.push(Fm2Frame {
                commands,
                word: held,
            });
            commands = 0;
        }
    }
    frames.resize(
        frame_count,
        Fm2Frame {
            commands: 0,
            word: held,
        },
    );
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `.fcm` file with the controller data `data`, `frames` frames long.
    fn file(flags: u8, frames: u32, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(SIGNATURE);
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&[flags, 0, 0, 0]);
        bytes.extend_from_slice(&frames.to_le_bytes());
        bytes.extend_from_slice(&7u32.to_le_bytes());
        let length = u32::try_from(data.len()).unwrap_or(0);
        bytes.extend_from_slice(&length.to_le_bytes());
        // The savestate at 0x40 (4 bytes), the controller data at 0x44.
        bytes.extend_from_slice(&0x40u32.to_le_bytes());
        bytes.extend_from_slice(&0x44u32.to_le_bytes());
        bytes.extend_from_slice(&[0xab; 16]);
        bytes.extend_from_slice(&9812u32.to_le_bytes());
        bytes.extend_from_slice(b"Game\0Me\0");
        bytes.resize(0x44, 0);
        bytes.extend_from_slice(data);
        bytes
    }

    #[test]
    fn reads_the_header_and_toggles_buttons() {
        // Power at frame 0, wait 2; A down, wait 1; Right down, wait 0; A up, wait 2.
        let data = [0xa2, 2, 0x20, 1, 0x07, 0x20, 2];
        let movie = parse(&file(0, 5, &data)).unwrap_or_default();
        assert_eq!(movie.header.rerecord_count, Some(7));
        assert_eq!(movie.header.rom_filename.as_deref(), Some("Game"));
        assert_eq!(movie.header.comments, ["author Me"]);
        assert_eq!(movie.header.rom_md5, Some([0xab; 16]));
        assert!(!movie.header.pal);
        // Frames 0-1: nothing held (the power cycle on frame 0); frame 2: A; frames 3-4: Right.
        assert_eq!(movie.words(), [0, 0, 0x01, 0x80, 0x80]);
        assert_eq!(movie.resets(), []);
    }

    #[test]
    fn deltas_are_little_endian_and_the_length_is_the_headers() {
        // Reset (from reset), delta of 0x0102 frames on two bytes; then Start toggled.
        let data = [0xc1, 0x02, 0x01, 0x03];
        let movie = parse(&file(FLAG_FROM_RESET | FLAG_PAL, 0x0104, &data)).unwrap_or_default();
        assert!(movie.header.pal);
        assert_eq!(movie.frames.len(), 0x0104);
        assert_eq!(movie.frames[0x0101].word, 0);
        assert_eq!(movie.frames[0x0102].word, 0x08);
        assert_eq!(movie.frames[0x0103].word, 0x08);
        assert_eq!(movie.resets(), []);
    }

    #[test]
    fn a_later_reset_is_a_soft_reset() {
        let data = [0xa2, 1, 0xa1, 1];
        let movie = parse(&file(0, 2, &data)).unwrap_or_default();
        assert_eq!(movie.resets(), [(1, COMMAND_SOFT_RESET)]);
    }

    #[test]
    fn refuses_what_it_cannot_replay() {
        assert_eq!(parse(b"FCM").err(), Some(FcmError::NotFcm));
        let mut v1 = file(0, 1, &[0xa2, 1]);
        v1[4] = 1;
        assert_eq!(parse(&v1).err(), Some(FcmError::Version(1)));
        // A button first, no reset or power, flag clear: from the savestate.
        assert_eq!(
            parse(&file(0, 1, &[0x20, 1])).err(),
            Some(FcmError::StartsFromSavestate)
        );
        assert_eq!(
            parse(&file(0, 2, &[0xa2, 1, 0xb8, 1])).err(),
            Some(FcmError::UnsupportedCommand { code: 24, frame: 1 })
        );
        assert_eq!(
            parse(&file(FLAG_FROM_RESET, 1, &[0x10, 1])).err(),
            Some(FcmError::FourScore)
        );
        let mut cut = file(0, 1, &[0xa2, 1]);
        cut.pop();
        assert_eq!(parse(&cut).err(), Some(FcmError::Truncated));
    }
}
