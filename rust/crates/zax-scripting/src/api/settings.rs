//! The global `Setting` and the objects it carries — `Layer`, one layer and `Clipping` — with
//! the members, aliases and rights of SPEC-09 §4.2 (REQ-LUA-089).

use std::sync::Arc;

use mlua::Value;
use zax_settings::catalogue::SettingId;

use super::{assigned_bool, boolean, num, type_def};
use crate::binding::cx::{Call, Cx};
use crate::binding::payload::{HostType, Payload};
use crate::binding::table::{Reply, TypeDef};
use crate::settings::{
    ScriptSettings, CLIPPING_COUNT, DEFAULT_SMOOTH_TIME, LAYER_COUNT, NON_VR_SCALE, VR_SCALE,
};
use crate::values::LuaColor;

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.push(setting());
    types.extend(layers());
}

fn reply(value: Value) -> mlua::Result<Reply> {
    Ok(Some(value))
}

/// Writes through the settings of the runtime, then mirrors the change into the pipeline's own
/// slice when the engine state is at hand: the model is the living model REQ-PERS-007 demands,
/// so a write is visible to the very next read, from Lua, from the UI or from the renderer.
pub(super) fn with_settings(
    cx: &Cx,
    operation: impl FnOnce(&mut ScriptSettings),
) -> mlua::Result<()> {
    let mut data = cx.data()?;
    operation(&mut data.env.settings);
    if cx.has_state() {
        data.env.settings.sync(cx.state()?.settings_mut());
    }

    Ok(())
}

fn require(call: &Call, member: &str) -> mlua::Result<()> {
    if call.host(1) == Some(Payload::Table(HostType::SettingApi)) {
        Ok(())
    } else {
        Err(call.invalid(member))
    }
}

fn flag(definition: &mut TypeDef, name: &'static str, id: SettingId) {
    definition.property(
        name,
        move |cx, _| Ok(boolean(cx.data()?.env.settings.flag(id))),
        move |cx, _, value| {
            let wanted = assigned_bool(value);
            with_settings(cx, |settings| settings.write_flag(id, wanted))
        },
        None,
    );
}

fn real(definition: &mut TypeDef, name: &'static str, id: SettingId) {
    definition.property(
        name,
        move |cx, _| Ok(num(cx.data()?.env.settings.real(id))),
        move |cx, _, value| {
            let wanted = cx.real(value, name)?;
            with_settings(cx, |settings| settings.write_real(id, wanted))
        },
        None,
    );
}

fn whole(definition: &mut TypeDef, name: &'static str, id: SettingId) {
    definition.property(
        name,
        move |cx, _| Ok(num(cx.data()?.env.settings.int(id))),
        move |cx, _, value| {
            let wanted = cx.integer(value, name)?;
            with_settings(cx, |settings| settings.write_int(id, wanted))
        },
        None,
    );
}

/// Kept and handed back, with no model of their own in this version (REQ-PERS-016).
fn stored(definition: &mut TypeDef, name: &'static str) {
    definition.property(name, |_, _| Ok(Value::Nil), |_, _, _| Ok(()), None);
}

/// The notification identifiers and the two scale constants of SPEC-09 §4.3 (REQ-PERS-014); no
/// built-in profile reads them.
const SETTING_CONSTANTS: [(&str, &str); 51] = [
    ("SelectionModeId", "SelectionModeId"),
    ("ControlMethodId", "ControlMethodId"),
    ("SceneScaleId", "SceneScaleId"),
    ("LightId", "LightId"),
    ("LightDirectionId", "Light.DirectionId"),
    ("LightColorId", "Light.ColorId"),
    ("LightIntensityId", "Light.IntensityId"),
    ("BgColorId", "BgColorId"),
    ("CustomBgColorId", "CustomBgColorId"),
    ("ClippingId", "Clipping"),
    ("ClippingElementId", "Clipping"),
    ("MapEnableId", "MapEnable"),
    ("LayerId", "Layer"),
    ("LayerAlignedId", "Layer.Aligned"),
    ("LayerOffsetId", "Layer.Offset"),
    ("LayerDefLayerId", "Layer.DefLayer"),
    ("LayerAutoId", "Layer.Auto"),
    ("LocalCamId", "LocalCamId"),
    ("GraphicQualityId", "GraphicQuality"),
    ("VrRuntimeId", "VrRuntimeId"),
    ("PreVrRuntimeId", "PreVrRuntimeId"),
    ("VSyncId", "VSyncId"),
    ("FrameRateId", "FrameRateId"),
    ("FovId", "FovId"),
    ("GameSpeedId", "GameSpeedId"),
    ("VolumeId", "VolumeId"),
    ("RenderModeId", "RenderModeId"),
    ("SkyboxId", "SkyboxId"),
    ("CoreId", "CoreId"),
    ("MirroringId", "MirroringId"),
    ("EmptyTileId", "EmptyTileId"),
    ("InputTypeId", "InputTypeId"),
    ("MovieModeId", "MovieModeId"),
    ("SceneRatioId", "SceneRatioId"),
    ("EmuMenuId", "EmuMenuId"),
    ("RenderMethodId", "RenderMethodId"),
    ("ResScaleId", "ResScaleId"),
    ("SkipFrameId", "SkipFrameId"),
    ("BgMusicId", "BgMusicId"),
    ("AntiAliasId", "AntiAliasId"),
    ("AutoTVModeId", "AutoTVModeId"),
    ("TVModeId", "TVModeId"),
    ("TVBoxId", "TVBoxId"),
    ("SortOptionId", "SortOptionId"),
    ("ShadowId", "ShadowId"),
    ("SameBgPaletteId", "SameBgPaletteId"),
    ("SameSpPaletteId", "SameSpPaletteId"),
    ("SpriteToleranceId", "SpriteToleranceId"),
    ("ScanDirectionId", "ScanDirectionId"),
    ("SavedPattern3DId", "SavePatternId"),
    ("SavedAnimPattern3DId", "SaveAnimPatternId"),
];

fn setting() -> TypeDef {
    let mut definition = type_def(HostType::SettingApi, "SettingAPI");

    definition.read_only(
        "Light",
        |cx, _| cx.object(Payload::Table(HostType::Light)),
        Some(HostType::Light),
    );
    definition.read_only(
        "Layer",
        |cx, _| cx.object(Payload::Table(HostType::LayerConfig)),
        Some(HostType::LayerConfig),
    );
    definition.read_only(
        "Clipping",
        |cx, _| cx.object(Payload::Table(HostType::Clipping)),
        Some(HostType::Clipping),
    );

    flag(&mut definition, "EmptyTile", SettingId::ConvEmptyTile);
    flag(&mut definition, "BgColor", SettingId::SceneUseNesBgColor);
    flag(
        &mut definition,
        "SameBgPalette",
        SettingId::ConvSameBgPalette,
    );
    definition.alias("SamePalette", "SameBgPalette");
    flag(
        &mut definition,
        "SameSpPalette",
        SettingId::ConvSameSpPalette,
    );
    flag(&mut definition, "LocalCam", SettingId::CameraLocalCam);
    definition.alias("TopDownCam", "LocalCam");
    flag(&mut definition, "Shadow", SettingId::RenderShadow);
    flag(&mut definition, "MapEnable", SettingId::SceneMapEnable);
    flag(&mut definition, "Mirroring", SettingId::VrMirroring);
    flag(&mut definition, "SkipFrame", SettingId::VideoSkipFrame);
    flag(&mut definition, "BGMusic", SettingId::AudioBgMusic);
    flag(&mut definition, "AutoTVMode", SettingId::EmuAutoTvMode);
    flag(
        &mut definition,
        "EmulatedMenuButton",
        SettingId::InputEmulatedMenuButton,
    );

    real(&mut definition, "GameSpeed", SettingId::EmuGameSpeed);
    real(&mut definition, "SmoothTime", SettingId::CameraSmoothTime);
    real(&mut definition, "Fov", SettingId::CameraFov);
    real(&mut definition, "Volume", SettingId::AudioVolume);
    real(&mut definition, "SceneRatio", SettingId::VrSceneRatio);
    real(&mut definition, "SceneScale", SettingId::SceneSceneScale);
    real(&mut definition, "RootScale", SettingId::SceneRootScale);

    whole(&mut definition, "VrRuntime", SettingId::VrRuntime);
    whole(&mut definition, "VSync", SettingId::VideoVSync);
    whole(&mut definition, "FrameRate", SettingId::VideoFrameRate);
    whole(
        &mut definition,
        "GraphicQuality",
        SettingId::VideoGraphicQuality,
    );
    whole(&mut definition, "RenderMode", SettingId::RenderMeshAlgo);
    whole(
        &mut definition,
        "RenderMethod",
        SettingId::VideoRenderMethod,
    );
    whole(&mut definition, "ResScaleIndex", SettingId::VideoResScale);
    whole(&mut definition, "AntiAlias", SettingId::VideoAntiAlias);
    whole(
        &mut definition,
        "SpriteTolerance",
        SettingId::ConvSpriteTolerance,
    );
    whole(
        &mut definition,
        "ScanDirection",
        SettingId::ConvScanDirection,
    );
    whole(&mut definition, "Skybox", SettingId::SceneSkybox);
    whole(&mut definition, "Core", SettingId::EmuCore);
    whole(&mut definition, "TVMode", SettingId::EmuTvMode);

    // AMD-031 §3: the TV box is gone but the property stays, so a script that reads or writes
    // it runs without error. The reading is always 0 — « none ».
    definition.property(
        "TVBox",
        |_, _| Ok(num(0)),
        |cx, _, value| cx.integer(value, "TVBox").map(|_| ()),
        None,
    );
    whole(&mut definition, "SortOption", SettingId::UiSortOption);
    whole(&mut definition, "MovieMode", SettingId::MovieMode);
    whole(
        &mut definition,
        "SelectionMode",
        SettingId::EditorSelectionMode,
    );
    whole(&mut definition, "ControlMethod", SettingId::VrControlMethod);

    definition.property(
        "CustomBgColor",
        |cx, _| cx.color(cx.data()?.env.settings.custom_bg_color()),
        |cx, _, value| {
            let Some(color) = cx.try_color(value) else {
                return Err(cx.invalid("CustomBgColor"));
            };

            with_settings(cx, |settings| settings.set_custom_bg_color(color))
        },
        Some(HostType::Color),
    );

    // Derived, read only (REQ-PERS-013).
    definition.read_only(
        "TargetFramerate",
        |cx, _| {
            Ok(num(cx
                .data()?
                .env
                .settings
                .real(SettingId::VideoTargetFramerate)))
        },
        None,
    );
    definition.read_only(
        "TargetTimePerFrame",
        |cx, _| {
            Ok(num(cx
                .data()?
                .env
                .settings
                .real(SettingId::VideoTargetTimePerFrame)))
        },
        None,
    );
    definition.read_only(
        "SkyboxEnable",
        |cx, _| {
            Ok(boolean(
                cx.data()?.env.settings.flag(SettingId::SceneSkyboxEnable),
            ))
        },
        None,
    );
    definition.read_only(
        "ResScale",
        |cx, _| {
            Ok(num(cx
                .data()?
                .env
                .settings
                .real(SettingId::VideoResScaleFactor)))
        },
        None,
    );
    definition.read_only(
        "ZapperEnable",
        |cx, _| {
            let data = cx.data()?;
            Ok(boolean(
                data.env.input.port_type(0) == 1 || data.env.input.port_type(1) == 1,
            ))
        },
        None,
    );

    // The engine mode of SPEC-09 §4.1.10, read only. AMD-020 (Q-052) settles the contradiction
    // between SPEC-09, which publishes the three names, and SPEC-05 §4, which listed none of
    // them: the three are required.
    // Q-052
    definition.read_only(
        "EngineMode",
        |cx, _| Ok(num(cx.data()?.env.settings.engine_mode())),
        None,
    );
    definition.read_only(
        "IsProfileMode",
        |cx, _| {
            Ok(boolean(
                cx.data()?.env.settings.engine_mode() == crate::settings::PROFILE_ENGINE_MODE,
            ))
        },
        None,
    );
    definition.read_only(
        "IsRomMode",
        |cx, _| {
            Ok(boolean(
                cx.data()?.env.settings.engine_mode() == crate::settings::ROM_ENGINE_MODE,
            ))
        },
        None,
    );

    stored(&mut definition, "SavedPattern3D");
    stored(&mut definition, "SaveAnimPattern3D");
    stored(&mut definition, "TVData");

    for (name, value) in SETTING_CONSTANTS {
        definition.read_only(name, move |cx, _| cx.text(value), None);
    }

    definition.read_only(
        "DefaultSmoothTime",
        |_, _| Ok(num(DEFAULT_SMOOTH_TIME)),
        None,
    );
    definition.read_only("VRScale", |_, _| Ok(num(VR_SCALE)), None);
    definition.read_only("NonVRScale", |_, _| Ok(num(NON_VR_SCALE)), None);

    definition.function(
        "GetInputType",
        |call| {
            require(call, "GetInputType")?;
            let port = call.integer(2, "GetInputType")?;
            let kind = usize::try_from(port).map_or(0, |port| {
                call.cx
                    .data()
                    .map_or(0, |data| data.env.input.port_type(port))
            });
            reply(num(kind))
        },
        None,
    );
    definition.function(
        "SetInputType",
        |call| {
            require(call, "SetInputType")?;
            let first = call.integer(2, "SetInputType")?;
            let second = call.integer(3, "SetInputType")?;
            call.cx.data()?.env.input.set_port_types(first, second);
            with_settings(call.cx, |settings| settings.set_port_types(first, second))?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "SetRuntime",
        |call| {
            require(call, "SetRuntime")?;

            // No effect on desktop (REQ-PERS-015).
            Ok(None)
        },
        None,
    );
    definition.function(
        "Save",
        |call| {
            require(call, "Save")?;

            // Writes the settings file at once; nothing happens when the host set no hook
            // (REQ-PERS-015, P3).
            call.cx.data()?.env.settings.request_save();
            Ok(None)
        },
        None,
    );
    definition.function(
        "Load",
        |call| {
            require(call, "Load")?;

            // Re-reads the file and applies only the user preferences it holds (REQ-PERS-015).
            call.cx.data()?.env.settings.request_load();
            Ok(None)
        },
        None,
    );
    definition
}

fn layer_index(payload: &Payload) -> usize {
    match payload {
        Payload::Layer(index) => *index,
        _ => 0,
    }
}

/// `Setting.Layer` and `Setting.Clipping`. Both read and write the settings model through the
/// runtime's [`ScriptSettings`], which is the living model REQ-PERS-007 demands.
fn layers() -> [TypeDef; 3] {
    // Setting.Layer, the global Layer (§4.2, type ConfigCalques).
    let mut config = type_def(HostType::LayerConfig, "ConfigCalques");
    config.property(
        "DefLayer",
        |cx, _| {
            Ok(num(cx
                .data()?
                .env
                .settings
                .int(SettingId::LayerDefaultLayer)))
        },
        |cx, _, value| {
            let wanted = cx.integer(value, "DefLayer")?;
            if usize::try_from(wanted).map_or(true, |layer| layer >= LAYER_COUNT) {
                return Err(cx.raise(&format!("layer {wanted} is out of range for DefLayer")));
            }

            with_settings(cx, |settings| {
                settings.write_int(SettingId::LayerDefaultLayer, wanted);
            })
        },
        None,
    );
    config.property(
        "Auto",
        |cx, _| Ok(boolean(cx.data()?.env.settings.flag(SettingId::LayerAuto))),
        |cx, _, value| {
            let wanted = assigned_bool(value);
            with_settings(cx, |settings| {
                settings.write_flag(SettingId::LayerAuto, wanted);
            })
        },
        None,
    );

    // Layer[i] accepts 0 to 5; out of range raises (REQ-LUA-045, REQ-LUA-063).
    config.element = Some(HostType::Layer);
    config.indexer = Some(Arc::new(|cx, _, key| {
        let index = cx.integer(key, "Layer")?;
        let index = cx.check_bounds(index, LAYER_COUNT, "Layer")?;
        cx.value(Payload::Layer(index))
    }));

    // One layer (§4.2, type Calque).
    let mut layer = type_def(HostType::Layer, "Calque");
    layer.property(
        "Offset",
        |cx, this| Ok(num(cx.data()?.env.settings.layer_offset(layer_index(this)))),
        |cx, this, value| {
            let wanted = cx.real(value, "Offset")?;
            let index = layer_index(this);
            with_settings(cx, |settings| settings.set_layer_offset(index, wanted))
        },
        None,
    );
    layer.property(
        "Aligned",
        |cx, this| {
            Ok(num(cx
                .data()?
                .env
                .settings
                .layer_aligned(layer_index(this))))
        },
        |cx, this, value| {
            let wanted = cx.integer(value, "Aligned")?;
            let index = layer_index(this);
            with_settings(cx, |settings| settings.set_layer_aligned(index, wanted))
        },
        None,
    );
    layer.read_only(
        "Coef",
        |cx, this| Ok(num(cx.data()?.env.settings.layer_coef(layer_index(this)))),
        None,
    );
    layer.property(
        "CastShadow",
        |cx, this| {
            Ok(boolean(
                cx.data()?.env.settings.layer_cast_shadow(layer_index(this)),
            ))
        },
        |cx, this, value| {
            let wanted = assigned_bool(value);
            let index = layer_index(this);
            with_settings(cx, |settings| settings.set_layer_cast_shadow(index, wanted))
        },
        None,
    );
    layer.property(
        "RecvShadow",
        |cx, this| {
            Ok(boolean(
                cx.data()?.env.settings.layer_recv_shadow(layer_index(this)),
            ))
        },
        |cx, this, value| {
            let wanted = assigned_bool(value);
            let index = layer_index(this);
            with_settings(cx, |settings| settings.set_layer_recv_shadow(index, wanted))
        },
        None,
    );

    // Setting.Clipping, the global Clipping (§4.2, type Découpe). Index 0 is the left margin,
    // 1 the bottom, 2 the right, 3 the top (DEC-R2-T1).
    let mut clipping = type_def(HostType::Clipping, "Decoupe");
    for (name, index) in [("x", 0), ("y", 1), ("z", 2), ("w", 3)] {
        clipping.property(
            name,
            move |cx, _| Ok(num(cx.data()?.env.settings.clipping(index))),
            move |cx, _, value| {
                let wanted = cx.real(value, name)?;
                with_settings(cx, |settings| settings.set_clipping(index, wanted))
            },
            None,
        );
    }

    clipping.property(
        "target",
        |cx, _| {
            let margins = cx.data()?.env.settings.clipping_vector();
            cx.color(LuaColor::new(margins.x, margins.y, margins.z, margins.w))
        },
        |cx, _, value| {
            let Some(color) = cx.try_color(value) else {
                return Err(cx.invalid("target"));
            };

            with_settings(cx, |settings| {
                settings.set_clipping_vector(zax_profiles::values::Vec4F::new(
                    color.r, color.g, color.b, color.a,
                ));
            })
        },
        Some(HostType::Color),
    );
    clipping.indexer = Some(Arc::new(|cx, _, key| {
        let index = cx.integer(key, "Clipping")?;
        let index = cx.check_bounds(index, CLIPPING_COUNT, "Clipping")?;
        Ok(num(cx.data()?.env.settings.clipping(index)))
    }));
    clipping.index_setter = Some(Arc::new(|cx, _, key, value| {
        let index = cx.integer(key, "Clipping")?;
        let index = cx.check_bounds(index, CLIPPING_COUNT, "Clipping")?;
        let wanted = cx.real(value, "Clipping")?;
        with_settings(cx, |settings| settings.set_clipping(index, wanted))
    }));
    [config, layer, clipping]
}
