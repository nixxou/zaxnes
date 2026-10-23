use crate::NesRegion;

/// Frame geometry and clock rates of one [`NesRegion`] (REQ-CORE-015).
///
/// The values come from the public nesdev "Cycle reference chart" and "Clock rate" pages.
/// PPU dots per CPU cycle is expressed as the exact fraction
/// `dots_per_cycle_numerator / dots_per_cycle_denominator` so that PAL's 3.2 dots per cycle
/// stays an integer ratio (16 dots every 5 CPU cycles) and the core remains bit-for-bit
/// deterministic (REQ-ARCH-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the C# record: three independent flags.
pub struct RegionTiming {
    /// The region these timings describe.
    pub region: NesRegion,
    /// CPU clock in Hz.
    pub cpu_clock_hz: i32,
    /// Numerator of the PPU-dots-per-CPU-cycle ratio.
    pub dots_per_cycle_numerator: i32,
    /// Denominator of the PPU-dots-per-CPU-cycle ratio.
    pub dots_per_cycle_denominator: i32,
    /// Total scanlines in a frame, visible and non-visible.
    pub scanlines_per_frame: i32,
    /// Scanline on which the `VBlank` flag is raised and NMI fires (dot 1). 241 for NTSC and
    /// PAL, 291 for Dendy.
    pub vblank_scanline: i32,
    /// Whether the idle dot of the pre-render line is skipped on odd frames while rendering
    /// is enabled. True for NTSC only.
    pub skips_dot_on_odd_frames: bool,
    /// Rate at which the reference host chains frames (SPEC-07, DEC-R2-T3). The audio
    /// resampler is calibrated on this value, not on the exact hardware frame rate
    /// (REQ-CORE-070).
    pub nominal_frames_per_second: i32,
    /// Whether the APU noise and DMC period tables are the PAL ones. True for
    /// [`NesRegion::Pal`] only; MULTI and DENDY use the NTSC tables (REQ-CORE-015).
    pub uses_pal_noise_and_dmc_periods: bool,
}

impl RegionTiming {
    /// Visible scanlines of every region (REQ-CORE-015).
    pub const VISIBLE_SCANLINES: i32 = 240;

    /// PPU dots in one scanline on every region.
    pub const DOTS_PER_SCANLINE: i32 = 341;

    const NTSC: Self = Self::make(NesRegion::Ntsc, 1_789_773, 3, 1, 262, 241, true, 60, false);
    const PAL: Self = Self::make(NesRegion::Pal, 1_662_607, 16, 5, 312, 241, false, 50, true);
    const MULTI: Self = Self::make(NesRegion::Multi, 1_789_773, 3, 1, 262, 241, true, 60, false);
    const DENDY: Self = Self::make(
        NesRegion::Dendy,
        1_773_448,
        3,
        1,
        312,
        291,
        false,
        50,
        false,
    );

    #[allow(clippy::too_many_arguments)] // Private table constructor, one row per region.
    #[allow(clippy::fn_params_excessive_bools)] // Same.
    const fn make(
        region: NesRegion,
        cpu_clock_hz: i32,
        dots_per_cycle_numerator: i32,
        dots_per_cycle_denominator: i32,
        scanlines_per_frame: i32,
        vblank_scanline: i32,
        skips_dot_on_odd_frames: bool,
        nominal_frames_per_second: i32,
        uses_pal_noise_and_dmc_periods: bool,
    ) -> Self {
        Self {
            region,
            cpu_clock_hz,
            dots_per_cycle_numerator,
            dots_per_cycle_denominator,
            scanlines_per_frame,
            vblank_scanline,
            skips_dot_on_odd_frames,
            nominal_frames_per_second,
            uses_pal_noise_and_dmc_periods,
        }
    }

    /// Returns the timings of `region`.
    #[must_use]
    pub const fn for_region(region: NesRegion) -> Self {
        match region {
            NesRegion::Ntsc => Self::NTSC,
            NesRegion::Pal => Self::PAL,
            NesRegion::Multi => Self::MULTI,
            NesRegion::Dendy => Self::DENDY,
        }
    }

    /// Mono samples produced per frame at `sample_rate`, speed 1.
    #[must_use]
    pub fn samples_per_frame(&self, sample_rate: i32) -> f64 {
        f64::from(sample_rate) / f64::from(self.nominal_frames_per_second)
    }
}

impl Default for RegionTiming {
    fn default() -> Self {
        Self::NTSC
    }
}
