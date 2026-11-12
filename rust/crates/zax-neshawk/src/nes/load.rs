//! Loading a cartridge: `NES.Init`.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/NES.cs`'s `Init` and `DetectRegion`, with
//! the database lookups of `NES.BoardSystem.cs`. UNIF, FDS and NSF images are out of scope and
//! are refused rather than guessed at.
//! MIT; see `LICENSE-BizHawk`.

use std::fmt::Write as _;

use super::{Bus, BusLog, DisplayType, Nes};
use crate::apu::Apu;
use crate::boards::{self, BoardKind};
use crate::cart::{detect_from_ines, CartInfo, DetectionOrigin};
use crate::controllers::{ControllerDeck, PortDevice};
use crate::cpu::Mos6502x;
use crate::gamedb::GameDb;
use crate::ppu::Ppu;

/// Why a cartridge would not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// "Alleged NES rom too small to be anything useful"
    TooSmall,
    /// A UNIF, FDS or NSF image, which this port does not implement.
    UnsupportedFormat(&'static str),
    /// "iNES header not found and no gamedb entry"
    NoHeaderAndNoDatabaseEntry,
    /// "No class implements the necessary board type: …". The payload is the board type, so that
    /// the caller can say which mapper to fall back to.
    NoBoardFor(String),
    /// An iNES v2 header this port cannot read (an exponent-form size out of range).
    MalformedHeader,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall => write!(f, "alleged NES rom too small to be anything useful"),
            Self::UnsupportedFormat(what) => {
                write!(f, "{what} images are not implemented by the NesHawk port")
            }
            Self::NoHeaderAndNoDatabaseEntry => {
                write!(f, "iNES header not found and no gamedb entry")
            }
            Self::NoBoardFor(board) => {
                write!(f, "no ported board implements the board type {board}")
            }
            Self::MalformedHeader => write!(f, "the iNES v2 header could not be read"),
        }
    }
}

impl std::error::Error for LoadError {}

/// `NES.RomDetails`: what the load decided, and from where.
#[derive(Debug, Clone)]
pub struct LoadReport {
    pub lines: Vec<String>,
    pub origin: DetectionOrigin,
    pub board_name: &'static str,
    pub cart: CartInfo,
    /// True when the board came from `zax-nes` instead, which this crate never sets; the harness
    /// and the engine use it to mark a game as not NesHawk-exact.
    pub board_from_zaxnes: bool,
}

impl LoadReport {
    #[must_use]
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }
}

/// SHA-1, enough of it to key the databases. Written out rather than taken from a crate because
/// this crate has no dependencies and the workspace's hashing lives behind `zax-nes`.
fn sha1_hex(data: &[u8]) -> String {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut block = [0u8; 64];
    let mut w = [0u32; 80];
    let total_bits = (data.len() as u64) * 8;

    let process = |block: &[u8; 64], h: &mut [u32; 5], w: &mut [u32; 80]| {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    };

    let full = data.len() / 64;
    for i in 0..full {
        block.copy_from_slice(&data[i * 64..i * 64 + 64]);
        process(&block, &mut h, &mut w);
    }
    let rest = &data[full * 64..];
    block = [0u8; 64];
    block[..rest.len()].copy_from_slice(rest);
    block[rest.len()] = 0x80;
    if rest.len() >= 56 {
        process(&block, &mut h, &mut w);
        block = [0u8; 64];
    }
    block[56..64].copy_from_slice(&total_bits.to_be_bytes());
    process(&block, &mut h, &mut w);

    let mut out = String::with_capacity(40);
    for v in h {
        let _ = write!(out, "{v:08X}");
    }
    out
}

/// MD5, for the BizHawk game database's `md5:` keys.
fn md5_hex(data: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let mut k = [0u32; 64];
    for (i, slot) in k.iter_mut().enumerate() {
        // floor(abs(sin(i+1)) * 2^32), computed without libm: the published table, derived here
        // by the standard recurrence is not possible, so use the fractional-sine identity via
        // f64::sin, which is available in core as an intrinsic through std.
        *slot = ((f64::from(i as u32 + 1).sin().abs()) * 4_294_967_296.0) as u32;
    }

    let mut h: [u32; 4] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476];
    let total_bits = (data.len() as u64).wrapping_mul(8);

    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&total_bits.to_le_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | ((!b) & d), i),
                1 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
    }

    let mut out = String::with_capacity(32);
    for b in h.iter().flat_map(|v| v.to_le_bytes()) {
        let _ = write!(out, "{b:02X}");
    }
    out
}

impl Nes {
    /// Load a cartridge image and power the machine on. `NES.Init` followed by `HardReset`.
    ///
    /// `db` is the BizHawk game database and NesCartDB; without them detection falls through to
    /// the iNES header, which for some cartridges is a **different machine** (see
    /// [`crate::gamedb`]).
    ///
    /// # Errors
    ///
    /// [`LoadError`] when the image is too short, is a UNIF/FDS/NSF image this port does not
    /// implement, cannot be identified at all, or names a board outside
    /// [`crate::boards`]'s list.
    pub fn load(
        rom: &[u8],
        db: &GameDb,
        left: PortDevice,
        right: PortDevice,
    ) -> Result<Self, LoadError> {
        let mut lines: Vec<String> = Vec::new();
        let mut log = |s: String| lines.push(s);

        log("------".to_string());
        log("BEGIN NES rom analysis:".to_string());

        if rom.len() < 16 {
            return Err(LoadError::TooSmall);
        }
        if &rom[..4] == b"UNIF" {
            return Err(LoadError::UnsupportedFormat("UNIF"));
        }
        if rom.len() >= 5 && &rom[..5] == b"NESM\x1A" {
            return Err(LoadError::UnsupportedFormat("NSF"));
        }
        if &rom[..4] == b"FDS\x1A" || &rom[..4] == b"\x01*NI" {
            return Err(LoadError::UnsupportedFormat("FDS"));
        }

        let mut hash_sha1_several: Vec<String> = Vec::new();
        let hash_sha1: Option<String>;
        let hash_md5: Option<String>;
        let mut choice: Option<CartInfo> = None;
        let mut origin = DetectionOrigin::None;

        let headers = detect_from_ines(&rom[..16]);
        let ines_header_info = headers.as_ref().map(|h| h.v1.clone());
        let ines_header_info_v2 = headers.as_ref().and_then(|h| h.v2.clone());

        if let Some(hi) = ines_header_info.as_ref() {
            //now that we know we have an iNES header, we can try to ignore it.
            let trimmed = &rom[16..];
            let sha1 = format!("SHA1:{}", sha1_hex(trimmed));
            hash_sha1_several.push(sha1.clone());
            hash_sha1 = Some(sha1.clone());
            hash_md5 = Some(format!("MD5:{}", md5_hex(trimmed)));

            log("Found iNES header:".to_string());
            log(hi.describe());
            log(format!("headerless rom hash: {sha1}"));

            if hi.prg_size == 16 {
                //8KB prg can't be stored in iNES format, which counts 16KB prg banks.
                //so a correct hash will include only 8KB.
                let mut temp: Vec<u8> = Vec::new();
                temp.extend_from_slice(&rom[16..16 + 8 * 1024]);
                let chr_bytes = (hi.chr_size as usize) * 1024;
                if rom.len() >= 16 * 1024 + chr_bytes + 16 {
                    temp.extend_from_slice(&rom[16 + 16 * 1024..16 + 16 * 1024 + chr_bytes]);
                } else if rom.len() >= 8 * 1024 + chr_bytes + 16 {
                    temp.extend_from_slice(&rom[16 + 8 * 1024..16 + 8 * 1024 + chr_bytes]);
                }
                hash_sha1_several.push(format!("SHA1:{}", sha1_hex(&temp)));
            }
        } else {
            // we don't have an ines header, check if the game hash is in the game db
            log("headerless ROM, using Game DB".to_string());
            let md5 = format!("MD5:{}", md5_hex(rom));
            let sha1 = format!("SHA1:{}", sha1_hex(rom));
            choice = db
                .identify_from_game_db(&md5)
                .or_else(|| db.identify_from_game_db(&sha1));
            hash_md5 = Some(md5);
            hash_sha1 = Some(sha1.clone());
            if choice.is_none() {
                hash_sha1_several.push(sha1);
                choice = db.identify_from_bootgod(&hash_sha1_several);
                if choice.is_none() {
                    log("Could not locate game in nescartdb".to_string());
                } else {
                    log("Chose board from nescartdb:".to_string());
                    origin = DetectionOrigin::BootGodDb;
                }
            }
            if choice.is_none() {
                return Err(LoadError::NoHeaderAndNoDatabaseEntry);
            }
        }

        // USE_DATABASE
        if let Some(md5) = &hash_md5 {
            choice = db.identify_from_game_db(md5);
        }
        if choice.is_none() {
            if let Some(sha1) = &hash_sha1 {
                choice = db.identify_from_game_db(sha1);
            }
        }
        if let Some(c) = choice.as_mut() {
            origin = DetectionOrigin::GameDb;
            log(format!("Chose board from bizhawk gamedb: {}", c.board_type));
            //gamedb entries that don't specify prg/chr sizes can infer it from the ines header
            if let Some(hi) = &ines_header_info {
                if c.prg_size == -1 {
                    c.prg_size = hi.prg_size;
                }
                if c.chr_size == -1 {
                    c.chr_size = hi.chr_size;
                }
                if c.vram_size == -1 {
                    c.vram_size = hi.vram_size;
                }
                if c.wram_size == -1 {
                    c.wram_size = hi.wram_size;
                }
            }
        }

        //if this is still null, we have to try it some other way. nescartdb perhaps?
        if choice.is_none() {
            choice = db.identify_from_bootgod(&hash_sha1_several);
            if choice.is_none() {
                log("Could not locate game in nescartdb".to_string());
            } else {
                log("Chose board from nescartdb:".to_string());
                origin = DetectionOrigin::BootGodDb;
            }
        }

        //if choice is still null, try iNES
        let mut board_probe: Option<(BoardKind, Box<dyn crate::boards::Board>, CartInfo)> = None;
        if choice.is_none() {
            if let Some(hi) = &ines_header_info {
                log("Attempting inference from iNES header".to_string());
                // try to spin up V2 header first, then V1 header
                if let Some(v2) = &ines_header_info_v2 {
                    board_probe = boards::find_board(v2, origin);
                    if board_probe.is_none() {
                        log("Failed to load as iNES V2".to_string());
                    } else {
                        choice = Some(v2.clone());
                    }
                }
                if board_probe.is_none() {
                    choice = Some(hi.clone()); // we're out of options, really
                    board_probe = boards::find_board(hi, origin);
                    if board_probe.is_none() {
                        log("Failed to load as iNES V1".to_string());
                    }
                }

                log("Chose board from iNES heuristics:".to_string());
                origin = DetectionOrigin::Ines;
            }
        }

        let choice = choice.ok_or(LoadError::NoHeaderAndNoDatabaseEntry)?;
        let game_name = choice.name.clone().unwrap_or_default();

        //find a INESBoard to handle this
        let Some((board_kind, mut board, cart)) = boards::find_board(&choice, origin) else {
            return Err(LoadError::NoBoardFor(choice.board_type.clone()));
        };
        drop(board_probe);

        log("Final game detection results:".to_string());
        log(cart.describe());
        log(format!("\"{game_name}\""));
        log(format!("Implemented by: class {}", board.name()));
        if cart.bad || cart.game_info_bad {
            log("~~ ONE WAY OR ANOTHER, THIS DUMP IS KNOWN TO BE *BAD* ~~".to_string());
        }
        log("END NES rom analysis".to_string());
        log("------".to_string());

        //create the board's rom and vrom
        let mut trainer: Option<Vec<u8>> = None;
        if let Some(hi) = &ines_header_info {
            let mut pos = 16usize;
            if hi.trainer_size != 0 {
                let end = (pos + 512).min(rom.len());
                trainer = Some(rom[pos..end].to_vec());
                pos = end;
            }

            let prg_len = (cart.prg_size as usize) * 1024;
            let mut r = vec![0u8; prg_len];
            let n = prg_len.min(rom.len().saturating_sub(pos));
            r[..n].copy_from_slice(&rom[pos..pos + n]);
            pos += n;
            board.base_mut().rom = r;

            if cart.chr_size > 0 {
                let chr_len = (cart.chr_size as usize) * 1024;
                let mut v = vec![0u8; chr_len];
                let n = chr_len.min(rom.len().saturating_sub(pos));
                v[..n].copy_from_slice(&rom[pos..pos + n]);
                if n < chr_len {
                    log(format!(
                        "Less than the expected VROM was found in the file: {n} < {chr_len}"
                    ));
                }
                board.base_mut().vrom = v;
            }
            if cart.prg_size != hi.prg_size || cart.chr_size != hi.chr_size {
                log(
                    "Warning: Detected choice has different filesizes than the INES header!"
                        .to_string(),
                );
            }
        } else {
            // we should only get here for boards with no header
            let prg_len = (cart.prg_size as usize) * 1024;
            let mut r = vec![0u8; prg_len];
            let n = prg_len.min(rom.len());
            r[..n].copy_from_slice(&rom[..n]);
            board.base_mut().rom = r;

            if cart.chr_size > 0 {
                let chr_len = (cart.chr_size as usize) * 1024;
                let mut v = vec![0u8; chr_len];
                let n = chr_len.min(rom.len().saturating_sub(prg_len));
                v[..n].copy_from_slice(&rom[prg_len..prg_len + n]);
                board.base_mut().vrom = v;
            }
        }

        //create the vram and wram if necessary
        if cart.wram_size != 0 {
            board.base_mut().wram = vec![0u8; (cart.wram_size as usize) * 1024];
        }
        if cart.vram_size != 0 {
            board.base_mut().vram = vec![0u8; (cart.vram_size as usize) * 1024];
        }

        board.post_configure();

        // set up display type
        let display_type = match detect_region(cart.system.as_deref()) {
            Some(d) => d,
            None => DisplayType::Ntsc,
        };

        let board_name = board.name();
        let has_clock_ppu = board.has_clock_ppu();

        let mut nes = Self {
            cpu: Mos6502x::new(),
            b: Bus {
                ppu: Ppu::new(),
                apu: Apu::new(None, false),
                ram: [0; 0x800],
                ciram: [0; 0x800],
                game_name,
                cart: cart.clone(),
                board,
                board_kind,
                origin,
                sprdma_countdown: 0,
                irq_apu: false,
                cpuclockrate: 1_789_773,
                controller_deck: ControllerDeck::new(left, right),
                latched4016: 0,
                display_type,
                vsync_num: 0,
                vsync_den: 1,
                old_s: 0,
                double_controller_read: 0,
                double_controller_read_address: 0,
                previous_controller1_read: 0,
                previous_controller2_read: 0,
                dmc_dma_controller_conflict: false,
                joypad_strobed: false,
                joypad_strobe_value: 0,
                cpu_sequence: [3, 3, 3, 3, 3],
                cpu_deadcounter: 0,
                oam_dma_index: 0,
                oam_dma_exec: false,
                oam_dma_addr: 0,
                oam_dma_byte: 0,
                dmc_dma_exec: false,
                dmc_realign: false,
                db: 0,
                address_bus: 0,
                frame: 0,
                lagged: false,
                islag: false,
                lagcount: 0,
                controller_was_latched: false,
                frame_is_done: false,
                current_strobe: false,
                new_strobe: false,
                reset_signal: false,
                hard_reset_signal: false,
                initial_wram_state_pattern: Vec::new(),
                audio_deltas: Vec::new(),
                bus_log: BusLog::default(),
                load_report: None,
                // `MOS6502X.Reset` sets these two in the C#; they live on the machine here.
                rdy: true,
                rdy_freeze: false,
                total_executed_cycles: 0,
            },
        };
        nes.b.ppu.has_clock_ppu = has_clock_ppu;

        nes.hard_reset();

        if let Some(trainer) = trainer {
            for (i, byte) in trainer.iter().enumerate().take(512) {
                nes.b.write_memory(0x7000 + i as u16, *byte);
            }
        }

        nes.b.load_report = Some(LoadReport {
            lines,
            origin,
            board_name,
            cart,
            board_from_zaxnes: false,
        });

        Ok(nes)
    }
}

/// `NES.DetectRegion`.
fn detect_region(system: Option<&str>) -> Option<DisplayType> {
    match system {
        Some("NES-PAL" | "NES-PAL-A" | "NES-PAL-B") => Some(DisplayType::Pal),
        Some("NES-NTSC" | "Famicom") => Some(DisplayType::Ntsc),
        // this is in bootgod, but not used at all
        Some("Dendy") => Some(DisplayType::Dendy),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hashes_match_the_published_digests() {
        assert_eq!(sha1_hex(b"abc"), "A9993E364706816ABA3E25717850C26C9CD0D89D");
        assert_eq!(sha1_hex(b""), "DA39A3EE5E6B4B0D3255BFEF95601890AFD80709");
        assert_eq!(md5_hex(b"abc"), "900150983CD24FB0D6963F7D28E17F72");
        assert_eq!(md5_hex(b""), "D41D8CD98F00B204E9800998ECF8427E");
    }
}
