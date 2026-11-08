//! Port of `MapperFixture.cs`: numbered-bank ROM images for the mapper tests. Every 8 KiB PRG
//! bank and every 1 KiB CHR bank is filled with its own number, so the bank a window shows
//! can be read straight off the first byte.

// Bank numbers and mapper numbers are small: the casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use zax_nes::cartridges::ChrFetch;
use zax_nes::NesCore;

/// Description of a mapper test image.
#[derive(Debug, Clone, Copy)]
pub struct MapperRom {
    mapper: usize,
    prg_banks_8k: usize,
    chr_banks_1k: usize,
    mirroring: u8,
    battery: bool,
    submapper: Option<u8>,
    prg_ram_byte10: Option<u8>,
    chr_ram_byte11: Option<u8>,
    prg_bank_all_ones: Option<usize>,
}

impl MapperRom {
    /// Defaults of the C# fixture: 8 PRG banks of 8 KiB, 16 CHR banks of 1 KiB.
    pub fn new(mapper: usize) -> Self {
        Self {
            mapper,
            prg_banks_8k: 8,
            chr_banks_1k: 16,
            mirroring: 0,
            battery: false,
            submapper: None,
            prg_ram_byte10: None,
            chr_ram_byte11: None,
            prg_bank_all_ones: None,
        }
    }

    pub fn prg(mut self, banks_8k: usize) -> Self {
        self.prg_banks_8k = banks_8k;
        self
    }

    pub fn chr(mut self, banks_1k: usize) -> Self {
        self.chr_banks_1k = banks_1k;
        self
    }

    pub fn mirroring(mut self, bits: u8) -> Self {
        self.mirroring = bits;
        self
    }

    pub fn battery(mut self) -> Self {
        self.battery = true;
        self
    }

    /// Fills 8 KiB PRG bank `bank` with $FF instead of its own number, so that a write landing
    /// in that window on a board with **bus conflicts** (MR-BUS-1: mappers 2, 3, 7, 11, 30, 34
    /// BNROM and 66) latches the value the CPU wrote rather than the AND of it with a
    /// zero-filled bank.
    ///
    /// Needed because the numbered banks only ever carry bits 0-3, so no register field above
    /// bit 3 could survive a conflicting write through one. A test that uses this cannot
    /// identify that bank by number any more, which is why it takes the bank rather than
    /// filling the image.
    pub fn prg_bank_all_ones(mut self, bank: usize) -> Self {
        self.prg_bank_all_ones = Some(bank);
        self
    }

    pub fn submapper(mut self, submapper: u8) -> Self {
        self.submapper = Some(submapper);
        self
    }

    /// NES 2.0 byte 10: `volatile` bytes of work RAM and `non_volatile` bytes of
    /// battery-backed RAM. The header stores each as `64 << shift`, so both must be 64 times a
    /// power of two, or zero. Setting this makes the image an NES 2.0 one.
    ///
    /// The MMC1 boards need it: SOROM is 8 KiB of each and SXROM 32 KiB of one, and the `S`
    /// bits of CHR bank 0 are read differently on the two (MR-1-3, MR-1-5).
    pub fn prg_ram(mut self, volatile: usize, non_volatile: usize) -> Self {
        let shift = |bytes: usize| -> u8 {
            if bytes == 0 {
                0
            } else {
                assert!(
                    bytes >= 64 && (bytes / 64).is_power_of_two(),
                    "byte 10 stores 64 << shift"
                );
                (bytes / 64).trailing_zeros() as u8
            }
        };
        self.prg_ram_byte10 = Some((shift(non_volatile) << 4) | shift(volatile));
        self
    }

    /// NES 2.0 byte 11, the **CHR-RAM** size, stored as `64 << shift` like byte 10's two
    /// nibbles. Setting it makes the image an NES 2.0 one.
    ///
    /// The MMC1 needs it since MF-LEG-1 step 1: an SNROM is told from an SGROM by the header
    /// declaring 8 KiB of CHR-RAM and 8 KiB of PRG-RAM, where REQ-CORE-012 gives an iNES 1.0
    /// image with no CHR-ROM 32 KiB of CHR-RAM and no way to say otherwise.
    pub fn chr_ram(mut self, bytes: usize) -> Self {
        assert!(
            bytes == 0 || (bytes >= 64 && (bytes / 64).is_power_of_two()),
            "byte 11 stores 64 << shift"
        );
        self.chr_ram_byte11 = Some(if bytes == 0 {
            0
        } else {
            (bytes / 64).trailing_zeros() as u8
        });
        self
    }

    /// The iNES image.
    pub fn image(&self) -> Vec<u8> {
        let mapper = self.mapper;
        let prg_size = self.prg_banks_8k * 8 * 1024;
        let chr_size = self.chr_banks_1k * 1024;
        let mut image = vec![0u8; 16 + prg_size + chr_size];
        image[..4].copy_from_slice(&[0x4E, 0x45, 0x53, 0x1A]);
        image[4] = (prg_size / (16 * 1024)) as u8;
        image[5] = (chr_size / (8 * 1024)) as u8;
        image[6] =
            (((mapper & 0x0F) << 4) as u8) | self.mirroring | if self.battery { 0x02 } else { 0 };
        image[7] = (mapper & 0xF0) as u8;
        if self.submapper.is_some()
            || self.prg_ram_byte10.is_some()
            || self.chr_ram_byte11.is_some()
            || mapper >> 8 != 0
        {
            image[7] |= 0x08; // NES 2.0.
            image[8] = (self.submapper.unwrap_or(0) << 4) | ((mapper >> 8) & 0x0F) as u8;
            // Byte 10 of zero reads back as the 8 KiB default, which is what an iNES image
            // gets too, so an image that only wants a submapper is unaffected.
            image[10] = self.prg_ram_byte10.unwrap_or(0);
            // Byte 11 of zero on an image with no CHR-ROM reads back as REQ-CORE-012's 32 KiB,
            // which is what an iNES image gets, so the same holds for this one.
            image[11] = self.chr_ram_byte11.unwrap_or(0);
        }

        for bank in 0..self.prg_banks_8k {
            let start = 16 + bank * 8 * 1024;
            let fill = if self.prg_bank_all_ones == Some(bank) {
                0xFF
            } else {
                bank as u8
            };
            image[start..start + 8 * 1024].fill(fill);
        }

        for bank in 0..self.chr_banks_1k {
            let start = 16 + prg_size + bank * 1024;
            image[start..start + 1024].fill(bank as u8);
        }

        // An endless loop at the start of the last PRG bank, and the three vectors pointing
        // at it, so that the CPU has somewhere to go.
        let last = 16 + prg_size - 8 * 1024;
        image[last..last + 3].copy_from_slice(&[0x4C, 0x00, 0xE0]);
        let vectors = 16 + prg_size - 6;
        for i in 0..3 {
            image[vectors + 2 * i] = 0x00;
            image[vectors + 2 * i + 1] = 0xE0;
        }

        image
    }

    /// Loads the image and checks the mapper number.
    pub fn load(&self) -> NesCore {
        let mut core = NesCore::new();
        let load = core.load_rom(&self.image(), &format!("Mapper{}.nes", self.mapper));
        assert!(load.success, "{:?}", load.error);
        assert_eq!(
            self.mapper,
            core.cartridge().unwrap().mapper_number() as usize
        );
        core
    }
}

/// `MapperFixture.Load(mapper)` with the defaults.
pub fn load(mapper: usize) -> NesCore {
    MapperRom::new(mapper).load()
}

/// The helpers of `MapperFixture` that act on a machine.
pub trait MapperMachine {
    fn prg_windows(&self) -> [u8; 4];
    fn chr_bank(&self, address: u16) -> u8;
    fn chr_windows(&self) -> [u8; 8];
    fn mapper_write(&mut self, address: u16, value: u8);
    fn mmc1_write(&mut self, address: u16, value: u8);
    fn mmc3_bank(&mut self, register: u8, value: u8);
}

impl MapperMachine for NesCore {
    fn prg_windows(&self) -> [u8; 4] {
        [0x8100, 0xA100, 0xC100, 0xE100].map(|address| self.read_memory(address))
    }

    fn chr_bank(&self, address: u16) -> u8 {
        self.cartridge()
            .unwrap()
            .ppu_peek(address, ChrFetch::Background)
    }

    fn chr_windows(&self) -> [u8; 8] {
        std::array::from_fn(|slot| self.chr_bank(slot as u16 * 0x0400))
    }

    /// Two ticks first, so that two writes are never on consecutive cycles (MMC1).
    fn mapper_write(&mut self, address: u16, value: u8) {
        self.bus_mut().tick();
        self.bus_mut().tick();
        self.write_memory(address, value);
    }

    fn mmc1_write(&mut self, address: u16, value: u8) {
        for bit in 0..5 {
            self.mapper_write(address, (value >> bit) & 0x01);
        }
    }

    fn mmc3_bank(&mut self, register: u8, value: u8) {
        self.mapper_write(0x8000, register);
        self.mapper_write(0x8001, value);
    }
}

/// Drives the MMC5's in-frame detection the way the PPU's fetch path does, so that a test can
/// arm the flag without running a frame (finding MR-5-4; nesdev, "MMC5").
///
/// Five reads: the same nametable address three times — dots 337, 338 and 340 of a line, the
/// fetch of the next line's first tile and the two the hardware performs and discards — then the
/// attribute fetch that is "the point at which the scanline is detected", then the nametable
/// fetch that turns the flag on.
///
/// Leaves the scanline counter at 0, i.e. the board has detected its **first** line.
pub fn arm_mmc5_in_frame(core: &mut NesCore) {
    detect_mmc5_scanline(core, 1);
    let cartridge = core.cartridge_mut().expect("a cartridge");
    cartridge.on_ppu_address_bus(0x2001, 0);
}

/// Drives `lines` scanline detections on the MMC5's bus: per line, the three identical
/// nametable reads and the attribute read that follows them.
///
/// The nametable fetch that raises the flag is **not** included, so a caller can tell the
/// detection apart from the flag; [`arm_mmc5_in_frame`] adds it.
pub fn detect_mmc5_scanline(core: &mut NesCore, lines: usize) {
    let cartridge = core.cartridge_mut().expect("a cartridge");
    for _ in 0..lines {
        for address in [0x2000u16, 0x2000, 0x2000, 0x23C0] {
            cartridge.on_ppu_address_bus(address, 0);
        }
    }
}
