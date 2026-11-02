//! The global `Setting` and the objects it carries — `Layer`, one layer and `Clipping` — with
//! the members, aliases and rights of SPEC-09 §4.2 (REQ-LUA-089).

use std::sync::Arc;

use mlua::Value;
use zax_settings::catalogue::SettingId;
use zax_settings::styles;

use super::{assigned_bool, boolean, num, type_def};
use crate::binding::cx::{host_payload, try_color, Call, Cx};
use crate::binding::payload::{HostType, Payload, StyleParameterLog};
use crate::binding::table::TypeDef;
use crate::settings::{
    ScriptSettings, CLIPPING_COUNT, DEFAULT_SMOOTH_TIME, LAYER_COUNT, NON_VR_SCALE, VR_SCALE,
};
use crate::values::LuaColor;

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.push(setting());
    types.extend(layers());
    types.push(style_parameter_list());
    types.push(style_parameter());
}

/// Writes through the settings of the runtime, then mirrors the change into the pipeline's own
/// slice when the engine state is at hand: the model is the living model REQ-PERS-007 demands,
/// so a write is visible to the very next read, from Lua, from the UI or from the renderer.
pub(super) fn with_settings(
    cx: &Cx,
    operation: impl FnOnce(&mut ScriptSettings),
) -> mlua::Result<()> {
    let has_state = cx.has_state();
    let view_reset = cx.data(|data| {
        operation(&mut data.env.settings);
        if has_state {
            cx.state(|state| data.settle_settings(Some(state.settings_mut())))
        } else {
            Ok(data.settle_settings(None))
        }
    })??;
    if view_reset {
        // The field of view reaches the camera first, so that `Camera.ResetDistance` and the
        // distance the reset writes follow the setting (AMD-040 REQ-LUA-113).
        let fov = read_settings(cx, |settings| settings.real(SettingId::CameraFov))?;
        cx.host_write(move |ctx| {
            ctx.camera.set_fov(fov);
            ctx.camera.reset_view();
        })?;
    }
    Ok(())
}

/// Reads from the settings of the runtime.
fn read_settings<R>(cx: &Cx, operation: impl FnOnce(&ScriptSettings) -> R) -> mlua::Result<R> {
    cx.data(|data| operation(&data.env.settings))
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
        move |cx, _| Ok(boolean(cx.data(|data| data.env.settings.flag(id))?)),
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
        move |cx, _| Ok(num(cx.data(|data| data.env.settings.real(id))?)),
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
        move |cx, _| Ok(num(cx.data(|data| data.env.settings.int(id))?)),
        move |cx, _, value| {
            let wanted = cx.integer(value, name)?;
            with_settings(cx, |settings| settings.write_int(id, wanted))
        },
        None,
    );
}

/// A whole member whose written value goes through `rule` before it reaches the model (AMD-059
/// REQ-LUA-116). The four members of the 2.0 need it because the general rule of REQ-PERS-003
/// sends an out-of-domain value to the **default**, where the amendment asks for a precise
/// mapping; the value that reaches the model is therefore always inside its domain, which is
/// what makes "no numeric write raises an error" true whatever the number.
fn mapped(definition: &mut TypeDef, name: &'static str, id: SettingId, rule: fn(i32) -> i32) {
    definition.property(
        name,
        move |cx, _| Ok(num(cx.data(|data| data.env.settings.int(id))?)),
        move |cx, _, value| {
            let wanted = rule(cx.integer(value, name)?);
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

    setting_objects(&mut definition);

    setting_flags(&mut definition);

    setting_values(&mut definition);

    setting_derived(&mut definition);

    setting_engine_mode_and_constants(&mut definition);

    setting_functions(&mut definition);
    definition
}

/// The objects `Setting` carries: the light, the layers and the clipping.
fn setting_objects(definition: &mut TypeDef) {
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
}

/// The boolean settings.
fn setting_flags(definition: &mut TypeDef) {
    flag(definition, "EmptyTile", SettingId::ConvEmptyTile);
    flag(definition, "BgColor", SettingId::SceneUseNesBgColor);
    flag(definition, "SameBgPalette", SettingId::ConvSameBgPalette);
    definition.alias("SamePalette", "SameBgPalette");
    flag(definition, "SameSpPalette", SettingId::ConvSameSpPalette);
    flag(definition, "LocalCam", SettingId::CameraLocalCam);
    definition.alias("TopDownCam", "LocalCam");
    flag(definition, "Shadow", SettingId::RenderShadow);
    flag(definition, "MapEnable", SettingId::SceneMapEnable);
    flag(definition, "Mirroring", SettingId::VrMirroring);
    flag(definition, "SkipFrame", SettingId::VideoSkipFrame);
    flag(definition, "BGMusic", SettingId::AudioBgMusic);
    flag(definition, "AutoTVMode", SettingId::EmuAutoTvMode);
    flag(
        definition,
        "EmulatedMenuButton",
        SettingId::InputEmulatedMenuButton,
    );
}

/// The real and whole settings.
fn setting_values(definition: &mut TypeDef) {
    real(definition, "GameSpeed", SettingId::EmuGameSpeed);
    real(definition, "SmoothTime", SettingId::CameraSmoothTime);
    real(definition, "Fov", SettingId::CameraFov);
    real(definition, "Volume", SettingId::AudioVolume);
    real(definition, "SceneRatio", SettingId::VrSceneRatio);
    real(definition, "SceneScale", SettingId::SceneSceneScale);
    real(definition, "RootScale", SettingId::SceneRootScale);

    whole(definition, "VrRuntime", SettingId::VrRuntime);
    whole(definition, "VSync", SettingId::VideoVSync);
    whole(definition, "FrameRate", SettingId::VideoFrameRate);
    whole(definition, "GraphicQuality", SettingId::VideoGraphicQuality);
    whole(definition, "RenderMode", SettingId::RenderMeshAlgo);
    whole(definition, "RenderMethod", SettingId::VideoRenderMethod);
    whole(definition, "ResScaleIndex", SettingId::VideoResScale);
    whole(definition, "AntiAlias", SettingId::VideoAntiAlias);
    whole(
        definition,
        "SpriteTolerance",
        SettingId::ConvSpriteTolerance,
    );
    whole(definition, "ScanDirection", SettingId::ConvScanDirection);
    whole(definition, "Skybox", SettingId::SceneSkybox);
    whole(definition, "Core", SettingId::EmuCore);
    whole(definition, "TVMode", SettingId::EmuTvMode);

    // AMD-031 §3: the TV box is gone but the property stays, so a script that reads or writes
    // it runs without error. The reading is always 0 — « none ».
    definition.property(
        "TVBox",
        |_, _| Ok(num(0)),
        |cx, _, value| cx.integer(value, "TVBox").map(|_| ()),
        None,
    );
    // AMD-040 REQ-LUA-114: the run-ahead of AMD-041, read and written, a write brought back
    // into [0, 3] by the setting's domain. The frames it asks the loop to run ahead are a later
    // lot's; the value is persisted and handed back from here on.
    whole(definition, "RunAheadFrames", SettingId::EmuRunAhead);

    // AMD-040 REQ-LUA-114, amended by AMD-055 REQ-SCN-078: the visual style. Zaxnes has a
    // single renderer in this lot, so the style is kept and handed back — 4 for Enhanced, with
    // 1 accepted in writing, and 0 for anything else.
    whole(definition, "VisualStyle", SettingId::RenderVisualStyle);

    // AMD-059 REQ-LUA-116: the four scalar members 3dSen 2.0 added. `MaxQueuedFrames` **is** the
    // `video.max_queued_frames` of AMD-052, not a second key; `LibraryView` keeps whatever is
    // written; the other two are kept and handed back with no effect **(D)**.
    whole(definition, "LibraryView", SettingId::UiLibraryView);
    mapped(
        definition,
        "MaxQueuedFrames",
        SettingId::VideoMaxQueuedFrames,
        |value| if value <= 1 { 1 } else { 2 },
    );
    mapped(
        definition,
        "RefreshRate",
        SettingId::VrRefreshRate,
        // The nearest rung of {72, 90, 120}, the smaller one at equal distance: the halfway
        // points are 81 and 105.
        |value| {
            if value <= 81 {
                72
            } else if value <= 105 {
                90
            } else {
                120
            }
        },
    );
    mapped(definition, "Msaa", SettingId::VideoMsaa20, |value| {
        if value <= 1 {
            0
        } else if value <= 3 {
            2
        } else {
            4
        }
    });
    whole(definition, "SortOption", SettingId::UiSortOption);
    whole(definition, "MovieMode", SettingId::MovieMode);
    whole(definition, "SelectionMode", SettingId::EditorSelectionMode);
    whole(definition, "ControlMethod", SettingId::VrControlMethod);
}

/// The background colour and the derived, read-only members (REQ-PERS-013).
fn setting_derived(definition: &mut TypeDef) {
    definition.property(
        "CustomBgColor",
        |cx, _| cx.color(cx.data(|data| data.env.settings.custom_bg_color())?),
        |cx, _, value| {
            let Some(color) = try_color(value) else {
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
            Ok(num(read_settings(cx, |settings| {
                settings.real(SettingId::VideoTargetFramerate)
            })?))
        },
        None,
    );
    definition.read_only(
        "TargetTimePerFrame",
        |cx, _| {
            Ok(num(read_settings(cx, |settings| {
                settings.real(SettingId::VideoTargetTimePerFrame)
            })?))
        },
        None,
    );
    definition.read_only(
        "SkyboxEnable",
        |cx, _| {
            Ok(boolean(cx.data(|data| {
                data.env.settings.flag(SettingId::SceneSkyboxEnable)
            })?))
        },
        None,
    );
    definition.read_only(
        "ResScale",
        |cx, _| {
            Ok(num(read_settings(cx, |settings| {
                settings.real(SettingId::VideoResScaleFactor)
            })?))
        },
        None,
    );
    definition.read_only(
        "ZapperEnable",
        |cx, _| {
            Ok(boolean(cx.host(|ctx| {
                ctx.machine.port_type(0) == 1 || ctx.machine.port_type(1) == 1
            })?))
        },
        None,
    );
}

/// The engine mode, the stored members and the constants of SPEC-09 §4.3.
fn setting_engine_mode_and_constants(definition: &mut TypeDef) {
    // The engine mode of SPEC-09 §4.1.10, read only. AMD-020 (Q-052) settles the contradiction
    // between SPEC-09, which publishes the three names, and SPEC-05 §4, which listed none of
    // them: the three are required.
    // Q-052
    definition.read_only(
        "EngineMode",
        |cx, _| Ok(num(cx.data(|data| data.env.settings.engine_mode())?)),
        None,
    );
    definition.read_only(
        "IsProfileMode",
        |cx, _| {
            Ok(boolean(
                cx.data(|data| data.env.settings.engine_mode())?
                    == crate::settings::PROFILE_ENGINE_MODE,
            ))
        },
        None,
    );
    definition.read_only(
        "IsRomMode",
        |cx, _| {
            Ok(boolean(
                cx.data(|data| data.env.settings.engine_mode())?
                    == crate::settings::ROM_ENGINE_MODE,
            ))
        },
        None,
    );

    stored(definition, "SavedPattern3D");
    stored(definition, "SaveAnimPattern3D");
    stored(definition, "TVData");

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
}

/// The methods of `Setting`.
fn setting_functions(definition: &mut TypeDef) {
    definition.function(
        "GetInputType",
        |call| {
            require(call, "GetInputType")?;
            let port = call.integer(2, "GetInputType")?;
            let kind = match usize::try_from(port) {
                Ok(port) => call.cx.host(move |ctx| ctx.machine.port_type(port))?,
                Err(_) => 0,
            };
            Ok(Some(num(kind)))
        },
        None,
    );
    definition.function(
        "SetInputType",
        |call| {
            require(call, "SetInputType")?;
            let first = call.integer(2, "SetInputType")?;
            let second = call.integer(3, "SetInputType")?;
            call.cx
                .host_write(move |ctx| ctx.machine.set_port_types(first, second))?;
            with_settings(call.cx, |settings| settings.set_port_types(first, second))?;
            Ok(None)
        },
        None,
    );
    style_parameter_functions(definition);
    style_parameter_list_member(definition);
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
            call.cx.data(|data| data.env.settings.request_save())?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "Load",
        |call| {
            require(call, "Load")?;

            // Re-reads the file and applies only the user preferences it holds (REQ-PERS-015).
            call.cx.data(|data| data.env.settings.request_load())?;
            Ok(None)
        },
        None,
    );
}

/// The three style-parameter methods of AMD-040 REQ-LUA-114. The identifiers they take are
/// facts of interoperability — a 2.0 script passes them as strings — and each stands for a
/// setting of the catalogue, so a write from a script and a write from the options window reach
/// the same value.
fn style_parameter_functions(definition: &mut TypeDef) {
    definition.function(
        "GetVisualStyleParameter",
        |call| {
            require(call, "GetVisualStyleParameter")?;
            let style = call.integer(2, "GetVisualStyleParameter")?;
            let id = call.string(3, "GetVisualStyleParameter")?;
            let value = call
                .cx
                .data(|data| data.env.settings.visual_style_parameter(style, &id))?;
            Ok(Some(num(value)))
        },
        None,
    );
    definition.function(
        "SetVisualStyleParameter",
        |call| {
            require(call, "SetVisualStyleParameter")?;
            let style = call.integer(2, "SetVisualStyleParameter")?;
            let id = call.string(3, "SetVisualStyleParameter")?;
            let wanted = call.number(4, "SetVisualStyleParameter")?;
            let mut written = false;
            with_settings(call.cx, |settings| {
                written = settings.set_visual_style_parameter(style, &id, wanted);
            })?;
            Ok(Some(boolean(written)))
        },
        None,
    );
    definition.function(
        "ResetVisualStyleParameters",
        |call| {
            require(call, "ResetVisualStyleParameters")?;
            let style = call.integer(2, "ResetVisualStyleParameters")?;
            with_settings(call.cx, |settings| {
                settings.reset_visual_style_parameters(style);
            })?;
            Ok(None)
        },
        None,
    );
}

/// `Setting.VisualStyleParameters` (AMD-059 REQ-LUA-116). Reading hands out a snapshot of the
/// write log; writing nil takes every parameter of every style back to its default and empties
/// it; writing a list a read handed out puts exactly its elements back; anything else raises,
/// which is the one write of the requirement that may — it is not a numeric value.
fn style_parameter_list_member(definition: &mut TypeDef) {
    definition.property(
        "VisualStyleParameters",
        |cx, _| {
            let log = cx.data(|data| {
                StyleParameterLog::new(data.env.settings.written_style_parameters())
            })?;
            cx.value(Payload::StyleParameterList(log))
        },
        |cx, _, value| {
            if matches!(value, Value::Nil) {
                return with_settings(cx, ScriptSettings::reset_all_visual_style_parameters);
            }

            if let Some(Payload::StyleParameterList(log)) = host_payload(value) {
                let entries: Vec<(u8, u8)> = log.iter().collect();
                return with_settings(cx, move |settings| {
                    settings.restore_visual_style_parameters(&entries);
                });
            }

            Err(cx.invalid("VisualStyleParameters"))
        },
        Some(HostType::StyleParameterList),
    );
}

fn log_of(payload: &Payload) -> StyleParameterLog {
    match payload {
        Payload::StyleParameterList(log) | Payload::StyleParameter(log, _) => *log,
        _ => StyleParameterLog::EMPTY,
    }
}

/// The list `Setting.VisualStyleParameters` reads, with the `Count` and the indexer of every
/// list of SPEC-05 §4.2 (indices 0 to `Count − 1`).
fn style_parameter_list() -> TypeDef {
    let mut definition = type_def(HostType::StyleParameterList, "Liste<ParamStyle>");
    definition.read_only(
        "Count",
        |_, this| Ok(num(i32::try_from(log_of(this).len()).unwrap_or(i32::MAX))),
        None,
    );
    definition.element = Some(HostType::StyleParameter);
    definition.indexer = Some(Arc::new(|cx, this, key| {
        let log = log_of(this);
        let index = cx.integer(key, "Liste<ParamStyle>")?;
        let index = cx.check_bounds(index, log.len(), "Liste<ParamStyle>")?;
        cx.value(Payload::StyleParameter(
            log,
            u8::try_from(index).unwrap_or(u8::MAX),
        ))
    }));
    // Two lists of the same elements are the same list: a read with nothing in between gives
    // the same value (REQ-LUA-043), the snapshot being the value itself.
    definition.equal = Some(Arc::new(|left, right| log_of(left) == log_of(right)));
    definition
}

/// The entry of a parameter in the list: the style read back by REQ-SCN-078, the identifier and
/// the normalised value (AMD-059 REQ-LUA-116), all three read only.
fn style_parameter() -> TypeDef {
    fn entry(payload: &Payload) -> Option<(i32, &'static str, f64)> {
        let (log, index) = match payload {
            Payload::StyleParameter(log, index) => (*log, usize::from(*index)),
            _ => return None,
        };
        let (position, rung) = log.get(index)?;
        let (style, id, key) = styles::parameter_of_position(usize::from(position))?;
        Some((style, id, styles::value_of_rung(key, usize::from(rung))))
    }

    let mut definition = type_def(HostType::StyleParameter, "ParamStyle");
    definition.read_only(
        "Style",
        |_, this| {
            Ok(num(
                entry(this).map_or(styles::CLASSIC_STYLE, |(style, _, _)| style)
            ))
        },
        None,
    );
    definition.read_only(
        "ParameterId",
        |cx, this| cx.text(entry(this).map_or("", |(_, id, _)| id)),
        None,
    );
    definition.read_only(
        "Value",
        |_, this| Ok(num(entry(this).map_or(0.0, |(_, _, value)| value))),
        None,
    );
    definition.equal = Some(Arc::new(|left, right| match (left, right) {
        (Payload::StyleParameter(a, i), Payload::StyleParameter(b, j)) => a == b && i == j,
        _ => false,
    }));
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
    [layer_config(), layer(), clipping()]
}

/// `Setting.Layer`, the global `Layer` (§4.2, type `ConfigCalques`).
fn layer_config() -> TypeDef {
    // Setting.Layer, the global Layer (§4.2, type ConfigCalques).
    let mut config = type_def(HostType::LayerConfig, "ConfigCalques");
    config.property(
        "DefLayer",
        |cx, _| {
            Ok(num(read_settings(cx, |settings| {
                settings.int(SettingId::LayerDefaultLayer)
            })?))
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
        |cx, _| {
            Ok(boolean(cx.data(|data| {
                data.env.settings.flag(SettingId::LayerAuto)
            })?))
        },
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
    config
}

/// One layer (§4.2, type `Calque`).
fn layer() -> TypeDef {
    // One layer (§4.2, type Calque).
    let mut layer = type_def(HostType::Layer, "Calque");
    layer.property(
        "Offset",
        |cx, this| {
            Ok(num(cx.data(|data| {
                data.env.settings.layer_offset(layer_index(this))
            })?))
        },
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
            Ok(num(read_settings(cx, |settings| {
                settings.layer_aligned(layer_index(this))
            })?))
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
        |cx, this| {
            Ok(num(cx.data(|data| {
                data.env.settings.layer_coef(layer_index(this))
            })?))
        },
        None,
    );
    layer.property(
        "CastShadow",
        |cx, this| {
            Ok(boolean(cx.data(|data| {
                data.env.settings.layer_cast_shadow(layer_index(this))
            })?))
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
            Ok(boolean(cx.data(|data| {
                data.env.settings.layer_recv_shadow(layer_index(this))
            })?))
        },
        |cx, this, value| {
            let wanted = assigned_bool(value);
            let index = layer_index(this);
            with_settings(cx, |settings| settings.set_layer_recv_shadow(index, wanted))
        },
        None,
    );
    layer
}

/// `Setting.Clipping`, the global `Clipping` (§4.2, type `Découpe`).
fn clipping() -> TypeDef {
    // Setting.Clipping, the global Clipping (§4.2, type Découpe). Index 0 is the left margin,
    // 1 the bottom, 2 the right, 3 the top (DEC-R2-T1).
    let mut clipping = type_def(HostType::Clipping, "Decoupe");
    for (name, index) in [("x", 0), ("y", 1), ("z", 2), ("w", 3)] {
        clipping.property(
            name,
            move |cx, _| Ok(num(cx.data(|data| data.env.settings.clipping(index))?)),
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
            let margins = cx.data(|data| data.env.settings.clipping_vector())?;
            cx.color(LuaColor::new(margins.x, margins.y, margins.z, margins.w))
        },
        |cx, _, value| {
            let Some(color) = try_color(value) else {
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
        Ok(num(cx.data(|data| data.env.settings.clipping(index))?))
    }));
    clipping.index_setter = Some(Arc::new(|cx, _, key, value| {
        let index = cx.integer(key, "Clipping")?;
        let index = cx.check_bounds(index, CLIPPING_COUNT, "Clipping")?;
        let wanted = cx.real(value, "Clipping")?;
        with_settings(cx, |settings| settings.set_clipping(index, wanted))
    }));
    clipping
}
