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
use crate::NesRegion;

/// Edges pending at once: an access posts one, and the next can only come a cycle later.
pub(super) const EDGE_SLOTS: usize = 4;

/// What an edge does when it is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// fetching dot the fetch X set up is then read again with the buffered byte as its low
    /// address byte (`AccuracyCoin`, `ALE + Read`).
    ReadIncrement,
    /// A `$2007` read whose latch dot X is an even fetching dot: after X's work, the byte X's fetch
    /// read. Then the increment, merged with X's own.
    ReadLatchAfterFetch,
}

/// One pending edge: applied after the work of the `dots`-th dot from now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Edge {
    pub(super) dots: i32,
    pub(super) kind: EdgeKind,
}

impl Edge {
    pub(super) const NONE: Self = Self {
        dots: 0,
        kind: EdgeKind::LoadAddress,
    };
}

/// Latency of the `$2006` load, in half master clocks from M2 fall: `v` is loaded in the middle
/// of the third dot after the access dot in all four alignments (A3c, step 33; A5 `g2006x`).
const LOAD_ADDRESS_LATENCY: i32 = 18;

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
        self.fidelity_plus = enabled;
    }

    /// Whether edges place the register effects: Fidélité+ on the NTSC 2C02's timing.
    pub(super) const fn edges_on(&self) -> bool {
        self.fidelity_plus && matches!(self.timing.region, NesRegion::Ntsc | NesRegion::Multi)
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
        // Dots another access already ran ahead are behind the edge's count.
        let dots = dots - self.prepaid_dots;
        let count = usize::from(self.edge_count);
        if count == EDGE_SLOTS {
            // Cannot happen (one access a cycle, each edge at most five dots out); apply at once
            // rather than lose it.
            debug_assert!(false, "edge queue full");
            return;
        }
        self.edges[count] = Edge { dots, kind };
        self.edge_count += 1;
    }

    /// Posts an edge from inside another edge: `dots` after the dot being applied, whatever dots
    /// an access ran ahead.
    fn post_edge_now(&mut self, dots: i32, kind: EdgeKind) {
        let count = usize::from(self.edge_count);
        if count < EDGE_SLOTS {
            self.edges[count] = Edge { dots, kind };
            self.edge_count += 1;
        }
    }

    /// Runs `count` dots one at a time, applying each pending edge after the work of its dot.
    pub(super) fn run_dots_with_edges<K: BoardKind>(
        &mut self,
        mut count: i64,
        cart: Cart<'_, '_, K>,
    ) {
        while count > 0 {
            #[cfg(feature = "ppu-dot-trace")]
            self.dot_trace_bus.set(0xFFFF);
            self.step_dot(cart);
            // Only the edges pending before this dot: one an edge posts counts from the next.
            let mut index = 0;
            let mut pending = usize::from(self.edge_count);
            while index < pending {
                self.edges[index].dots -= 1;
                if self.edges[index].dots <= 0 {
                    let kind = self.edges[index].kind;
                    self.edges
                        .copy_within(index + 1..usize::from(self.edge_count), index);
                    self.edge_count -= 1;
                    pending -= 1;
                    self.apply_edge(kind, cart);
                } else {
                    index += 1;
                }
            }
            #[cfg(feature = "ppu-dot-trace")]
            self.record_traced_dot();
            count -= 1;
        }
    }

    fn apply_edge<K: BoardKind>(&mut self, kind: EdgeKind, cart: Cart<'_, '_, K>) {
        match kind {
            EdgeKind::LoadAddress => self.edge_load_address(cart),
            EdgeKind::StoreData => {
                let address = self.v & 0x3FFF;
                self.write_vram_inner(address, self.data_latch, PpuBusAccess::Cpu, cart);
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
                        // same lines in the same half: the copy wins, the Y increment is left
                        // (A5, netlist).
                        self.increment_y();
                        self.v_row_next = self.v;
                        self.notify_address_bus(self.v & 0x3FFF, cart);
                    } else {
                        self.increment_vram_address(cart);
                    }
                    self.refetch_with_low_byte(self.read_buffer, cart.as_deref());
                } else {
                    self.increment_vram_address(cart);
                }
            }
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
                    self.increment_y();
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
        // The read of a nametable or attribute fetch whose address the old `v` set up on this
        // dot: high lines from the new `v`, low byte from the latch (`AccuracyCoin`, `Hybrid
        // Addresses`). Step 3 holds the address latch as state; until then, the same patch as the
        // standard core's `delay_address_load`.
        let next = self.dot + 1;
        if self.fetching() && matches!(next, 2..=256 | 322..=336) {
            let hybrid = |address: u16, old: u16| (address & 0xFF00) | (old & 0x00FF);
            match (next - 1) & 0x07 {
                1 => {
                    let address = hybrid(0x2000 | (self.t & 0x0FFF), self.v);
                    self.nametable_latch =
                        self.peek_vram::<K>(address, ChrFetch::Background, cart.as_deref());
                }
                3 => {
                    let address = hybrid(0x23C0 | (self.t & 0x0C00), self.attribute_address());
                    let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
                    self.attribute_latch =
                        (self.peek_vram::<K>(address, ChrFetch::Background, cart.as_deref())
                            >> shift)
                            & 0x03;
                }
                _ => {}
            }
        }
        let previous = self.v;
        self.v = self.t;
        if is_palette_address(previous) {
            self.palette_load_corruption(previous);
        }
        self.v_row_start = self.v;
        self.v_row_next = self.v;
        self.notify_address_bus_without_read(self.v & 0x3FFF, cart);
    }

    /// The second `$2006` write in Fidélité+: posted, every time (the load is never immediate on
    /// the chip; whether anything can see the three dots is the chip's business).
    pub(super) fn post_address_load(&mut self) {
        if self.dot == 257 && self.fetching() {
            // The write's low byte reaches `t`'s bus while the horizontal copy of this dot reads
            // it (netlist, `g2006x`, both values of the corpus, `$08` after `$D7` and back): in
            // alignment 2 the copy takes the written coarse X, in alignment 3, where the write
            // and the copy overlap, the old `t`'s AND the written one. Alignments 0 and 1 are
            // given the written value, as the `$2005` write on that dot measured (A4).
            if self.alignment == 3 {
                self.v &= self.t | !0x001F;
            } else {
                self.v = (self.v & !0x001F) | (self.t & 0x001F);
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
        // X, the latch dot, is the dot after the one the edge follows.
        let latch_dot = self.dot + dots + 1;
        let kind = if self.fetching_at(latch_dot) && latch_dot % 2 == 0 {
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

    /// Whether the fetches still run on `dot` of this line (a later dot of the same line; a read
    /// is never posted across a line end that matters: the fetches stop at 340).
    const fn fetching_at(&self, dot: i32) -> bool {
        self.fetching() && dot <= 340
    }

    /// A `$2007` write in Fidélité+ outside rendering: the data latch now, the store and the
    /// increment later.
    pub(super) fn post_data_store(&mut self, value: u8) {
        self.data_latch = value;
        let dots = self.edge_dot(STORE_DATA_LATENCY);
        self.post_edge(dots, EdgeKind::StoreData);
    }
}
