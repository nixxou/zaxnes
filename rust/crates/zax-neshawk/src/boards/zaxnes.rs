//! The fallback: a mapper with no ported NesHawk board runs on **Zaxnes's** board.
//!
//! **Not a port of anything.** The owner's decision of 2026-11-12: port faithfully only the
//! boards covering about 80-95 % of the commercial library, and for any other mapper fall back on
//! `zax-nes`'s own board through an adapter, accepting the loss of NesHawk timing fidelity and
//! marking that case explicitly — "board from Zaxnes, not NesHawk-exact" — in the capabilities,
//! the log and the harness output. Never a load error.
//!
//! Why a fallback rather than a refusal: a user who picks the NesHawk engine for a game has a
//! reason (a `.bk2` to replay, a second opinion on Zaxnes's core), and a game that will not start
//! at all serves that reason worse than one whose cartridge is emulated by the other engine's
//! board. The CPU, the PPU and the APU are still NesHawk's — it is one component that differs,
//! and the marker says so everywhere a user or a tool can see it.
//!
//! # What is lost, precisely
//!
//! The two emulators drive a board on different pins, and three differences are structural rather
//! than a matter of effort. They are listed here because "not NesHawk-exact" is otherwise a
//! phrase that means nothing:
//!
//! 1. **The IRQ counter's clock.** A NesHawk board counts PPU A12 rises seen through
//!    `AddressPPU`/`ClockPPU`; a Zaxnes board of the same mapper may count them through its own
//!    `A12Watcher` with its own filter, or count scanlines. For an MMC3-like mapper that is a
//!    difference of a cycle or two on the IRQ, which is exactly the kind of thing a `.bk2`
//!    notices.
//! 2. **CPU-bus reads with side effects.** NesHawk's `INesBoard` read hooks are `&self` in this
//!    port (every ported board leaves no trace on a read), while `Cartridge::try_cpu_read` is
//!    `&mut`: the MMC5's `$5204` clears its IRQ flag when read, and a VRC has registers like it.
//!    The adapter keeps the cartridge in a `RefCell` so those side effects **do** happen, which
//!    is the one place this file is cleverer than it looks.
//! 3. **The nametables.** NesHawk's CIRAM is on the console and a board only supplies a mirroring
//!    map; Zaxnes's cartridge may serve nametable bytes itself (the MMC5's `ExRAM`, UNROM 512's
//!    four-screen CHR-RAM). The adapter honours that through `nametable_page`, but the console's
//!    2 KiB and the cartridge's view of them are two memories here where in Zaxnes they are one.
//!
//! Everything else — the banking, the PRG-RAM, the CHR-RAM, the battery, the bus conflicts — is
//! Zaxnes's board doing its job, which it does well: `test-roms --engine zaxnes` passes 212.

use std::cell::RefCell;

use zax_nes::cartridges::{Cartridge, ChrFetch};
use zax_nes::rom::RomImage;
use zax_nes::NametableMirroring;

use super::{Board, BoardBase, BoardEnv};
use crate::cart::DetectionOrigin;
use crate::state::{StateReader, StateWriter};

/// The name this board reports, which is what the load report, the capabilities and the harness
/// print. It names the other engine on purpose: a line of a log that says `ZaxnesBoard` cannot be
/// mistaken for a ported NesHawk class.
pub const BOARD_NAME: &str = "ZaxnesBoard (not NesHawk-exact)";

/// A `zax-nes` cartridge behind NesHawk's [`Board`].
pub struct ZaxnesBoard {
    /// Empty but for the mirroring map: this board's memories are the cartridge's, and
    /// `BoardBase`'s `rom`/`vrom`/`wram`/`vram` stay unused so that nothing reads them by
    /// accident.
    base: BoardBase,
    /// Behind a `RefCell` because NesHawk's CPU-side read hooks are `&self` and
    /// `Cartridge::try_cpu_read` is `&mut` — see point 2 of this module's header. Safe Rust, one
    /// borrow per call, never held across a call out.
    cart: RefCell<Cartridge>,
    /// The mapper number, for the log and the report.
    mapper: i32,
    /// PPU cycles since power-on, which `Cartridge::on_ppu_address_bus` takes and NesHawk's
    /// `AddressPPU` does not carry. Counted here from the board's own `ClockPPU`.
    ppu_clock: i64,
}

impl ZaxnesBoard {
    /// Builds the adapter from the raw `.nes` image, or `None` when `zax-nes` has no board for it
    /// either — in which case the load fails, as it must.
    ///
    /// `name` is only for `zax-nes`'s own error messages.
    pub fn try_new(image: &[u8], name: &str) -> Option<Self> {
        let rom = RomImage::try_load(image, name).ok()?;
        let mapper = rom.mapper();
        let cart = Cartridge::create(rom).ok()?;
        let mut board = Self {
            base: BoardBase::default(),
            cart: RefCell::new(cart),
            mapper,
            ppu_clock: 0,
        };
        board.refresh_mirroring();
        Some(board)
    }

    /// Whether `zax-nes` has a board for mapper `mapper`, which is what decides whether the
    /// fallback can be offered at all.
    #[must_use]
    pub fn supports(mapper: i32) -> bool {
        Cartridge::is_supported(mapper)
    }

    /// The mapper number this board is emulating.
    #[must_use]
    pub const fn mapper(&self) -> i32 {
        self.mapper
    }

    /// Copies the cartridge's current nametable layout into [`BoardBase`]'s four-entry map, so
    /// that `apply_mirroring` — which the PPU reaches through this board — gives the page Zaxnes's
    /// own PPU would read. Called after every write that could have changed it, which is what a
    /// NesHawk board does in its own `SetMirroring` call.
    fn refresh_mirroring(&mut self) {
        let cart = self.cart.borrow();
        let pages = [0, 1, 2, 3].map(|table| {
            let page = cart.nametable_page(table);
            if page >= 0 {
                return page;
            }
            // `FOLLOW_MIRRORING`: the layout `Cartridge::mirroring` describes. `FourScreen` wants
            // 4 KiB of VRAM, which the console has not got — NesHawk gives it to the *board*
            // (`Cart.VramSize = 4`), and a fallback board has no VRAM of its own, so the two
            // extra tables mirror the first two. That is a stated limitation, not a guess: a
            // four-screen game on the fallback shows the wrong two nametables, and the load
            // report says the board is not NesHawk-exact.
            match cart.mirroring() {
                NametableMirroring::Horizontal => table / 2,
                NametableMirroring::Vertical => table % 2,
                NametableMirroring::SingleScreenA => 0,
                NametableMirroring::SingleScreenB => 1,
                NametableMirroring::FourScreen => table % 2,
            }
        });
        drop(cart);
        self.base
            .set_mirroring(pages[0], pages[1], pages[2], pages[3]);
    }

    /// Whether the cartridge answers nametable address `addr` itself rather than through the
    /// console's CIRAM (the MMC5's `ExRAM`, UNROM 512's four-screen CHR-RAM).
    fn cartridge_serves_nametable(&self, addr: i32) -> bool {
        let table = (addr >> 10) & 3;
        self.cart.borrow().nametable_page(table) == Cartridge::SERVED_BY_CARTRIDGE
    }
}

impl Clone for ZaxnesBoard {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            cart: RefCell::new(self.cart.borrow().clone()),
            mapper: self.mapper,
            ppu_clock: self.ppu_clock,
        }
    }
}

impl std::fmt::Debug for ZaxnesBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZaxnesBoard")
            .field("mapper", &self.mapper)
            .finish_non_exhaustive()
    }
}

impl Board for ZaxnesBoard {
    fn base(&self) -> &BoardBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }

    fn name(&self) -> &'static str {
        BOARD_NAME
    }

    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    /// Always accepts: this board is not in `FindBoard`'s list and is only ever built by name,
    /// after every ported class has refused the cartridge.
    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        true
    }

    /// Nothing to do: `Cartridge::create` has already sized the PRG-RAM and the CHR-RAM, and
    /// `BoardBase`'s own memories stay empty.
    fn post_configure(&mut self) {}

    /// Always, so that `ClockPPU` is called and [`Self::ppu_clock`] advances —
    /// `Cartridge::on_ppu_address_bus` takes a PPU clock and there is nowhere else to count it.
    fn has_clock_ppu(&self) -> bool {
        true
    }

    fn clock_ppu(&mut self) {
        self.ppu_clock += 1;
    }

    fn clock_cpu(&mut self) {
        self.cart.borrow_mut().step_cpu_cycle();
    }

    /// `BoardSystemHardReset` for this board: `Cartridge::reset` is Zaxnes's own power-on, which
    /// clears the CHR-RAM and the non-battery PRG-RAM and puts every bank register back.
    fn power_on_in_place(&mut self) {
        self.cart.borrow_mut().reset();
        self.ppu_clock = 0;
        self.refresh_mirroring();
    }

    fn nes_soft_reset(&mut self) {
        self.cart.borrow_mut().soft_reset();
        self.refresh_mirroring();
    }

    fn read_prg(&self, addr: i32) -> u8 {
        // `$8000-$FFFF`, so `addr` is the offset inside that window.
        let address = 0x8000u16.wrapping_add(addr as u16);
        self.cart
            .borrow_mut()
            .try_cpu_read(address)
            // Unreachable for any mapper `zax-nes` supports: every one of them maps the whole
            // of `$8000-$FFFF`. NesHawk's `ReadPRG` has no data-bus argument to answer with, so
            // an unmapped read is the open bus's usual all-ones rather than a guess at `DB`.
            .unwrap_or(0xFF)
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        let address = 0x8000u16.wrapping_add(addr as u16);
        self.cart.borrow_mut().cpu_write(address, value);
        self.refresh_mirroring();
    }

    fn read_wram(&self, addr: i32, db: u8) -> u8 {
        let address = 0x6000u16.wrapping_add((addr as u16) & 0x1FFF);
        self.cart.borrow_mut().try_cpu_read(address).unwrap_or(db)
    }

    fn write_wram(&mut self, addr: i32, value: u8) {
        let address = 0x6000u16.wrapping_add((addr as u16) & 0x1FFF);
        self.cart.borrow_mut().cpu_write(address, value);
        self.refresh_mirroring();
    }

    /// `$4020-$5FFF`, which NesHawk calls the expansion area. The MMC5's registers are there, and
    /// they are the reason the `RefCell` exists.
    fn read_exp(&self, addr: i32, db: u8) -> u8 {
        let address = 0x4020u16.wrapping_add((addr as u16) & 0x1FDF);
        self.cart.borrow_mut().try_cpu_read(address).unwrap_or(db)
    }

    fn write_exp(&mut self, addr: i32, value: u8) {
        let address = 0x4020u16.wrapping_add((addr as u16) & 0x1FDF);
        self.cart.borrow_mut().cpu_write(address, value);
        self.refresh_mirroring();
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            // `ChrFetch::Background` is the pessimistic choice: it is what a board that
            // distinguishes the two fetches (MMC5's split, MMC2/MMC4's latches) is told on a
            // fetch whose kind this adapter cannot know, because NesHawk's `ReadPPU` does not
            // carry one. A board that does not distinguish them is unaffected.
            return self
                .cart
                .borrow_mut()
                .ppu_read(addr as u16, ChrFetch::Background);
        }
        if self.cartridge_serves_nametable(addr) {
            return self.cart.borrow().read_nametable(addr as u16);
        }
        env.ciram[self.base.apply_mirroring(addr) as usize]
    }

    fn write_ppu(&mut self, addr: i32, value: u8, env: &mut BoardEnv<'_>) {
        if addr < 0x2000 {
            self.cart.borrow_mut().ppu_write(addr as u16, value);
            return;
        }
        if self.cartridge_serves_nametable(addr) {
            self.cart.borrow_mut().write_nametable(addr as u16, value);
            return;
        }
        env.ciram[self.base.apply_mirroring(addr) as usize] = value;
    }

    /// NesHawk's `AddressPPU`: the address lines moving with no read, which is what clocks an
    /// A12-counting IRQ. `Cartridge::on_ppu_address_bus_without_read` is the same notification on
    /// the Zaxnes side, and it is what makes an MMC3-like fallback count at all.
    fn address_ppu(&mut self, addr: i32) {
        let clock = self.ppu_clock;
        self.cart
            .borrow_mut()
            .on_ppu_address_bus_without_read(addr as u16, clock);
    }

    /// The side-effect-free read the 3D capture and the host's memory viewers use
    /// (REQ-CORE-033). `Cartridge::ppu_peek` is Zaxnes's own promise of the same thing.
    fn peek_ppu(&self, addr: i32, ciram: &[u8; 0x800]) -> u8 {
        if addr < 0x2000 {
            return self
                .cart
                .borrow()
                .ppu_peek(addr as u16, ChrFetch::Background);
        }
        if self.cartridge_serves_nametable(addr) {
            return self.cart.borrow().read_nametable(addr as u16);
        }
        ciram[self.base.apply_mirroring(addr) as usize]
    }

    /// The side-effect-free CPU read, for the debugger and the test-ROM readers.
    fn peek_cart(&self, addr: i32, db: u8) -> u8 {
        self.cart.borrow().try_cpu_peek(addr as u16).unwrap_or(db)
    }

    fn irq_signal(&self) -> bool {
        self.cart.borrow().irq_asserted()
    }

    fn save_ram_into(&self, out: &mut Vec<u8>) {
        out.clear();
        out.extend_from_slice(self.cart.borrow().battery_ram());
    }

    fn save_ram_mut(&mut self) -> Option<&mut [u8]> {
        // `RefCell::get_mut` needs `&mut self`, which this has: no borrow flag is involved, so
        // the slice can be lent out for the length of the borrow.
        let battery = self.cart.get_mut().battery_ram_mut();
        (!battery.is_empty()).then_some(battery)
    }

    fn write_state(&self, writer: &mut StateWriter) {
        crate::state::SyncState::write_state(&self.base, writer);
        crate::state::SyncState::write_state(&self.ppu_clock, writer);
        // Zaxnes's own board state, through `zax-nes`'s own serialiser — which is the right
        // thing: this board's state is defined by `zax-nes`, and duplicating its field list here
        // would be a second place to forget a field.
        let mut zax = zax_nes::state::StateWriter::with_capacity(0x2000);
        self.cart.borrow().save_state(&mut zax);
        crate::state::SyncState::write_state(&zax.into_vec(), writer);
    }

    fn read_state(&mut self, reader: &mut StateReader<'_>) {
        let mut base = std::mem::take(&mut self.base);
        crate::state::SyncState::read_state(&mut base, reader);
        self.base = base;
        crate::state::SyncState::read_state(&mut self.ppu_clock, reader);
        let mut inner: Vec<u8> = Vec::new();
        crate::state::SyncState::read_state(&mut inner, reader);
        if reader.failed() {
            return;
        }
        let Ok(mut zax) = zax_nes::state::StateReader::new(&inner, 0) else {
            reader.fail();
            return;
        };
        if self.cart.get_mut().load_state(&mut zax).is_err() {
            reader.fail();
            return;
        }
        self.refresh_mirroring();
    }
}
