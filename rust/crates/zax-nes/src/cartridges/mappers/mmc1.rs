use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 1, MMC1 (SxROM), SUROM and SOROM included (REQ-CORE-100).
///
/// Each write to $8000-$FFFF shifts one bit into a five-bit serial register and the fifth
/// write commits it to the register the address selects. Two writes on consecutive CPU
/// cycles only count once (read-modify-write instructions). On 512 KiB boards bit 4 of CHR
/// bank 0 supplies the top PRG address line, and its `S` bits bank the PRG-RAM of SOROM and
/// SXROM when the header announces more than 8 KiB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc1 {
    banks: BankWindows,
    shift: i32,
    shift_count: i32,
    control: u8,
    chr0: u8,
    chr1: u8,
    prg: u8,
    last_write_cycle: i64,
    /// NES 2.0 submapper 5 (MR-1-4), decided at load.
    fixed_prg: bool,
    /// How to read the `S` bits of CHR bank 0 for this board's PRG-RAM (MR-1-3), decided at
    /// load so that `prg_ram_offset` keeps exactly one shift and one mask.
    ram_bank_shift: u32,
    ram_bank_mask: u8,
    /// Bit 4 of the PRG register, masked: `$10` on a board that wires the PRG-RAM disable and
    /// **`$00` on mapper 155**, which has no such wire.
    ///
    /// A mask and not a `bool` tested beside the register, for the reason `ram_bank_mask` above
    /// gives and which MF-6a2 step 2 then measured: MF-6a step 4 added mapper 155 as
    /// `!self.no_ram_disable && …`, which is free on `Mapper::prg_ram_enabled`'s path — that
    /// reads the cached answer — but not on the **commit** path, where
    /// [`Mmc1::refresh_prg_ram_disable`] runs. `Mmc1::update_banks` grew **+8 001 instructions
    /// over 600 level-2 frames of Mega Man 2**, +2 217 on Metroid and +4 311 on Zelda II, all
    /// three plain mapper-1 games paying for mapper 155's existence. Folded into the mask the
    /// test disappears and mapper 1 is back to one load and one `and` per register.
    prg_disable_mask: u8,
    /// The same bit of CHR bank 0, which only an SNROM wires (see [`Mmc1::is_snrom`]) as a
    /// second chip enable for the PRG-RAM: `$10` on an SNROM that is not a mapper 155, `$00`
    /// everywhere else.
    chr_disable_mask: u8,
    /// What [`Mapper::prg_ram_enabled`] answers, recomputed whenever `prg` or `chr0` commits.
    ///
    /// A field rather than an expression because on an SNROM the answer needs *both* registers:
    /// kept as `(self.prg | (self.chr0 & mask)) & 0x10 == 0` it would put a second load and a
    /// mask on the CPU's $6000-$7FFF decode for every MMC1 game. Derived state, so it is not
    /// written to the savestate — `load_registers` recomputes it (REQ-CORE-091).
    prg_ram_disabled: bool,
}

impl Mmc1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        // MR-1-3, nesdev ("MMC1"): "For SXROM, the upper `S` (bit 3) selects the SRAM's A14,
        // and the lower `S` (bit 2) selects A13", and "The SOROM board only implements the
        // upper `S` bit". SOROM is the 16 KiB board, SXROM the 32 KiB one; our old
        // `(chr0 >> 2) & 0x03` reduced to bit 2 over a 16 KiB RAM, one bit off, which put the
        // SOROM titles' saves in the half that is not battery-backed.
        let (ram_bank_shift, ram_bank_mask) =
            match memory.prg_ram().len() / BankWindows::PRG_WINDOW_SIZE {
                0 | 1 => (0, 0x00),
                2 => (3, 0x01),
                _ => (2, 0x03),
            };
        // nesdev ("INES Mapper 155") gives the board as the MMC1 "with the WRAM disable bit
        // removed", so on mapper 155 neither wire exists and both masks are zero.
        let wired = u8::from(memory.rom().mapper() != 155) << 4;
        Self {
            banks: windows_for(memory),
            shift: 0,
            shift_count: 0,
            control: 0x0C,
            chr0: 0,
            chr1: 0,
            prg: 0,
            last_write_cycle: -4,
            fixed_prg: header.is_nes20 && header.submapper == 5,
            ram_bank_shift,
            ram_bank_mask,
            prg_disable_mask: wired,
            chr_disable_mask: if Self::is_snrom(memory) { wired } else { 0 },
            prg_ram_disabled: false,
        }
    }

    /// Whether this image is on an SNROM board, where bit 4 of the CHR bank registers disables
    /// the PRG-RAM.
    ///
    /// nesdev ("MMC1"), SNROM: the board, "which only supports 8 KiB of CHR-ROM/-RAM, uses the
    /// upper CHR bank select line coming out of the mapper (CHR A16; bit 4 of bank number) as an
    /// additional chip enable for the PRG-RAM", the `E` bit of both CHR bank registers being
    /// "PRG-RAM disable (0: enable, 1: open bus)". The same line is something else on every
    /// other MMC1 board that has it at all: on SUROM and SXROM bit 4 is `P`, the 256 KiB outer
    /// PRG bank, and on the CHR-ROM boards (SJROM, SKROM, SLROM) it is CHR A16, a real bank
    /// line. So the three sizes of the board page's table settle it, and they are exactly the
    /// three the Holy Mapperel driver probes before it decides which way its `4xxx` digit goes
    /// (`holy-mapperel/README.md`: "$A000 bit 4 does not disable WRAM (SNROM), or $A000 bit 4
    /// disables WRAM (all but SNROM)"):
    ///
    /// | board | PRG-ROM | PRG-RAM | CHR |
    /// | --- | --- | --- | --- |
    /// | SNROM | 128/256 KiB | 8 KiB | 8 KiB RAM/ROM |
    /// | SOROM | 128/256 KiB | **16 KiB** | 8 KiB RAM/ROM |
    /// | SUROM | **512 KiB** | 8 KiB | 8 KiB RAM/ROM |
    /// | SXROM | 128/256/512 KiB | **32 KiB** | 8 KiB RAM/ROM |
    ///
    /// A 512 KiB PRG-ROM is excluded by the same comparison `update_banks` uses for `outer`, so
    /// the two readings of bit 4 can never both be live.
    ///
    /// **The CHR test is on the header's CHR-*ROM*, not on the CHR-RAM buffer.** An NES 2.0
    /// image gives its CHR-RAM size exactly (`M1_P128K.nes` declares 8 KiB in byte 11), but
    /// REQ-CORE-012 hands an iNES 1.0 image with no CHR-ROM **32 KiB** of CHR-RAM, which no
    /// MMC1 board carries: every CHR-RAM MMC1 board — SNROM, SOROM, SUROM, SXROM, SGROM — has
    /// 8 KiB and no more, so a declared CHR-ROM of 0 is itself the 8 KiB board. Reading the
    /// padded buffer instead would leave every iNES 1.0 SNROM dump — The Legend of Zelda,
    /// Zelda II, Final Fantasy — on the wrong wiring. The PRG-RAM size is the effective one,
    /// which for iNES 1.0 is the 8 KiB default: a dump that cannot be told from an SNROM is an
    /// SNROM here, as it is for the test ROM's own driver.
    fn is_snrom(memory: &CartridgeMemory) -> bool {
        let header = memory.rom().header();
        header.prg_rom_size <= 256 * 1024
            && header.chr_rom_size <= BankWindows::PRG_WINDOW_SIZE
            && memory.prg_ram().len() == BankWindows::PRG_WINDOW_SIZE
    }

    /// Recomputes [`Mmc1::prg_ram_disabled`] from the two registers that drive it.
    ///
    /// Bit 4 of the PRG register disables the RAM on every board that has the chip; on an SNROM
    /// bit 4 of CHR bank 0 does too. The page gives CHR bank 1 the same `E` bit, "ignored in
    /// 8 KiB mode", and then requires that "SNROM's `E` bit and SO/U/XROM's `P` and `S` bits in
    /// both CHR bank registers must be set to the same values" — so for any program the page
    /// admits, CHR bank 0 is the whole answer, and that is also the register Holy Mapperel
    /// names. (Mesen 2 implements only the `$E000` half, `MMC1.h:87`; AMD-061 makes the public
    /// documentation prevail where the two disagree.)
    ///
    /// Mapper 155 is this chip on a board that does not wire the disable: nesdev
    /// ("INES Mapper 155") gives it as MMC1 "with the WRAM disable bit removed", and Mesen 2
    /// implements it by holding the flag at false (`MMC1_155.h:9`). Here it is the two masks
    /// being zero, so the whole expression is two loads, two `and`s and a test whatever the
    /// board — see [`Mmc1::prg_disable_mask`] for the measurement that made it a mask.
    fn refresh_prg_ram_disable(&mut self) {
        self.prg_ram_disabled =
            ((self.prg & self.prg_disable_mask) | (self.chr0 & self.chr_disable_mask)) != 0;
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.refresh_prg_ram_disable();

        // A18 of a 512 KiB board, in 16 KiB units.
        let outer = if memory.rom().prg_rom().len() > 256 * 1024 {
            i32::from((self.chr0 >> 4) & 0x01) << 4
        } else {
            0
        };
        let bank = i32::from(self.prg & 0x0F) | outer;

        // MR-1-4. NES 2.0 submapper 5 is SEROM, SHROM and SH1ROM, which "use a fixed 32k PRG
        // ROM with no banking support" (nesdev, "NES 2.0 submappers"); Dr. Mario is the dump
        // that names the board. The PRG register does nothing there.
        if self.fixed_prg {
            self.banks.map_prg_32k(0);
        } else {
            match (self.control >> 2) & 0x03 {
                // 32 KiB at $8000, the low bit of the bank number ignored.
                0 | 1 => self.banks.map_prg_32k(bank >> 1),
                // First bank of the block fixed at $8000, switchable at $C000.
                2 => {
                    self.banks.map_prg_16k(0, outer);
                    self.banks.map_prg_16k(2, bank);
                }
                // Switchable at $8000, last bank of the block fixed at $C000.
                _ => {
                    self.banks.map_prg_16k(0, bank);
                    self.banks.map_prg_16k(2, outer | 0x0F);
                }
            }
        }

        if self.control & 0x10 != 0 {
            self.banks.map_chr_4k(0, i32::from(self.chr0));
            self.banks.map_chr_4k(4, i32::from(self.chr1));
        } else {
            self.banks.map_chr_8k(i32::from(self.chr0 >> 1));
        }

        memory.set_mirroring(match self.control & 0x03 {
            0 => NametableMirroring::SingleScreenA,
            1 => NametableMirroring::SingleScreenB,
            2 => NametableMirroring::Vertical,
            _ => NametableMirroring::Horizontal,
        });
    }
}

impl Mapper for Mmc1 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_i32(self.shift);
        writer.write_i32(self.shift_count);
        writer.write_u8(self.control);
        writer.write_u8(self.chr0);
        writer.write_u8(self.chr1);
        writer.write_u8(self.prg);
        writer.write_i64(self.last_write_cycle);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.shift = reader.read_i32()?;
        self.shift_count = reader.read_i32()?;
        self.control = reader.read_u8()?;
        self.chr0 = reader.read_u8()?;
        self.chr1 = reader.read_u8()?;
        self.prg = reader.read_u8()?;
        self.last_write_cycle = reader.read_i64()?;
        self.refresh_prg_ram_disable();
        Ok(())
    }

    /// A zero [`Mmc1::prg_disable_mask`] is mapper 155 and nothing else: every other MMC1 board
    /// wires bit 4 of the PRG register to the PRG-RAM's chip enable.
    fn mapper_number(&self) -> i32 {
        if self.prg_disable_mask == 0 {
            155
        } else {
            1
        }
    }

    /// Bit 4 of the PRG register disables the PRG-RAM chip, and on an SNROM bit 4 of CHR bank 0
    /// as well (nesdev, "MMC1"). Both are folded into one bool by
    /// [`Mmc1::refresh_prg_ram_disable`].
    fn prg_ram_enabled(&self) -> bool {
        !self.prg_ram_disabled
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.shift = 0;
        self.shift_count = 0;
        // Power-on: the last bank is fixed at $C000, the only state the chip is known to come
        // up in (nesdev); everything else starts at 0.
        self.control = 0x0C;
        self.chr0 = 0;
        self.chr1 = 0;
        self.prg = 0;
        // Far enough back that the very first write is never taken for the second half of a
        // read-modify-write pair.
        self.last_write_cycle = -4;
        self.update_banks(memory);
    }

    /// The `S` bits of CHR bank 0 pick the 8 KiB PRG-RAM chip; which bits, and whether there
    /// are any, is `Mmc1::new`'s decision (MR-1-3).
    fn prg_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        let bank = usize::from((self.chr0 >> self.ram_bank_shift) & self.ram_bank_mask);
        (bank * BankWindows::PRG_WINDOW_SIZE + usize::from(address - 0x6000))
            % memory.prg_ram().len()
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let ignored = memory.cpu_cycle() - self.last_write_cycle < 2;
        self.last_write_cycle = memory.cpu_cycle();

        // MR-1-1, nesdev ("MMC1"): the consecutive-write restriction "only applies to the data
        // being written on bit 0; the bit 7 reset is never ignored" — the sentence that names
        // Shinsenden, whose reset arrives as the second half of a read-modify-write. The test
        // used to sit above this write, so the reset was swallowed with the data.
        if value & 0x80 != 0 {
            self.shift = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            self.update_banks(memory);
            return;
        }

        if ignored {
            return;
        }

        self.shift |= i32::from(value & 0x01) << self.shift_count;
        self.shift_count += 1;
        if self.shift_count < 5 {
            return;
        }

        let data = u8::try_from(self.shift & 0x1F).unwrap_or(0);
        self.shift = 0;
        self.shift_count = 0;
        match address & 0x6000 {
            0x0000 => self.control = data,
            0x2000 => self.chr0 = data,
            0x4000 => self.chr1 = data,
            _ => self.prg = data,
        }

        self.update_banks(memory);
    }
}
