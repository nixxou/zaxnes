//! The globals `Frame`, `FrameManager`, `Palette`, `Pattern` and `Script` of SPEC-05 §4.2.

use mlua::Value;
use zax_engine3d::engine::ConversionState;
use zax_engine3d::frame::FrameInstanceId;
use zax_engine3d::instance::InstanceRef;
use zax_engine3d::palette::FramePalette;
use zax_engine3d::pattern::EnginePattern;
use zax_engine3d::vectors::{Rect, Vec3};
use zax_settings::catalogue::SettingId;

use super::{boolean, count, num, type_def};
use crate::binding::cx::{try_rect, Call, Cx};
use crate::binding::payload::{
    HostType, InstanceListKind, InstanceTarget, PatternTarget, Payload, ShapeTarget,
};
use crate::binding::table::{Reply, TypeDef};
use crate::values::{number_format, LuaColor};

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.extend(frame());
    types.push(palette());
    types.push(pattern_table());
    types.extend(script());
}

/// Frame's members are methods: calling one with `.` leaves the first argument in the
/// receiver's place and must raise (REQ-LUA-081).
fn require(call: &Call, kind: HostType, member: &str) -> mlua::Result<()> {
    if call.host(1) == Some(Payload::Table(kind)) {
        Ok(())
    } else {
        Err(call.invalid(member))
    }
}

/// Which of the three shape-injecting calls is being made.
#[derive(Clone, Copy)]
enum Inject {
    Add,
    Create,
    Insert,
}

impl Inject {
    const fn member(self) -> &'static str {
        match self {
            Self::Add => "AddNewShape",
            Self::Create => "CreateNewShape",
            Self::Insert => "InsertShape",
        }
    }
}

/// The pattern instance an `Instance` value designates, for the calls that copy it into the
/// frame.
fn reference_of(cx: &Cx, target: &InstanceTarget) -> mlua::Result<Option<InstanceRef>> {
    match target {
        InstanceTarget::Pattern(reference) => Ok(Some(*reference)),
        InstanceTarget::Frame(..) => Ok(cx
            .with_instance(target, |view| {
                view.frame().map(|(instance, _)| instance.reference())
            })?
            .flatten()),
    }
}

/// `Frame:AddNewShape`, `CreateNewShape` and `InsertShape` (REQ-LUA-086): the first argument
/// decides the variant — an instance gives back the instance of the same index of the new
/// shape, a pattern gives back the shape.
fn inject(call: &Call, kind: Inject) -> mlua::Result<Reply> {
    let member = kind.member();
    require(call, HostType::FrameApi, member)?;
    let position = call.vec3(3, member)?;
    let palette = call.integer(4, member)?;
    let cx = call.cx;

    match call.host(2) {
        Some(Payload::Instance(target)) => {
            let Some(reference) = reference_of(cx, &target)? else {
                return Err(call.invalid(member));
            };

            let created = cx.state(|state| match kind {
                Inject::Add => state.add_new_shape_from_instance(reference, position, palette),
                Inject::Create => {
                    state.create_new_shape_from_instance(reference, position, palette)
                }
                // InsertShape is CreateNewShape with the pattern's bottom-left corner removed
                // (REQ-E3D-232).
                Inject::Insert => {
                    let corner = state
                        .find_pattern(reference)
                        .and_then(|id| state.patterns().get(id))
                        .map(EnginePattern::bottom_left);
                    let shifted = corner.map_or(position, |corner| {
                        Vec3::new(position.x - corner.x, position.y - corner.y, position.z)
                    });
                    state.create_new_shape_from_instance(reference, shifted, palette)
                }
            })?;

            let id = created.map_err(|error| cx.raise(&error.to_string()))?;
            let key = cx.frame_key(false)?;
            Ok(Some(cx.object(Payload::Instance(
                InstanceTarget::Frame(key, id),
            ))?))
        }
        Some(Payload::Pattern(PatternTarget::Registry(id))) => {
            let created = cx.state(|state| match kind {
                Inject::Add => state.add_new_shape(id, position, palette),
                Inject::Create => state.create_new_shape(id, position, palette),
                Inject::Insert => state.insert_shape(id, position, palette),
            })?;

            match created {
                Some(shape) => {
                    let key = cx.frame_key(false)?;
                    Ok(Some(
                        cx.object(Payload::Shape(ShapeTarget { key, id: shape }))?,
                    ))
                }
                None => Ok(Some(Value::Nil)),
            }
        }
        // Passing nil where an object is needed raises an ordinary Lua error (REQ-LUA-047).
        _ => Err(call.invalid(member)),
    }
}

fn frame() -> [TypeDef; 2] {
    let mut definition = type_def(HostType::FrameApi, "FrameAPI");

    frame_queries(&mut definition);
    frame_injection(&mut definition);

    frame_pinned(&mut definition);

    // FrameManager is the class table of the frame manager; FrameManager.Ins is Frame.
    let mut manager = type_def(HostType::FrameManagerApi, "FrameManagerClasse");
    manager.read_only(
        "Ins",
        |cx, _| cx.object(Payload::Table(HostType::FrameApi)),
        Some(HostType::FrameApi),
    );
    [definition, manager]
}

/// The queries over the frame being converted (REQ-LUA-084, REQ-LUA-085).
fn frame_queries(definition: &mut TypeDef) {
    definition.read_only(
        "shapeCount",
        |cx, _| Ok(count(cx.state(|state| state.scriptable_count())?)),
        None,
    );
    definition.read_only(
        "frameCounter",
        |cx, _| Ok(num(cx.data(|data| data.frame_counter)?)),
        None,
    );
    #[allow(clippy::cast_precision_loss)] // The C# divides an int by a float.
    definition.read_only(
        "time",
        |cx, _| Ok(num(cx.data(|data| data.frame_counter)? as f32 / 60.0)),
        None,
    );

    // The shared list of REQ-LUA-048: emptied and refilled at every call, the same object
    // every time.
    definition.function(
        "GetShapesWithTag",
        |call| {
            require(call, HostType::FrameApi, "GetShapesWithTag")?;
            let tag = call.string(2, "GetShapesWithTag")?;
            let key = call.cx.frame_key(false)?;
            let mut found: Vec<FrameInstanceId> = Vec::new();
            call.cx
                .state(|state| state.collect_shapes_with_tag(&tag, &mut found))?;
            call.cx.data(|data| {
                data.instance_query.clear();
                data.instance_query
                    .extend(found.into_iter().map(|id| InstanceTarget::Frame(key, id)));
            })?;
            Ok(Some(call.cx.value(Payload::InstanceList(
                InstanceListKind::ShapesWithTag,
            ))?))
        },
        Some(HostType::InstanceList),
    );
    definition.function(
        "GetShapeWithTag",
        |call| {
            require(call, HostType::FrameApi, "GetShapeWithTag")?;
            let tag = call.string(2, "GetShapeWithTag")?;
            let found = call.cx.state(|state| state.get_shape_with_tag(&tag))?;
            Ok(Some(frame_instance(call.cx, found)?))
        },
        Some(HostType::Instance),
    );
    definition.function(
        "GetShape",
        |call| {
            require(call, HostType::FrameApi, "GetShape")?;
            let index = call.integer(2, "GetShape")?;
            let total = call.cx.state(|state| state.scriptable_count())?;
            let position = call.cx.check_bounds(index, total, "GetShape")?;
            let found = call.cx.state(|state| state.get_shape(position))?;
            Ok(Some(frame_instance(call.cx, found)?))
        },
        Some(HostType::Instance),
    );
    // Without a tag the third argument is the empty string, which matches no instance, so the
    // two-argument call always gives nil (REQ-LUA-084 point 5).
    definition.function(
        "GetShapeWithPos",
        |call| {
            require(call, HostType::FrameApi, "GetShapeWithPos")?;
            let x = call.real(2, "GetShapeWithPos")?;
            let y = call.real(3, "GetShapeWithPos")?;
            let tag = call.optional_string(4, "GetShapeWithPos", "")?;
            let found = call
                .cx
                .state(|state| state.get_shape_with_pos(x, y, &tag))?;
            Ok(Some(frame_instance(call.cx, found)?))
        },
        Some(HostType::Instance),
    );
}

/// The injection of shapes into the frame (REQ-LUA-086).
fn frame_injection(definition: &mut TypeDef) {
    // The result type depends on the first argument — an instance gives an instance, a
    // pattern gives a shape (REQ-LUA-086) — so no single result type is declared and an
    // access chain through one of them stops at the static check.
    definition.function("AddNewShape", |call| inject(call, Inject::Add), None);
    definition.function("CreateNewShape", |call| inject(call, Inject::Create), None);
    definition.function("InsertShape", |call| inject(call, Inject::Insert), None);
}

/// The pinned frame (REQ-E3D-233).
fn frame_pinned(definition: &mut TypeDef) {
    definition.function(
        "PinNewFrame",
        |call| {
            require(call, HostType::FrameApi, "PinNewFrame")?;
            call.cx.state(ConversionState::pin_new_frame)?;
            call.cx
                .data(|data| data.pin_epoch = data.pin_epoch.wrapping_add(1))?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "ReleasePinnedFrame",
        |call| {
            require(call, HostType::FrameApi, "ReleasePinnedFrame")?;
            call.cx.state(ConversionState::release_pinned_frame)?;
            call.cx
                .data(|data| data.pin_epoch = data.pin_epoch.wrapping_add(1))?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "AddRectFromPinnedFrame",
        |call| {
            require(call, HostType::FrameApi, "AddRectFromPinnedFrame")?;
            let x = call.real(2, "AddRectFromPinnedFrame")?;
            let y = call.real(3, "AddRectFromPinnedFrame")?;
            let width = call.real(4, "AddRectFromPinnedFrame")?;
            let height = call.real(5, "AddRectFromPinnedFrame")?;
            call.cx
                .state(|state| state.add_rect_from_pinned_frame(x, y, width, height))?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "RestoreFromPinnedFrame",
        |call| {
            require(call, HostType::FrameApi, "RestoreFromPinnedFrame")?;
            if call.is_none_or_nil(2) {
                call.cx.state(|state| {
                    state.restore_from_pinned_frame();
                })?;
            } else {
                let rectangle = call.rect(2, "RestoreFromPinnedFrame")?;
                call.cx
                    .state(|state| state.restore_from_pinned_frame_rect(rectangle))?;
            }

            Ok(None)
        },
        None,
    );
    // Either a rectangle or a position and a size (§4.2).
    definition.function(
        "GetShapesFromPinnedFrame",
        |call| {
            require(call, HostType::FrameApi, "GetShapesFromPinnedFrame")?;
            let rectangle = if let Some(given) = try_rect(call.value(2)) {
                given
            } else if let (Some(origin), Some(size)) = (
                call.cx.try_vec2(call.value(2))?,
                call.cx.try_vec2(call.value(3))?,
            ) {
                Rect::new(origin.x, origin.y, size.x, size.y)
            } else {
                return Err(call.invalid("GetShapesFromPinnedFrame"));
            };

            let key = call.cx.frame_key(true)?;
            let mut found = Vec::new();
            call.cx
                .state(|state| state.collect_shapes_from_pinned_frame(rectangle, &mut found))?;
            call.cx.data(|data| {
                data.pinned_query.clear();
                data.pinned_query
                    .extend(found.into_iter().map(|id| ShapeTarget { key, id }));
            })?;
            Ok(Some(call.cx.value(Payload::ShapeList)?))
        },
        Some(HostType::ShapeList),
    );
}

/// The `Instance` value of a frame instance of the frame being converted, nil when there is
/// none.
fn frame_instance(cx: &Cx, found: Option<FrameInstanceId>) -> mlua::Result<Value> {
    match found {
        Some(id) => {
            let key = cx.frame_key(false)?;
            cx.object(Payload::Instance(InstanceTarget::Frame(key, id)))
        }
        None => Ok(Value::Nil),
    }
}

/// The palette entry at argument 1, which has to be one of the 64 (REQ-LUA-087).
fn entry(call: &Call, member: &str) -> mlua::Result<usize> {
    let entry = call.integer(1, member)?;
    match usize::try_from(entry) {
        Ok(position) if position < FramePalette::ENTRY_COUNT => Ok(position),
        _ => Err(call.cx.raise(&format!(
            "palette entry {entry} is out of range for {member}"
        ))),
    }
}

fn read_entry(call: &Call, member: &str) -> mlua::Result<Reply> {
    let entry = entry(call, member)?;
    let current = call.optional_number(2, member, 0.0)? != 0.0;
    Ok(Some(num(call
        .cx
        .state(|state| state.frame_color_index(entry, current))?)))
}

fn palette() -> TypeDef {
    let mut definition = type_def(HostType::PaletteApi, "PaletteAPI");

    // Table functions, called with '.', so the entry is argument 1 (REQ-LUA-081).
    definition.function(
        "FrameColorIndex",
        |call| read_entry(call, "FrameColorIndex"),
        None,
    );
    definition.function(
        "GetColorIndex",
        |call| read_entry(call, "GetColorIndex"),
        None,
    );
    definition.function(
        "SetColorIndex",
        |call| {
            let entry = entry(call, "SetColorIndex")?;
            let value = call.integer(2, "SetColorIndex")?;
            let current = call.optional_number(3, "SetColorIndex", 0.0)? != 0.0;
            call.cx
                .state(|state| state.set_color_index(entry, value & 0x3F, current))?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "FrameColor",
        |call| {
            let entry = entry(call, "FrameColor")?;
            let colour = call.cx.state(|state| state.frame_color(entry))?;
            Ok(Some(call.cx.color(LuaColor::from_bytes(
                colour.r, colour.g, colour.b,
            ))?))
        },
        Some(HostType::Color),
    );
    definition.read_only(
        "bgColor",
        |cx, _| {
            let skybox = cx.data(|data| data.env.settings.flag(SettingId::SceneSkyboxEnable))?;
            let colour = cx.state(|state| state.background_color(skybox))?;
            cx.color(LuaColor::from_bytes(colour.r, colour.g, colour.b))
        },
        Some(HostType::Color),
    );
    definition
}

fn pattern_table() -> TypeDef {
    let mut definition = type_def(HostType::PatternApi, "PatternAPI");

    definition.function(
        "GetPatternWithTag",
        |call| {
            let tag = call.string(1, "GetPatternWithTag")?;
            let found = call.cx.state(|state| state.get_pattern_with_tag(&tag))?;
            Ok(Some(match found {
                Some(reference) => call
                    .cx
                    .object(Payload::Instance(InstanceTarget::Pattern(reference)))?,
                None => Value::Nil,
            }))
        },
        Some(HostType::Instance),
    );
    definition.function(
        "GetPatternsWithTag",
        |call| {
            let tag = call.string(1, "GetPatternsWithTag")?;
            let mut found = Vec::new();
            call.cx
                .state(|state| state.get_patterns_with_tag(&tag, &mut found))?;
            call.cx.data(|data| {
                data.pattern_query.clear();
                data.pattern_query
                    .extend(found.into_iter().map(InstanceTarget::Pattern));
            })?;
            Ok(Some(call.cx.value(Payload::InstanceList(
                InstanceListKind::PatternsWithTag,
            ))?))
        },
        Some(HostType::InstanceList),
    );
    definition
}

fn script() -> [TypeDef; 2] {
    let mut definition = type_def(HostType::ScriptApi, "ScriptAPI");
    definition.read_only(
        "Count",
        |cx, _| Ok(count(cx.data(|data| data.scripts.len())?)),
        None,
    );
    definition.function(
        "Get",
        |call| {
            require(call, HostType::ScriptApi, "Get")?;
            let name = call.string(2, "Get")?;
            let position = call
                .cx
                .data(|data| data.scripts.iter().position(|script| script.name() == name))?;
            Ok(Some(script_value(call.cx, position)?))
        },
        Some(HostType::ScriptObj),
    );
    // GetScript takes a name or a 0-based position in script order (§4.2).
    definition.function(
        "GetScript",
        |call| {
            require(call, HostType::ScriptApi, "GetScript")?;
            if call.is_number(2) {
                let index = call.integer(2, "GetScript")?;
                let total = call.cx.data(|data| data.scripts.len())?;
                let position = usize::try_from(index).ok().filter(|&index| index < total);
                return Ok(Some(script_value(call.cx, position)?));
            }

            let name = call.string(2, "GetScript")?;
            let position = call
                .cx
                .data(|data| data.scripts.iter().position(|script| script.name() == name))?;
            Ok(Some(script_value(call.cx, position)?))
        },
        Some(HostType::ScriptObj),
    );
    definition.function(
        "ReadMem",
        |call| {
            require(call, HostType::ScriptApi, "ReadMem")?;
            let address = number_format::to_address(call.number(2, "ReadMem")?);
            Ok(Some(num(call
                .cx
                .host(move |ctx| ctx.machine.read(address))?)))
        },
        None,
    );
    definition.function(
        "WriteMem",
        |call| {
            require(call, HostType::ScriptApi, "WriteMem")?;
            let address = number_format::to_address(call.number(2, "WriteMem")?);
            let value = number_format::to_byte(call.number(3, "WriteMem")?);
            call.cx
                .host_write(move |ctx| ctx.machine.write(address, value))?;
            Ok(None)
        },
        None,
    );

    let object = script_object();
    [definition, object]
}

/// The `ScriptObj` type: one script of the profile.
fn script_object() -> TypeDef {
    let mut object = type_def(HostType::ScriptObj, "ScriptObj");
    object.property(
        "Enable",
        |cx, this| {
            Ok(boolean(read_script(cx, this, false, |script| {
                script.enabled()
            })?))
        },
        |cx, this, value| {
            if let Payload::Script(position) = this {
                let wanted = super::assigned_bool(value);
                cx.data(|data| {
                    if let Some(script) = data.scripts.get_mut(*position) {
                        script.enabled = wanted;
                    }
                })?;
            }

            Ok(())
        },
        None,
    );
    object.alias("enable", "Enable");
    object.read_only(
        "Name",
        |cx, this| {
            let name = read_script(cx, this, String::new(), |script| script.name().to_owned())?;
            cx.text(&name)
        },
        None,
    );
    object.alias("name", "Name");
    object.read_only(
        "Error",
        |cx, this| {
            Ok(boolean(read_script(cx, this, false, |script| {
                script.failed()
            })?))
        },
        None,
    );
    object.read_only(
        "TagStr",
        |cx, this| {
            let tags = read_script(cx, this, String::new(), |script| {
                script.tag_string().to_owned()
            })?;
            cx.text(&tags)
        },
        None,
    );
    object
}

fn script_value(cx: &Cx, position: Option<usize>) -> mlua::Result<Value> {
    position.map_or(Ok(Value::Nil), |position| {
        cx.object(Payload::Script(position))
    })
}

fn read_script<R>(
    cx: &Cx,
    this: &Payload,
    fallback: R,
    operation: impl FnOnce(&crate::script::LoadedScript) -> R,
) -> mlua::Result<R> {
    let Payload::Script(position) = this else {
        return Ok(fallback);
    };

    cx.data(|data| data.scripts.get(*position).map_or(fallback, operation))
}
