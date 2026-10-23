/// TV region of the emulated console (SPEC-02 §4.5).
///
/// The numeric values are observable through the persisted settings (SPEC-09) and must not
/// change: NTSC = 0, PAL = 1, MULTI = 2, DENDY = 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum NesRegion {
    /// NTSC timing, 60 fps nominal host rate.
    #[default]
    Ntsc = 0,
    /// PAL timing, 50 fps nominal host rate.
    Pal = 1,
    /// Dual-compatible cartridge; the core runs NTSC timing (REQ-CORE-015).
    Multi = 2,
    /// Dendy timing: PAL-length frames with an NTSC-like CPU clock.
    Dendy = 3,
}

impl NesRegion {
    /// The region of numeric value `value` (0 to 3), `None` otherwise.
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Ntsc),
            1 => Some(Self::Pal),
            2 => Some(Self::Multi),
            3 => Some(Self::Dendy),
            _ => None,
        }
    }
}
