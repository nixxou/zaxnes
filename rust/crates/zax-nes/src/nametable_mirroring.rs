/// Layout of the four logical nametables on cartridge/console VRAM (SPEC-02 §2, "current
/// mirroring"). Mappers may change it while the game runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum NametableMirroring {
    /// $2000 = $2400 and $2800 = $2C00.
    #[default]
    Horizontal = 0,
    /// $2000 = $2800 and $2400 = $2C00.
    Vertical = 1,
    /// All four logical nametables show the first physical one.
    SingleScreenA = 2,
    /// All four logical nametables show the second physical one.
    SingleScreenB = 3,
    /// Four distinct nametables, backed by 4 KiB of VRAM.
    FourScreen = 4,
}

impl NametableMirroring {
    /// The layout stored as `value` in a savestate, or `None` for an unknown value.
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Horizontal),
            1 => Some(Self::Vertical),
            2 => Some(Self::SingleScreenA),
            3 => Some(Self::SingleScreenB),
            4 => Some(Self::FourScreen),
            _ => None,
        }
    }
}
