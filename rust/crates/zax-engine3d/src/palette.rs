//! The NES colour table and the palette of one frame (SPEC-04 §4.1, REQ-E3D-250, REQ-E3D-251).

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};

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

/// Mesen 2, its default palette (`NesConfig.UserPalette`).
const MESEN: [i32; NES_COLOR_COUNT] = [
    0x66_6666, 0x00_2A88, 0x14_12A7, 0x3B_00A4, 0x5C_007E, 0x6E_0040, 0x6C_0600, 0x56_1D00,
    0x33_3500, 0x0B_4800, 0x00_5200, 0x00_4F08, 0x00_404D, 0x00_0000, 0x00_0000, 0x00_0000,
    0xAD_ADAD, 0x15_5FD9, 0x42_40FF, 0x75_27FE, 0xA0_1ACC, 0xB7_1E7B, 0xB5_3120, 0x99_4E00,
    0x6B_6D00, 0x38_8700, 0x0C_9300, 0x00_8F32, 0x00_7C8D, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFF_FEFF, 0x64_B0FF, 0x92_90FF, 0xC6_76FF, 0xF3_6AFF, 0xFE_6ECC, 0xFE_8170, 0xEA_9E22,
    0xBC_BE00, 0x88_D800, 0x5C_E430, 0x45_E082, 0x48_CDDE, 0x4F_4F4F, 0x00_0000, 0x00_0000,
    0xFF_FEFF, 0xC0_DFFF, 0xD3_D2FF, 0xE8_C8FF, 0xFB_C2FF, 0xFE_C4EA, 0xFE_CCC5, 0xF7_D8A5,
    0xE4_E594, 0xCF_EF96, 0xBD_F4AB, 0xB3_F3CC, 0xB5_EBF2, 0xB8_B8B8, 0x00_0000, 0x00_0000,
];

/// FCEUX, `output/palettes/FCEUX.pal`.
const FCEUX: [i32; NES_COLOR_COUNT] = [
    0x74_7474, 0x24_188C, 0x00_00A8, 0x44_009C, 0x8C_0074, 0xA8_0010, 0xA4_0000, 0x7C_0800,
    0x40_2C00, 0x00_4400, 0x00_5000, 0x00_3C14, 0x18_3C5C, 0x00_0000, 0x00_0000, 0x00_0000,
    0xBC_BCBC, 0x00_70EC, 0x20_38EC, 0x80_00F0, 0xBC_00BC, 0xE4_0058, 0xD8_2800, 0xC8_4C0C,
    0x88_7000, 0x00_9400, 0x00_A800, 0x00_9038, 0x00_8088, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFC_FCFC, 0x3C_BCFC, 0x5C_94FC, 0xCC_88FC, 0xF4_78FC, 0xFC_74B4, 0xFC_7460, 0xFC_9838,
    0xF0_BC3C, 0x80_D010, 0x4C_DC48, 0x58_F898, 0x00_E8D8, 0x78_7878, 0x00_0000, 0x00_0000,
    0xFC_FCFC, 0xA8_E4FC, 0xC4_D4FC, 0xD4_C8FC, 0xFC_C4FC, 0xFC_C4D8, 0xFC_BCB0, 0xFC_D8A8,
    0xFC_E4A0, 0xE0_FCA0, 0xA8_F0BC, 0xB0_FCCC, 0x9C_FCF0, 0xC4_C4C4, 0x00_0000, 0x00_0000,
];

/// Nestopia's default (YUV), as FCEUX ships it (`nestopia_yuv.pal`).
const NESTOPIA_YUV: [i32; NES_COLOR_COUNT] = [
    0x66_6666, 0x00_2A88, 0x14_12A7, 0x3B_00A4, 0x5C_007E, 0x6E_0040, 0x6C_0700, 0x56_1D00,
    0x33_3500, 0x0C_4800, 0x00_5200, 0x00_4F08, 0x00_404D, 0x00_0000, 0x00_0000, 0x00_0000,
    0xAD_ADAD, 0x15_5FD9, 0x42_40FF, 0x75_27FE, 0xA0_1ACC, 0xB7_1E7B, 0xB5_3120, 0x99_4E00,
    0x6B_6D00, 0x38_8700, 0x0D_9300, 0x00_8F32, 0x00_7C8D, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0x64_B0FF, 0x92_90FF, 0xC6_76FF, 0xF2_6AFF, 0xFF_6ECC, 0xFF_8170, 0xEA_9E22,
    0xBC_BE00, 0x88_D800, 0x5C_E430, 0x45_E082, 0x48_CDDE, 0x4F_4F4F, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0xC0_DFFF, 0xD3_D2FF, 0xE8_C8FF, 0xFA_C2FF, 0xFF_C4EA, 0xFF_CCC5, 0xF7_D8A5,
    0xE4_E594, 0xCF_EF96, 0xBD_F4AB, 0xB3_F3CC, 0xB5_EBF2, 0xB8_B8B8, 0x00_0000, 0x00_0000,
];

/// Nestopia RGB (`nestopia_rgb.pal`).
const NESTOPIA_RGB: [i32; NES_COLOR_COUNT] = [
    0x6D_6D6D, 0x00_2492, 0x00_00DB, 0x6D_49DB, 0x92_006D, 0xB6_006D, 0xB6_2400, 0x92_4900,
    0x6D_4900, 0x24_4900, 0x00_6D24, 0x00_9200, 0x00_4949, 0x00_0000, 0x00_0000, 0x00_0000,
    0xB6_B6B6, 0x00_6DDB, 0x00_49FF, 0x92_00FF, 0xB6_00FF, 0xFF_0092, 0xFF_0000, 0xDB_6D00,
    0x92_6D00, 0x24_9200, 0x00_9200, 0x00_B66D, 0x00_9292, 0x24_2424, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0x6D_B6FF, 0x92_92FF, 0xDB_6DFF, 0xFF_00FF, 0xFF_6DFF, 0xFF_9200, 0xFF_B600,
    0xDB_DB00, 0x6D_DB00, 0x00_FF00, 0x49_FFDB, 0x00_FFFF, 0x49_4949, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0xB6_DBFF, 0xDB_B6FF, 0xFF_B6FF, 0xFF_92FF, 0xFF_B6B6, 0xFF_DB92, 0xFF_FF49,
    0xFF_FF6D, 0xB6_FF49, 0x92_FF6D, 0x49_FFDB, 0x92_DBFF, 0x92_9292, 0x00_0000, 0x00_0000,
];

/// `FirebrandX`, Smooth (`Smooth_FBX.pal`).
const SMOOTH_FBX: [i32; NES_COLOR_COUNT] = [
    0x6A_6D6A, 0x00_1380, 0x1E_008A, 0x39_007A, 0x55_0056, 0x5A_0018, 0x4F_1000, 0x3D_1C00,
    0x25_3200, 0x00_3D00, 0x00_4000, 0x00_3924, 0x00_2E55, 0x00_0000, 0x00_0000, 0x00_0000,
    0xB9_BCB9, 0x18_50C7, 0x4B_30E3, 0x73_22D6, 0x95_1FA9, 0x9D_285C, 0x98_3700, 0x7F_4C00,
    0x5E_6400, 0x22_7700, 0x02_7E02, 0x00_7645, 0x00_6E8A, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0x68_A6FF, 0x8C_9CFF, 0xB5_86FF, 0xD9_75FD, 0xE3_77B9, 0xE5_8D68, 0xD4_9D29,
    0xB3_AF0C, 0x7B_C211, 0x55_CA47, 0x46_CB81, 0x47_C1C5, 0x4A_4D4A, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0xCC_EAFF, 0xDD_DEFF, 0xEC_DAFF, 0xF8_D7FE, 0xFC_D6F5, 0xFD_DBCF, 0xF9_E7B5,
    0xF1_F0AA, 0xDA_FAA9, 0xC9_FFBC, 0xC3_FBD7, 0xC4_F6F6, 0xBE_C1BE, 0x00_0000, 0x00_0000,
];

/// `FirebrandX`, the NES Classic's (`NES_Classic_FBX.pal`).
const NES_CLASSIC_FBX: [i32; NES_COLOR_COUNT] = [
    0x61_6161, 0x00_0088, 0x1F_0D99, 0x37_1379, 0x56_1260, 0x5D_0010, 0x52_0E00, 0x3A_2308,
    0x21_350C, 0x0D_410E, 0x17_4417, 0x00_3A1F, 0x00_2F57, 0x00_0000, 0x00_0000, 0x00_0000,
    0xAA_AAAA, 0x0D_4DC4, 0x4B_24DE, 0x69_12CF, 0x90_14AD, 0x9D_1C48, 0x92_3404, 0x73_5005,
    0x5D_6913, 0x16_7A11, 0x13_8008, 0x12_7649, 0x1C_6691, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFC_FCFC, 0x63_9AFC, 0x8A_7EFC, 0xB0_6AFC, 0xDD_6DF2, 0xE7_71AB, 0xE3_8658, 0xCC_9E22,
    0xA8_B100, 0x72_C100, 0x5A_CD4E, 0x34_C28E, 0x4F_BECE, 0x42_4242, 0x00_0000, 0x00_0000,
    0xFC_FCFC, 0xBE_D4FC, 0xCA_CAFC, 0xD9_C4FC, 0xEC_C1FC, 0xFA_C3E7, 0xF7_CEC3, 0xE2_CDA7,
    0xDA_DB9C, 0xC8_E39E, 0xBF_E5B8, 0xB2_EBC8, 0xB7_E5EB, 0xAC_ACAC, 0x00_0000, 0x00_0000,
];

/// `FirebrandX`, Composite Direct (`Composite_Direct_FBX.pal`).
const COMPOSITE_DIRECT_FBX: [i32; NES_COLOR_COUNT] = [
    0x65_6565, 0x00_127D, 0x18_008E, 0x36_0082, 0x56_005D, 0x5A_0018, 0x4F_0500, 0x38_1900,
    0x1D_3100, 0x00_3D00, 0x00_4100, 0x00_3B17, 0x00_2E55, 0x00_0000, 0x00_0000, 0x00_0000,
    0xAF_AFAF, 0x19_4EC8, 0x47_2FE3, 0x6B_1FD7, 0x93_1BAE, 0x9E_1A5E, 0x99_3200, 0x7B_4B00,
    0x5B_6700, 0x26_7A00, 0x00_8200, 0x00_7A3E, 0x00_6E8A, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0x64_A9FF, 0x8E_89FF, 0xB6_76FF, 0xE0_6FFF, 0xEF_6CC4, 0xF0_806A, 0xD8_982C,
    0xB9_B40A, 0x83_CB0C, 0x5B_D63F, 0x4A_D17E, 0x4D_C7CB, 0x4C_4C4C, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0xC7_E5FF, 0xD9_D9FF, 0xE9_D1FF, 0xF9_CEFF, 0xFF_CCF1, 0xFF_D4CB, 0xF8_DFB1,
    0xED_EAA4, 0xD6_F4A4, 0xC5_F8B8, 0xBE_F6D3, 0xBF_F1F1, 0xB9_B9B9, 0x00_0000, 0x00_0000,
];

/// `FirebrandX`, PVM Style D93 (`PVM_Style_D93_FBX.pal`).
const PVM_STYLE_FBX: [i32; NES_COLOR_COUNT] = [
    0x69_6B63, 0x00_1774, 0x1E_0087, 0x34_0073, 0x56_0057, 0x5E_0013, 0x53_1A00, 0x3B_2400,
    0x24_3000, 0x06_3A00, 0x00_3F00, 0x00_3B1E, 0x00_334E, 0x00_0000, 0x00_0000, 0x00_0000,
    0xB9_BBB3, 0x14_53B9, 0x4D_2CDA, 0x67_1EDE, 0x98_189C, 0x9D_2344, 0xA0_3E00, 0x8D_5500,
    0x65_6D00, 0x2C_7900, 0x00_8100, 0x00_7D42, 0x00_788A, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0x69_A8FF, 0x96_91FF, 0xB2_8AFA, 0xEA_7DFA, 0xF3_7BC7, 0xF2_8E59, 0xE6_AD27,
    0xD7_C805, 0x90_DF07, 0x64_E53C, 0x45_E27D, 0x48_D5D9, 0x4E_5048, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0xD2_EAFF, 0xE2_E2FF, 0xE9_D8FF, 0xF5_D2FF, 0xF8_D9EA, 0xFA_DEB9, 0xF9_E89B,
    0xF3_F28C, 0xD3_FA91, 0xB8_FCA8, 0xAE_FACA, 0xCA_F3F3, 0xBE_C0B8, 0x00_0000, 0x00_0000,
];

/// the Sony CXA2025AS decoder, US (`SONY_CXA2025AS_US.pal`).
const SONY_CXA2025_AS: [i32; NES_COLOR_COUNT] = [
    0x58_5858, 0x00_238C, 0x00_139B, 0x2D_0585, 0x5D_0052, 0x7A_0017, 0x7A_0800, 0x5F_1800,
    0x35_2A00, 0x09_3900, 0x00_3F00, 0x00_3C22, 0x00_325D, 0x00_0000, 0x00_0000, 0x00_0000,
    0xA1_A1A1, 0x00_53EE, 0x15_3CFE, 0x60_28E4, 0xA9_1D98, 0xD4_1E41, 0xD2_2C00, 0xAA_4400,
    0x6C_5E00, 0x2D_7300, 0x00_7D06, 0x00_7852, 0x00_69A9, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0x1F_A5FE, 0x5E_89FE, 0xB5_72FE, 0xFE_65F6, 0xFE_6790, 0xFE_773C, 0xFE_9308,
    0xC4_B200, 0x79_CA10, 0x3A_D54A, 0x11_D1A4, 0x06_BFFE, 0x42_4242, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0xA0_D9FE, 0xBD_CCFE, 0xE1_C2FE, 0xFE_BCFB, 0xFE_BDD0, 0xFE_C5A9, 0xFE_D18E,
    0xE9_DE86, 0xC7_E992, 0xA8_EEB0, 0x95_ECD9, 0x91_E4FE, 0xAC_ACAC, 0x00_0000, 0x00_0000,
];

/// Wavebeam (`Wavebeam.pal`).
const WAVEBEAM: [i32; NES_COLOR_COUNT] = [
    0x6B_6B6B, 0x00_1B88, 0x21_009A, 0x40_008C, 0x60_0067, 0x64_001E, 0x59_0800, 0x48_1600,
    0x28_3600, 0x00_4500, 0x00_4908, 0x00_421D, 0x00_3659, 0x00_0000, 0x00_0000, 0x00_0000,
    0xB4_B4B4, 0x15_55D3, 0x43_37EF, 0x74_25DF, 0x9C_19B9, 0xAC_0F64, 0xAA_2C00, 0x8A_4B00,
    0x66_6B00, 0x21_8300, 0x00_8A00, 0x00_8144, 0x00_7691, 0x00_0000, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0x63_B2FF, 0x7C_9CFF, 0xC0_7DFE, 0xE9_77FF, 0xF5_72CD, 0xF4_886B, 0xDD_A029,
    0xBD_BD0A, 0x89_D20E, 0x5C_DE3E, 0x4B_D886, 0x4D_CFD2, 0x52_5252, 0x00_0000, 0x00_0000,
    0xFF_FFFF, 0xBC_DFFF, 0xD2_D2FF, 0xE1_C8FF, 0xEF_C7FF, 0xFF_C3E1, 0xFF_CAC6, 0xF2_DAAD,
    0xEB_E3A0, 0xD2_EDA2, 0xBC_F4B4, 0xB5_F1CE, 0xB6_ECF1, 0xBF_BFBF, 0x00_0000, 0x00_0000,
];

/// The table of RGB colours the 64 NES colours are shown in (Rust-only, owner request:
/// `video.nes_palette`). The console outputs a composite signal, not RGB: each emulator decodes
/// it into a table of its own. `Zaxnes` is the fixed table of SPEC-04 §4.1, the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NesPalette {
    /// Zaxnes' own (3DSen's), the default.
    #[default]
    Zaxnes,
    /// Mesen: Mesen 2, its default palette (`NesConfig.UserPalette`).
    Mesen,
    /// FCEUX: FCEUX, `output/palettes/FCEUX.pal`.
    Fceux,
    /// Nestopia (YUV): Nestopia's default (YUV), as FCEUX ships it (`nestopia_yuv.pal`).
    NestopiaYuv,
    /// Nestopia (RGB): Nestopia RGB (`nestopia_rgb.pal`).
    NestopiaRgb,
    /// Smooth (`FirebrandX`): `FirebrandX`, Smooth (`Smooth_FBX.pal`).
    SmoothFbx,
    /// NES Classic (`FirebrandX`): `FirebrandX`, the NES Classic's (`NES_Classic_FBX.pal`).
    NesClassicFbx,
    /// Composite Direct (`FirebrandX`): `FirebrandX`, Composite Direct (`Composite_Direct_FBX.pal`).
    CompositeDirectFbx,
    /// PVM Style (`FirebrandX`): `FirebrandX`, PVM Style D93 (`PVM_Style_D93_FBX.pal`).
    PvmStyleFbx,
    /// Sony CXA2025AS: the Sony CXA2025AS decoder, US (`SONY_CXA2025AS_US.pal`).
    SonyCxa2025As,
    /// Wavebeam: Wavebeam (`Wavebeam.pal`).
    Wavebeam,
}

impl NesPalette {
    /// The palettes in the order of the `video.nes_palette` choices.
    pub const LABELS: [&'static str; 11] = [
        "Zaxnes",
        "Mesen",
        "FCEUX",
        "Nestopia (YUV)",
        "Nestopia (RGB)",
        "Smooth (FirebrandX)",
        "NES Classic (FirebrandX)",
        "Composite Direct (FirebrandX)",
        "PVM Style (FirebrandX)",
        "Sony CXA2025AS",
        "Wavebeam",
    ];

    /// The palette of choice `index` of `video.nes_palette` (`Zaxnes` for any other).
    #[must_use]
    pub const fn from_index(index: u8) -> Self {
        match index {
            1 => Self::Mesen,
            2 => Self::Fceux,
            3 => Self::NestopiaYuv,
            4 => Self::NestopiaRgb,
            5 => Self::SmoothFbx,
            6 => Self::NesClassicFbx,
            7 => Self::CompositeDirectFbx,
            8 => Self::PvmStyleFbx,
            9 => Self::SonyCxa2025As,
            10 => Self::Wavebeam,
            _ => Self::Zaxnes,
        }
    }

    const fn table(self) -> &'static [i32; NES_COLOR_COUNT] {
        match self {
            Self::Zaxnes => &PACKED,
            Self::Mesen => &MESEN,
            Self::Fceux => &FCEUX,
            Self::NestopiaYuv => &NESTOPIA_YUV,
            Self::NestopiaRgb => &NESTOPIA_RGB,
            Self::SmoothFbx => &SMOOTH_FBX,
            Self::NesClassicFbx => &NES_CLASSIC_FBX,
            Self::CompositeDirectFbx => &COMPOSITE_DIRECT_FBX,
            Self::PvmStyleFbx => &PVM_STYLE_FBX,
            Self::SonyCxa2025As => &SONY_CXA2025_AS,
            Self::Wavebeam => &WAVEBEAM,
        }
    }
}

/// The palette chosen, by its `video.nes_palette` index: one for the whole process, the colours
/// being the host's display, not the emulation's.
static SELECTED: AtomicU8 = AtomicU8::new(0);

/// Shows the NES colours in `index`'s palette of `video.nes_palette` from now on.
pub fn select_nes_palette(index: u8) {
    SELECTED.store(index, Ordering::Relaxed);
}

/// The palette the NES colours are shown in.
#[must_use]
pub fn selected_nes_palette() -> NesPalette {
    NesPalette::from_index(SELECTED.load(Ordering::Relaxed))
}

/// The RGB colour of NES colour index `index` (six bits used), in the palette chosen
/// ([`select_nes_palette`]; by default the fixed table of the 64 NES colours, SPEC-04 §4.1,
/// REQ-E3D-251, DEC-04B-Q1).
#[must_use]
pub fn nes_color(index: usize) -> Rgb24 {
    nes_color_in(selected_nes_palette(), index)
}

/// The RGB colour of NES colour index `index` (six bits used) in `palette`.
#[must_use]
pub const fn nes_color_in(palette: NesPalette, index: usize) -> Rgb24 {
    Rgb24::from_packed(palette.table()[index & 0x3F])
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
    pub fn universal_color(&self) -> Rgb24 {
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

#[cfg(test)]
mod palette_choice_tests {
    use super::*;

    #[test]
    fn each_palette_has_its_own_colours() {
        // Colour $00 (the dark grey) and $30 (white) of a few of them, as their emulators give.
        assert_eq!(
            nes_color_in(NesPalette::Zaxnes, 0),
            Rgb24::new(0x78, 0x80, 0x84)
        );
        assert_eq!(
            nes_color_in(NesPalette::Mesen, 0),
            Rgb24::new(0x66, 0x66, 0x66)
        );
        assert_eq!(
            nes_color_in(NesPalette::Fceux, 0),
            Rgb24::new(0x74, 0x74, 0x74)
        );
        assert_eq!(
            nes_color_in(NesPalette::Fceux, 0x30),
            Rgb24::new(0xFC, 0xFC, 0xFC)
        );
        assert_eq!(NesPalette::from_index(0), NesPalette::Zaxnes);
        assert_eq!(NesPalette::from_index(2), NesPalette::Fceux);
        assert_eq!(NesPalette::from_index(200), NesPalette::Zaxnes);
        assert_eq!(
            NesPalette::from_index(u8::try_from(NesPalette::LABELS.len() - 1).unwrap()),
            NesPalette::Wavebeam
        );
    }
}
