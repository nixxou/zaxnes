//! What a board needs to set itself up, and how it is read out of an iNES header.
//!
//! Ported from BizHawk 2.11.1 `src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/CartInfo.cs`,
//! `NES.iNES.cs` and the `EDetectionOrigin` enumeration of `Boards/INesBoard.cs`.
//! MIT; see `LICENSE-BizHawk`.

/// Where the cartridge description came from. `INesBoard.cs`'s `EDetectionOrigin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetectionOrigin {
    #[default]
    None,
    BootGodDb,
    GameDb,
    Ines,
    Unif,
    Fds,
    Nsf,
}

/// `CartInfo.cs`. The sizes are in KiB, as in the C#.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CartInfo {
    /// The friendly name, from whichever database named it.
    pub name: Option<String>,
    /// The dump status flag of the BizHawk game database, which only the loader reports.
    pub game_info_hash: Option<String>,
    /// Whether the BizHawk game database says the dump is bad.
    pub game_info_bad: bool,

    pub trainer_size: i32,
    pub chr_size: i32,
    pub prg_size: i32,
    pub wram_size: i32,
    pub vram_size: i32,
    pub pad_h: u8,
    pub pad_v: u8,
    pub wram_battery: bool,
    pub bad: bool,

    /// Bits 0 and 3 of flags 6, when the description came from a header. `None` from BootGod.
    pub ines_mirroring: Option<i32>,

    pub board_type: String,
    pub pcb: Option<String>,

    pub sha1: Option<String>,
    pub system: Option<String>,
    pub chips: Vec<String>,

    pub palette: Option<String>,
    pub vs_security: u8,
}

impl CartInfo {
    /// `CartInfo.ToString`, which the load report prints.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "pr={},ch={},wr={},vr={},ba={},pa={}|{},brd={},sys={}",
            self.prg_size,
            self.chr_size,
            self.wram_size,
            self.vram_size,
            u8::from(self.wram_battery),
            self.pad_h,
            self.pad_v,
            self.board_type,
            self.system.as_deref().unwrap_or(""),
        )
    }
}

/// `NES.iNES.cs`'s `iNES2Wram`. The C# throws on 15; this returns `None` so that a malformed
/// header becomes a load failure rather than a panic.
fn ines2_wram(i: i32) -> Option<i32> {
    if i == 0 {
        return Some(0);
    }
    if i == 15 {
        return None;
    }
    Some(1 << (i + 6))
}

/// What `DetectFromINES` produces: the v1 reading always, the v2 reading when the header says so.
#[derive(Debug, Clone)]
pub struct InesHeaders {
    pub v1: CartInfo,
    pub v2: Option<CartInfo>,
}

/// `NES.DetectFromINES`. `None` when the magic is absent, which is the C#'s `return false`.
///
/// `data` is the first 16 bytes of the file.
#[must_use]
pub fn detect_from_ines(data: &[u8]) -> Option<InesHeaders> {
    if data.len() < 16 || &data[..4] != b"NES\x1A" {
        return None;
    }
    let d = |i: usize| i32::from(data[i]);

    let mut cart_v2: Option<CartInfo> = None;

    if (data[7] & 0x0C) == 0x08 {
        // process as iNES v2
        // ReadEMFormAreaSize: `if (msn is not 0xF) return ((msn << 8) | lsb) * nonEMFormBlockSizeKiB;`
        fn read_em_form_area_size(msn: i32, lsb: i32, non_em_block_size_kib: i32) -> Option<i32> {
            if msn != 0xF {
                return Some(((msn << 8) | lsb) * non_em_block_size_kib);
            }
            let exponent = (lsb >> 2) & 0x3F;
            if !(10..=30).contains(&exponent) {
                return None;
            }
            let multiplier = (lsb & 0x03) * 2 + 1;
            let prg_size = (1u64 << exponent) * multiplier as u64;
            if prg_size > i32::MAX as u64 {
                return None;
            }
            Some((prg_size >> 10) as i32)
        }

        let mut v2 = CartInfo {
            prg_size: read_em_form_area_size(d(9) & 0x0F, d(4), 16)?,
            chr_size: read_em_form_area_size(d(9) >> 4, d(5), 8)?,
            ..CartInfo::default()
        };

        v2.wram_battery = (data[6] & 2) != 0;

        let wrambat = ines2_wram(d(10) >> 4)?;
        let wramnon = ines2_wram(d(10) & 15)?;
        v2.wram_battery |= wrambat > 0;
        // fixme - doesn't handle sizes not divisible by 1024  (the C#'s own comment)
        v2.wram_size = i32::from(((wrambat + wramnon) / 1024) as i16);

        let mapper = d(6) >> 4 | d(7) & 0xF0 | d(8) << 8 & 0xF00;
        let submapper = d(8) >> 4;
        v2.board_type = format!("MAPPER{mapper:04}-{submapper:02}");

        let vrambat = ines2_wram(d(11) >> 4)?;
        let vramnon = ines2_wram(d(11) & 15)?;
        v2.wram_battery |= vrambat > 0;
        v2.vram_size = (vrambat + vramnon) / 1024;

        v2.ines_mirroring = Some(d(6) & 1 | d(6) >> 2 & 2);
        match v2.ines_mirroring {
            Some(0) => v2.pad_v = 1,
            Some(1) => v2.pad_h = 1,
            _ => {}
        }

        v2.system = match data[12] & 3 {
            // 2 indicates dual NTSC/PAL compatibility, might as well use NTSC
            0 | 2 => Some("NES-NTSC".to_string()),
            1 => Some("NES-PAL".to_string()),
            3 => Some("Dendy".to_string()),
            _ => v2.system,
        };

        if (data[6] & 4) != 0 {
            v2.trainer_size = 512;
        }

        cart_v2 = Some(v2);
    }

    // process as iNES v1
    let mut cart = CartInfo {
        prg_size: d(4),
        chr_size: d(5),
        ..CartInfo::default()
    };

    if cart.prg_size == 0 {
        cart.prg_size = 256;
    }
    cart.prg_size *= 16;
    cart.chr_size *= 8;

    cart.wram_battery = (data[6] & 2) != 0;
    cart.wram_size = 8; // should be data[8], but that never worked

    {
        let mapper = d(6) >> 4 | d(7) & 0xF0;
        cart.board_type = format!("MAPPER{mapper:03}");
    }

    cart.vram_size = if cart.chr_size > 0 { 0 } else { 8 };

    cart.ines_mirroring = Some(d(6) & 1 | d(6) >> 2 & 2);
    match cart.ines_mirroring {
        Some(0) => cart.pad_v = 1,
        Some(1) => cart.pad_h = 1,
        _ => {}
    }

    if (data[6] & 0x04) != 0 {
        cart.trainer_size = 512;
    }

    // iNES v1 has no reliable region; v2's is used when it is there.
    cart.system = cart_v2.as_ref().and_then(|v| v.system.clone());

    Some(InesHeaders {
        v1: cart,
        v2: cart_v2,
    })
}
