//! Resolution of the handles a script holds into the engine state, and the bound vectors.
//!
//! A host value names its object by handle. A handle whose object is gone — a frame instance
//! of an earlier frame, a pattern dropped from the registry — resolves to nothing, and the
//! members then answer their defaults (REQ-LUA-028).

use std::ops::Deref;

use zax_engine3d::engine::{ConversionState, FrameRole};
use zax_engine3d::frame::{ConversionFrame, FrameInstance, FrameShape};
use zax_engine3d::instance::PatternInstance;
use zax_engine3d::pattern::EnginePattern;
use zax_engine3d::registry::PatternId;
use zax_engine3d::settings::ConversionSettings;
use zax_engine3d::vectors::{Mat4, Vec2, Vec3};

use super::cx::Cx;
use super::payload::{
    FrameKey, InstanceTarget, InstanceVec, Mat4Kind, ShapeTarget, Vec2Kind, Vec3Ref,
};

/// A read view of an instance a script holds.
pub(crate) enum InstanceView<'a> {
    /// An instance of a registry pattern.
    Pattern(&'a PatternInstance),
    /// An instance of a frame, with the state its quantities are computed under.
    Frame {
        instance: &'a FrameInstance,
        state: &'a ConversionState,
    },
}

impl Deref for InstanceView<'_> {
    type Target = PatternInstance;

    fn deref(&self) -> &PatternInstance {
        match self {
            Self::Pattern(instance) => instance,
            Self::Frame { instance, .. } => instance,
        }
    }
}

impl<'a> InstanceView<'a> {
    /// The frame instance and the state, when the view is of a frame instance.
    pub const fn frame(&self) -> Option<(&'a FrameInstance, &'a ConversionState)> {
        match self {
            Self::Pattern(_) => None,
            Self::Frame { instance, state } => Some((*instance, *state)),
        }
    }
}

const fn role_of(key: FrameKey) -> FrameRole {
    if key.pinned {
        FrameRole::Pinned
    } else {
        FrameRole::Converting
    }
}

impl Cx<'_> {
    /// The key a handle to the frame being converted (or the pinned frame) gets now.
    pub fn frame_key(&self, pinned: bool) -> mlua::Result<FrameKey> {
        self.data(|data| FrameKey {
            pinned,
            epoch: if pinned { data.pin_epoch } else { data.epoch },
        })
    }

    fn is_current(&self, key: FrameKey) -> mlua::Result<bool> {
        self.data(|data| {
            key.epoch
                == if key.pinned {
                    data.pin_epoch
                } else {
                    data.epoch
                }
        })
    }

    /// Runs `operation` on the instance `target` names, or returns `None` when it is gone.
    pub fn with_instance<R>(
        &self,
        target: &InstanceTarget,
        operation: impl FnOnce(InstanceView<'_>) -> R,
    ) -> mlua::Result<Option<R>> {
        match *target {
            InstanceTarget::Pattern(reference) => {
                let mut pending = Some(operation);
                let built = self
                    .build_target(reference, |building| {
                        pending
                            .take()
                            .map(|operation| operation(InstanceView::Pattern(building)))
                    })?
                    .flatten();
                if built.is_some() {
                    return Ok(built);
                }

                let Some(operation) = pending else {
                    return Ok(None);
                };
                self.state(|state| {
                    state
                        .patterns()
                        .instance(reference)
                        .map(|instance| operation(InstanceView::Pattern(instance)))
                })
            }
            InstanceTarget::Frame(key, id) => {
                if !self.is_current(key)? {
                    return Ok(None);
                }

                self.state(|state| {
                    let state: &ConversionState = state;
                    state
                        .frame(role_of(key))
                        .and_then(|frame| frame.get_instance(id))
                        .map(|instance| operation(InstanceView::Frame { instance, state }))
                })
            }
        }
    }

    /// Runs `operation` on the instance `target` names for writing, or returns `None` when it
    /// is gone. The pinned frame is never written.
    pub fn with_instance_mut<R>(
        &self,
        target: &InstanceTarget,
        operation: impl FnOnce(&mut PatternInstance) -> R,
    ) -> mlua::Result<Option<R>> {
        match *target {
            InstanceTarget::Pattern(reference) => {
                let mut pending = Some(operation);
                let built = self.build_target(reference, |building| {
                    pending.take().map(|operation| operation(building))
                })?;
                if built.is_some() {
                    return Ok(built.flatten());
                }

                let Some(operation) = pending else {
                    return Ok(None);
                };
                self.state(|state| {
                    state
                        .pattern_mut(reference.pattern)
                        .and_then(|pattern| pattern.instances_mut().get_mut(reference.index))
                        .map(operation)
                })
            }
            InstanceTarget::Frame(key, id) => {
                if key.pinned || !self.is_current(key)? {
                    return Ok(None);
                }

                self.state(|state| {
                    state
                        .converting_frame_mut()
                        .get_instance_mut(id)
                        .map(|instance| operation(instance))
                })
            }
        }
    }

    /// Runs `operation` on the frame instance `target` names for writing, together with the
    /// settings its positions are computed with.
    pub fn with_frame_instance_mut<R>(
        &self,
        target: &InstanceTarget,
        operation: impl FnOnce(&mut FrameInstance, &ConversionSettings) -> R,
    ) -> mlua::Result<Option<R>> {
        let InstanceTarget::Frame(key, id) = *target else {
            return Ok(None);
        };

        if key.pinned || !self.is_current(key)? {
            return Ok(None);
        }

        self.state(|state| {
            let (frame, settings) = state.converting_frame_and_settings_mut();
            frame
                .get_instance_mut(id)
                .map(|instance| operation(instance, settings))
        })
    }

    /// Runs `operation` on two frame instances at once, or returns `None` when either is gone
    /// or is not an instance of a frame.
    pub fn with_frame_pair<R>(
        &self,
        first: &InstanceTarget,
        second: &InstanceTarget,
        operation: impl FnOnce(&FrameInstance, &FrameInstance) -> R,
    ) -> mlua::Result<Option<R>> {
        let (
            InstanceTarget::Frame(first_key, first_id),
            InstanceTarget::Frame(second_key, second_id),
        ) = (*first, *second)
        else {
            return Ok(None);
        };

        if !self.is_current(first_key)? || !self.is_current(second_key)? {
            return Ok(None);
        }

        self.state(|state| {
            let first = state
                .frame(role_of(first_key))
                .and_then(|frame| frame.get_instance(first_id));
            let second = state
                .frame(role_of(second_key))
                .and_then(|frame| frame.get_instance(second_id));
            first
                .zip(second)
                .map(|(first, second)| operation(first, second))
        })
    }

    /// Runs `operation` on two shapes at once, or returns `None` when either is gone.
    pub fn with_shape_pair<R>(
        &self,
        first: &ShapeTarget,
        second: &ShapeTarget,
        operation: impl FnOnce(&FrameShape, &FrameShape) -> R,
    ) -> mlua::Result<Option<R>> {
        if !self.is_current(first.key)? || !self.is_current(second.key)? {
            return Ok(None);
        }

        self.state(|state| {
            let first_shape = state
                .frame(role_of(first.key))
                .and_then(|frame| frame.get_shape(first.id));
            let second_shape = state
                .frame(role_of(second.key))
                .and_then(|frame| frame.get_shape(second.id));
            first_shape
                .zip(second_shape)
                .map(|(first, second)| operation(first, second))
        })
    }

    /// Runs `operation` on the shape `target` names, or returns `None` when it is gone.
    pub fn with_shape<R>(
        &self,
        target: &ShapeTarget,
        operation: impl FnOnce(&FrameShape, &ConversionFrame, &ConversionState) -> R,
    ) -> mlua::Result<Option<R>> {
        if !self.is_current(target.key)? {
            return Ok(None);
        }

        self.state(|state| {
            let state: &ConversionState = state;
            state.frame(role_of(target.key)).and_then(|frame| {
                frame
                    .get_shape(target.id)
                    .map(|shape| operation(shape, frame, state))
            })
        })
    }

    /// Runs `operation` on the shape `target` names for writing. The pinned frame is never
    /// written.
    pub fn with_shape_mut<R>(
        &self,
        target: &ShapeTarget,
        operation: impl FnOnce(&mut FrameShape) -> R,
    ) -> mlua::Result<Option<R>> {
        if target.key.pinned || !self.is_current(target.key)? {
            return Ok(None);
        }

        self.state(|state| {
            state
                .converting_frame_mut()
                .get_shape_mut(target.id)
                .map(operation)
        })
    }

    /// Runs `operation` on the shape `target` names for writing, together with the settings its
    /// instances are positioned with.
    pub fn with_shape_settings_mut<R>(
        &self,
        target: &ShapeTarget,
        operation: impl FnOnce(&mut FrameShape, &ConversionSettings) -> R,
    ) -> mlua::Result<Option<R>> {
        if target.key.pinned || !self.is_current(target.key)? {
            return Ok(None);
        }

        self.state(|state| {
            let (frame, settings) = state.converting_frame_and_settings_mut();
            frame
                .get_shape_mut(target.id)
                .map(|shape| operation(shape, settings))
        })
    }

    /// Runs `operation` on the registry pattern `id`, or returns `None` when it is gone.
    pub fn with_pattern<R>(
        &self,
        id: PatternId,
        operation: impl FnOnce(&EnginePattern, &ConversionState) -> R,
    ) -> mlua::Result<Option<R>> {
        self.state(|state| {
            let state: &ConversionState = state;
            state
                .patterns()
                .get(id)
                .map(|pattern| operation(pattern, state))
        })
    }

    /// Runs `operation` on the registry pattern `id` for writing.
    pub fn with_pattern_mut<R>(
        &self,
        id: PatternId,
        operation: impl FnOnce(&mut EnginePattern) -> R,
    ) -> mlua::Result<Option<R>> {
        self.state(|state| state.pattern_mut(id).map(operation))
    }
}

/// Reads the three components a `Vec3R` designates; zero when the owner is gone.
pub(crate) fn read_vec3_ref(cx: &Cx, reference: &Vec3Ref) -> mlua::Result<Vec3> {
    match *reference {
        Vec3Ref::Instance(target, field) => Ok(cx
            .with_instance(&target, |instance| match field {
                InstanceVec::Offset => instance.offset,
                InstanceVec::Scale => instance.scale,
                InstanceVec::Rotation => instance.rotation,
                InstanceVec::Pivot => instance.pivot,
                InstanceVec::DeformAmplitude => instance.deform_amplitude,
                InstanceVec::DeformPivot => instance.deform_pivot,
            })?
            .unwrap_or_default()),
        Vec3Ref::CameraRotation => cx.host(move |ctx| ctx.camera.angles()),
        Vec3Ref::CameraTarget => cx.host(move |ctx| ctx.camera.look_at()),
        Vec3Ref::ZapperOffset => cx.host(move |ctx| ctx.machine.zapper_offset()),
        Vec3Ref::BackdropSize => cx.host(move |ctx| ctx.backdrop.size()),
        Vec3Ref::BackdropOffset => cx.host(move |ctx| ctx.backdrop.offset()),
    }
}

/// Writes the three components a `Vec3R` designates; a write to a gone object does nothing.
pub(crate) fn write_vec3_ref(cx: &Cx, reference: &Vec3Ref, value: Vec3) -> mlua::Result<()> {
    match *reference {
        Vec3Ref::Instance(target, field) => {
            cx.with_instance_mut(&target, |instance| match field {
                InstanceVec::Offset => instance.offset = value,
                InstanceVec::Scale => instance.scale = value,
                InstanceVec::Rotation => instance.rotation = value,
                InstanceVec::Pivot => instance.pivot = value,
                InstanceVec::DeformAmplitude => instance.deform_amplitude = value,
                InstanceVec::DeformPivot => instance.deform_pivot = value,
            })?;
        }
        Vec3Ref::CameraRotation => cx.host(move |ctx| ctx.camera.set_angles(value))?,
        Vec3Ref::CameraTarget => cx.host(move |ctx| ctx.camera.set_look_at(value))?,
        Vec3Ref::ZapperOffset => cx.host(move |ctx| ctx.machine.set_zapper_offset(value))?,
        Vec3Ref::BackdropSize => cx.host(move |ctx| ctx.backdrop.set_size(value))?,
        Vec3Ref::BackdropOffset => cx.host(move |ctx| ctx.backdrop.set_offset(value))?,
    }

    Ok(())
}

/// Reads the 2D vector of an instance a `Vec2R` designates.
pub(crate) fn read_vec2_ref(
    cx: &Cx,
    target: &InstanceTarget,
    kind: Vec2Kind,
) -> mlua::Result<Vec2> {
    Ok(cx
        .with_instance(target, |instance| match kind {
            Vec2Kind::TexOffset => instance.tex_offset,
            Vec2Kind::TexTilting => instance.tex_tilting,
        })?
        .unwrap_or_default())
}

/// Writes the 2D vector of an instance a `Vec2R` designates.
pub(crate) fn write_vec2_ref(
    cx: &Cx,
    target: &InstanceTarget,
    kind: Vec2Kind,
    value: Vec2,
) -> mlua::Result<()> {
    cx.with_instance_mut(target, |instance| match kind {
        Vec2Kind::TexOffset => instance.tex_offset = value,
        Vec2Kind::TexTilting => instance.tex_tilting = value,
    })?;
    Ok(())
}

/// Reads the deformation matrix of an instance a `Mat4R` designates.
pub(crate) fn read_matrix(cx: &Cx, target: &InstanceTarget, kind: Mat4Kind) -> mlua::Result<Mat4> {
    Ok(cx
        .with_instance(target, |instance| match kind {
            Mat4Kind::DeformSpeed => instance.deform_speed,
            Mat4Kind::DeformOffset => instance.deform_offset,
        })?
        .unwrap_or_default())
}

/// Writes the deformation matrix of an instance a `Mat4R` designates.
pub(crate) fn write_matrix(
    cx: &Cx,
    target: &InstanceTarget,
    kind: Mat4Kind,
    value: Mat4,
) -> mlua::Result<()> {
    cx.with_instance_mut(target, |instance| match kind {
        Mat4Kind::DeformSpeed => instance.deform_speed = value,
        Mat4Kind::DeformOffset => instance.deform_offset = value,
    })?;
    Ok(())
}
