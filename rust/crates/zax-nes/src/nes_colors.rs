//! Brightness classification of the 64 NES colours, as the Zapper's photodiode sees it
//! (REQ-CORE-062).
//!
//! The RGB table itself belongs to the palette management of SPEC-04 (DEC-04B-Q1); only the
//! light/dark decision is a core concern. The threshold is very low: exactly nine colours
//! count as dark, the blacks.

/// Number of colours in the NES palette.
pub const COUNT: usize = 64;

/// The nine colour indices the Zapper reads as dark: $0D, $0E, $0F, $1E, $1F, $2E, $2F, $3E
/// and $3F (REQ-CORE-062, SPEC-02 §5.4).
pub const DARK_COLORS: [u8; 9] = [0x0D, 0x0E, 0x0F, 0x1E, 0x1F, 0x2E, 0x2F, 0x3E, 0x3F];

/// Whether `color_index` (6 significant bits) is dark for the Zapper.
#[must_use]
pub const fn is_dark(color_index: u8) -> bool {
    matches!(
        color_index & 0x3F,
        0x0D | 0x0E | 0x0F | 0x1E | 0x1F | 0x2E | 0x2F | 0x3E | 0x3F
    )
}

/// Whether `color_index` is light for the Zapper.
#[must_use]
pub const fn is_light(color_index: u8) -> bool {
    !is_dark(color_index)
}
