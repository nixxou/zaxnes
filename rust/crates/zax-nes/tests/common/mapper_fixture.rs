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

    pub fn submapper(mut self, submapper: u8) -> Self {
        self.submapper = Some(submapper);
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
        if let Some(submapper) = self.submapper {
            image[7] |= 0x08; // NES 2.0.
            image[8] = (submapper << 4) | ((mapper >> 8) & 0x0F) as u8;
        } else if mapper >> 8 != 0 {
            image[7] |= 0x08;
            image[8] = ((mapper >> 8) & 0x0F) as u8;
        }

        for bank in 0..self.prg_banks_8k {
            let start = 16 + bank * 8 * 1024;
            image[start..start + 8 * 1024].fill(bank as u8);
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
