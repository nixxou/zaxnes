use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 185, CNROM with the security diodes — *Bird Week*, *B-Wings*, *Mighty Bomb Jack*
/// (J) and the *Sansuu* series (REQ-CORE-102).
///
/// nesdev ("INES Mapper 185"), one latch at $8000-$FFFF:
///
/// ```text
/// D~[..DC ..BA]
///      ||   ||
///      ||   |+- Chip Select 2 (CS2)
///      ||   +-- Chip Select 1 (CS1)
///      |+------ Output to Diode 2 (D2)
///      +------- Output to Diode 1 (D1)
/// ```
///
/// There is **no banking at all**: 32 KiB of PRG and 8 KiB of CHR, fixed. The latch is a
/// security test — the two chip-select bits must hold the one value the board's diodes are wired
/// for, or "CHR-ROM is disabled, meaning that reading from the pattern tables returns open bus
/// behavior". Submappers 4 to 7 give that value as 0, 1, 2 and 3 respectively. The page's own
/// instruction to this role is quoted so that a later lot does not add the diodes: **"emulators
/// need not emulate the functionality of security diodes to run games correctly"**.
///
/// # Why this is its own board and not a parameter of `CnRom`
///
/// §5 of `docs/review/MF-6-inventory.md` has 185 as "a submapper parameter of `CnRom`". It is
/// not, and the reason is MF-6a step 2's measurement. A disabled CHR changes **what a fetch
/// reads**, so the parameter would have to reach `Mapper::chr_view` — and a body of `CnRom`'s
/// own there is one more distinct body in the dispatch that `Ppu2C02::run_visible_span` inlines
/// into `run_dots`, which step 2 priced at **0.067 % to 0.131 % on eight of lot PR-1's thirteen
/// ROMs**. One of those thirteen, `test_ppu_read_buffer`, *is* a CNROM. So 185 goes where the
/// cost is already paid: inside [`super::SmallBoard`], whose `chr_view` answers `None` for the
/// whole variant, and **mapper 3 is not touched at all**.
///
/// # Submapper 0, and Q-131
///
/// The page's heuristic for an image with no NES 2.0 header is "disable CHR-ROM for the first
/// two reads from $2007 after a reset and then enable it" — and this core cannot see that: a
/// $2007 read and a background fetch arrive at [`Mapper::ppu_read`] as the same
/// [`ChrFetch::Background`], and telling them apart needs a hook no board has. **Q-131** holds
/// the question; the provisional choice is the *other* documented heuristic, the one Mesen 2
/// uses, which is a property of the latch and costs no read path anything: "if C AND $0F is
/// nonzero, and if C does not equal $13: CHR is enabled" (`CnromProtect.h:26-37`).
///
/// The disabled read is `the CHR byte | $01` and not zero, which is also Mesen's, for the reason
/// its comment gives: "simulate pull-up resistor on D0 when PPU reads the CHR ROM while the
/// protection is enabled — this is needed for the original version of Mighty Bomb Jack to boot".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CnRomProtect {
    banks: BankWindows,
    latch: u8,
    /// The chip-select value this board's diodes are wired for, 0 to 3, or `None` for an image
    /// whose submapper does not say (submapper 0, and every iNES 1.0 image).
    chip_select: Option<u8>,
}

impl CnRomProtect {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        let chip_select = match (header.is_nes20, header.submapper) {
            (true, submapper @ 4..=7) => u8::try_from(submapper - 4).ok(),
            _ => None,
        };
        Self {
            banks: windows_for(memory),
            latch: 0,
            chip_select,
        }
    }

    /// Whether the CHR-ROM chip is selected now.
    fn chr_enabled(&self) -> bool {
        match self.chip_select {
            Some(value) => self.latch & 0x03 == value,
            // Q-131: submapper 0's heuristic; see the note on this struct.
            None => self.latch & 0x0F != 0 && self.latch != 0x13,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(0);
        self.banks.map_chr_8k(0);
    }
}

impl Mapper for CnRomProtect {
    banked!();

    fn mapper_number(&self) -> i32 {
        185
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.latch);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.latch = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.latch = 0;
        self.update_banks();
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        let chr_rom = memory.rom().chr_rom();
        let value = self.banks.read_chr(
            address,
            if chr_rom.is_empty() {
                memory.chr_ram()
            } else {
                chr_rom
            },
        );
        if self.chr_enabled() {
            value
        } else {
            // The chip is deselected and D0 floats high.
            value | 0x01
        }
    }

    /// `#[inline(never)]`, as the other boards of this lot: see `Namcot108::write_register`.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address >= 0x8000 {
            self.latch = value;
            self.update_banks();
        }
    }
}
