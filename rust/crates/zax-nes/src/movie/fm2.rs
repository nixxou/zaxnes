//! The `.fm2` movie format (`FCEUX` text movies), read from its public documentation (`TASVideos`,
//! "FM2"): no code of FCEUX is used.
//!
//! A file is a header of `key value` lines, then one line per frame:
//! `|commands|port0|port1|port2|`. `commands` is a decimal bit field (1 soft reset, 2 hard reset,
//! 4 FDS disk insert, 8 FDS side select, 16 VS coin); a gamepad is 8 characters `RLDUTSBA`, each
//! released as `.` or a space and pressed as anything else; an empty field is a port with nothing
//! plugged in. Movies that start from a savestate or use the binary input encoding are refused.

use std::fmt;

/// Soft reset (the console's reset button) before the frame.
pub const COMMAND_SOFT_RESET: u8 = 1;
/// Hard reset (power cycle) before the frame.
pub const COMMAND_HARD_RESET: u8 = 2;
/// The commands of the Famicom Disk System and the VS. System, which Zaxnes does not replay.
const UNSUPPORTED_COMMANDS: u8 = 4 | 8 | 16;

/// One frame of an `.fm2` movie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fm2Frame {
    /// The commands column ([`COMMAND_SOFT_RESET`], [`COMMAND_HARD_RESET`]).
    pub commands: u8,
    /// The input word as `NesCore::run_frame` takes it: port 1 in bits 0-7 (A, B, Select, Start,
    /// Up, Down, Left, Right), port 2 in bits 8-15.
    pub word: u16,
}

/// What an `.fm2` header says.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fm2Header {
    /// `version` (3 for every FCEUX movie).
    pub version: u32,
    /// `emuVersion`.
    pub emu_version: Option<u32>,
    /// `rerecordCount`.
    pub rerecord_count: Option<u64>,
    /// `palFlag`: the movie was made on a PAL console.
    pub pal: bool,
    /// `romFilename`.
    pub rom_filename: Option<String>,
    /// `romChecksum`: the MD5 of the ROM's data after its 16-byte iNES header, when it was given
    /// as `base64:...`.
    pub rom_md5: Option<[u8; 16]>,
    /// `guid`.
    pub guid: Option<String>,
    /// `comment` lines, in order.
    pub comments: Vec<String>,
    /// `subtitle` lines, in order (`<frame> <text>` as written).
    pub subtitles: Vec<String>,
}

/// A parsed `.fm2` movie.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fm2Movie {
    /// The header.
    pub header: Fm2Header,
    /// One entry per frame, from power on.
    pub frames: Vec<Fm2Frame>,
}

/// Why an `.fm2` file was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fm2Error {
    /// Not UTF-8 text.
    NotText,
    /// No `version` line, or one this reader does not know.
    Version,
    /// The movie starts from a savestate (`savestate` in the header).
    StartsFromSavestate,
    /// The inputs are in the binary encoding (`binary 1`).
    Binary,
    /// A port holds a device other than a gamepad or nothing (`port0`, `port1`: 1 gamepad, 0
    /// none; Zapper and others are not replayed), or the Four Score is used.
    Device(String),
    /// The Famicom Disk System (`FDS 1`) or a command of the FDS or VS. System at a frame.
    UnsupportedCommand {
        /// The line, counted from 1.
        line: usize,
    },
    /// A frame line that does not read.
    Frame {
        /// The line, counted from 1.
        line: usize,
    },
}

impl fmt::Display for Fm2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotText => f.write_str("the file is not text"),
            Self::Version => f.write_str("not an fm2 movie of version 3"),
            Self::StartsFromSavestate => {
                f.write_str("the movie starts from a savestate, which is not supported")
            }
            Self::Binary => f.write_str("binary fm2 movies are not supported"),
            Self::Device(what) => write!(f, "unsupported input device: {what}"),
            Self::UnsupportedCommand { line } => write!(
                f,
                "line {line}: Famicom Disk System or VS. System commands are not supported"
            ),
            Self::Frame { line } => write!(f, "line {line}: not a frame line"),
        }
    }
}

impl std::error::Error for Fm2Error {}

impl Fm2Movie {
    /// Reads an `.fm2` file.
    ///
    /// # Errors
    /// The file is not a movie this reader replays (see [`Fm2Error`]).
    pub fn parse(file: &[u8]) -> Result<Self, Fm2Error> {
        let text = std::str::from_utf8(file).map_err(|_| Fm2Error::NotText)?;
        let mut header = Fm2Header::default();
        let mut frames = Vec::new();
        let mut ports = [1u8, 0, 0];
        for (index, raw) in text.lines().enumerate() {
            let line_number = index + 1;
            let line = raw.trim_end_matches('\r');
            if line.starts_with('|') {
                frames.push(parse_frame(line, ports).ok_or(Fm2Error::Frame { line: line_number })?);
                if frames
                    .last()
                    .is_some_and(|frame: &Fm2Frame| frame.commands & UNSUPPORTED_COMMANDS != 0)
                {
                    return Err(Fm2Error::UnsupportedCommand { line: line_number });
                }
                continue;
            }
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            let value = value.trim();
            match key {
                "version" => header.version = value.parse().map_err(|_| Fm2Error::Version)?,
                "emuVersion" => header.emu_version = value.parse().ok(),
                "rerecordCount" => header.rerecord_count = value.parse().ok(),
                "palFlag" => header.pal = value == "1",
                "romFilename" => header.rom_filename = Some(value.to_owned()),
                "romChecksum" => {
                    header.rom_md5 = value.strip_prefix("base64:").and_then(md5_base64);
                }
                "guid" => header.guid = Some(value.to_owned()),
                "comment" => header.comments.push(value.to_owned()),
                "subtitle" => header.subtitles.push(value.to_owned()),
                "binary" if value != "0" => return Err(Fm2Error::Binary),
                "savestate" if !value.is_empty() => return Err(Fm2Error::StartsFromSavestate),
                "fourscore" if value != "0" => {
                    return Err(Fm2Error::Device("Four Score".to_owned()));
                }
                "FDS" if value != "0" => {
                    return Err(Fm2Error::UnsupportedCommand { line: line_number });
                }
                "port0" | "port1" | "port2" => {
                    let port = usize::from(key.as_bytes()[4] - b'0');
                    let device: u8 = value
                        .parse()
                        .map_err(|_| Fm2Error::Device(format!("{key} {value}")))?;
                    // port0/port1: 0 none, 1 gamepad, 2 Zapper; port2 (expansion): 0 none.
                    let supported = if port == 2 { device == 0 } else { device <= 1 };
                    if !supported {
                        return Err(Fm2Error::Device(format!("{key} {value}")));
                    }
                    ports[port] = device;
                }
                _ => {}
            }
        }
        if header.version != 3 {
            return Err(Fm2Error::Version);
        }
        Ok(Self { header, frames })
    }

    /// The input words, one per frame.
    #[must_use]
    pub fn words(&self) -> Vec<u16> {
        self.frames.iter().map(|frame| frame.word).collect()
    }

    /// The frames whose commands ask for a reset, with the command, a hard reset on the very
    /// first frame left out (FCEUX writes it to say "from power on").
    #[must_use]
    pub fn resets(&self) -> Vec<(usize, u8)> {
        self.frames
            .iter()
            .enumerate()
            .filter_map(|(index, frame)| {
                let reset = frame.commands & (COMMAND_SOFT_RESET | COMMAND_HARD_RESET);
                let power_on = index == 0 && reset == COMMAND_HARD_RESET;
                (reset != 0 && !power_on).then_some((index, reset))
            })
            .collect()
    }

    /// Whether `rom` (a whole iNES file) is the ROM the movie names, by the header's MD5 (the
    /// data after the 16-byte header). `None` when the movie gives no checksum.
    #[must_use]
    pub fn rom_matches(&self, rom: &[u8]) -> Option<bool> {
        let expected = self.header.rom_md5?;
        Some(rom.get(16..).is_some_and(|data| md5(data) == expected))
    }
}

/// One frame line: `|commands|port0|port1|port2|`.
fn parse_frame(line: &str, ports: [u8; 3]) -> Option<Fm2Frame> {
    let mut fields = line.strip_prefix('|')?.split('|');
    let commands = fields.next()?.trim().parse().ok()?;
    let mut word = 0u16;
    for (port, device) in ports.iter().take(2).enumerate() {
        let field = fields.next().unwrap_or("");
        if *device == 1 {
            word |= u16::from(pad_byte(field)?) << (8 * port);
        }
    }
    Some(Fm2Frame { commands, word })
}

/// A gamepad field, `RLDUTSBA`, as A, B, Select, Start, Up, Down, Left, Right in bits 0-7.
fn pad_byte(field: &str) -> Option<u8> {
    let bytes = field.as_bytes();
    if bytes.len() != 8 {
        return None;
    }
    Some(
        bytes
            .iter()
            .enumerate()
            .filter(|(_, &c)| c != b'.' && c != b' ')
            .fold(0u8, |byte, (index, _)| byte | (1 << (7 - index))),
    )
}

/// A base64 MD5 digest (24 characters with padding).
fn md5_base64(text: &str) -> Option<[u8; 16]> {
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::with_capacity(18);
    for c in text.bytes().take_while(|&c| c != b'=') {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push(((bits >> count) & 0xff) as u8);
        }
    }
    out.try_into().ok()
}

/// MD5 (RFC 1321), for the ROM checksum of the header.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn md5(data: &[u8]) -> [u8; 16] {
    const SHIFTS: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let constants: [u32; 64] = std::array::from_fn(|i| {
        // floor(abs(sin(i + 1)) * 2^32), the table of the RFC.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let value = ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32;
        value
    });
    let mut state: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    let mut message = data.to_vec();
    let bit_length = (data.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_le_bytes());
    for block in message.chunks_exact(64) {
        let words: [u32; 16] = std::array::from_fn(|i| {
            u32::from_le_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ])
        });
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let rotated = a
                .wrapping_add(f)
                .wrapping_add(constants[i])
                .wrapping_add(words[g])
                .rotate_left(SHIFTS[i]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(rotated);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
    let mut digest = [0u8; 16];
    for (chunk, word) in digest.chunks_exact_mut(4).zip(state) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    const SAMPLE: &str = "version 3\n\
emuVersion 22020\n\
rerecordCount 12\n\
palFlag 0\n\
romFilename Some Game\n\
romChecksum base64:o6cWwGS6c6ChZeWRi8pBUw==\n\
guid CE875AAD-0640-BD94-9678-7E6A671F1C29\n\
fourscore 0\n\
port0 1\n\
port1 0\n\
port2 0\n\
comment author someone\n\
|2|........|||\n\
|0|.......A|||\n\
|0|R..U..B.|||\n\
|1|    T   |||\n";

    #[test]
    fn reads_header_and_frames() {
        let movie = Fm2Movie::parse(SAMPLE.as_bytes()).unwrap_or_default();
        assert_eq!(movie.header.version, 3);
        assert_eq!(movie.header.emu_version, Some(22020));
        assert_eq!(movie.header.rerecord_count, Some(12));
        assert!(!movie.header.pal);
        assert_eq!(movie.header.rom_filename.as_deref(), Some("Some Game"));
        assert_eq!(movie.header.comments, ["author someone"]);
        assert_eq!(
            movie.header.rom_md5,
            Some([
                0xa3, 0xa7, 0x16, 0xc0, 0x64, 0xba, 0x73, 0xa0, 0xa1, 0x65, 0xe5, 0x91, 0x8b, 0xca,
                0x41, 0x53
            ])
        );
        // A = bit 0, B = 1, Start = 3, Up = 4, Right = 7.
        assert_eq!(movie.words(), [0, 0x01, 0x80 | 0x10 | 0x02, 0x08]);
        assert_eq!(movie.resets(), [(3, COMMAND_SOFT_RESET)]);
    }

    #[test]
    fn second_pad_goes_to_the_high_byte() {
        let text = "version 3\nport0 1\nport1 1\n|0|.......A|R.......|\n";
        let movie = Fm2Movie::parse(text.as_bytes()).unwrap_or_default();
        assert_eq!(movie.words(), [0x8001]);
    }

    #[test]
    fn refuses_what_it_cannot_replay() {
        let refuse = |text: &str| Fm2Movie::parse(text.as_bytes()).err();
        assert_eq!(refuse("version 3\nbinary 1\n"), Some(Fm2Error::Binary));
        assert_eq!(
            refuse("version 3\nsavestate base64:AAAA\n"),
            Some(Fm2Error::StartsFromSavestate)
        );
        assert!(matches!(
            refuse("version 3\nport0 2\n"),
            Some(Fm2Error::Device(_))
        ));
        assert!(matches!(
            refuse("version 3\nfourscore 1\n"),
            Some(Fm2Error::Device(_))
        ));
        assert_eq!(
            refuse("version 3\n|4|........|||\n"),
            Some(Fm2Error::UnsupportedCommand { line: 2 })
        );
        assert_eq!(
            refuse("version 3\n|0|....|||\n"),
            Some(Fm2Error::Frame { line: 2 })
        );
        assert_eq!(refuse("|0|........|||\n"), Some(Fm2Error::Version));
    }

    #[test]
    fn md5_matches_the_rfc_examples() {
        let hex = |digest: [u8; 16]| {
            digest.iter().fold(String::new(), |mut text, b| {
                let _ = write!(text, "{b:02x}");
                text
            })
        };
        assert_eq!(hex(md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hex(md5(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            )),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn rom_check_uses_the_data_after_the_header() {
        let mut rom = vec![0u8; 16];
        rom.extend_from_slice(b"abc");
        let mut movie = Fm2Movie::default();
        assert_eq!(movie.rom_matches(&rom), None);
        movie.header.rom_md5 = Some(md5(b"abc"));
        assert_eq!(movie.rom_matches(&rom), Some(true));
        rom.push(0);
        assert_eq!(movie.rom_matches(&rom), Some(false));
    }
}
