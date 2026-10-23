//! Enumerations of the profile model (SPEC-03 §4). The files store them as plain integers and
//! the built-in profiles hold values outside the documented sets, so every stored enumeration
//! keeps the raw number in an `Other` variant instead of normalising it.

/// Declares an enumeration read from an integer field: the named values plus `Other(raw)` for
/// any other number, so that a value outside the documented set survives a read and a write.
macro_rules! open_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $($(#[$vmeta:meta])* $variant:ident = $value:literal,)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
            /// A value outside the documented set, kept as read.
            Other(i32),
        }

        impl $name {
            /// The enumeration value of a stored integer.
            #[must_use]
            pub const fn from_raw(raw: i32) -> Self {
                match raw {
                    $($value => Self::$variant,)+
                    other => Self::Other(other),
                }
            }

            /// The stored integer.
            #[must_use]
            pub const fn raw(self) -> i32 {
                match self {
                    $(Self::$variant => $value,)+
                    Self::Other(raw) => raw,
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::from_raw(0)
            }
        }
    };
}

/// Declares a set of bit flags stored in an integer; unknown bits are kept.
macro_rules! flag_set {
    (
        $(#[$meta:meta])*
        $name:ident {
            $($(#[$cmeta:meta])* $constant:ident = $value:literal,)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name(i32);

        impl $name {
            $($(#[$cmeta])* pub const $constant: Self = Self($value);)+

            /// The flags of a stored integer.
            #[must_use]
            pub const fn from_bits(bits: i32) -> Self {
                Self(bits)
            }

            /// The stored integer.
            #[must_use]
            pub const fn bits(self) -> i32 {
                self.0
            }

            /// True when every bit of `other` is set.
            #[must_use]
            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }
        }

        impl std::ops::BitOr for $name {
            type Output = Self;

            fn bitor(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }
    };
}

flag_set! {
    /// Mirroring flags of a tile (SPEC-03 REQ-PROF-031): bit 0 horizontal, bit 1 vertical.
    TileFlip {
        /// No mirroring.
        NONE = 0,
        /// Mirror horizontally (columns reversed).
        HORIZONTAL = 1,
        /// Mirror vertically (rows reversed).
        VERTICAL = 2,
    }
}

flag_set! {
    /// Colours present in a pattern (SPEC-03 §4.3, `colorFlags`). Colour 0 is transparent and
    /// has no flag.
    PatternColorFlags {
        /// No colour recorded.
        NONE = 0,
        /// Colour 1 occurs in the pattern.
        COLOR1 = 2,
        /// Colour 2 occurs in the pattern.
        COLOR2 = 4,
        /// Colour 3 occurs in the pattern.
        COLOR3 = 8,
    }
}

open_enum! {
    /// Stored density of a pattern, in tiles (`tDensity`) or in pixels (`pDensity`),
    /// SPEC-03 §4.3. The built-in profiles also hold the undefined values 6 and 8, once each;
    /// they are kept as read.
    PatternDensity {
        /// Empty.
        Empty = 0,
        /// Low.
        Low = 1,
        /// Average.
        Average = 2,
        /// Medium.
        Medium = 3,
        /// High.
        High = 4,
        /// Solid.
        Solid = 5,
    }
}

open_enum! {
    /// Geometry family of a 3D variant (SPEC-03 §4.4, `geo`).
    VariantGeometry {
        /// Default geometry, derived from the tiles.
        Default = 0,
        /// Horizontal cylinder.
        HorizontalCylinder = 1,
        /// Vertical cylinder.
        VerticalCylinder = 2,
        /// Cube.
        Cube = 3,
        /// Half horizontal cylinder.
        HalfHorizontalCylinder = 4,
        /// Half vertical cylinder.
        HalfVerticalCylinder = 5,
    }
}

open_enum! {
    /// Depth layer of a 3D variant and index of the six stored layer parameter sets
    /// (SPEC-03 §4.4 and §4.6, SPEC-09 §4.1.6).
    ProfileLayer {
        /// Not identified.
        Unidentified = 0,
        /// Game interface.
        Ui = 1,
        /// First depth layer.
        L1 = 2,
        /// Second depth layer.
        L2 = 3,
        /// Third depth layer.
        L3 = 4,
        /// Fourth depth layer.
        L4 = 5,
    }
}

open_enum! {
    /// Depth alignment of a layer (SPEC-03 §4.6, `Aligned`).
    LayerAlignment {
        /// Front.
        Front = 0,
        /// Centre.
        Center = 1,
        /// Back.
        Back = 2,
    }
}

open_enum! {
    /// Mesh algorithm override of a variant (SPEC-03 §4.4, `rm`, where 0 means "use the global
    /// setting").
    MeshAlgorithm {
        /// Use the global setting.
        FromSettings = 0,
        /// Greedy meshing.
        Greedy = 1,
        /// Marching cubes.
        Marching = 2,
        /// Greedy meshing, multicolour.
        GreedyMulticolor = 3,
    }
}

open_enum! {
    /// Global mesh algorithm setting (SPEC-03 §4.6 field 13, `RenderMode`).
    GlobalMeshAlgorithm {
        /// Greedy meshing.
        Greedy = 0,
        /// Marching cubes.
        Marching = 1,
        /// Greedy meshing, multicolour.
        GreedyMulticolor = 2,
    }
}

open_enum! {
    /// How a 3D variant is drawn (SPEC-03 §4.4, `renderStyle`).
    VariantRenderStyle {
        /// Hidden.
        Hidden = 0,
        /// Drawn normally.
        Normal = 1,
        /// Shadow only.
        ShadowOnly = 2,
    }
}

open_enum! {
    /// Pixel format of an explicit variant texture (SPEC-03 REQ-PROF-035).
    TextureFormat {
        /// Palette index, three bits per pixel, split between `data` and `extra`.
        Indexed = 0,
        /// One packed ARGB integer per pixel.
        Direct = 1,
    }
}

open_enum! {
    /// Video region of the embedded settings (SPEC-03 §4.6 field 34).
    TvRegionMode {
        /// NTSC.
        Ntsc = 0,
        /// PAL.
        Pal = 1,
        /// Multi-region.
        Multi = 2,
        /// Dendy.
        Dendy = 3,
    }
}

/// Format version of a profile, after the version string has been filtered
/// (SPEC-03 REQ-PROF-021).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileFormatVersion {
    /// Version `"1"`: not supported, the profile is refused.
    Unsupported,
    /// Version `"2"`: the "V2 format" flag is set, which moves the reference point of the
    /// deformation pivot (SPEC-04B) and is exposed to scripts as `IsV2Format` (SPEC-05).
    V2,
    /// Any other value, including `"3"` and an absent version: current format.
    Current,
}
