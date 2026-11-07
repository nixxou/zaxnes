//! The master palettes of the VS. System PPUs, as **index remaps** into the ordered palette
//! (nesdev, "PPU palettes"; lot MF-VS step 2).
//!
//! # Why a remap and not a table of colours
//!
//! This core never produces RGB. A pixel of a frame is a 6-bit NES colour index
//! (REQ-CORE-048), the 32 bytes of palette RAM travel with the frame batch (REQ-CORE-045), and
//! the RGB table they are shown in belongs to the host: `zax_engine3d::palette` holds eleven of
//! them and the player picks one (`video.nes_palette`, SPEC-04 §4.1, DEC-04B-Q1).
//!
//! The four RP2C04 PPUs are the same colours in a different order — the page says so: "All four
//! 2C04 PPUs contain the same master palette, but in different permutations" — and it offers the
//! implementation this module uses: **"Emulator authors may implement the 2C04 variants as a LUT
//! indexing the ordered palette"**, with one `PaletteLUT_2C04_000x` array of 64 bytes per
//! variant. Those four arrays are below, value for value.
//!
//! So a VS palette is a permutation of the indices the host already knows how to colour, the
//! player's chosen RGB table keeps working on a VS image, and the remap is a single array
//! lookup applied where palette RAM is **written** — a handful of times a frame — instead of
//! per pixel.
//!
//! # The RP2C03, the RC2C03 and the RC2C05 are the ordered palette itself
//!
//! The 2C03 "is used in Vs. Duck Hunt, Vs. Tennis, all `PlayChoice` games" and its master
//! palette is the ordered one, which is why byte 13's `$0`, `$1`, `$6` and `$7` share one
//! [`VsPpuModel`] entry. The 2C05 differs from the 2C03 by its two register quirks (step 3 of
//! the lot) and not by its colours — the page's note on the 2C05-99 variant says its palette is
//! "believed to be identical to that of the 2C03". Both therefore have **no LUT at all**:
//! [`VsPpuModel::palette_lut`] answers `None`, the identity is not applied, and the brief's rule
//! for an unidentified VS image — "gets the RP2C03 (plain NES palette)" — is that `None`.
//!
//! One consequence named rather than hidden: the real 2C03 renders a few of the 2C02's grey
//! ramp entries as black, which an index remap into the host's table cannot express, because
//! the host's table is the player's choice and not the chip's DAC. The brief settles it —
//! "plain NES palette" — and `docs/progress/MF-VS.md` records it with the other gaps.

use super::VsPpuModel;

/// Number of entries in a master palette.
pub const LUT_LEN: usize = 64;

/// `PaletteLUT_2C04_0001` (nesdev, "PPU palettes").
static LUT_2C04_0001: [u8; LUT_LEN] = [
    0x35, 0x23, 0x16, 0x22, 0x1C, 0x09, 0x1D, 0x15, 0x20, 0x00, 0x27, 0x05, 0x04, 0x28, 0x08, 0x20,
    0x21, 0x3E, 0x1F, 0x29, 0x3C, 0x32, 0x36, 0x12, 0x3F, 0x2B, 0x2E, 0x1E, 0x3D, 0x2D, 0x24, 0x01,
    0x0E, 0x31, 0x33, 0x2A, 0x2C, 0x0C, 0x1B, 0x14, 0x2E, 0x07, 0x34, 0x06, 0x13, 0x02, 0x26, 0x2E,
    0x2E, 0x19, 0x10, 0x0A, 0x39, 0x03, 0x37, 0x17, 0x0F, 0x11, 0x0B, 0x0D, 0x38, 0x25, 0x18, 0x3A,
];

/// `PaletteLUT_2C04_0002` (nesdev, "PPU palettes").
static LUT_2C04_0002: [u8; LUT_LEN] = [
    0x2E, 0x27, 0x18, 0x39, 0x3A, 0x25, 0x1C, 0x31, 0x16, 0x13, 0x38, 0x34, 0x20, 0x23, 0x3C, 0x0B,
    0x0F, 0x21, 0x06, 0x3D, 0x1B, 0x29, 0x1E, 0x22, 0x1D, 0x24, 0x0E, 0x2B, 0x32, 0x08, 0x2E, 0x03,
    0x04, 0x36, 0x26, 0x33, 0x11, 0x1F, 0x10, 0x02, 0x14, 0x3F, 0x00, 0x09, 0x12, 0x2E, 0x28, 0x20,
    0x3E, 0x0D, 0x2A, 0x17, 0x0C, 0x01, 0x15, 0x19, 0x2E, 0x2C, 0x07, 0x37, 0x35, 0x05, 0x0A, 0x2D,
];

/// `PaletteLUT_2C04_0003` (nesdev, "PPU palettes").
static LUT_2C04_0003: [u8; LUT_LEN] = [
    0x14, 0x25, 0x3A, 0x10, 0x0B, 0x20, 0x31, 0x09, 0x01, 0x2E, 0x36, 0x08, 0x15, 0x3D, 0x3E, 0x3C,
    0x22, 0x1C, 0x05, 0x12, 0x19, 0x18, 0x17, 0x1B, 0x00, 0x03, 0x2E, 0x02, 0x16, 0x06, 0x34, 0x35,
    0x23, 0x0F, 0x0E, 0x37, 0x0D, 0x27, 0x26, 0x20, 0x29, 0x04, 0x21, 0x24, 0x11, 0x2D, 0x2E, 0x1F,
    0x2C, 0x1E, 0x39, 0x33, 0x07, 0x2A, 0x28, 0x1D, 0x0A, 0x2E, 0x32, 0x38, 0x13, 0x2B, 0x3F, 0x0C,
];

/// `PaletteLUT_2C04_0004` (nesdev, "PPU palettes").
static LUT_2C04_0004: [u8; LUT_LEN] = [
    0x18, 0x03, 0x1C, 0x28, 0x2E, 0x35, 0x01, 0x17, 0x10, 0x1F, 0x2A, 0x0E, 0x36, 0x37, 0x0B, 0x39,
    0x25, 0x1E, 0x12, 0x34, 0x2E, 0x1D, 0x06, 0x26, 0x3E, 0x1B, 0x22, 0x19, 0x04, 0x2E, 0x3A, 0x21,
    0x05, 0x0A, 0x07, 0x02, 0x13, 0x14, 0x00, 0x15, 0x0C, 0x3D, 0x11, 0x0F, 0x0D, 0x38, 0x2D, 0x24,
    0x33, 0x20, 0x08, 0x16, 0x3F, 0x2B, 0x20, 0x3C, 0x2E, 0x27, 0x23, 0x31, 0x29, 0x32, 0x2C, 0x09,
];

impl VsPpuModel {
    /// The master palette as a remap into the ordered palette, or `None` when the PPU **is**
    /// the ordered palette — every RP2C03, RC2C03 and RC2C05 (see the module note).
    ///
    /// Chosen once at load: the answer is stored on the PPU and the per-pixel path never sees
    /// this function.
    #[must_use]
    pub const fn palette_lut(self) -> Option<&'static [u8; LUT_LEN]> {
        match self {
            Self::Rp2c04_0001 => Some(&LUT_2C04_0001),
            Self::Rp2c04_0002 => Some(&LUT_2C04_0002),
            Self::Rp2c04_0003 => Some(&LUT_2C04_0003),
            Self::Rp2c04_0004 => Some(&LUT_2C04_0004),
            Self::Rc2c03
            | Self::Rc2c05_01
            | Self::Rc2c05_02
            | Self::Rc2c05_03
            | Self::Rc2c05_04
            | Self::Rc2c05_05 => None,
        }
    }
}
