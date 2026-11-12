use super::{Mapper, Mmc3ChrRam192, Mmc3ChrRam194, Mmc3ChrRam74, WaixingFs303};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The four numbers of `board-mmc3-chrram` — **74, 192, 194 and 195** — behind one
/// [`super::IrqBoard`] variant (MF-UNL-2 step 2).
///
/// # Why an inner enum and not four variants
///
/// Each number is its own `const` monomorphisation of [`super::Mmc3Board`] — that is what the
/// `CHR_RAM_*` parameters are for, and `Mmc3Board`'s own note has the six shapes MF-6a2 step 3
/// measured for exactly this choice — so the four cannot share one Rust type. They *can* share
/// one `IrqBoard` arm, which is what this enum is: one inner match is **one body** in
/// `IrqBoard::ppu_peek`, `IrqBoard::write_register` and the rest, for the whole feature, instead
/// of four. That is [`super::IrqBoard`]'s own argument read one dispatch further down, and it is
/// what keeps the price of this feature on the four boards that need it.
///
/// Boxed in the variant that holds it, for the reason `IrqBoard::Somari` is boxed: MF-6b step 1
/// measured **one byte** of displacement in that enum's payload at +0.009 % to +0.027 % on
/// eleven of lot PR-1's thirteen ROMs, and this enum is an `Mmc3Board` plus a discriminant.
/// The price is one dereference per forwarded call for a cartridge of this family.
///
/// # What is forwarded
///
/// Everything [`super::Mmc3Board`] implements and nothing else, which for this family is the
/// whole of the MMC3 — the bank registers, the A12 counter, `$A001`'s PRG-RAM protection, the
/// two `#[inline(never)]` CHR bodies — plus mapper 195's GAL register. `ppu_irq_horizon` and
/// `chr_view` are **not** here: [`super::IrqBoard`] answers both for its whole variant, which is
/// the trade its note describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mmc3ChrRamBoard {
    /// Mapper 74, the Waixing 43-393/43-406/860908C: 2 KiB of CHR-RAM at banks `$08-$09`.
    Mapper074(Mmc3ChrRam74),
    /// Mapper 192, the Waixing FS308: 4 KiB at banks `$08-$0B`.
    Mapper192(Mmc3ChrRam192),
    /// Mapper 194: 2 KiB at banks `$00-$01`.
    Mapper194(Mmc3ChrRam194),
    /// Mapper 195, the Waixing FS303: 8 KiB wired, the window chosen at run time by the GAL.
    WaixingFs303(WaixingFs303),
}

impl Mmc3ChrRamBoard {
    /// The board of `mapper`, or `None` when the number is not one of this family's four.
    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Option<Self> {
        Some(match mapper {
            74 => Self::Mapper074(Mmc3ChrRam74::new(memory, mapper)),
            192 => Self::Mapper192(Mmc3ChrRam192::new(memory, mapper)),
            194 => Self::Mapper194(Mmc3ChrRam194::new(memory, mapper)),
            195 => Self::WaixingFs303(WaixingFs303::new(memory, mapper)),
            _ => return None,
        })
    }

    /// The CHR-RAM chip each number carries, in bytes: the `Size of RAM` column of nesdev's
    /// *MMC3 with CHR ROM and CHR RAM*, which `registry::board_memory_sizes` settles at load
    /// because no header field can state it.
    ///
    /// Mapper 195 is the one that is not its own window's width — see [`WaixingFs303`].
    pub(crate) const fn chr_ram_size(mapper: i32) -> Option<usize> {
        Some(match mapper {
            74 | 194 => Mmc3ChrRam74::CHR_RAM_SIZE,
            192 => Mmc3ChrRam192::CHR_RAM_SIZE,
            195 => WaixingFs303::CHR_RAM_SIZE,
            _ => return None,
        })
    }

    /// The 4 KiB of PRG-RAM the FS303's **compatible board** carries at CPU `$5000-$5FFF`, and
    /// `None` for the family's other three numbers (CHANGE 2 of `docs/review/Q-XCHECK.md`,
    /// Q-142, Q-173).
    ///
    /// nesdev ("INES Mapper 195"): "A compatible circuit board (with unknown PCB code) **adds**
    /// 4 KiB of non-battery-backed PRG-RAM (recognizable as such in the NES 2.0 header) mapped to
    /// CPU `$5000-$5FFF`", used by *天神之剑* — Waixing's translation of *Crystalis*.
    ///
    /// **It is a chip of its own and not a second view of the WRAM**, which is the page's "adds",
    /// and both implementations that have the window agree: FCEUX gives mapper 195 — and no other
    /// MMC3 clone — **16 KiB** of WRAM and maps 4 KiB bank 2 of it at `$5000`
    /// (`third_party/fceux/src/boards/mmc3.cpp:1036-1041`, `:1044`), and puNES maps the same bank
    /// 2 (`third_party/punes/src/core/mappers/mapper_195.c:134-137`). Both therefore put it
    /// **past** the 8 KiB that `$6000-$7FFF` shows.
    ///
    /// So this board's PRG-RAM is **12 KiB in one allocation**, laid out the way
    /// `registry::battery_offset`'s two other rows already lay out a board with two RAM chips —
    /// the volatile one first, the battery-backed one as the tail:
    ///
    /// | bytes | chip | window |
    /// | --- | --- | --- |
    /// | `0..0x1000` | the compatible board's 4 KiB, **non-battery-backed** | `$5000-$5FFF` |
    /// | `0x1000..0x3000` | the ordinary 8 KiB WRAM | `$6000-$7FFF` |
    ///
    /// That order is what keeps the page's "non-battery-backed" true and the save file 8 KiB:
    /// `battery_offset` answers `EXTRA_RAM_SIZE` for this number, so `Cartridge::battery_ram` is
    /// the WRAM alone and the eleven ordinary FS303 titles save exactly what they saved before.
    /// It is also why [`Mmc3ChrRamBoard::prg_ram_window`] exists: `$6000` is at `0x1000`.
    ///
    /// **Unconditional**, as in FCEUX and in puNES, and *not* gated on the NES 2.0 header. The
    /// page's "recognizable as such in the NES 2.0 header" is how a *reader* tells the two boards
    /// apart, not something a game can observe: an image that never touches `$5000-$5FFF` cannot
    /// see the window, and an iNES 1.0 dump of *天神之剑* would be dead under the gate. **Q-173**
    /// puts both halves — the gate and this layout — to R0.
    // Q-173
    #[cfg(feature = "board-195-prg-ram-5000")]
    pub(crate) const EXTRA_RAM_SIZE: usize = 4 * 1024;

    /// The index in PRG-RAM of an address in the `$5000-$5FFF` window, which is CPU A0-A11
    /// straight into the chip — the window is the chip's whole width, so there is nothing to
    /// wrap and nothing to bank.
    #[cfg(feature = "board-195-prg-ram-5000")]
    const fn extra_ram_offset(address: u16) -> usize {
        (address as usize) & (Self::EXTRA_RAM_SIZE - 1)
    }

    /// Whether this is the FS303, the one number of the four whose board has the `$5000-$5FFF`
    /// window. A `const`-free test of the discriminant, which is all the other three pay.
    #[cfg(feature = "board-195-prg-ram-5000")]
    const fn has_extra_ram(&self) -> bool {
        matches!(self, Self::WaixingFs303(_))
    }
}

/// Expands `$body` once per number with it bound to `$b`, as `IrqBoard`'s own `inner!` does.
macro_rules! inner {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            Mmc3ChrRamBoard::Mapper074($b) => $body,
            Mmc3ChrRamBoard::Mapper192($b) => $body,
            Mmc3ChrRamBoard::Mapper194($b) => $body,
            Mmc3ChrRamBoard::WaixingFs303($b) => $body,
        }
    };
}

impl Mapper for Mmc3ChrRamBoard {
    fn mapper_number(&self) -> i32 {
        inner!(self, b => b.mapper_number())
    }

    fn banks(&self) -> Option<&BankWindows> {
        inner!(self, b => b.banks())
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        inner!(self, b => b.banks_mut())
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        inner!(self, b => b.save_registers(writer));
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        inner!(self, b => b.load_registers(reader))
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        inner!(self, b => b.reset(memory));
    }

    /// A press on RESET, forwarded so that a **reset-detecting multicart** inside this dispatch
    /// hears it: see [`Mapper::soft_reset`], whose default is to do nothing. One cold body in a
    /// dispatch nothing hot inlines — this is called once per reset, not once per cycle.
    fn soft_reset(&mut self, memory: &mut CartridgeMemory) {
        inner!(self, b => b.soft_reset(memory));
    }

    fn irq_asserted(&self) -> bool {
        inner!(self, b => b.irq_asserted())
    }

    fn prg_ram_enabled(&self) -> bool {
        inner!(self, b => b.prg_ram_enabled())
    }

    fn prg_ram_writable(&self) -> bool {
        inner!(self, b => b.prg_ram_writable())
    }

    fn nametable_page(&self, table: i32) -> i32 {
        inner!(self, b => b.nametable_page(table))
    }

    /// Where `$6000-$7FFF` lands in PRG-RAM. On the FS303 it is `EXTRA_RAM_SIZE` and not zero,
    /// because the compatible board's 4 KiB is the head of the same allocation; see
    /// [`Mmc3ChrRamBoard::EXTRA_RAM_SIZE`]. The other three numbers answer the trait's own
    /// default, as they did before this method existed.
    ///
    /// The wrap is the whole allocation, so the window never reaches the extra chip from this
    /// side: `$6000-$7FFF` is 8 KiB and `0x1000 + 0x2000` is the allocation's end exactly.
    #[cfg(feature = "board-195-prg-ram-5000")]
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        let length = memory.prg_ram().len();
        if self.has_extra_ram() && length > Self::EXTRA_RAM_SIZE {
            (Self::EXTRA_RAM_SIZE, length)
        } else {
            (0, length)
        }
    }

    /// The FS303 compatible board's `$5000-$5FFF`, and the MMC3's own decode for everything else.
    ///
    /// Read **here** and not through `Cartridge`'s cached base, which only covers `$6000-$7FFF`
    /// — the shape `Mmc3Outer::try_read_prg` already uses for mapper 268's second window.
    ///
    /// The MMC3's `$A001` does **not** gate it: the page gives the chip to a different PCB with
    /// no register of its own, and both FCEUX and puNES install a plain read handler over the
    /// range. `prg_ram_enabled` is therefore not consulted, which is also what makes the window
    /// usable before the game has written `$A001`.
    #[cfg(feature = "board-195-prg-ram-5000")]
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if (0x5000..=0x5FFF).contains(&address) && self.has_extra_ram() {
            let ram = memory.prg_ram();
            if ram.len() >= Self::EXTRA_RAM_SIZE {
                return Some(ram[Self::extra_ram_offset(address)]);
            }
        }

        inner!(self, b => b.try_read_prg(memory, address))
    }

    /// `Cartridge::cpu_write` sends everything outside `$6000-$7FFF` here, so the FS303's
    /// `$5000-$5FFF` window takes its writes in this body — the counterpart of
    /// [`Mmc3ChrRamBoard::try_read_prg`] above, and ungated by `$A001` for the same reason.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        #[cfg(feature = "board-195-prg-ram-5000")]
        if (0x5000..=0x5FFF).contains(&address) && self.has_extra_ram() {
            let ram = memory.prg_ram_mut();
            if ram.len() >= Self::EXTRA_RAM_SIZE {
                ram[Self::extra_ram_offset(address)] = value;
            }

            return;
        }

        inner!(self, b => b.write_register(memory, address, value));
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        inner!(self, b => b.chr_peek_view(memory))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        inner!(self, b => b.ppu_peek(memory, address, fetch))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.ppu_write(memory, address, value));
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        inner!(self, b => b.on_ppu_address_bus(memory, address, clock));
    }
}
