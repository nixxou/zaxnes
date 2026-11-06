#[cfg(feature = "board-eeprom")]
use super::EepromPort;
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 16, Bandai FCG: registers at $6000-$FFFF decoded on the low nibble, a 16-bit
/// CPU-cycle IRQ counter (REQ-CORE-100). **Mapper 153 is the same ASIC on a board with
/// battery-backed WRAM** (MF-6c step 1).
///
/// NES 2.0 submapper 4 is the FCG-1/2, which has no IRQ latch at all: $x00B and $x00C write
/// the counter itself. Submapper 5 and an undeclared dump are the LZ93D50, where they write a
/// latch that $x00A copies across (MR-16-1).
///
/// # Mapper 153, and why it is a field of this struct and not a board of its own
///
/// nesdev ("INES Mapper 153"): "iNES Mapper 153 is used for a Bandai FCG board with an LZ93D50
/// ASIC and 8192 bytes of battery-backed WRAM", and its IRQ and mirroring registers "function the
/// same way as on INES Mapper 016, submapper 5". Three things are its own:
///
/// * **$8000-$8003 are an outer PRG bank**, not CHR banks: the page gives them "Select 256 KiB
///   outer PRG-ROM bank" with bit 0 carrying it and "The same value must be written to all four
///   registers". Both PRG windows move inside that block, so the fixed one is the block's last
///   16 KiB and not the image's. "No CHR banking is available" — the 8 KiB window is wired.
/// * **$800D bit 5 is the PRG-RAM chip enable** (1 = enable), where on the LZ93D50 proper the
///   same register is the EEPROM port.
/// * **the ports answer at $8000-$FFFF only**, because $6000-$7FFF *is* the WRAM. Mesen 2 says
///   the same in as many words (`BandaiFcg.h:76`, quoting the page: "For iNES Mapper 153 (with
///   SRAM), the writeable ports must only be mirrored across $8000-$FFFF").
///
/// So mapper 153 is a parameter, carried in two `bool`s **appended** to this struct so that no
/// existing field moves, and **not** a `Board` variant, a `const` monomorphisation or an
/// [`super::IrqBoard`] arm. The reason is §4.9 of
/// `docs/review/MF-6-inventory.md` as MF-6a3 step 1 sharpened it: two
/// more `Board` discriminants cost `mmc5` **+0.080 %**, one taken `jmp` per expansion-audio
/// sample, because `Board::expansion_audio_output`'s jump table is inlined whole into
/// `NesBus::push_expansion_audio`. This board already answers *yes* to [`Mapper::irq_asserted`]
/// and already clocks on a CPU cycle, so mapper 153 adds **no arm to any dispatch at all** — the
/// cheapest shape §4.9 allows, and the one the inventory's own step for 153 asks for ("a
/// `BandaiFcg` parameter; the easy one of the group").
///
/// Behind the Cargo feature `board-153`. With the feature off, [`BandaiFcg::is_153`] is
/// `cfg!(…) && …`, so every branch below folds away at compile time and the board is mapper 16's
/// alone.
///
/// # Mappers 157 and 159, and why they are a field too (MF-6c step 3)
///
/// The Datach Joint ROM System and the LZ93D50 with an X24C01 are the same ASIC again, with a
/// serial EEPROM on $800D where mapper 153 has its WRAM enable. That EEPROM is [`EepromPort`],
/// one `Option` field behind `board-eeprom`, and the three things the two numbers add to this
/// board are the port's `write_control` on $800D, the Datach's external clock on $8000-$8003, and
/// a readable register at $6000-$7FFF. Mapper 16 and mapper 153 are untouched: the field is
/// `None` for both, and `board-eeprom` off removes it altogether.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandaiFcg {
    banks: BankWindows,
    chr: [u8; 8],
    prg: u8,
    /// Submapper 4, decided at load.
    ///
    /// **`mapper_number == 16 &&` was added by MF-6c step 1**, the tightening MF-6a3 step 1 made
    /// on the MMC3's three submapper tests and for the same reason: nesdev's *NES 2.0
    /// submappers* defines submappers 4 and 5 for mapper 16 and **none** for 153, 157 or 159, so
    /// a NES 2.0 dump of one of those carrying a submapper byte of 4 would have been read as an
    /// FCG-1/2. Identical for every mapper-16 image.
    is_fcg12: bool,
    irq_latch: i32,
    irq_counter: i32,
    irq_enabled: bool,
    irq_pending: bool,
    // **Appended, and that is the whole of what this board pays.** rustc groups a struct's
    // fields by alignment and keeps declaration order inside a group, so two more `bool`s
    // declared *after* `irq_pending` leave every existing field — `irq_pending` above all, which
    // `Board::irq_asserted` reads once per CPU cycle — at the displacement it had. MF-6b step 1
    // measured what moving that one costs: **one byte** of displacement on `VrcIrq::pending`
    // re-laid out `Cpu6502::branch` and cost eleven of PR-1's thirteen ROMs +0.009 % to
    // +0.027 % with no code added at all. Measured with `offset_of!`: `irq_latch` 120, `prg`
    // 128, `irq_enabled` 130 and `irq_pending` 131 with the two new fields, which land at 132
    // and 133 — so `size_of::<BandaiFcg>()` goes 132 → **136** and not one existing byte moves.
    // `Board` does not feel the four either: its size is the MMC5's and nothing else.
    /// Mapper 153, decided at load. See the struct's own note.
    is_153: bool,
    /// $800D bit 5 on mapper 153, the WRAM chip enable; meaningless on every other number.
    ///
    /// **Power-on is enabled.** The page gives the bit no power-on value, and Mesen 2 only calls
    /// `SetCpuMemoryMapping` when $800D is written (`BandaiFcg.h:226`), so until then its
    /// cartridge keeps `BaseMapper`'s read/write mapping of $6000-$7FFF — enabled. Taking the
    /// WRAM away from a game that never writes the register is the worse of the two errors, which
    /// is the reading `registry::board_memory_sizes` already takes for mapper 97.
    prg_ram_on: bool,
    /// Mappers **157** and **159**: the board's serial EEPROM, or EEPROMs. `None` on mappers 16
    /// and 153, which pay one null test on a cold path and nothing else. See [`EepromPort`].
    ///
    /// Appended for the reason the comment above `is_153` gives, and `#[cfg]` and not `cfg!` this
    /// time because the field is tens of bytes rather than one: with `board-eeprom` off the struct
    /// is the one step 2 shipped, byte for byte.
    #[cfg(feature = "board-eeprom")]
    eeprom: Option<EepromPort>,
}

impl BandaiFcg {
    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        let header = memory.rom().header();
        Self {
            banks: windows_for(memory),
            chr: [0; 8],
            prg: 0,
            is_fcg12: mapper == 16 && header.is_nes20 && header.submapper == 4,
            irq_latch: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: false,
            is_153: mapper == 153,
            prg_ram_on: true,
            // The second Datach chip is declared by byte 10 alone; see
            // `registry::datach_has_external_eeprom`, which `board_memory_sizes` asks the same
            // question so that the PRG-RAM is exactly the size the two chips need.
            #[cfg(feature = "board-eeprom")]
            eeprom: EepromPort::new(mapper, header.is_nes20 && header.byte10 == 0x10),
        }
    }

    /// Whether this cartridge is a mapper-153 board.
    ///
    /// `cfg!` and not `#[cfg]` on the field: the byte stays so that the struct, its `PartialEq`
    /// and its save-state shape are the same in both configurations, and the constant `false`
    /// makes every branch that reads it compile-time dead when `board-153` is off — which is what
    /// the owner's rule "with a feature off, the build is the same as without it" asks for, shown
    /// in `docs/progress/MF-6c.md` step 1 on the post-LTO dump. The two `bool`s are appended; see
    /// the comment above them.
    const fn is_153(&self) -> bool {
        cfg!(feature = "board-153") && self.is_153
    }

    /// The lowest CPU address the register ports answer at.
    ///
    /// $6000 on mapper 16, which is what MR-16-1 and the submapper table describe and what
    /// `req_core_100_bandai_fcg_banks_prg_chr_and_mirroring` has pinned since MF-1: the FCG-1/2
    /// decodes $6000-$7FFF, an undeclared dump may be either, and this build answers both ranges
    /// for the whole number rather than branching on the submapper.
    ///
    /// **$8000 on mapper 153**, whose $6000-$7FFF is the board's WRAM: nesdev ("INES Mapper
    /// 153"), quoted by Mesen 2 at `BandaiFcg.h:76`, "the writeable ports must only be mirrored
    /// across $8000-$FFFF".
    ///
    /// **$8000 on mappers 157 and 159 too** (MF-6c step 3): "Mappers 157 and 159 do not need to
    /// support the FCG-1 and -2 and so should only mirror the ports across $8000-$FFFF" (the page,
    /// quoted by Mesen 2 at `BandaiFcg.h:77`). Their $6000-$7FFF is a **readable** register — the
    /// EEPROM's data out — which is `Mapper::try_read_prg`'s business and not this floor's.
    const fn register_floor(&self) -> u16 {
        #[cfg(feature = "board-eeprom")]
        if self.eeprom.is_some() {
            return 0x8000;
        }

        if self.is_153() {
            0x8000
        } else {
            0x6000
        }
    }

    /// The counter on an FCG-1/2, the latch on an LZ93D50: `nibble` replaces the low byte when
    /// it is false and the high byte when it is true (MR-16-1).
    fn write_irq_value(&mut self, high: bool, value: u8) {
        let target = if self.is_fcg12 {
            &mut self.irq_counter
        } else {
            &mut self.irq_latch
        };
        *target = if high {
            (*target & 0x00FF) | (i32::from(value) << 8)
        } else {
            (*target & 0xFF00) | i32::from(value)
        };
    }

    /// Mapper 153's outer PRG bank: bit 0 of each of $8000-$8003, as PRG A18.
    ///
    /// nesdev ("INES Mapper 153") gives the four registers "Select 256 KiB outer PRG-ROM bank"
    /// with bit 0 carrying it and adds "The same value must be written to all four registers", so
    /// OR is the reading that agrees with the page for every conforming write and keeps a
    /// *Famicom Jump II* that writes them one at a time on the block it has just asked for.
    ///
    /// **One divergence from Mesen 2, in the direction AMD-061 §1 requires.** Mesen ORs bit 0 of
    /// all **eight** registers and applies the result to mapper 16 as well whenever an image has
    /// 512 KiB or more of PRG (`BandaiFcg.h:164-170`). The page documents the outer bank on
    /// mapper **153** only and names $8000-$8003 only, so this build does neither: mapper 16's
    /// banking is the instruction-identical code it was before this step.
    fn outer_prg_153(&self) -> i32 {
        i32::from(self.chr[0] | self.chr[1] | self.chr[2] | self.chr[3]) & 0x01
    }

    fn update_banks(&mut self) {
        if self.is_153() {
            // Both windows live inside the 256 KiB block, so the fixed one is the block's last
            // 16 KiB bank and not the image's: the page's "fixed to the last bank" with a 4-bit
            // inner field, which is `0x0F` of the block. Mesen 2 agrees
            // (`BandaiFcg.h:169-170`, `SelectPrgPage(1, 0x0F | _prgBankSelect)`).
            let outer = self.outer_prg_153() << 4;
            self.banks
                .map_prg_16k(0, outer | i32::from(self.prg & 0x0F));
            self.banks.map_prg_16k(2, outer | 0x0F);
            // "No CHR banking is available": the linear 8 KiB `BankWindows::new` laid down
            // stays, so nothing touches the CHR windows here.
            return;
        }

        self.banks.map_prg_16k(0, i32::from(self.prg));
        self.banks.map_prg_16k(2, -1);

        // nesdev ("INES Mapper 157"): the Datach board has "No CHR banking is available" and
        // "8 KiB of unbanked CHR-RAM", so its eight registers are the EEPROM clock and nothing
        // else. Mesen 2 excludes the number from its CHR path for the same reason
        // (`BandaiFcg.h:171`).
        #[cfg(feature = "board-eeprom")]
        if self.no_chr_banking() {
            return;
        }

        for slot in 0..BankWindows::CHR_WINDOWS {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }

    /// Mapper 157 alone: "No CHR banking is available."
    #[cfg(feature = "board-eeprom")]
    fn no_chr_banking(&self) -> bool {
        self.eeprom
            .as_ref()
            .is_some_and(|port| port.mapper_number() == 157)
    }
}

impl Mapper for BandaiFcg {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_u8(self.prg);
        writer.write_i32(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        // Appended by MF-6c step 1, behind the **board** that is the only reason the byte
        // exists, so that no byte of mapper 16's block moves and the format of an existing
        // board is untouched (REQ-CORE-090). `is_153` itself is not serialised: it comes from
        // the header, which `load_state` cannot contradict.
        if self.is_153() {
            writer.write_bool(self.prg_ram_on);
        }
        // Appended by MF-6c step 3, behind the board again: mapper 16's and mapper 153's blocks
        // keep every byte they had (REQ-CORE-090).
        #[cfg(feature = "board-eeprom")]
        if let Some(port) = self.eeprom.as_ref() {
            port.save_state(writer);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        self.prg = reader.read_u8()?;
        self.irq_latch = reader.read_i32()?;
        self.irq_counter = reader.read_i32()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        if self.is_153() {
            self.prg_ram_on = reader.read_bool()?;
        }
        #[cfg(feature = "board-eeprom")]
        if let Some(port) = self.eeprom.as_mut() {
            port.load_state(reader)?;
        }
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        #[cfg(feature = "board-eeprom")]
        if let Some(port) = self.eeprom.as_ref() {
            return port.mapper_number();
        }

        if self.is_153() {
            153
        } else {
            16
        }
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `false` on mapper 16: the LZ93D50's $6000-$7FFF is the register window and the EEPROM
    /// port, never RAM. **Mapper 153's board carries 8 KiB of battery-backed WRAM there**, gated
    /// by $800D bit 5; see the struct's note.
    fn prg_ram_enabled(&self) -> bool {
        self.is_153() && self.prg_ram_on
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        self.prg = 0;
        self.prg_ram_on = true;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        // The bus goes back to power-on; the bytes do not, because they are the battery's
        // (REQ-CORE-004, REQ-CORE-095). See `Eeprom24C0X::reset`.
        #[cfg(feature = "board-eeprom")]
        if let Some(port) = self.eeprom.as_mut() {
            port.reset();
        }
        self.update_banks();
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address >= self.register_floor() {
            self.write_register(memory, address, value);
            return true;
        }

        false
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < self.register_floor() {
            return;
        }

        match address & 0x000F {
            0x0..=0x7 => {
                self.chr[usize::from(address & 0x07)] = value;
                // nesdev ("INES Mapper 157"), "External EEPROM Clock ($8000-$8003 write)": bit 3
                // of the **first four** registers is the Datach subcartridge chip's SCL, because
                // the line is CHR A13 on that header. Registers 4-7 are not it.
                #[cfg(feature = "board-eeprom")]
                if address & 0x0F <= 3 {
                    if let Some(port) = self.eeprom.as_mut() {
                        port.write_external_clock(memory.prg_ram_mut(), value);
                    }
                }
                self.update_banks();
            }
            0x8 => {
                self.prg = value;
                self.update_banks();
            }
            0x9 => memory.set_mirroring(match value & 0x03 {
                0 => NametableMirroring::Vertical,
                1 => NametableMirroring::Horizontal,
                2 => NametableMirroring::SingleScreenA,
                _ => NametableMirroring::SingleScreenB,
            }),
            0xA => {
                self.irq_pending = false;
                self.irq_enabled = value & 0x01 != 0;
                // The LZ93D50's $x00A copies the latch into the counter. The FCG-1/2 has no
                // latch to copy: "writing to these two registers directly modifies the counter
                // itself" (MR-16-1).
                if !self.is_fcg12 {
                    self.irq_counter = self.irq_latch;
                }
            }
            0xB => self.write_irq_value(false, value),
            0xC => self.write_irq_value(true, value),
            // $x00D is the EEPROM port on the LZ93D50 (REQ-CORE-100: P3), and the **WRAM chip
            // enable** on mapper 153: nesdev ("INES Mapper 153") gives it "PRG-RAM Chip Enable
            // (1=Enable)" at bit 5, which is the bit Mesen 2 reads (`BandaiFcg.h:226`,
            // `value & 0x20`).
            0xD if self.is_153() => self.prg_ram_on = value & 0x20 != 0,
            // The EEPROM port proper, on mappers 157 and 159 (MF-6c step 3): bit 5 the internal
            // chip's SCL, bit 6 the SDA both chips see, bit 7 the direction (Q-134). See
            // [`EepromPort::write_control`].
            #[cfg(feature = "board-eeprom")]
            0xD => {
                if let Some(port) = self.eeprom.as_mut() {
                    port.write_control(memory.prg_ram_mut(), value);
                }
            }
            _ => {}
        }
    }

    /// PRG-ROM above $8000, and on mappers **157 and 159** the EEPROM's data out at
    /// $6000-$7FFF: "bit 4 the EEPROMs' data out, bit 3 the barcode scanner's" (nesdev's own
    /// diagram, `xxxE Bxxx`). The other six bits read **0** and not open bus, and the barcode
    /// scanner's bit reads 0 because this build has no host surface for one — both recorded on
    /// [`EepromPort`] with the measurement that forbids the alternative.
    ///
    /// Mapper 16 and mapper 153 answer exactly what `Mapper::try_read_prg`'s default answered
    /// them: the banked read above $8000 and `None` below it. **This is the hook and not
    /// [`Mapper::cpu_peek_override`]**, for the reason MF-6c step 2 measured —
    /// `Board::cpu_read_override` is `#[inline(always)]` and one more body in it cost `mmc5`
    /// +0.066 %, where a body here cost nothing.
    #[cfg(feature = "board-eeprom")]
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if address >= 0x6000 {
            if let Some(port) = self.eeprom.as_ref() {
                return Some(port.read_register());
            }
        }

        None
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.irq_enabled {
            return;
        }

        self.irq_counter -= 1;
        if self.irq_counter < 0 {
            self.irq_counter = 0xFFFF;
            self.irq_pending = true;
        }
    }
}
