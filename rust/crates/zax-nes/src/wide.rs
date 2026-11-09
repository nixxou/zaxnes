//! The widescreen extension: a small register window a ROM unlocks to have the core capture
//! tiles beyond the 256-pixel picture (see `docs/WIDESCREEN-EXTENSION.md`).
//!
//! Real hardware has no such thing: a ROM that uses it is made for this emulator. Nothing
//! changes for a ROM that never writes the unlock sequence: the window stays transparent (the
//! bus keeps reaching the cartridge and the open bus) and every capture is the standard one.
//!
//! Registers, once unlocked (`$5000`-`$5007` read back what was written unless noted):
//!
//! | Address | Access | Meaning |
//! |---|---|---|
//! | `$5000` | W | unlock sequence `W I D E` (`$57 $49 $44 $45`); R `$57` once unlocked |
//! | `$5001` | R | version, 1 |
//! | `$5002` | R | the margin the host offers on each side, in pixels (0 = widescreen off) |
//! | `$5003` | R/W | the margin the game asks for on each side, in pixels |
//! | `$5004` | R/W | how far left of x = 0 the picture may show, in pixels |
//! | `$5005` | R/W | how far right of x = 255 the picture may show, in pixels |
//! | `$5006` | R/W | overclock: extra CPU time each frame, in scanlines, before the vblank |
//! | `$5007` | R/W | bit 0: four separate nametables (2 x 2), whatever the cartridge's mirroring; bit 5: sprite high bytes and flags kept per sprite page (`$500C`), an OAM DMA taking those of its source page; bit 4: the PRG window goes on over `$6000`-`$7FFF` (8 KiB more, for a game with nothing there) |
//! | `$5100`-`$51FF` | W | high byte of the x of sprite `(address & $FF) / 4`, taken at the next OAM DMA; `$5101 + 4n` instead: sprite *n*'s flags (bit 0: drawn in the "asleep" tint, grey), taken the same way |
//! | `$5008` | R/W | PRG window: bit 7 on, bits 0-6 the 16 KiB PRG-ROM bank shown at `$5800`-`$5FFF` |
//! | `$5009` | R/W | PRG window: first 256-byte page of that bank it shows |
//! | `$500C` | R/W | with `$5007` bit 5: the sprite page the ROM builds (its CPU page, `$02` for `$0200`); a write clears that page's high bytes and flags, which `$5100`-`$51FF` then write |
//! | `$500B` | R/W | execution zone: bit 7 on, bit 6 its data at `$3000`-`$3FFF` (below), bits 0-5 the page of the pack's code shown at `$2800`-`$3FFF` (`$2FFF` with bit 6) |
//! | `$5200`-`$5FFF` | R/W | 3.5 KiB of RAM for the ROM's own variables and code (the CPU runs code from it); `$5800`-`$5FFF` shows PRG-ROM instead while the window is on |
//! | `$57F0`-`$57F7` | R/W | in that RAM: the camera report of a hack's smooth scrolling ([`CameraReport`]), which the core reads and clears after each frame |
//!
//! The margin in use is `min($5003, $5002)` rounded down to a multiple of 8. `$5004` and
//! `$5005` are read each time a band of tiles is captured, so a game can hide the sides behind
//! a status bar or past the end of a level by writing them mid-frame.
//!
//! The execution zone (`$500B`) shows a hack pack's code to the CPU's instruction fetches only
//! in `$2000`-`$3FFF` (the PPU's register mirrors, where no game runs code): `$2000`-`$27FF` its
//! first 2 KiB, always, `$2800`-`$3FFF` the 6 KiB of page `n` (`$800 + n * $1800` on). Every data
//! read and write there still reaches the PPU, those of the hack's own code too, unless the zone
//! holds its data (`$500B` bit 6): then the data accesses at `$3000`-`$3FFF` made by an
//! instruction fetched from the zone reach the extension's registers and RAM (`$3xxx` standing for
//! `$5xxx`), the page's code stops at `$2FFF`, and `$5000`-`$5FFF` is left to the cartridge (MMC5,
//! Namco 163). The game's own accesses, from its ROM, still reach the PPU: it sees no change.
//!
//! The overclock (`$5006`) holds the PPU and the APU still for that many
//! scanlines' worth of CPU cycles when the picture is done (line 240), so the game's frame
//! logic gets more time before the NMI while the picture and the sound stay as they were.

use crate::state::{InvalidState, StateReader, StateWriter};

/// The unlock sequence written to `$5000`.
const UNLOCK: [u8; 4] = *b"WIDE";

/// First address of the register window.
pub const FIRST_REGISTER: u16 = 0x5000;

/// Last address of the register window.
pub const LAST_REGISTER: u16 = 0x5FFF;

/// `$5007` bit 5: sprite high bytes and flags kept per sprite page (`$500C`).
const PAGED_SPRITES: u8 = 0x20;

/// The sprite high bytes and flags of each of the eight CPU pages a ROM may build sprites in
/// (`$5007` bit 5 with `$500C`). Boxed and allocated on first use: see
/// [`WideExtension::paged`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct PagedSprites {
    high: [[u8; 64]; 8],
    flags: [[u8; 64]; 8],
}

impl PagedSprites {
    fn new() -> Box<Self> {
        Box::new(Self {
            high: [[0; 64]; 8],
            flags: [[0; 64]; 8],
        })
    }
}

/// First address of the RAM of the window.
const FIRST_RAM: u16 = 0x5200;

/// Bytes of RAM in the window.
pub(crate) const RAM_SIZE: usize = 0x0E00;

/// CPU cycles one overclock scanline stands for: 341 dots at three dots per cycle, rounded up.
const CYCLES_PER_SCANLINE: u32 = 114;

/// The execution zone: `$2000`-`$27FF` shows the code's first bytes, always; `$2800`-`$3FFF` a
/// page of it.
const ZONE_FIRST: u16 = 0x2000;
const ZONE_PAGED: u16 = 0x2800;
const ZONE_LAST: u16 = 0x3FFF;
/// `$500B`: the zone on (bit 7), its data at `$3000`-`$3FFF` (bit 6), the page (bits 0-5).
const ZONE_ON: u8 = 0x80;
const ZONE_DATA: u8 = 0x40;
const ZONE_PAGE: u8 = 0x3F;
/// Where the zone's data starts, when it holds its data.
const ZONE_DATA_FIRST: u16 = 0x3000;
/// Bytes of the code a page of the zone shows.
const ZONE_PAGE_SIZE: usize = 0x1800;

/// Version the ROM reads at `$5001`.
const VERSION: u8 = 1;

/// The widest margin, in pixels on each side, the core will ever capture: 64 tile columns, a
/// 1280 x 240 picture. Past 128 pixels only a margin decoder ([`crate::wide_decode`]) has
/// anything to show: the nametables wrap there.
pub const MAX_MARGIN: i32 = 512;

/// The widest margin the nametables can show: past it they wrap.
const NAMETABLE_MARGIN: i32 = 128;

/// The second sprite page in the extension's RAM (`$5007` bit 2), and the high bytes of its
/// sprites' x (sprite n at `EXTRA_HIGH + 4n`).
const EXTRA_OAM: u16 = 0x5300;

/// The camera report of a hack's smooth scrolling, in the window's RAM: a status byte the code
/// writes last, then the camera across, down and the playfield's bottom ([`CameraReport`]).
const CAMERA_REPORT: u16 = 0x57F0;
const REPORT_X: u8 = 0x01;
const REPORT_X_FRACTION: u8 = 0x02;
const REPORT_Y: u8 = 0x04;
const REPORT_Y_FRACTION: u8 = 0x08;
const REPORT_END: u8 = 0x10;

/// What a hack's code reported of the game's camera in the frame just run, for the smooth
/// scrolling: how the game keeps it is the hack's business (one variable, several modes, a
/// camera worked out), the core only reads what it was told. Each part is optional; a frame
/// without a report (status 0) has no smooth scrolling, but a frame or two the game runs late.
///
/// | Address | Contents |
/// |---|---|
/// | `$57F0` | status, written last: bit 0 the camera across, bit 1 its fraction, bit 2 the camera down, bit 3 its fraction, bit 4 the playfield's bottom |
/// | `$57F1`-`$57F2` | the camera across, in pixels (low, high) |
/// | `$57F3` | its fraction of a pixel, in 256ths |
/// | `$57F4`-`$57F5` | the camera down, in pixels (low, high) |
/// | `$57F6` | its fraction of a pixel, in 256ths |
/// | `$57F7` | the first screen line under the scrolled playfield (a status bar's top); without it, the core finds it from the rows the PPU drew |
///
/// The camera is the one of the picture the next frame shows (written during the game's logic,
/// after it moved the camera and before the frame is drawn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraReport {
    /// The camera across, in pixels, and its fraction (0 when not reported).
    pub x: Option<(i32, u8)>,
    /// The camera down, in pixels, and its fraction.
    pub y: Option<(i32, u8)>,
    /// The first screen line under the scrolled playfield.
    pub end: Option<i16>,
}
const EXTRA_HIGH: u16 = 0x5400;

/// The widest margin a ROM can ask for or be told of through its 8-bit registers.
const MAX_ROM_MARGIN: i32 = 248;

/// The tallest margin, in pixels above and below, the core will ever capture: 30 tile rows. Only
/// a margin decoder fills it, from rows past the ones the PPU drew.
pub const MAX_VERTICAL_MARGIN: i32 = 240;

/// Extra background rows above and below at the tallest margin.
pub const MAX_EXTRA_ROWS: i32 = MAX_VERTICAL_MARGIN / 8;

/// Extra background columns on each side at the widest margin.
pub const MAX_EXTRA_COLUMNS: i32 = MAX_MARGIN / 8;

/// State of the widescreen extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WideExtension {
    /// Margin the host offers on each side, in pixels, a multiple of 8 (0 = off).
    host_margin: i32,
    /// Margin the host offers above and below, in pixels, a multiple of 8 (0 = off).
    host_vertical_margin: i32,
    /// The options of the ROM's built-in patch the player turned on (`$500A`).
    host_options: u8,
    /// How many bytes of [`UNLOCK`] have been written in a row.
    unlock_progress: u8,
    unlocked: bool,
    requested_margin: u8,
    left_extent: u8,
    right_extent: u8,
    /// Sprite x high bytes written since the last OAM DMA.
    pending_high: [u8; 64],
    /// Sprite x high bytes of the sprites now in OAM.
    active_high: [u8; 64],
    /// Sprite flags (`$5101 + 4n`) written since the last OAM DMA, and those of the sprites now
    /// in OAM. Not saved: a display aid the ROM writes again each frame.
    pending_flags: [u8; 64],
    active_flags: [u8; 64],
    /// With `$5007` bit 5: the high bytes and flags of each sprite page (CPU page & 7), which an
    /// OAM DMA takes from its source page. Not saved: the ROM writes them again as it builds
    /// each page.
    ///
    /// **Behind a `Box`, and allocated only by a ROM that asks for paged sprites** (lot
    /// KIRBY-MERGE step 4). The two tables are 1 KiB together, and `WideExtension` lives *inside*
    /// `Ppu2C02`, which is the struct the hottest loop of the program works on: a kilobyte of
    /// state that only Kirby's pack ever touches has no business sitting in it for every game.
    /// `None` reads as all zeros, which is what the tables hold until a `$500C` write.
    paged: Option<Box<PagedSprites>>,
    /// The page the ROM is building (`$500C`).
    built_page: u8,
    /// The second sprite page (`$5007` bit 2) latched at the last OAM DMA: 64 sprites in OAM
    /// format, then the high bytes of their x. Not saved: the next DMA latches it again.
    extra_oam: [u8; 256],
    extra_high: [u8; 64],
    /// RAM at `$5200`-`$5FFF`.
    scratch: Vec<u8>,
    /// Overclock register: extra scanlines of CPU time per frame.
    overclock: u8,
    /// Layout register: bit 0 asks for four separate nametables.
    layout: u8,
    /// Whether a margin decoder fills the margins: the host's margin then applies whatever
    /// the ROM asked. Set by the core, not saved.
    decoded: bool,
    /// PRG window: bit 7 on, bits 0-6 the 16 KiB bank.
    window_bank: u8,
    /// PRG window: first page of the bank.
    window_page: u8,
    /// Execution zone: bit 7 on, bits 0-6 the page shown at `$2800`-`$3FFF`.
    zone: u8,
    /// The code of the hack pack that runs, which the PRG window shows instead of PRG-ROM
    /// (empty: PRG-ROM). Not part of a state: the pack gives it again on a load or a restore.
    code: Vec<u8>,
    /// CPU cycles of the current overclock still to run with the machine held.
    stall: u32,
}

impl Default for WideExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl WideExtension {
    /// The extension of a core with widescreen off.
    #[must_use]
    pub fn new() -> Self {
        Self {
            host_margin: 0,
            host_vertical_margin: 0,
            host_options: 0,
            unlock_progress: 0,
            unlocked: false,
            requested_margin: 0,
            left_extent: 0xFF,
            right_extent: 0xFF,
            pending_high: [0; 64],
            active_high: [0; 64],
            pending_flags: [0; 64],
            active_flags: [0; 64],
            paged: None,
            built_page: 0,
            extra_oam: [0xFF; 256],
            extra_high: [0; 64],
            scratch: vec![0; RAM_SIZE],
            overclock: 0,
            layout: 0,
            decoded: false,
            window_bank: 0,
            window_page: 0,
            zone: 0,
            code: Vec::new(),
            stall: 0,
        }
    }

    /// Sets the margin the host offers on each side, in pixels; rounded down to a multiple of 8
    /// and capped at [`MAX_MARGIN`]. It survives resets and state loads: it is a host setting.
    pub fn set_host_margin(&mut self, pixels: i32) {
        self.host_margin = (pixels.clamp(0, MAX_MARGIN) / 8) * 8;
    }

    /// Sets the margin the host offers above and below, in pixels: rounded down to a multiple of
    /// 8 and capped at [`MAX_VERTICAL_MARGIN`]. Only a margin decoder fills it.
    pub fn set_host_vertical_margin(&mut self, pixels: i32) {
        self.host_vertical_margin = (pixels.clamp(0, MAX_VERTICAL_MARGIN) / 8) * 8;
    }

    /// The margin captured above and below, in pixels: the host's when a margin decoder fills
    /// the margins, otherwise 0.
    #[must_use]
    pub const fn vertical_margin(&self) -> i32 {
        if self.decoded {
            self.host_vertical_margin
        } else {
            0
        }
    }

    /// Sets the options of the ROM's built-in patch the player turned on, which the ROM reads at
    /// `$500A` (one bit each, see `builtin_patches::HackOption`).
    pub fn set_host_options(&mut self, options: u8) {
        self.host_options = options;
    }

    /// Tells the extension a margin decoder fills the margins of this game.
    pub fn set_decoded(&mut self, decoded: bool) {
        self.decoded = decoded;
    }

    /// The margin the host offers on each side, in pixels.
    #[must_use]
    pub const fn host_margin(&self) -> i32 {
        self.host_margin
    }

    /// Whether the ROM wrote the unlock sequence since the last reset.
    #[must_use]
    pub const fn unlocked(&self) -> bool {
        self.unlocked
    }

    /// The margin captured on each side, in pixels: the host's when a margin decoder fills the
    /// margins, otherwise [`WideExtension::rom_margin`].
    #[must_use]
    pub fn margin(&self) -> i32 {
        if self.decoded {
            return self.host_margin.max(self.rom_margin());
        }
        self.rom_margin()
    }

    /// The margin the nametables and sprites may show on each side, in pixels: 0 unless the
    /// ROM unlocked the window and asked for one and the host offers one, at most the 128 the
    /// nametables hold.
    #[must_use]
    pub fn rom_margin(&self) -> i32 {
        if !self.unlocked {
            return 0;
        }

        (i32::from(self.requested_margin)
            .min(self.host_margin)
            .min(NAMETABLE_MARGIN)
            / 8)
            * 8
    }

    /// The range of screen x a tile may start at and still show: `[low, high)`, `low` being the
    /// leftmost pixel and `high` one past the rightmost, both in the standard frame of
    /// reference (0 = left edge of the 256-pixel picture).
    #[must_use]
    pub fn visible_range(&self) -> (i32, i32) {
        let margin = self.rom_margin();
        (
            -margin.min(i32::from(self.left_extent)),
            256 + margin.min(i32::from(self.right_extent)),
        )
    }

    /// The range of screen x a sprite may start at and still show: [`WideExtension::visible_range`],
    /// or, when the ROM asked for sprites free of the background extents (`$5007` bit 1: a margin
    /// decoder draws the background), the margin the ROM asked for, up to 248.
    #[must_use]
    pub fn sprite_range(&self) -> (i32, i32) {
        if self.unlocked && self.layout & 2 != 0 {
            // Sprites are not read from the nametables: up to what the ROM can ask for.
            let margin = (i32::from(self.requested_margin)
                .min(self.host_margin)
                .min(MAX_ROM_MARGIN)
                / 8)
                * 8;
            return (-margin, 256 + margin);
        }
        self.visible_range()
    }

    /// Whether OAM sprite `sprite` (0..64) is drawn in the "asleep" tint (`$5101 + 4n` bit 0):
    /// an enemy a hack keeps waiting in a margin, shown apart from the ones that play.
    #[must_use]
    pub fn sprite_tinted(&self, sprite: usize) -> bool {
        self.rom_margin() > 0
            && self
                .active_flags
                .get(sprite)
                .is_some_and(|flags| flags & 1 != 0)
    }

    /// The screen x of OAM sprite `sprite` (0..64) whose OAM x byte is `low`.
    #[must_use]
    pub fn sprite_x(&self, sprite: usize, low: u8) -> i32 {
        let high = if self.rom_margin() > 0 {
            self.active_high.get(sprite).copied().unwrap_or(0)
        } else {
            0
        };
        i32::from(i16::from_le_bytes([low, high]))
    }

    /// Unlocked with the execution zone on, page 0, as a built-in patch whose code runs there
    /// asks at power-on and reset; with its data at `$3000`-`$3FFF` too when `data` (bit 6).
    pub fn boot_zone(&mut self, data: bool) {
        self.unlocked = true;
        self.zone = ZONE_ON | if data { ZONE_DATA } else { 0 };
    }

    /// Whether the execution zone holds the extension's registers and RAM at `$3000`-`$3FFF`
    /// for the zone's own instructions (`$500B` bit 6): `$5000`-`$5FFF` is then the cartridge's
    /// alone (MMC5, Namco 163), and the zone's code stops at `$2FFF`.
    #[must_use]
    pub const fn zone_data(&self) -> bool {
        self.unlocked && self.zone & (ZONE_ON | ZONE_DATA) == ZONE_ON | ZONE_DATA
    }

    /// The offset in the hack pack's code an instruction fetch of `address` reads in the
    /// execution zone, or `None` when the zone is off, there is no code, or `address` is
    /// outside it. The offset may be past the end of the code: the caller reads `$FF` there.
    ///
    /// **`#[inline(always)]`, and it has to stay.** `NesBus::read_code` calls this on **every
    /// instruction fetch**, before anything can tell whether the zone is on, so a `callq` here
    /// is a `callq` per fetch for every game whether or not it runs a pack's code. Lot MF-5b
    /// folded `Cpu6502::execute_without_operand` into `Cpu6502::step` and the cost model then
    /// stopped pasting this function at **fifteen** of its sites: out of line it costs Zelda II
    /// with its 3D pack **+16 642 696 instructions over 600 frames** against the 5 M of it that
    /// left `step`, `ppu_vbl_nmi` 9 922 675 over 300 with no pack at all, and Super Mario Bros.
    /// 40 M once `execute_without_operand` is pinned as well.
    ///
    /// Two cheaper-looking shapes were measured and are worse, so that nobody builds them again
    /// (`docs/progress/MF-5b-fix.md`, step 3): a `zone_on()` guard inlined in front of an
    /// out-of-line `zone_offset` (variant 11, +0.14 % to +0.32 % on five ROMs of PR-1's set, the
    /// guard's three loads landing on every fetch), and the same guard in front of the pinned
    /// form (variant 9, worse still). Splitting the guard chain into three early returns
    /// (variant 12) is a wash: within 0.01 % of this on every ROM of both sets.
    #[must_use]
    #[allow(clippy::inline_always)] // Measured: MF-5b-fix step 3, +16.6 M on Zelda II's pack.
    #[inline(always)]
    pub fn zone_offset(&self, address: u16) -> Option<usize> {
        if !(ZONE_FIRST..=ZONE_LAST).contains(&address)
            || !self.unlocked
            || self.zone & ZONE_ON == 0
            || self.code.is_empty()
            || (self.zone_data() && address >= ZONE_DATA_FIRST)
        {
            return None;
        }
        if address < ZONE_PAGED {
            return Some(usize::from(address - ZONE_FIRST));
        }
        Some(
            usize::from(ZONE_PAGED - ZONE_FIRST)
                + (usize::from(self.zone & ZONE_PAGE) * ZONE_PAGE_SIZE)
                + usize::from(address - ZONE_PAGED),
        )
    }

    /// Unlocked with the PRG window open on `window_bank` (bit 7 set), `window_page`, as a
    /// built-in patch with a boot window asks at power-on and reset.
    pub fn boot(&mut self, window_bank: u8, window_page: u8, over_prg_ram: bool) {
        self.unlocked = true;
        self.window_bank = window_bank;
        self.window_page = window_page;
        // The whole window, $6000-$7FFF included, for a ROM with no room for anything there;
        // up to $5FFF only for one with PRG-RAM there.
        if over_prg_ram {
            self.layout |= 0x10;
        }
    }

    /// Puts the registers back to their power-on values; the host margin is kept.
    pub fn reset(&mut self) {
        *self = Self {
            host_margin: self.host_margin,
            host_vertical_margin: self.host_vertical_margin,
            host_options: self.host_options,
            decoded: self.decoded,
            code: std::mem::take(&mut self.code),
            ..Self::new()
        };
    }

    /// Whether `address` is in the register window.
    #[must_use]
    pub const fn claims(address: u16) -> bool {
        address >= FIRST_REGISTER && address <= LAST_REGISTER
    }

    /// The value a CPU read of `address` returns, or `None` when the window does not drive the
    /// bus there (still locked, or an address without a readable register).
    #[must_use]
    pub fn read(&self, address: u16) -> Option<u8> {
        if !self.unlocked {
            return None;
        }

        match address {
            0x5000 => Some(UNLOCK[0]),
            0x5001 => Some(VERSION),
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=128.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // 0..=248.
            0x5002 => Some(self.host_margin.min(MAX_ROM_MARGIN) as u8),
            0x5003 => Some(self.requested_margin),
            0x5004 => Some(self.left_extent),
            0x5005 => Some(self.right_extent),
            0x5006 => Some(self.overclock),
            0x5007 => Some(self.layout),
            0x5008 => Some(self.window_bank),
            0x5009 => Some(self.window_page),
            0x500A => Some(self.host_options),
            0x500B => Some(self.zone),
            0x500C => Some(self.built_page),
            FIRST_RAM..=LAST_REGISTER => {
                self.scratch.get(usize::from(address - FIRST_RAM)).copied()
            }
            _ => None,
        }
    }

    /// Takes a CPU write of `address`. The write still reaches the cartridge: the window only
    /// listens.
    pub fn write(&mut self, address: u16, value: u8) {
        if address == 0x5000 {
            self.advance_unlock(value);
            return;
        }

        if !self.unlocked {
            return;
        }

        match address {
            0x5003 => self.requested_margin = value,
            0x5004 => self.left_extent = value,
            0x5005 => self.right_extent = value,
            0x5006 => self.overclock = value,
            0x5007 => self.layout = value,
            0x5008 => self.window_bank = value,
            0x5009 => self.window_page = value,
            0x500B => self.zone = value,
            0x500C => {
                self.built_page = value;
                let page = usize::from(value & 7);
                let paged = self.paged.get_or_insert_with(PagedSprites::new);
                paged.high[page] = [0; 64];
                paged.flags[page] = [0; 64];
            }
            0x5100..=0x51FF if self.layout & PAGED_SPRITES != 0 => {
                let page = usize::from(self.built_page & 7);
                let sprite = usize::from(address & 0xFF) / 4;
                let paged = self.paged.get_or_insert_with(PagedSprites::new);
                if address & 3 == 1 {
                    paged.flags[page][sprite] = value;
                } else {
                    paged.high[page][sprite] = value;
                }
            }
            0x5100..=0x51FF if address & 3 == 1 => {
                self.pending_flags[usize::from(address & 0xFF) / 4] = value;
            }
            0x5100..=0x51FF => {
                self.pending_high[usize::from(address & 0xFF) / 4] = value;
            }
            FIRST_RAM..=LAST_REGISTER => {
                if let Some(byte) = self.scratch.get_mut(usize::from(address - FIRST_RAM)) {
                    *byte = value;
                }
            }
            _ => {}
        }
    }

    fn advance_unlock(&mut self, value: u8) {
        let progress = usize::from(self.unlock_progress);
        if value == UNLOCK[progress] {
            self.unlock_progress += 1;
            if usize::from(self.unlock_progress) == UNLOCK.len() {
                self.unlocked = true;
                self.unlock_progress = 0;
            }
        } else {
            self.unlock_progress = u8::from(value == UNLOCK[0]);
        }
    }

    /// The camera report of the frame just run, if a hack's code wrote one: read and cleared,
    /// so that a frame without a report has none.
    pub fn take_camera_report(&mut self) -> Option<CameraReport> {
        let base = usize::from(CAMERA_REPORT - FIRST_RAM);
        let report = self.scratch.get_mut(base..base + 8)?;
        let status = report[0];
        if status == 0 {
            return None;
        }
        report[0] = 0;
        let coordinate = |given: u8, fraction: u8, low: usize| {
            (status & given != 0).then(|| {
                let pixels = i32::from(u16::from_le_bytes([report[low], report[low + 1]]));
                let part = if status & fraction != 0 {
                    report[low + 2]
                } else {
                    0
                };
                (pixels, part)
            })
        };
        Some(CameraReport {
            x: coordinate(REPORT_X, REPORT_X_FRACTION, 1),
            y: coordinate(REPORT_Y, REPORT_Y_FRACTION, 4),
            end: (status & REPORT_END != 0).then(|| i16::from(report[7])),
        })
    }

    /// An OAM DMA just copied a new sprite page, from CPU page `page`: the high bytes written
    /// for it become the ones in use.
    pub fn latch_sprites(&mut self, page: u8) {
        if !self.unlocked {
            return;
        }

        if self.layout & PAGED_SPRITES != 0 {
            // Each page keeps its own, whatever page the game builds meanwhile. No table yet
            // means the ROM has written none of it, which reads as zeros.
            let page = usize::from(page & 7);
            (self.active_high, self.active_flags) = self
                .paged
                .as_ref()
                .map_or(([0; 64], [0; 64]), |paged| (paged.high[page], paged.flags[page]));
        } else {
            self.latch_pending();
        }
        self.latch_extra_page();
    }

    /// Without `$5007` bit 5: the high bytes written since the last DMA become the ones in use,
    /// and the next page starts from zero.
    fn latch_pending(&mut self) {
        self.active_high = self.pending_high;
        self.active_flags = self.pending_flags;
        // Bit 3: the ROM writes the high byte of every sprite it places, so they stay with the
        // sprite page as it is, finished or not (a frame the game runs late); otherwise each page
        // starts from 0.
        if self.layout & 8 == 0 {
            self.pending_high = [0; 64];
            self.pending_flags = [0; 64];
        }
    }

    /// `$5007` bit 2: the second sprite page, latched at each OAM DMA.
    fn latch_extra_page(&mut self) {
        if self.layout & 4 != 0 {
            let page = usize::from(EXTRA_OAM - FIRST_RAM);
            let high = usize::from(EXTRA_HIGH - FIRST_RAM);
            if let Some(sprites) = self.scratch.get(page..page + 256) {
                self.extra_oam.copy_from_slice(sprites);
            }
            for (n, byte) in self.extra_high.iter_mut().enumerate() {
                *byte = self.scratch.get(high + (4 * n)).copied().unwrap_or(0);
            }
        } else {
            self.extra_oam = [0xFF; 256];
        }
    }

    /// The second sprite page latched at the last OAM DMA (`$5007` bit 2), in OAM format.
    #[must_use]
    pub const fn extra_oam(&self) -> &[u8; 256] {
        &self.extra_oam
    }

    /// The screen x of sprite `sprite` (0..64) of the second page, whose x byte is `low`.
    #[must_use]
    pub fn extra_sprite_x(&self, sprite: usize, low: u8) -> i32 {
        let high = self.extra_high.get(sprite).copied().unwrap_or(0);
        i32::from(i16::from_le_bytes([low, high]))
    }

    /// The code of the hack pack that runs (see [`crate::hack_pack`]), shown by the PRG window
    /// instead of PRG-ROM; empty for none. A reset keeps it.
    pub fn set_code(&mut self, code: &[u8]) {
        code.clone_into(&mut self.code);
    }

    /// The code [`WideExtension::set_code`] gave, empty for none.
    #[must_use]
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// The offset in PRG-ROM a CPU read of `address` sees through the PRG window, or `None`
    /// when the window is off or `address` is outside it. The offset may be past the end of
    /// a small PRG-ROM: the caller wraps it.
    #[must_use]
    pub fn prg_window(&self, address: u16) -> Option<usize> {
        // Bit 4 of $5007: the window goes on over $6000-$7FFF (a game with nothing there).
        let end = if self.layout & 0x10 == 0 {
            0x5FFF
        } else {
            0x7FFF
        };
        if !self.unlocked || self.window_bank & 0x80 == 0 || !(0x5800..=end).contains(&address) {
            return None;
        }

        Some(
            (usize::from(self.window_bank & 0x7F) * 0x4000)
                + (usize::from(self.window_page) * 0x100)
                + usize::from(address - 0x5800),
        )
    }

    /// Whether the ROM asked for four separate nametables (2 x 2): the 4 KiB of console VRAM
    /// are all used, whatever the mirroring the cartridge selects.
    #[must_use]
    pub const fn four_screen(&self) -> bool {
        self.unlocked && (self.layout & 1) != 0
    }

    /// The PPU just finished the picture (start of line 240): the overclock, if any, starts.
    pub fn arm_overclock(&mut self) {
        if self.unlocked && self.overclock > 0 {
            self.stall = u32::from(self.overclock) * CYCLES_PER_SCANLINE;
        }
    }

    /// Whether the overclock is running (the PPU and the APU still).
    #[must_use]
    pub const fn stalling(&self) -> bool {
        self.stall > 0
    }

    /// Clears the overclock register (`$5006`): no extra CPU time from the next frame on.
    pub fn clear_overclock(&mut self) {
        self.overclock = 0;
    }

    /// Ends the overclock now: the game is waiting for the next frame (see
    /// `BuiltinPatch::idle_loop`), the cycles left would only run its wait loop.
    pub fn end_stall(&mut self) {
        self.stall = 0;
    }

    /// Takes one CPU cycle of the overclock: true when the machine must stay still for it (the
    /// CPU alone runs it).
    #[inline]
    pub fn take_stall_cycle(&mut self) -> bool {
        if self.stall == 0 {
            return false;
        }

        self.stall -= 1;
        true
    }

    /// Appends the register state (the host margin excluded) to `writer`.
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.unlock_progress);
        writer.write_bool(self.unlocked);
        writer.write_u8(self.requested_margin);
        writer.write_u8(self.left_extent);
        writer.write_u8(self.right_extent);
        writer.write_blob(&self.pending_high);
        writer.write_blob(&self.active_high);
        writer.write_blob(&self.scratch);
        writer.write_u8(self.overclock);
        writer.write_u8(self.layout);
        writer.write_u8(self.window_bank);
        writer.write_u8(self.window_page);
        writer.write_u32(self.stall);
        writer.write_u8(self.zone);
    }

    /// Restores the register state written by [`WideExtension::save_state`]; `with_window`
    /// false reads the earlier block, without the PRG window registers (the window stays off),
    /// `with_zone` false the one before the execution zone (the zone stays off).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(
        &mut self,
        reader: &mut StateReader,
        with_window: bool,
        with_zone: bool,
    ) -> Result<(), InvalidState> {
        self.unlock_progress = reader.read_u8()?.min(3);
        self.unlocked = reader.read_bool()?;
        self.requested_margin = reader.read_u8()?;
        self.left_extent = reader.read_u8()?;
        self.right_extent = reader.read_u8()?;
        reader.read_blob_into(&mut self.pending_high)?;
        reader.read_blob_into(&mut self.active_high)?;
        reader.read_blob_into(&mut self.scratch)?;
        self.overclock = reader.read_u8()?;
        self.layout = reader.read_u8()?;
        if with_window {
            self.window_bank = reader.read_u8()?;
            self.window_page = reader.read_u8()?;
        } else {
            self.window_bank = 0;
            self.window_page = 0;
        }
        self.stall = reader.read_u32()?;
        self.zone = if with_zone { reader.read_u8()? } else { 0 };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlocked(host: i32) -> WideExtension {
        let mut wide = WideExtension::new();
        wide.set_host_margin(host);
        for byte in UNLOCK {
            wide.write(0x5000, byte);
        }
        wide
    }

    #[test]
    fn the_execution_zone_maps_its_resident_and_paged_parts() {
        let mut wide = unlocked(0);
        wide.set_code(&[0; 0x10]);
        assert_eq!(wide.zone_offset(0x2000), None, "the zone off");
        wide.write(0x500B, 0x80);
        assert_eq!(wide.zone_offset(0x2000), Some(0));
        assert_eq!(wide.zone_offset(0x27FF), Some(0x7FF));
        assert_eq!(wide.zone_offset(0x2800), Some(0x800));
        wide.write(0x500B, 0x82);
        assert_eq!(wide.read(0x500B), Some(0x82));
        assert_eq!(wide.zone_offset(0x2800), Some(0x800 + 2 * 0x1800));
        assert_eq!(wide.zone_offset(0x3FFF), Some(0x800 + 2 * 0x1800 + 0x17FF));
        assert_eq!(wide.zone_offset(0x1FFF), None);
        assert_eq!(wide.zone_offset(0x4000), None);
        wide.set_code(&[]);
        assert_eq!(wide.zone_offset(0x2000), None, "no pack code");
    }

    #[test]
    fn the_execution_zone_is_saved_and_an_earlier_block_leaves_it_off() {
        let mut wide = unlocked(0);
        wide.write(0x500B, 0x83);
        let mut writer = StateWriter::with_capacity(0);
        wide.save_state(&mut writer);
        let block = writer.into_vec();

        let mut restored = WideExtension::new();
        restored
            .load_state(&mut StateReader::new(&block, 0).unwrap(), true, true)
            .unwrap();
        assert_eq!(restored.read(0x500B), Some(0x83));

        // The block before the zone: the same bytes without its last one.
        let mut earlier = WideExtension::new();
        earlier
            .load_state(
                &mut StateReader::new(&block[..block.len() - 1], 0).unwrap(),
                true,
                false,
            )
            .unwrap();
        assert_eq!(earlier.read(0x500B), Some(0));
    }

    #[test]
    fn stays_transparent_until_unlocked() {
        let mut wide = WideExtension::new();
        wide.set_host_margin(88);
        wide.write(0x5003, 88);
        assert_eq!(wide.read(0x5000), None);
        assert_eq!(wide.margin(), 0);
        assert_eq!(wide.visible_range(), (0, 256));
    }

    #[test]
    fn unlock_tolerates_a_restart() {
        let mut wide = WideExtension::new();
        for byte in *b"WIWIDE" {
            wide.write(0x5000, byte);
        }
        assert!(wide.unlocked());
    }

    #[test]
    fn margin_is_the_smaller_of_both_rounded_to_a_tile() {
        let mut wide = unlocked(88);
        wide.write(0x5003, 100);
        assert_eq!(wide.margin(), 88);
        wide.write(0x5003, 60);
        assert_eq!(wide.margin(), 56);
        assert_eq!(wide.read(0x5002), Some(88));
    }

    #[test]
    fn extents_narrow_each_side() {
        let mut wide = unlocked(88);
        wide.write(0x5003, 88);
        wide.write(0x5004, 0);
        wide.write(0x5005, 40);
        assert_eq!(wide.visible_range(), (0, 296));
    }

    #[test]
    fn sprite_high_bytes_apply_from_the_next_dma_for_one_page() {
        let mut wide = unlocked(88);
        wide.write(0x5003, 88);
        wide.write(0x5100 + 4 * 5 + 3, 0xFF);
        assert_eq!(wide.sprite_x(5, 0xF0), 0xF0);
        wide.latch_sprites(2);
        assert_eq!(wide.sprite_x(5, 0xF0), -16);
        wide.latch_sprites(2);
        assert_eq!(wide.sprite_x(5, 0xF0), 0xF0);
    }

    #[test]
    fn paged_sprite_high_bytes_follow_the_page_a_dma_sends() {
        let mut wide = unlocked(88);
        wide.write(0x5003, 88);
        wide.write(0x5007, 0x20);
        wide.write(0x500C, 2);
        wide.write(0x5100 + 4 * 5, 0xFF);
        wide.write(0x5100 + 4 * 5 + 1, 1);
        wide.write(0x500C, 3);
        wide.latch_sprites(2);
        assert_eq!(wide.sprite_x(5, 0xF0), -16);
        assert!(wide.sprite_tinted(5));
        wide.latch_sprites(3);
        assert_eq!(wide.sprite_x(5, 0xF0), 0xF0);
        wide.latch_sprites(2);
        assert_eq!(wide.sprite_x(5, 0xF0), -16);
        wide.write(0x500C, 2);
        wide.latch_sprites(2);
        assert_eq!(wide.sprite_x(5, 0xF0), 0xF0);
    }

    #[test]
    fn overclock_holds_the_machine_for_whole_scanlines() {
        let mut wide = unlocked(0);
        wide.arm_overclock();
        assert!(!wide.take_stall_cycle());
        wide.write(0x5006, 2);
        wide.arm_overclock();
        let held = std::iter::from_fn(|| wide.take_stall_cycle().then_some(())).count();
        assert_eq!(held, 228);
    }

    #[test]
    fn the_prg_window_maps_a_bank_page() {
        let mut wide = unlocked(0);
        assert_eq!(wide.prg_window(0x5800), None);
        wide.write(0x5008, 0x80);
        wide.write(0x5009, 0x2B);
        assert_eq!(wide.prg_window(0x5800), Some(0x2B00));
        assert_eq!(wide.prg_window(0x5FFF), Some(0x2B00 + 0x7FF));
        assert_eq!(wide.prg_window(0x57FF), None);
        assert_eq!(
            wide.prg_window(0x6000),
            None,
            "not past $5FFF without bit 4"
        );
        wide.write(0x5007, 0x10);
        assert_eq!(wide.prg_window(0x6000), Some(0x2B00 + 0x800));
        assert_eq!(wide.prg_window(0x7FFF), Some(0x2B00 + 0x27FF));
    }

    #[test]
    fn scratch_ram_answers_once_unlocked() {
        let mut wide = WideExtension::new();
        wide.write(0x5280, 7);
        assert_eq!(wide.read(0x5280), None);
        let mut wide = unlocked(0);
        wide.write(0x5280, 7);
        assert_eq!(wide.read(0x5280), Some(7));
    }
}
