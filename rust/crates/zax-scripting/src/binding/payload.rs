//! The identity and the payload of a Lua-visible host value (SPEC-05 §4.2 and §4.3).

use zax_engine3d::frame::{FrameInstanceId, ShapeId};
use zax_engine3d::instance::InstanceRef;
use zax_engine3d::registry::PatternId;
use zax_engine3d::vectors::{Cell, Rect, Vec2, Vec3};

use crate::values::{LuaColor, LuaIntVector3, LuaXrState};

/// Identity of a Lua-visible host type (SPEC-05 §4.2 and §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HostType {
    Instance,
    Shape,
    Pattern,
    FrameApi,
    FrameManagerApi,
    PaletteApi,
    PatternApi,
    ScriptApi,
    ScriptObj,
    CameraApi,
    SettingApi,
    LayerConfig,
    Layer,
    Clipping,
    Light,
    BackdropApi,
    VideoApi,
    ProfileApi,
    ScriptPadApi,
    GamepadApi,
    Player,
    NotiApi,
    CustomData,
    Slot,
    InstanceList,
    ShapeList,
    Vec3Ref,
    Vec2Ref,
    Mat4Ref,
    Vector3,
    Vector2,
    Quaternion,
    Color,
    IntVector2,
    IntVector3,
    Rect,
    XrState,
    Vector3Class,
    Vector2Class,
    QuaternionClass,
    ColorClass,
    IntVector2Class,
    IntVector3Class,
    RectClass,
}

impl HostType {
    /// How many host types there are.
    pub const COUNT: usize = 44;

    /// The position of the type in the tables indexed by type.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// True for the types whose payload is the value itself, so that a Lua value is a copy
    /// (REQ-LUA-044).
    pub const fn is_value(self) -> bool {
        matches!(
            self,
            Self::Vector3
                | Self::Vector2
                | Self::Quaternion
                | Self::Color
                | Self::IntVector2
                | Self::IntVector3
                | Self::Rect
                | Self::XrState
        )
    }
}

/// Which frame a frame-local handle belongs to. Shapes, tiles and instances of a frame are
/// handles into that frame; a handle outlives its frame only as a reference to nothing, so it
/// carries the epoch of the runtime that issued it (REQ-LUA-028).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameKey {
    /// True for the pinned frame, false for the frame being converted.
    pub pinned: bool,
    /// The epoch the handle was issued in.
    pub epoch: u32,
}

/// An instance a script holds: one of a registry pattern or one of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum InstanceTarget {
    /// An instance of a registry pattern.
    Pattern(InstanceRef),
    /// An instance of a frame.
    Frame(FrameKey, FrameInstanceId),
}

/// A shape of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeTarget {
    pub key: FrameKey,
    pub id: ShapeId,
}

/// A pattern as a script sees it: a registry pattern, or the frame pattern of a shape, which is
/// the same object seen as a `Pattern` (SPEC-05 §4.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PatternTarget {
    Registry(PatternId),
    Frame(ShapeTarget),
}

/// The instance fields a bound `Vec3R` designates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstanceVec {
    Offset,
    Scale,
    Rotation,
    Pivot,
    DeformAmplitude,
    DeformPivot,
}

/// Which bound vector a `Vec3R` userdata designates (SPEC-05 §4.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Vec3Ref {
    Instance(InstanceTarget, InstanceVec),
    CameraRotation,
    CameraTarget,
    ZapperOffset,
    BackdropSize,
    BackdropOffset,
}

/// Which bound 2D vector of an instance a `Vec2R` designates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vec2Kind {
    TexOffset,
    TexTilting,
}

/// Which deformation matrix of an instance a `Mat4R` designates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mat4Kind {
    DeformSpeed,
    DeformOffset,
}

/// Which shared list a `Liste<Instance>` designates (REQ-LUA-048).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum InstanceListKind {
    /// The result of `Frame:GetShapesWithTag`.
    ShapesWithTag,
    /// The result of `Pattern.GetPatternsWithTag`.
    PatternsWithTag,
    /// The instances of a pattern (`pattern3DList`).
    PatternInstances(PatternTarget),
}

/// What a Lua-visible host value holds. Reference values name the object they stand for;
/// value types hold the value itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Payload {
    Instance(InstanceTarget),
    Shape(ShapeTarget),
    Pattern(PatternTarget),
    /// A script of the runtime, by position in script order.
    Script(usize),
    /// An API table or settings object that exists once per runtime.
    Table(HostType),
    Layer(usize),
    Player(usize),
    InstanceList(InstanceListKind),
    ShapeList,
    Vec3Ref(Vec3Ref),
    Vec2Ref(InstanceTarget, Vec2Kind),
    Mat4Ref(InstanceTarget, Mat4Kind),
    CustomData(InstanceTarget),
    Slot(InstanceRef),
    Vector3(Vec3),
    Vector2(Vec2),
    Quaternion([f32; 4]),
    Color(LuaColor),
    IntVector2(Cell),
    IntVector3(LuaIntVector3),
    Rect(Rect),
    XrState(LuaXrState),
}

impl Payload {
    /// The host type of this value.
    pub const fn host_type(&self) -> HostType {
        match self {
            Self::Instance(_) => HostType::Instance,
            Self::Shape(_) => HostType::Shape,
            Self::Pattern(_) => HostType::Pattern,
            Self::Script(_) => HostType::ScriptObj,
            Self::Table(kind) => *kind,
            Self::Layer(_) => HostType::Layer,
            Self::Player(_) => HostType::Player,
            Self::InstanceList(_) => HostType::InstanceList,
            Self::ShapeList => HostType::ShapeList,
            Self::Vec3Ref(_) => HostType::Vec3Ref,
            Self::Vec2Ref(..) => HostType::Vec2Ref,
            Self::Mat4Ref(..) => HostType::Mat4Ref,
            Self::CustomData(_) => HostType::CustomData,
            Self::Slot(_) => HostType::Slot,
            Self::Vector3(_) => HostType::Vector3,
            Self::Vector2(_) => HostType::Vector2,
            Self::Quaternion(_) => HostType::Quaternion,
            Self::Color(_) => HostType::Color,
            Self::IntVector2(_) => HostType::IntVector2,
            Self::IntVector3(_) => HostType::IntVector3,
            Self::Rect(_) => HostType::Rect,
            Self::XrState(_) => HostType::XrState,
        }
    }
}

/// The userdata every host value is. The payload of a value type is rewritten in place when a
/// script assigns one of its components: the value is the script's own copy (REQ-LUA-044).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Host {
    payload: std::cell::Cell<Payload>,
}

impl Host {
    pub const fn new(payload: Payload) -> Self {
        Self {
            payload: std::cell::Cell::new(payload),
        }
    }

    pub fn payload(&self) -> Payload {
        self.payload.get()
    }

    pub fn set_payload(&self, payload: Payload) {
        self.payload.set(payload);
    }
}

impl Payload {
    /// The key under which the Lua value of a reference object is cached, so that the same
    /// object always yields the same Lua value (REQ-LUA-043), or `None` for a value type or a
    /// descriptor, which is a fresh value every time.
    pub fn cache_key(&self) -> Option<[u8; 16]> {
        fn index(value: usize) -> u128 {
            u128::from(u32::try_from(value).unwrap_or(u32::MAX))
        }

        fn pattern(id: PatternId) -> u128 {
            (u128::from(id.slot()) << 32) | u128::from(id.generation())
        }

        fn frame(key: FrameKey) -> u128 {
            (u128::from(key.pinned) << 32) | u128::from(key.epoch)
        }

        fn shape(target: ShapeTarget) -> u128 {
            (frame(target.key) << 32) | index(target.id.index())
        }

        let (tag, body): (u128, u128) = match self {
            Self::Instance(InstanceTarget::Pattern(r)) => {
                (1, (pattern(r.pattern) << 32) | index(r.index))
            }
            Self::Instance(InstanceTarget::Frame(key, id)) => (
                2,
                (shape(ShapeTarget {
                    key: *key,
                    id: id.shape,
                }) << 32)
                    | index(id.index),
            ),
            Self::Shape(target) => (3, shape(*target)),
            Self::Pattern(PatternTarget::Registry(id)) => (4, pattern(*id)),
            Self::Pattern(PatternTarget::Frame(target)) => (5, shape(*target)),
            Self::Script(position) => (6, index(*position)),
            Self::Table(kind) => (7, kind.index() as u128),
            Self::Slot(r) => (8, (pattern(r.pattern) << 32) | index(r.index)),
            _ => return None,
        };

        Some(((tag << 120) | body).to_le_bytes())
    }
}
