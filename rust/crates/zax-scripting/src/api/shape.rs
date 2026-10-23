//! The types `Shape`, `Pattern` and `Liste<T>` of SPEC-05 §4.2 and §4.3.

use std::sync::Arc;

use mlua::Value;
use zax_engine3d::frame::{FrameInstanceId, FrameShape};
use zax_engine3d::instance::InstanceRef;
use zax_engine3d::vectors::{Cell, Vec2, Vec3};

use super::{assigned_bool, boolean, count, num, type_def};
use crate::binding::cx::Cx;
use crate::binding::payload::{
    HostType, InstanceListKind, InstanceTarget, PatternTarget, Payload, ShapeTarget,
};
use crate::binding::table::{Reply, TypeDef};

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.push(shape());
    types.push(pattern());
    types.extend(lists());
}

fn reply(value: Value) -> mlua::Result<Reply> {
    Ok(Some(value))
}

const fn shape_target(payload: &Payload) -> Option<ShapeTarget> {
    match payload {
        Payload::Shape(target) => Some(*target),
        _ => None,
    }
}

/// Reads from the shape, `fallback` when it is gone.
fn read<R>(
    cx: &Cx,
    this: &Payload,
    fallback: R,
    operation: impl FnOnce(&FrameShape) -> R,
) -> mlua::Result<R> {
    let Some(target) = shape_target(this) else {
        return Ok(fallback);
    };

    Ok(cx
        .with_shape(&target, |shape, _, _| operation(shape))?
        .unwrap_or(fallback))
}

fn tiles(definition: &mut TypeDef, name: &'static str, get: fn(&FrameShape) -> Cell) {
    definition.read_only(
        name,
        move |cx, this| cx.int_vector2(read(cx, this, Cell::default(), get)?),
        Some(HostType::IntVector2),
    );
}

fn instance_of(target: &ShapeTarget, index: usize) -> Payload {
    Payload::Instance(InstanceTarget::Frame(
        target.key,
        FrameInstanceId {
            shape: target.id,
            index,
        },
    ))
}

/// The first instance of the shape as an `Instance` value, nil when there is none.
fn first_instance(cx: &Cx, this: &Payload) -> mlua::Result<Value> {
    let Some(target) = shape_target(this) else {
        return Ok(Value::Nil);
    };

    let has_instances = read(cx, this, false, |shape| !shape.instances().is_empty())?;
    if has_instances {
        cx.object(instance_of(&target, 0))
    } else {
        Ok(Value::Nil)
    }
}

/// The `index`-th instance of the shape, with the bounds check of REQ-LUA-045.
fn nth_instance(
    cx: &Cx,
    target: Option<ShapeTarget>,
    index: i32,
    member: &str,
) -> mlua::Result<Value> {
    let total = match &target {
        Some(shape) => cx
            .with_shape(shape, |data, _, _| data.instances().len())?
            .unwrap_or(0),
        None => 0,
    };
    let position = cx.check_bounds(index, total, member)?;
    match target {
        Some(shape) => cx.object(instance_of(&shape, position)),
        None => Ok(Value::Nil),
    }
}

/// `layer`, `geo` and `sizeZ` are those of instance 0 of the frame pattern (§4.2).
fn first_instance_int(
    definition: &mut TypeDef,
    name: &'static str,
    get: fn(&zax_engine3d::frame::FrameInstance) -> i32,
) {
    definition.read_only(
        name,
        move |cx, this| {
            Ok(num(read(cx, this, 0, |shape| {
                shape.instances().first().map_or(0, get)
            })?))
        },
        None,
    );
}

fn shape() -> TypeDef {
    let mut definition = type_def(HostType::Shape, "Shape");

    tiles(&mut definition, "tStart", FrameShape::tile_start);
    tiles(&mut definition, "tEnd", FrameShape::tile_end);
    tiles(&mut definition, "tSize", FrameShape::tile_size);
    definition.alias("TStart", "tStart");
    definition.alias("TEnd", "tEnd");
    definition.alias("TSize", "tSize");

    tiles(&mut definition, "Start", FrameShape::pixel_start);
    tiles(&mut definition, "End", FrameShape::pixel_end);
    definition.alias("pStart", "Start");
    definition.alias("pEnd", "End");
    tiles(&mut definition, "Size", FrameShape::pixel_size);

    definition.read_only(
        "BottomLeft",
        |cx, this| cx.vector2(read(cx, this, Vec2::ZERO, FrameShape::bottom_left)?),
        Some(HostType::Vector2),
    );
    definition.read_only(
        "TopRight",
        |cx, this| cx.vector2(read(cx, this, Vec2::ZERO, FrameShape::top_right)?),
        Some(HostType::Vector2),
    );
    definition.alias("BL", "BottomLeft");
    definition.alias("TR", "TopRight");

    definition.read_only(
        "Bg",
        |cx, this| Ok(boolean(read(cx, this, false, FrameShape::background)?)),
        None,
    );
    definition.alias("bg", "Bg");
    definition.read_only(
        "Hidden",
        |cx, this| Ok(boolean(read(cx, this, false, FrameShape::hidden)?)),
        None,
    );
    definition.alias("hidden", "Hidden");
    definition.read_only(
        "Palette",
        |cx, this| Ok(num(read(cx, this, 0, FrameShape::palette)?)),
        None,
    );
    definition.alias("palIndex", "Palette");
    definition.read_only(
        "TileCount",
        |cx, this| Ok(count(read(cx, this, 0, FrameShape::tile_count)?)),
        None,
    );
    definition.read_only(
        "Inside",
        |cx, this| Ok(boolean(read(cx, this, false, FrameShape::inside)?)),
        None,
    );
    definition.read_only(
        "AtBorder",
        |cx, this| Ok(boolean(read(cx, this, true, FrameShape::at_border)?)),
        None,
    );
    definition.read_only(
        "Completed",
        |cx, this| Ok(boolean(read(cx, this, false, FrameShape::completed)?)),
        None,
    );
    definition.read_only("Age", |_, _| Ok(num(FrameShape::AGE)), None);
    definition.read_only(
        "InsCount",
        |cx, this| Ok(count(read(cx, this, 0, |shape| shape.instances().len())?)),
        None,
    );
    definition.alias("Count", "InsCount");

    definition.read_only(
        "shape3D",
        |cx, this| first_instance(cx, this),
        Some(HostType::Instance),
    );
    definition.read_only(
        "refPattern",
        |cx, this| {
            let id = read(cx, this, None, FrameShape::reference_pattern)?;
            id.map_or(Ok(Value::Nil), |id| {
                cx.object(Payload::Pattern(PatternTarget::Registry(id)))
            })
        },
        Some(HostType::Pattern),
    );
    // A shape's own pattern is the frame pattern of §4.2: the same object seen as a Pattern.
    definition.read_only(
        "pattern",
        |cx, this| match shape_target(this) {
            Some(target) => cx.object(Payload::Pattern(PatternTarget::Frame(target))),
            None => Ok(Value::Nil),
        },
        Some(HostType::Pattern),
    );

    first_instance_int(&mut definition, "layer", |instance| instance.layer as i32);
    first_instance_int(&mut definition, "geo", |instance| instance.geometry as i32);
    first_instance_int(&mut definition, "sizeZ", |instance| instance.depth);

    definition.read_only(
        "center",
        |cx, this| cx.vector3(read(cx, this, Vec3::ZERO, FrameShape::center)?),
        Some(HostType::Vector3),
    );

    definition.element = Some(HostType::Instance);
    definition.indexer = Some(Arc::new(|cx, this, key| {
        let index = cx.integer(key, "Shape")?;
        nth_instance(cx, shape_target(this), index, "Shape")
    }));

    definition.function(
        "Shape3D",
        |call| {
            let index = if call.is_none_or_nil(2) {
                0
            } else {
                call.integer(2, "Shape3D")?
            };
            let target = call.host(1).as_ref().and_then(shape_target);
            reply(nth_instance(call.cx, target, index, "Shape3D")?)
        },
        Some(HostType::Instance),
    );

    definition.function(
        "CalculatePos",
        |call| {
            if let Some(target) = call.host(1).as_ref().and_then(shape_target) {
                call.cx
                    .with_shape_settings_mut(&target, |shape, settings| {
                        for instance in shape.instances_mut() {
                            instance.calculate_pos(settings);
                        }
                    })?;
            }

            Ok(None)
        },
        None,
    );

    // Distance(s): the squared distance between the Pos of instance 0 of the two shapes.
    definition.function(
        "Distance",
        |call| {
            let Some(Payload::Shape(other)) = call.host(2) else {
                return Err(call.invalid("Distance"));
            };

            let Some(mine) = call.host(1).as_ref().and_then(shape_target) else {
                return reply(num(0));
            };

            let distance = call
                .cx
                .with_shape_pair(&mine, &other, |mine, other| {
                    match (mine.instances().first(), other.instances().first()) {
                        (Some(mine), Some(other)) => (mine.pos - other.pos).squared_length(),
                        _ => 0.0,
                    }
                })?
                .unwrap_or(0.0);
            reply(num(distance))
        },
        None,
    );

    definition
}

/// The pattern a `Pattern` value stands for.
const fn pattern_target(payload: &Payload) -> Option<PatternTarget> {
    match payload {
        Payload::Pattern(target) => Some(*target),
        _ => None,
    }
}

/// How many instances the pattern holds.
fn instance_total(cx: &Cx, target: &PatternTarget) -> mlua::Result<usize> {
    Ok(match target {
        PatternTarget::Registry(id) => cx
            .with_pattern(*id, |pattern, _| pattern.instances().len())?
            .unwrap_or(0),
        PatternTarget::Frame(shape) => cx
            .with_shape(shape, |data, _, _| data.instances().len())?
            .unwrap_or(0),
    })
}

/// The `index`-th instance of the pattern.
const fn pattern_instance(target: &PatternTarget, index: usize) -> InstanceTarget {
    match target {
        PatternTarget::Registry(id) => InstanceTarget::Pattern(InstanceRef {
            pattern: *id,
            index,
        }),
        PatternTarget::Frame(shape) => InstanceTarget::Frame(
            shape.key,
            FrameInstanceId {
                shape: shape.id,
                index,
            },
        ),
    }
}

/// Reads from the pattern a `Pattern` value stands for: the registry pattern, or the shape of a
/// frame pattern.
fn read_pattern<R>(
    cx: &Cx,
    this: &Payload,
    fallback: R,
    registry: impl FnOnce(&zax_engine3d::pattern::EnginePattern) -> R,
    frame: impl FnOnce(&FrameShape) -> R,
) -> mlua::Result<R> {
    let Some(target) = pattern_target(this) else {
        return Ok(fallback);
    };

    let found = match target {
        PatternTarget::Registry(id) => cx.with_pattern(id, |pattern, _| registry(pattern))?,
        PatternTarget::Frame(shape) => cx.with_shape(&shape, |data, _, _| frame(data))?,
    };
    Ok(found.unwrap_or(fallback))
}

fn pattern() -> TypeDef {
    let mut definition = type_def(HostType::Pattern, "Pattern");

    definition.property(
        "Disable",
        |cx, this| {
            Ok(boolean(read_pattern(
                cx,
                this,
                false,
                |p| p.disabled,
                |_| false,
            )?))
        },
        |cx, this, value| {
            // A frame pattern lives for one frame and is never a candidate, so the flag only
            // means something on a registry pattern (REQ-E3D-093).
            let wanted = assigned_bool(value);
            if let Some(PatternTarget::Registry(id)) = pattern_target(this) {
                cx.with_pattern_mut(id, |pattern| pattern.disabled = wanted)?;
            }

            Ok(())
        },
        None,
    );

    definition.read_only(
        "tSize",
        |cx, this| {
            let size = read_pattern(
                cx,
                this,
                Cell::default(),
                |p| Cell::new(p.width(), p.height()),
                FrameShape::tile_size,
            )?;
            cx.int_vector2(size)
        },
        Some(HostType::IntVector2),
    );

    #[allow(clippy::cast_precision_loss)] // Pixel sizes of a pattern are small.
    definition.read_only(
        "Size",
        |cx, this| {
            let size = read_pattern(
                cx,
                this,
                Vec2::ZERO,
                |p| {
                    Vec2::new(
                        (p.pixel_end().x - p.pixel_start().x + 1) as f32,
                        (p.pixel_end().y - p.pixel_start().y + 1) as f32,
                    )
                },
                |shape| {
                    let size = shape.pixel_size();
                    Vec2::new(size.x as f32, size.y as f32)
                },
            )?;
            cx.vector2(size)
        },
        Some(HostType::Vector2),
    );

    definition.read_only(
        "BottomLeft",
        |cx, this| {
            let corner = read_pattern(
                cx,
                this,
                Vec2::ZERO,
                zax_engine3d::pattern::EnginePattern::bottom_left,
                FrameShape::bottom_left,
            )?;
            cx.vector2(corner)
        },
        Some(HostType::Vector2),
    );
    definition.read_only(
        "TopRight",
        |cx, this| {
            let corner = read_pattern(
                cx,
                this,
                Vec2::ZERO,
                zax_engine3d::pattern::EnginePattern::top_right,
                FrameShape::top_right,
            )?;
            cx.vector2(corner)
        },
        Some(HostType::Vector2),
    );
    definition.alias("BL", "BottomLeft");
    definition.alias("TR", "TopRight");

    definition.read_only(
        "count",
        |cx, this| match pattern_target(this) {
            Some(target) => Ok(count(instance_total(cx, &target)?)),
            None => Ok(num(0)),
        },
        None,
    );

    definition.read_only(
        "pattern3D",
        |cx, this| {
            let Some(target) = pattern_target(this) else {
                return Ok(Value::Nil);
            };

            if instance_total(cx, &target)? == 0 {
                return Ok(Value::Nil);
            }

            cx.object(Payload::Instance(pattern_instance(&target, 0)))
        },
        Some(HostType::Instance),
    );
    definition.read_only(
        "pattern3DList",
        |cx, this| match pattern_target(this) {
            Some(target) => cx.value(Payload::InstanceList(InstanceListKind::PatternInstances(
                target,
            ))),
            None => Ok(Value::Nil),
        },
        Some(HostType::InstanceList),
    );

    definition.property(
        "constraintSize",
        |cx, this| {
            let size = read_pattern(
                cx,
                this,
                Cell::default(),
                |p| p.constraint_size,
                |shape| shape.constraint_size,
            )?;
            cx.int_vector2(size)
        },
        |cx, this, value| {
            let Some(size) = cx.try_int_vector2(value)? else {
                return Err(cx.invalid("constraintSize"));
            };

            match pattern_target(this) {
                Some(PatternTarget::Registry(id)) => {
                    cx.with_pattern_mut(id, |pattern| pattern.constraint_size = size)?;
                }
                Some(PatternTarget::Frame(shape)) => {
                    cx.with_shape_mut(&shape, |data| data.constraint_size = size)?;
                }
                None => {}
            }

            Ok(())
        },
        Some(HostType::IntVector2),
    );
    definition.alias("CSize", "constraintSize");

    definition.property(
        "SelfConnected",
        |cx, this| {
            Ok(boolean(read_pattern(
                cx,
                this,
                true,
                zax_engine3d::pattern::EnginePattern::self_connected,
                |shape| shape.self_connected,
            )?))
        },
        |cx, this, value| {
            let wanted = assigned_bool(value);
            match pattern_target(this) {
                Some(PatternTarget::Registry(id)) => {
                    cx.with_pattern_mut(id, |pattern| pattern.set_self_connected(wanted))?;
                }
                Some(PatternTarget::Frame(shape)) => {
                    cx.with_shape_mut(&shape, |data| data.self_connected = wanted)?;
                }
                None => {}
            }

            Ok(())
        },
        None,
    );
    definition.property(
        "SkipGeo",
        |cx, this| {
            Ok(boolean(read_pattern(
                cx,
                this,
                false,
                |p| p.skip_geometry,
                |shape| shape.skip_geometry,
            )?))
        },
        |cx, this, value| {
            let wanted = assigned_bool(value);
            match pattern_target(this) {
                Some(PatternTarget::Registry(id)) => {
                    cx.with_pattern_mut(id, |pattern| pattern.skip_geometry = wanted)?;
                }
                Some(PatternTarget::Frame(shape)) => {
                    cx.with_shape_mut(&shape, |data| data.skip_geometry = wanted)?;
                }
                None => {}
            }

            Ok(())
        },
        None,
    );

    definition.read_only(
        "permanent",
        |cx, this| {
            Ok(boolean(read_pattern(
                cx,
                this,
                false,
                zax_engine3d::pattern::EnginePattern::permanent,
                |_| false,
            )?))
        },
        None,
    );

    definition.element = Some(HostType::Instance);
    definition.indexer = Some(Arc::new(|cx, this, key| {
        let index = cx.integer(key, "Pattern")?;
        let total = match pattern_target(this) {
            Some(target) => instance_total(cx, &target)?,
            None => 0,
        };
        let position = cx.check_bounds(index, total, "Pattern")?;
        match pattern_target(this) {
            Some(target) => cx.object(Payload::Instance(pattern_instance(&target, position))),
            None => Ok(Value::Nil),
        }
    }));
    definition
}

// the shared lists of REQ-LUA-048

/// The instances a list holds, as targets, and how many.
fn list_len(cx: &Cx, this: &Payload) -> mlua::Result<usize> {
    match this {
        Payload::InstanceList(InstanceListKind::ShapesWithTag) => {
            Ok(cx.data(|data| data.instance_query.len())?)
        }
        Payload::InstanceList(InstanceListKind::PatternsWithTag) => {
            Ok(cx.data(|data| data.pattern_query.len())?)
        }
        Payload::InstanceList(InstanceListKind::PatternInstances(target)) => {
            instance_total(cx, target)
        }
        Payload::ShapeList => Ok(cx.data(|data| data.pinned_query.len())?),
        _ => Ok(0),
    }
}

fn list_instance(cx: &Cx, this: &Payload, index: usize) -> mlua::Result<Option<InstanceTarget>> {
    Ok(match this {
        Payload::InstanceList(InstanceListKind::ShapesWithTag) => {
            cx.data(|data| data.instance_query.get(index).copied())?
        }
        Payload::InstanceList(InstanceListKind::PatternsWithTag) => {
            cx.data(|data| data.pattern_query.get(index).copied())?
        }
        Payload::InstanceList(InstanceListKind::PatternInstances(target)) => {
            (index < instance_total(cx, target)?).then(|| pattern_instance(target, index))
        }
        _ => None,
    })
}

fn list_shape(cx: &Cx, index: usize) -> mlua::Result<Option<ShapeTarget>> {
    Ok(cx.data(|data| data.pinned_query.get(index).copied())?)
}

fn lists() -> [TypeDef; 2] {
    let mut instances = type_def(HostType::InstanceList, "Liste<Instance>");
    instances.read_only("Count", |cx, this| Ok(count(list_len(cx, this)?)), None);
    instances.element = Some(HostType::Instance);
    instances.indexer = Some(Arc::new(|cx, this, key| {
        let index = cx.integer(key, "Liste<Instance>")?;
        let position = cx.check_bounds(index, list_len(cx, this)?, "Liste<Instance>")?;
        match list_instance(cx, this, position)? {
            Some(target) => cx.object(Payload::Instance(target)),
            None => Ok(Value::Nil),
        }
    }));
    instances.function(
        "Contains",
        |call| {
            let found = instance_position(call.cx, call.host(1), call.host(2))?;
            reply(boolean(found.is_some()))
        },
        None,
    );
    instances.function(
        "IndexOf",
        |call| {
            let found = instance_position(call.cx, call.host(1), call.host(2))?;
            reply(found.map_or_else(|| num(-1), count))
        },
        None,
    );

    let mut shapes = type_def(HostType::ShapeList, "Liste<Shape>");
    shapes.read_only("Count", |cx, this| Ok(count(list_len(cx, this)?)), None);
    shapes.element = Some(HostType::Shape);
    shapes.indexer = Some(Arc::new(|cx, _, key| {
        let index = cx.integer(key, "Liste<Shape>")?;
        let position =
            cx.check_bounds(index, list_len(cx, &Payload::ShapeList)?, "Liste<Shape>")?;
        match list_shape(cx, position)? {
            Some(target) => cx.object(Payload::Shape(target)),
            None => Ok(Value::Nil),
        }
    }));
    shapes.function(
        "Contains",
        |call| {
            let found = shape_position(call.cx, call.host(2))?;
            reply(boolean(found.is_some()))
        },
        None,
    );
    shapes.function(
        "IndexOf",
        |call| {
            let found = shape_position(call.cx, call.host(2))?;
            reply(found.map_or_else(|| num(-1), count))
        },
        None,
    );
    [instances, shapes]
}

/// The position of the instance `wanted` stands for in the list `list` stands for.
fn instance_position(
    cx: &Cx,
    list: Option<Payload>,
    wanted: Option<Payload>,
) -> mlua::Result<Option<usize>> {
    let (Some(list), Some(Payload::Instance(wanted))) = (list, wanted) else {
        return Ok(None);
    };

    for index in 0..list_len(cx, &list)? {
        if list_instance(cx, &list, index)? == Some(wanted) {
            return Ok(Some(index));
        }
    }

    Ok(None)
}

/// The position of the shape `wanted` stands for in the shared list of pinned shapes.
fn shape_position(cx: &Cx, wanted: Option<Payload>) -> mlua::Result<Option<usize>> {
    let Some(Payload::Shape(wanted)) = wanted else {
        return Ok(None);
    };

    cx.data(|data| data.pinned_query.iter().position(|shape| *shape == wanted))
}
