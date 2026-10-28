//! The NES colour table and the palette of one frame (SPEC-04 §4.1, REQ-E3D-250, REQ-E3D-251).

use std::fmt;

/// An opaque 24-bit colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb24 {
    /// Red component.
    pub r: u8,
    /// Green component.
    pub g: u8,
    /// Blue component.
    pub b: u8,
}

impl Rgb24 {
    /// Creates a colour.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Builds a colour from a packed `0xRRGGBB` integer.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // C# (byte) casts.
    pub const fn from_packed(packed: i32) -> Self {
        Self::new((packed >> 16) as u8, (packed >> 8) as u8, packed as u8)
    }

    /// The colour packed as `0xRRGGBB`.
    #[must_use]
    pub const fn packed(self) -> i32 {
        ((self.r as i32) << 16) | ((self.g as i32) << 8) | self.b as i32
    }
}

impl fmt::Display for Rgb24 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Number of NES colours.
pub const NES_COLOR_COUNT: usize = 64;

const PACKED: [i32; NES_COLOR_COUNT] = [
    0x78_8084, 0x00_00FC, 0x00_00C4, 0x40_28C4, 0x94_008C, 0xAC_0028, 0xAC_1000, 0x8C_1800,
    0x50_3000, 0x00_7800, 0x00_6800, 0x00_5800, 0x00_4058, 0x00_0000, 0x00_0000, 0x00_0000,
    0xBC_C0C4, 0x00_78FC, 0x00_88FC, 0x68_48FC, 0xDC_00D4, 0xE4_0060, 0xFC_3800, 0xE4_6018,
    0xAC_8000, 0x00_B800, 0x00_A800, 0x00_A848, 0x00_8894, 0x2C_2C2C, 0x00_0000, 0x00_0000,
    0xFC_F8FC, 0x38_C0FC, 0x68_88FC, 0x9C_78FC, 0xFC_78FC, 0xFC_589C, 0xFC_7858, 0xFC_A048,
    0xFC_B800, 0xBC_F818, 0x58_D858, 0x58_F89C, 0x00_E8E4, 0x60_6060, 0x00_0000, 0x00_0000,
    0xFC_F8FC, 0xA4_E8FC, 0xBC_B8FC, 0xDC_B8FC, 0xFC_B8FC, 0xF4_C0E0, 0xF4_D0B4, 0xFC_E0B4,
    0xFC_D884, 0xDC_F878, 0xB8_F878, 0xB0_F0D8, 0x00_F8FC, 0xC8_C0C0, 0x00_0000, 0x00_0000,
];

/// The RGB colour of NES colour index `index` (six bits used), from the fixed table of the
/// 64 NES colours (SPEC-04 §4.1, REQ-E3D-251, DEC-04B-Q1).
#[must_use]
pub const fn nes_color(index: usize) -> Rgb24 {
    Rgb24::from_packed(PACKED[index & 0x3F])
}

/// The palette of one frame: 16 tables of 4 NES colour indices (REQ-E3D-250).
///
/// Entry `e` is `p[0]` when `e mod 4 = 0` and `p[e mod 32]` otherwise, where `p` is the 32-byte
/// PPU palette of the batch (SPEC-02 REQ-CORE-045). Tables 0 to 3 are the background
/// sub-palettes, 4 to 7 the sprite ones, and tables 8 to 15 repeat tables 0 to 7 so that a
/// script can use them as custom palettes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramePalette {
    entries: [u8; Self::ENTRY_COUNT],
}

impl Default for FramePalette {
    fn default() -> Self {
        Self {
            entries: [0; Self::ENTRY_COUNT],
        }
    }
}

impl FramePalette {
    /// Number of entries: 16 tables × 4 columns.
    pub const ENTRY_COUNT: usize = 64;

    /// NES colour index (0 to 63) of `entry`, or `None` outside 0..63.
    #[must_use]
    pub fn get(&self, entry: usize) -> Option<u8> {
        self.entries.get(entry).copied()
    }

    /// Writes entry `entry` (six bits kept); false when the entry is outside 0..63.
    pub fn set(&mut self, entry: usize, value: u8) -> bool {
        match self.entries.get_mut(entry) {
            Some(slot) => {
                *slot = value & 0x3F;
                true
            }
            None => false,
        }
    }

    /// The RGB colour of entry `entry` (REQ-E3D-251), or `None` outside 0..63.
    #[must_use]
    pub fn color(&self, entry: usize) -> Option<Rgb24> {
        self.get(entry).map(|index| nes_color(usize::from(index)))
    }

    /// The RGB colour of entry 0, the universal background colour (REQ-E3D-254).
    #[must_use]
    pub const fn universal_color(&self) -> Rgb24 {
        nes_color(self.entries[0] as usize)
    }

    /// Rebuilds the 64 entries from the 32 palette bytes of a frame batch.
    pub fn load_from(&mut self, ppu_palette: &[u8; 32]) {
        let universal = ppu_palette[0] & 0x3F;
        for (e, entry) in self.entries.iter_mut().enumerate() {
            *entry = if e % 4 == 0 {
                universal
            } else {
                ppu_palette[e & 31] & 0x3F
            };
        }
    }

    /// Sets tables 8 to 11 (the copies of the background sub-palettes) to `margin`, a margin
    /// decoder's own background palette (16 NES colour indices, as `$3F00-$3F0F`): the
    /// background tiles in sub-palettes 4 to 7 use them. Entries 0 of the tables keep the
    /// universal background colour.
    pub fn load_margin(&mut self, margin: &[u8; 16]) {
        let universal = self.entries[0];
        for (e, entry) in self.entries[32..48].iter_mut().enumerate() {
            *entry = if e % 4 == 0 {
                universal
            } else {
                margin[e] & 0x3F
            };
        }
    }

    /// Copies the entries of `source` into this palette.
    pub fn copy_from(&mut self, source: &Self) {
        self.entries = source.entries;
    }

    /// Writes the 64 resolved RGB colours of the frame (SPEC-04B REQ-E3D-641).
    pub fn copy_colors_to(&self, destination: &mut [Rgb24; Self::ENTRY_COUNT]) {
        for (color, &entry) in destination.iter_mut().zip(&self.entries) {
            *color = nes_color(usize::from(entry));
        }
    }

    /// Sets every entry to zero.
    pub fn clear(&mut self) {
        self.entries = [0; Self::ENTRY_COUNT];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_margin_palette_fills_tables_8_to_11() {
        let mut ppu = [0u8; 32];
        for (index, byte) in ppu.iter_mut().enumerate() {
            *byte = u8::try_from(index).unwrap();
        }
        let mut palette = FramePalette::default();
        palette.load_from(&ppu);
        // Tables 8-11 repeat the background sub-palettes...
        assert_eq!(palette.get(32 + 5), Some(5));
        let margin: [u8; 16] = std::array::from_fn(|index| 0x30 + u8::try_from(index).unwrap());
        palette.load_margin(&margin);
        // ...until a margin palette replaces them, entry 0 of each staying the universal colour.
        assert_eq!(palette.get(32 + 5), Some(0x35));
        assert_eq!(palette.get(32 + 4), Some(0));
        assert_eq!(palette.get(47), Some(0x3F));
        // The background and sprite tables are left alone.
        assert_eq!(palette.get(5), Some(5));
        assert_eq!(palette.get(16 + 5), Some(21));
    }
}
