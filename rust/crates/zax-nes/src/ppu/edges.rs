//! Fidélité+ part A step 2 (`docs/design/HALF-CYCLE.md`): register effects as edges.
//!
//! The standard core places the effect of a register access by running dots ahead under the old
//! state and owing them back ([`Ppu2C02::prepaid_dots`]), with a delay chosen per alignment and a
//! rule for every effect found to land on the same dot as the PPU's own work (A5). In Fidélité+ an
//! access instead posts an **edge**: its effect is applied after the work of a dot computed from a
//! single latency per signal, in half master clocks from the CPU cycle's M2 fall, and from the
//! CPU/PPU alignment's master-clock offset ([`crate::fidplus::MasterClock`]). The PPU's own work of
//! the dots that follow then sees the new state, so the A5 rules on that order (the D+4 increment
//! after a `$2006` load, the first address of `INC $2007`) are not written: they follow.
//!
//! Only on the NTSC 2C02's timing (3 dots a cycle), which the netlist measured; elsewhere the
//! standard path runs.

use super::{is_palette_address, Cart, Ppu2C02, PpuBusAccess};
use crate::cartridges::{BoardKind, ChrFetch};
use crate::fidplus::MasterClock;
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NesRegion;

/// Edges pending at once: an access posts one, and the next can only come a cycle later.
pub(super) const EDGE_SLOTS: usize = 4;

/// What an edge does when it is reached. The discriminants are the savestate's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum EdgeKind {
    /// The second `$2006` write: `v` takes `t`.
    LoadAddress,
    /// A `$2007` write outside rendering: the data latch is stored at `v`, then `v` increments.
    StoreData,
    /// A `$2007` read, at the start of its latch dot X: outside the fetches the buffer takes the
    /// byte at `v`; on an odd X the byte the fetch of X - 1 left on the bus. The increment follows
    /// in the middle of X ([`EdgeKind::ReadIncrement`]).
    ReadLatch,
    /// The increment of an odd-X or blanking read, in the middle of X (after X's work here); on a
    /// fetching dot the fetch whose address X set up is read again on X + 1
    /// ([`EdgeKind::RefetchLow`]).
    ReadIncrement,
    /// After the work of X + 1, the read of the fetch X set up while the read latched the bus: the
    /// buffered byte as its low address byte, the high lines from the incremented `v`
    /// (`AccuracyCoin`, `ALE + Read`).
    RefetchLow,
    /// A `$2007` read whose latch dot X is an even fetching dot: after X's work, the byte X's fetch
    /// read. Then the increment, merged with X's own.
    ReadLatchAfterFetch,
    /// A `$2001` write that switches the rendering on or off: the mask, from the third dot after
    /// the write in every alignment (A3c, step 33).
    SwitchRendering,
    /// The horizontal copy of dot 257 after a switch off landing on it: made in the first half of
    /// the dot, before the switch (applied after dot 257's work, which with the rendering off
    /// does nothing else to `v`).
    CopyAfterSwitchOff,
    /// The second phase of a coarse-Y wrap from 29 (A7): coarse Y 30 back to 0, one dot later.
    SettleCoarseY,
    /// The increment of `v` after a `$2007` write made while rendering, after the work of the
    /// read's latch dot X: the write starts the read's state machine (netlist, audit `scr` e4,
    /// `docs/review/FIDELITY-LEADS.md` I6). Merged with X's own increments as the read's.
    WriteIncrement,
}

impl EdgeKind {
    const ALL: [Self; 10] = [
        Self::LoadAddress,
        Self::StoreData,
        Self::ReadLatch,
        Self::ReadIncrement,
        Self::RefetchLow,
        Self::ReadLatchAfterFetch,
        Self::SwitchRendering,
        Self::CopyAfterSwitchOff,
        Self::SettleCoarseY,
        Self::WriteIncrement,
    ];

    fn from_u8(byte: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| *kind as u8 == byte)
    }
}

/// The Fidélité+ loop's own state ([`Ppu2C02`]'s `fid`).
#[derive(Debug, Clone)]
pub(super) struct FidState {
    /// Fidélité+ (`edges`): register effects through edges. Set by the core for each Fidélité+
    /// frame; not part of the state (step 4).
    pub(super) fidelity_plus: bool,
    /// The edges pending (`edges`), the first `edge_count` of them. Not part of the state yet
    /// (step 4); cleared by a load.
    pub(super) edges: [Edge; EDGE_SLOTS],
    pub(super) edge_count: u8,
    /// The PPU's data latch of a `$2007` write (`edges`): the byte the store takes when it comes.
    pub(super) data_latch: u8,
    /// Fidélité+ step 3 (`edges`): the background fetch whose address an odd dot set up and whose
    /// read the next dot makes (0 none, 1 nametable, 2 attribute, 3 and 4 the pattern planes), and
    /// the address this core read it at. Not part of the state: it never outlives a dot pair.
    pub(super) fetch_pending: u8,
    pub(super) fetch_address: u16,
    /// `v` before the work of the dot whose edges are being applied: what a `$2007` increment
    /// landing on dot 257 meets the horizontal copy with. Not part of the state: rebuilt per dot.
    pub(super) v_before_dot: u16,
}

impl FidState {
    pub(super) const NEW: Self = Self {
        fidelity_plus: false,
        edges: [Edge::NONE; EDGE_SLOTS],
        edge_count: 0,
        data_latch: 0,
        fetch_pending: 0,
        fetch_address: 0,
        v_before_dot: 0,
    };
}

/// One pending edge: applied after the work of the `dots`-th dot from now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Edge {
    pub(super) dots: i32,
    pub(super) kind: EdgeKind,
    /// The byte the edge carries (`SwitchRendering`: the mask written).
    pub(super) value: u8,
}

impl Edge {
    pub(super) const NONE: Self = Self {
        dots: 0,
        kind: EdgeKind::LoadAddress,
        value: 0,
    };
}

/// Latency of the `$2006` load, in half master clocks from M2 fall: `v` is loaded in the middle
/// of the third dot after the access dot in all four alignments (A3c, step 33; A5 `g2006x`).
const LOAD_ADDRESS_LATENCY: i32 = 18;

/// Latency of a rendering switch by `$2001`: the fetches and `v`'s increments stop or start on the
/// third dot after the write in all four alignments (A3c, step 33).
const SWITCH_RENDERING_LATENCY: i32 = 10;

/// Latency of the `$2007` read latch: the very start of dot D+5 in alignments 0 and 1, D+6 in 2
/// and 3 (A3c, step 25, netlist), one latency with the alignment offsets.
const READ_LATCH_LATENCY: i32 = 30;

/// Latency of the store of a `$2007` write and of its increment of `v`: after the sixth dot (A5,
/// `g2006px`, palette writes, alignments 2 and 3: `v` moves on dot D+6). The second write of
/// `INC $2007`, three dots after the first, has replaced the data latch by then, so the first
/// address takes the second value (A5, `rmw`).
const STORE_DATA_LATENCY: i32 = 42;

impl Ppu2C02 {
    /// Fidélité+ (`docs/design/HALF-CYCLE.md`): register effects through edges. Set by the core
    /// for each Fidélité+ frame; off, the PPU is the standard one.
    pub const fn set_fidelity_plus(&mut self, enabled: bool) {
        self.fid.fidelity_plus = enabled;
    }

    /// Whether edges are pending: a state taken now has to carry them
    /// ([`Self::save_edge_state`]).
    #[must_use]
    pub const fn edges_pending(&self) -> bool {
        self.fid.edge_count != 0
    }

    /// Appends the pending edges and the data latch to `writer`: the Fidélité+ block of a
    /// savestate (`docs/design/HALF-CYCLE.md`, "Savestates").
    pub fn save_edge_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.fid.edge_count);
        for edge in &self.fid.edges[..usize::from(self.fid.edge_count)] {
            writer.write_i32(edge.dots);
            writer.write_u8(edge.kind as u8);
            writer.write_u8(edge.value);
        }
        writer.write_u8(self.fid.data_latch);
    }

    /// Restores what [`Self::save_edge_state`] wrote.
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or names an edge this build does not know.
    pub fn load_edge_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let count = reader.read_u8()?;
        if usize::from(count) > EDGE_SLOTS {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: {count} pending PPU edges (at most {EDGE_SLOTS})."
            )));
        }
        let mut edges = [Edge::NONE; EDGE_SLOTS];
        for edge in &mut edges[..usize::from(count)] {
            let dots = reader.read_i32()?;
            let byte = reader.read_u8()?;
            let kind = EdgeKind::from_u8(byte).ok_or_else(|| {
                InvalidState::corrupt(format!("Corrupt savestate: unknown PPU edge {byte}."))
            })?;
            let value = reader.read_u8()?;
            *edge = Edge { dots, kind, value };
        }
        self.fid.data_latch = reader.read_u8()?;
        self.fid.edges = edges;
        self.fid.edge_count = count;
        Ok(())
    }

    /// [`Self::edges_on`] on the path of the board kind `K`: false at compile time on the
    /// standard copies.
    pub(super) const fn edges_on_for<K: BoardKind>(&self) -> bool {
        // Not while the widescreen extension's overclock holds the PPU still: an edge lands a
        // number of dots after the access, and no dot runs then, so the margin writes the
        // overclock exists for would pile up unlanded (lot REWRITE-UI, Blaster Master's
        // widescreen with Fidelity+). They take the standard core's timing instead.
        K::FIDELITY_PLUS && self.edges_on() && !self.wide.stalling()
    }

    /// Whether edges place the register effects: Fidélité+ on the NTSC 2C02's timing.
    pub(super) const fn edges_on(&self) -> bool {
        self.fid.fidelity_plus && matches!(self.timing.region, NesRegion::Ntsc | NesRegion::Multi)
    }

    /// The dot, counted from the access dot `D` (the dot just run), after whose work an effect of
    /// latency `latency` lands. The cycle's three dots are `D - 2` to `D`; the CPU cycle starts
    /// with dot `D - 2`, shifted by the alignment's offset.
    pub(super) const fn edge_dot(&self, latency: i32) -> i32 {
        let clock = MasterClock::of(NesRegion::Ntsc);
        let half_dot = clock.per_dot as i32 * 2;
        let at = clock.m2_fall_half() as i32
            + MasterClock::alignment_offset_half(self.alignment) as i32
            + latency;
        // First dot starting at or after `at`, counted from `D - 2`; the effect lands before
        // its work, which is after the work of the dot before it.
        (at + half_dot - 1) / half_dot - 3
    }

    /// Posts an edge `dots` dots from now.
    pub(super) fn post_edge(&mut self, dots: i32, kind: EdgeKind) {
        self.post_edge_with(dots, kind, 0);
    }

    /// [`Self::post_edge`] with the byte the edge carries.
    pub(super) fn post_edge_with(&mut self, dots: i32, kind: EdgeKind, value: u8) {
        // Dots another access already ran ahead are behind the edge's count.
        let dots = dots - self.prepaid_dots;
        let count = usize::from(self.fid.edge_count);
        if count == EDGE_SLOTS {
            // Cannot happen (one access a cycle, each edge at most five dots out); apply at once
            // rather than lose it.
            debug_assert!(false, "edge queue full");
            return;
        }
        self.fid.edges[count] = Edge { dots, kind, value };
        self.fid.edge_count += 1;
    }

    /// Posts an edge from inside another edge: `dots` after the dot being applied, whatever dots
    /// an access ran ahead.
    pub(super) fn post_edge_now(&mut self, dots: i32, kind: EdgeKind) {
        let count = usize::from(self.fid.edge_count);
        if count < EDGE_SLOTS {
            self.fid.edges[count] = Edge {
                dots,
                kind,
                value: 0,
            };
            self.fid.edge_count += 1;
        }
    }

    /// Runs `count` dots one at a time, applying each pending edge after the work of its dot.
    pub(super) fn run_dots_with_edges<K: BoardKind>(
        &mut self,
        mut count: i64,
        cart: Cart<'_, '_, K>,
    ) {
        while count > 0 {
            if self.fid.edge_count == 0 && self.fid.fetch_pending == 0 && !self.tracing() {
                // Nothing pending: the standard core's stretches, up to the dot before 256 on a
                // line that renders, whose Y increment may post the coarse-Y wrap's edge (A7).
                let stretch = if self.rendering_now() && self.dot < 256 {
                    count.min(i64::from(255 - self.dot))
                } else {
                    count
                };
                if stretch > 0 {
                    self.run_dots_standard(stretch, cart);
                    count -= stretch;
                    continue;
                }
            }
            // Only the edges pending before this dot count it down: one its own work posts (the
            // coarse-Y wrap of dot 256) or an edge posts counts from the next.
            let mut pending = usize::from(self.fid.edge_count);
            let before = self.v;
            self.fid.v_before_dot = before;
            self.run_one_dot(cart);
            if self.dot == 256 && self.edges_on() {
                self.two_phase_y_wrap(before);
            }
            // The address latch as state (step 3): the read half of a fetch after its ALE half.
            if self.dot & 1 == 0 {
                self.settle_fetch_read(cart.as_deref());
            } else {
                self.note_fetch_ale();
            }
            let mut index = 0;
            while index < pending {
                self.fid.edges[index].dots -= 1;
                if self.fid.edges[index].dots <= 0 {
                    let Edge { kind, value, .. } = self.fid.edges[index];
                    self.fid
                        .edges
                        .copy_within(index + 1..usize::from(self.fid.edge_count), index);
                    self.fid.edge_count -= 1;
                    pending -= 1;
                    self.apply_edge(kind, value, cart);
                } else {
                    index += 1;
                }
            }
            #[cfg(feature = "ppu-dot-trace")]
            if self.dot_trace.is_some() {
                self.record_traced_dot();
            }
            count -= 1;
        }
    }

    /// A7 (netlist): a Y increment that wrapped coarse Y from 29 shows, for one dot, coarse Y 30
    /// with the nametable bit already flipped; the "30" detection clears it in the next phase
    /// ([`EdgeKind::SettleCoarseY`]). `before` is `v` before the increment. Kept out of
    /// `increment_y`, which the standard loop inlines.
    fn two_phase_y_wrap(&mut self, before: u16) {
        let wrapped = (before & 0x7000) == 0x7000 && (before >> 5) & 0x1F == 29;
        if wrapped && self.v & 0x03E0 == 0 {
            self.v = (self.v & !0x03E0) | (30 << 5);
            if self.v_row_next == self.v & !0x03E0 {
                self.v_row_next = self.v;
            }
            self.post_edge_now(1, EdgeKind::SettleCoarseY);
        }
    }

    /// The pipeline's Y increment made by an edge, with A7's two phases.
    fn increment_y_two_phase(&mut self) {
        let before = self.v;
        self.increment_y();
        self.two_phase_y_wrap(before);
    }

    /// Whether the per-dot trace is on (feature `ppu-dot-trace`): the dots are then all run one at
    /// a time, as the trace records them.
    #[allow(clippy::unused_self)]
    const fn tracing(&self) -> bool {
        #[cfg(feature = "ppu-dot-trace")]
        {
            self.dot_trace.is_some()
        }
        #[cfg(not(feature = "ppu-dot-trace"))]
        {
            false
        }
    }

    /// One dot of the standard core (`run_dots_standard`), never anything of the per-dot path
    /// directly (see there). With the per-dot trace on, the dot is recorded by
    /// [`Self::run_dots_with_edges`] after its edges, so it is run untraced here.
    fn run_one_dot<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        #[cfg(feature = "ppu-dot-trace")]
        if self.dot_trace.is_some() {
            self.dot_trace_bus.set(0xFFFF);
            self.step_dot(cart);
            return;
        }
        self.run_dots_standard(1, cart);
    }

    /// The address the background fetch `slot` puts on the bus from the current state.
    fn background_fetch_address(&self, slot: u8) -> u16 {
        match slot {
            1 => 0x2000 | (self.v & 0x0FFF),
            2 => self.attribute_address(),
            3 => self.pattern_address(0),
            _ => self.pattern_address(8),
        }
    }

    /// After an odd dot's work: the ALE half of a background fetch. The external octal latch takes
    /// the address's low byte; the read comes on the next dot. (This core reads on the odd dot
    /// itself; [`Self::settle_fetch_read`] corrects the byte when the bus says otherwise.)
    fn note_fetch_ale(&mut self) {
        let dot = self.dot;
        if !self.rendering_now() || !matches!(dot, 3..=255 | 321..=336) {
            // Dot 1 fetches nothing in this core; dot 257's copy of `v` follows its fetch, left
            // as the standard core has it.
            return;
        }
        let slot = match (dot - 1) & 0x07 {
            0 => 1,
            2 => 2,
            4 => 3,
            6 => 4,
            _ => return,
        };
        let address = self.background_fetch_address(slot);
        self.fid.fetch_pending = slot;
        self.fid.fetch_address = address;
        self.ale_low = address.to_le_bytes()[0];
    }

    /// After an even dot's work: the read half. The PPU drives the high address lines of this
    /// moment and the latch gives the low byte: when `v` moved between the halves (a `$2006`
    /// load, `AccuracyCoin`'s `Hybrid Addresses`) or the latch took another byte (a `$2007` read
    /// latching on the ALE half, `ALE + Read`), the byte read is the one at that address.
    fn settle_fetch_read(&mut self, cartridge: Option<&super::Cartridge>) {
        let slot = std::mem::take(&mut self.fid.fetch_pending);
        if slot == 0 || !self.rendering_now() {
            return;
        }
        let address = (self.background_fetch_address(slot) & 0xFF00) | u16::from(self.ale_low);
        if address == self.fid.fetch_address {
            return;
        }
        let byte =
            self.peek_vram::<crate::cartridges::AnyBoard>(address, ChrFetch::Background, cartridge);
        match slot {
            1 => self.nametable_latch = byte,
            2 => {
                let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
                self.attribute_latch = (byte >> shift) & 0x03;
            }
            3 => self.pattern_low_latch = byte,
            _ => self.pattern_high_latch = byte,
        }
    }

    fn apply_edge<K: BoardKind>(&mut self, kind: EdgeKind, value: u8, cart: Cart<'_, '_, K>) {
        match kind {
            EdgeKind::SwitchRendering => self.edge_switch_rendering(value, cart),
            EdgeKind::CopyAfterSwitchOff => self.transfer_address_x(),
            EdgeKind::SettleCoarseY => {
                if (self.v >> 5) & 0x1F == 30 {
                    let before = self.v;
                    self.v &= !0x03E0;
                    if self.v_row_next == before {
                        self.v_row_next = self.v;
                    }
                }
            }
            EdgeKind::LoadAddress => self.edge_load_address(cart),
            EdgeKind::StoreData => {
                let address = self.v & 0x3FFF;
                self.write_vram_inner(address, self.fid.data_latch, PpuBusAccess::Cpu, cart);
                self.increment_vram_address(cart);
            }
            EdgeKind::ReadLatch => {
                if self.fetching() {
                    // The start of an odd dot: the bus still holds the byte X - 1 read.
                    self.read_buffer = self.fetch_bus_value(cart.as_deref());
                } else {
                    let address = self.v & 0x3FFF;
                    let source = if is_palette_address(address) {
                        address & 0x2FFF
                    } else {
                        address
                    };
                    self.read_buffer =
                        self.read_vram_inner(source, PpuBusAccess::Cpu, ChrFetch::Background, cart);
                }
                self.post_edge_now(1, EdgeKind::ReadIncrement);
            }
            EdgeKind::ReadIncrement => {
                if self.fetching() {
                    if self.dot == 257 {
                        // The horizontal copy of X and the read's coarse-X increment drive the
                        // same lines in the same half: `v` and `t` keep their AND
                        // (FIDELITY-LEADS I7).
                        self.horizontal_copy_meets_increment(cart);
                    } else {
                        let before = self.v;
                        self.increment_vram_address(cart);
                        self.two_phase_y_wrap(before);
                    }
                    // The read latched on a cadence ALE half: the octal latch took the byte it
                    // read, which the next dot's fetch then reads with (step 3).
                    self.ale_low = self.read_buffer;
                } else {
                    self.increment_vram_address(cart);
                }
            }
            EdgeKind::RefetchLow => {
                if self.fetching() {
                    self.refetch_with_low_byte(self.read_buffer, cart.as_deref());
                }
            }
            EdgeKind::WriteIncrement => self.edge_write_increment(cart),
            EdgeKind::ReadLatchAfterFetch => {
                self.read_buffer = self.fetch_bus_value(cart.as_deref());
                // The read's X and Y increment signals and the pipeline's own are the same lines:
                // on a dot that increments coarse X anyway its X is already made, on dot 256 both.
                let dot = self.dot;
                let natural_x =
                    (dot % 8 == 0 && (8..=256).contains(&dot)) || dot == 328 || dot == 336;
                if !natural_x {
                    self.increment_coarse_x();
                }
                if dot != 256 {
                    self.increment_y_two_phase();
                }
                self.v_row_next = self.v;
                self.notify_address_bus(self.v & 0x3FFF, cart);
            }
        }
    }

    /// The second `$2006` write reaching `v`, after the work of the dot it lands in.
    fn edge_load_address<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        if self.glitch_2006 {
            self.address_load_collision();
        }
        // The read of a fetch whose address the old `v` set up: the address latch does it
        // (`settle_fetch_read`, step 3).
        let previous = self.v;
        self.v = self.t;
        if is_palette_address(previous) {
            self.palette_load_corruption(previous);
        }
        self.v_row_start = self.v;
        self.v_row_next = self.v;
        self.notify_address_bus_without_read(self.v & 0x3FFF, cart);
    }

    /// A `$2001` write that switches the rendering, in Fidélité+: the palette corruption of a
    /// switch off strikes the pixel the next dot draws (as the standard core), the rest is an edge.
    pub(super) fn post_switch_rendering(&mut self, value: u8) {
        let corrupts = (value & 0x18) == 0 && self.switch_off_corrupts_palettes();
        if corrupts {
            let renders = self.scanline < super::VISIBLE_SCANLINES
                || self.scanline == self.pre_render_scanline();
            // On a line that renders, the pixel the next dot draws; past line 239 or the
            // pre-render line, line 0's first, drawn with rendering off: colour index 0.
            let color = if renders {
                self.next_pixel_color_index()
            } else {
                0
            };
            self.corrupt_palettes(color);
        }
        let dots = self.edge_dot(SWITCH_RENDERING_LATENCY);
        self.post_edge_with(dots, EdgeKind::SwitchRendering, value);
    }

    /// The switch reaching the mask, after the work of the dot before the first one it governs.
    /// What follows is the standard core's own code after its prepaid dots
    /// (`Ppu2C02::switch_rendering`), with the line read when the switch lands.
    fn edge_switch_rendering<K: BoardKind>(&mut self, value: u8, cart: Cart<'_, '_, K>) {
        let renders =
            self.scanline < super::VISIBLE_SCANLINES || self.scanline == self.pre_render_scanline();
        let was_rendering = self.rendering_enabled();
        self.mask = value;
        let now_rendering = self.rendering_enabled();
        if was_rendering == now_rendering {
            return;
        }
        if renders && self.dot == 256 && self.alignment >= 2 {
            // The horizontal copy of dot 257 comes in the first half of the dot, before a switch
            // landing in it: it follows the old state (netlist, A5; alignments 0 and 1 hit the
            // switch-on race of step 33 and keep the plain order).
            if was_rendering {
                self.post_edge_now(1, EdgeKind::CopyAfterSwitchOff);
            } else {
                self.copy_257_suppressed = true;
            }
        }
        let on = now_rendering;
        self.rendering_switched(on);
        if renders {
            // `switch_pixel_edge` takes the write's dot, two before the first governed one.
            self.switch_pixel_edge(on, self.dot - 2);
            if on {
                self.first_fetch_on(self.dot, cart);
                self.skip_background_reload(cart);
            } else {
                self.note_last_ale(self.dot);
            }
        }
    }

    /// The second `$2006` write in Fidélité+: posted, every time (the load is never immediate on
    /// the chip; whether anything can see the three dots is the chip's business).
    pub(super) fn post_address_load(&mut self) {
        let horizontal = self.dot == 257;
        let vertical = self.dot == 304 && self.scanline == self.pre_render_scanline();
        if (horizontal || vertical) && self.fetching() {
            // The write's low byte reaches `t` while the copy of this dot reads it (netlist,
            // `g2006x`, both values of the corpus, `$08` after `$D7` and back, dots 257 and 304):
            // in alignment 2 the copy takes the written byte; in alignment 3 it takes the CPU
            // data bus as it stood before the write (`$20`, the address's high byte), the open
            // bus of the `$2005` write on these dots and of the `$2000` glitch (A4), under the
            // same switch. Alignments 0 and 1 are given the written byte, as A4 measured.
            let source = if self.alignment == 3 && self.glitch_2000 {
                self.cpu_open_bus
            } else {
                self.t.to_le_bytes()[0]
            };
            if horizontal {
                self.v = (self.v & !0x001F) | u16::from(source & 0x1F);
            } else {
                self.v = (self.v & !0x00E0) | u16::from(source & 0xE0);
                self.v_row_start = self.v;
            }
            self.v_row_next = self.v;
        }
        let dots = self.edge_dot(LOAD_ADDRESS_LATENCY);
        self.post_edge(dots, EdgeKind::LoadAddress);
    }

    /// A `$2007` read in Fidélité+: the buffer as it stands now (the palette answers at once), the
    /// latch and the increment later. Returns the value the CPU reads.
    pub(super) fn read_data_with_edges<K: BoardKind>(&mut self, _cart: Cart<'_, '_, K>) -> u8 {
        let address = self.v & 0x3FFF;
        // A `$2007` access is an ALE of `v` (step 39).
        self.ale_low = address.to_le_bytes()[0];
        let value = if is_palette_address(address) && !self.pre_g_revision {
            let value = (self.palette_raw[super::palette_index(address)]
                & self.palette_read_mask())
                | (self.decay & 0xC0);
            self.refresh_decay(value, 0x3F);
            value
        } else {
            let value = self.read_buffer;
            self.refresh_decay(value, 0xFF);
            value
        };
        let dots = self.edge_dot(READ_LATCH_LATENCY);
        // X, the latch dot, is the dot after the one the edge follows; past dot 340 it is on the
        // next line, whose fetches the line's own state decides (a visible line follows a visible
        // or pre-render one).
        let latch_dot = (self.dot + dots + 1) % super::DOTS_PER_SCANLINE;
        let kind = if self.fetching() && latch_dot % 2 == 0 {
            EdgeKind::ReadLatchAfterFetch
        } else {
            EdgeKind::ReadLatch
        };
        let dots = if kind == EdgeKind::ReadLatchAfterFetch {
            dots + 1
        } else {
            dots
        };
        self.post_edge(dots, kind);
        value
    }

    /// A `$2007` write in Fidélité+ while rendering: the byte is stored at the access, as before;
    /// the increment of `v` comes on the read's latch dot X (`D + 6` in alignments 2 and 3, `D + 5`
    /// in 0 and 1), after X's work ([`EdgeKind::WriteIncrement`]). Netlist, audit `scr` at reset
    /// phase e4: line 8, a write on dot 112 leaves `v` = `$652F` up to dot 117 and makes it `$7530`
    /// on 118; line 41, on dot 66, `$1539` becomes `$253A` on 72, one X increment merged with the
    /// pipeline's own plus Y.
    pub(super) fn post_write_increment(&mut self) {
        let dots = self.edge_dot(READ_LATCH_LATENCY) + 1;
        self.post_edge(dots, EdgeKind::WriteIncrement);
    }

    /// A `$2007` increment applied after the work of dot 257: `v` and `t` keep the AND of their
    /// horizontal bits ([`Ppu2C02::increment_meets_horizontal_copy`]), with A7's two phases.
    fn horizontal_copy_meets_increment<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        let before = self.fid.v_before_dot;
        self.increment_meets_horizontal_copy(before);
        self.two_phase_y_wrap(before);
        self.notify_address_bus(self.v & 0x3FFF, cart);
    }

    /// [`EdgeKind::WriteIncrement`], after the work of X: the read's rules without its buffer.
    fn edge_write_increment<K: BoardKind>(&mut self, cart: Cart<'_, '_, K>) {
        if !self.fetching() {
            self.increment_vram_address(cart);
            return;
        }
        let dot = self.dot;
        if dot == 257 {
            // The horizontal copy of 257 meets the coarse-X increment (FIDELITY-LEADS I7).
            self.horizontal_copy_meets_increment(cart);
            return;
        } else {
            let natural_x = (dot % 8 == 0 && (8..=256).contains(&dot)) || dot == 328 || dot == 336;
            if !natural_x {
                self.increment_coarse_x();
            }
            if dot != 256 {
                self.increment_y_two_phase();
            }
        }
        self.v_row_next = self.v;
        self.notify_address_bus(self.v & 0x3FFF, cart);
    }

    /// A `$2007` write in Fidélité+ outside rendering: the data latch now, the store and the
    /// increment later.
    pub(super) fn post_data_store(&mut self, value: u8) {
        self.fid.data_latch = value;
        let dots = self.edge_dot(STORE_DATA_LATENCY);
        self.post_edge(dots, EdgeKind::StoreData);
    }
}
