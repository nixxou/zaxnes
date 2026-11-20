//! The performance / fidelity presets (lot REWRITE-UI, owner's table `docs/design/PRESETS.csv`):
//! the five steps from the fastest to the most faithful, each with the values it writes. The
//! application's options window and console screens choose them (`zax-app/src/ui/presets.rs`);
//! the libretro core follows the ones that mean something in a core.

use crate::catalogue::SettingId;
use crate::values::SettingValue;

/// A value a preset writes (`SettingValue` holds a `String` variant, so it cannot sit in a
/// constant table itself).
#[derive(Clone, Copy)]
pub enum V {
    B(bool),
    I(i32),
    E(i32),
}

impl V {
    #[must_use]
    pub const fn value(self) -> SettingValue {
        match self {
            Self::B(on) => SettingValue::Bool(on),
            Self::I(number) => SettingValue::Int(number),
            Self::E(number) => SettingValue::Enum(number),
        }
    }
}

/// One preset of the slider. Every preset writes the whole set of settings the slider manages,
/// so that a mode puts back what another one changed; each value is either proposed (shown
/// under the slider, to be changed) or set and hidden (owner, 2026-11-18).
pub struct Preset {
    /// Its key in the string tables: `options.preset.<key>.name`, `.summary`, `.acts`.
    pub key: &'static str,
    /// The values the preset writes, in the order the proposed ones are shown, each with
    /// whether it is proposed.
    pub values: &'static [(SettingId, V, bool)],
}

/// The five presets, from the fastest to the most faithful (owner's table,
/// `docs/design/PRESETS.csv`, 2026-11-18).
pub const PRESETS: [Preset; 5] = [
    Preset {
        key: "fastest",
        values: &[
            (SettingId::EmuFastCore, V::B(true), true),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(0), true),
            (SettingId::EmuSmoothScrollAhead, V::B(true), true),
            (SettingId::EmuSmoothScroll3d, V::B(false), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(true), true),
            (SettingId::EmuPipeline, V::E(1), true),
            (SettingId::EmuLuaOptimized, V::B(true), true),
            (SettingId::EmuLuaExperimental, V::B(true), true),
            (SettingId::VideoDynamicResolution, V::B(true), true),
            (SettingId::VideoResScale, V::E(0), true),
            (SettingId::VideoHalfRate3d, V::B(false), true),
            (SettingId::VideoCompositeSignal, V::B(false), false),
            (SettingId::RenderShadow, V::B(false), true),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(0), true),
            (SettingId::RenderStyleReflections, V::I(0), false),
            (SettingId::RenderStyleSsrQuality, V::I(0), false),
            (SettingId::RenderStyleAo, V::I(0), false),
            (SettingId::RenderStyleAoQuality, V::I(0), false),
            (SettingId::RenderStylePixelShadow, V::I(0), false),
        ],
    },
    Preset {
        key: "fast",
        values: &[
            (SettingId::EmuFastCore, V::B(true), true),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuSmoothScrollAhead, V::B(true), true),
            (SettingId::EmuSmoothScroll3d, V::B(false), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(true), true),
            (SettingId::EmuPipeline, V::E(0), true),
            (SettingId::EmuLuaOptimized, V::B(false), true),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(true), true),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), true),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(0), true),
            (SettingId::RenderStyleReflections, V::I(0), false),
            (SettingId::RenderStyleSsrQuality, V::I(0), false),
            (SettingId::RenderStyleAo, V::I(0), false),
            (SettingId::RenderStyleAoQuality, V::I(0), false),
            (SettingId::RenderStylePixelShadow, V::I(0), false),
        ],
    },
    Preset {
        key: "balanced",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuSmoothScrollAhead, V::B(true), true),
            (SettingId::EmuSmoothScroll3d, V::B(false), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(1), false),
            (SettingId::RenderStyleAo, V::I(0), true),
            (SettingId::RenderStyleAoQuality, V::I(1), false),
            (SettingId::RenderStylePixelShadow, V::I(0), true),
        ],
    },
    Preset {
        key: "faithful",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(false), true),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuSmoothScrollAhead, V::B(true), true),
            (SettingId::EmuSmoothScroll3d, V::B(false), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), true),
            (SettingId::EmuLatePresent, V::B(false), true),
            (SettingId::EmuCatchUpLast3d, V::B(false), true),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(false), true),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(2), true),
            (SettingId::RenderStyleAo, V::I(1), true),
            (SettingId::RenderStyleAoQuality, V::I(1), true),
            (SettingId::RenderStylePixelShadow, V::I(1), true),
        ],
    },
    Preset {
        key: "most_faithful",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(true), true),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuSmoothScrollAhead, V::B(true), true),
            (SettingId::EmuSmoothScroll3d, V::B(false), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(true), true),
            (SettingId::EmuLatePresent, V::B(false), true),
            (SettingId::EmuCatchUpLast3d, V::B(false), false),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(false), true),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), true),
            (SettingId::EmuOamaddrWriteCorruption, V::B(true), true),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(2), true),
            (SettingId::RenderStyleAo, V::I(1), true),
            (SettingId::RenderStyleAoQuality, V::I(2), true),
            (SettingId::RenderStylePixelShadow, V::I(1), true),
        ],
    },
];
