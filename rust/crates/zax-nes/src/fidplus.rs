//! Fidélité+, part A: the half-cycle machine (`docs/design/HALF-CYCLE.md`).
//!
//! Step 1 of the plan puts the skeleton in place: the option ([`crate::NesCore::set_fidelity_plus`],
//! off by default), its own frame loop chosen once per frame as the board path is, and the clock
//! model below, in master clocks, that the next steps place every edge on. The loop runs the PPU
//! on every CPU cycle (no batching) through [`crate::nes_bus::FidelityBus`], which for now
//! forwards each access unchanged: the mode is equal to the standard core on every frame, which is
//! the step's acceptance (`tests/fidplus_equivalence_tests.rs`). Step 2 splits each cycle at its
//! edges.

use crate::NesRegion;

/// The clock of one region in master clocks (21.477 MHz NTSC, 26.601 MHz PAL; nesdev "Cycle
/// reference chart", `BeesNES`'s `CSystem` dividers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterClock {
    /// Master clocks in one CPU cycle (12 NTSC, PAL-M and PAL-N; 16 PAL; 15 Dendy).
    pub per_cpu_cycle: u8,
    /// Master clocks in one PPU dot (4 NTSC, PAL-M and PAL-N; 5 PAL and Dendy).
    pub per_dot: u8,
}

impl MasterClock {
    /// The dividers of `region`. Multi runs NTSC timing.
    #[must_use]
    pub const fn of(region: NesRegion) -> Self {
        match region {
            NesRegion::Pal => Self {
                per_cpu_cycle: 16,
                per_dot: 5,
            },
            NesRegion::Dendy => Self {
                per_cpu_cycle: 15,
                per_dot: 5,
            },
            _ => Self {
                per_cpu_cycle: 12,
                per_dot: 4,
            },
        }
    }

    /// Where M2 rises inside a CPU cycle, in **half** master clocks from the cycle's start.
    ///
    /// The 2A03G's M2 is high for 15/24 of the cycle and low first (nesdev "CPU pin out and
    /// signal description", "M2 duty cycle"); `BeesNES` uses the same 15/24 for its φ1/φ2 slots.
    /// The PAL 2A07 and the Dendy clone are taken with the same duty until measured
    /// (FIDELITY-LEADS A6).
    #[must_use]
    pub const fn m2_rise_half(self) -> u16 {
        // Low for 9/24 of the cycle.
        (self.per_cpu_cycle as u16 * 2 * 9) / 24
    }

    /// Where M2 falls, in half master clocks: the end of the cycle.
    #[must_use]
    pub const fn m2_fall_half(self) -> u16 {
        self.per_cpu_cycle as u16 * 2
    }

    /// Where the PPU's /CS falls for a register access, in half master clocks: one half dot
    /// after M2 rises (design note, "The clock"; the A3c measurement).
    #[must_use]
    pub const fn cs_fall_half(self) -> u16 {
        self.m2_rise_half() + self.per_dot as u16
    }

    /// The master-clock offset of the PPU's dots against the CPU's cycles for an alignment
    /// (`emu.cpu_ppu_alignment`, 0 to 3), in half master clocks: the four phases of the netlist
    /// bench (`0 = a2`, `1 = a0`, `2 = a6`, `3 = a4`, FIDELITY-LEADS A5), one master clock apart.
    #[must_use]
    pub const fn alignment_offset_half(alignment: u8) -> u16 {
        match alignment & 3 {
            0 => 2,
            1 => 0,
            2 => 6,
            _ => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MasterClock;
    use crate::NesRegion;

    #[test]
    fn ntsc_cpu_cycle_is_three_dots_with_m2_high_for_fifteen_twenty_fourths() {
        let clock = MasterClock::of(NesRegion::Ntsc);
        assert_eq!(clock.per_cpu_cycle, clock.per_dot * 3);
        assert_eq!(clock.m2_rise_half(), 9);
        assert_eq!(clock.m2_fall_half() - clock.m2_rise_half(), 15);
        assert_eq!(clock.cs_fall_half(), 13);
    }

    #[test]
    fn pal_cpu_cycle_is_three_point_two_dots() {
        let clock = MasterClock::of(NesRegion::Pal);
        assert_eq!(
            u32::from(clock.per_cpu_cycle) * 5,
            u32::from(clock.per_dot) * 16
        );
        assert_eq!(MasterClock::of(NesRegion::Dendy).per_cpu_cycle, 15);
        assert_eq!(
            MasterClock::of(NesRegion::PalN),
            MasterClock::of(NesRegion::Ntsc)
        );
    }

    #[test]
    fn the_four_alignments_are_one_master_clock_apart() {
        let mut offsets: Vec<u16> = (0..4).map(MasterClock::alignment_offset_half).collect();
        offsets.sort_unstable();
        assert_eq!(offsets, [0, 2, 4, 6]);
    }
}
