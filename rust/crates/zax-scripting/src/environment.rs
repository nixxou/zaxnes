//! What the Lua runtime needs from the host besides the conversion engine: the camera, the
//! settings, the backdrop, the inputs, the core memory, the notifications, the profile metadata
//! and the log (SPEC-05 REQ-LUA-088 to REQ-LUA-095).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use zax_engine3d::vectors::Vec3;

use crate::settings::ScriptSettings;
use crate::values::LuaColor;

/// Severity of a notification a script raises (SPEC-05 REQ-LUA-093, SPEC-10 REQ-UI-080).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptNotificationSeverity {
    /// Ordinary message.
    #[default]
    Normal = 0,
    /// Warning.
    Warning = 1,
    /// Error.
    Error = 2,
}

impl ScriptNotificationSeverity {
    /// The severity of the number a script passed, clamped to the three levels.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            i32::MIN..=0 => Self::Normal,
            1 => Self::Warning,
            _ => Self::Error,
        }
    }
}

/// A notification a script raised through `NotiCenter` (REQ-LUA-093).
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptNotification {
    /// The text.
    pub message: String,
    /// How long it stays visible; 4 by default.
    pub seconds: f32,
    /// Its severity.
    pub severity: ScriptNotificationSeverity,
}

/// Where the runtime writes `print` output and script errors (REQ-LUA-052, REQ-LUA-061).
pub trait ScriptLog: Send {
    /// One `print` line, channel "Lua", information level (REQ-LUA-052).
    fn print(&mut self, line: &str);

    /// One diagnostic line: a compilation, chunk or hook error (REQ-LUA-060, 061).
    fn error(&mut self, line: &str);

    /// One warning: a duplicate script name, for instance (REQ-LUA-003).
    fn warning(&mut self, line: &str);
}

/// A log that drops everything, the default for a headless runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullScriptLog;

impl ScriptLog for NullScriptLog {
    fn print(&mut self, _line: &str) {}

    fn error(&mut self, _line: &str) {}

    fn warning(&mut self, _line: &str) {}
}

#[derive(Debug, Default)]
struct CollectedLines {
    prints: Vec<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

/// A log that keeps every line in memory, used by the tests and the harness. Clones share the
/// same lines, so the host keeps one handle and gives the other to the runtime.
#[derive(Debug, Clone, Default)]
pub struct CollectingScriptLog {
    lines: Arc<Mutex<CollectedLines>>,
}

impl CollectingScriptLog {
    /// Creates an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The lines; a panic while they were held left them as they were.
    fn lines(&self) -> MutexGuard<'_, CollectedLines> {
        self.lines.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The `print` lines, in order.
    #[must_use]
    pub fn prints(&self) -> Vec<String> {
        self.lines().prints.clone()
    }

    /// The error lines, in order.
    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        self.lines().errors.clone()
    }

    /// The warning lines, in order.
    #[must_use]
    pub fn warnings(&self) -> Vec<String> {
        self.lines().warnings.clone()
    }

    /// Forgets every line.
    pub fn clear(&self) {
        *self.lines() = CollectedLines::default();
    }
}

impl ScriptLog for CollectingScriptLog {
    fn print(&mut self, line: &str) {
        self.lines().prints.push(line.to_owned());
    }

    fn error(&mut self, line: &str) {
        self.lines().errors.push(line.to_owned());
    }

    fn warning(&mut self, line: &str) {
        self.lines().warnings.push(line.to_owned());
    }
}

/// The camera as scripts see it (SPEC-05 REQ-LUA-088, `CameraAPI`). The host binds it to the
/// real camera of SPEC-06; [`DetachedScriptCamera`] is the headless stand-in.
pub trait ScriptCamera: Send {
    /// Orientation of the current mode in degrees, `(pitch, yaw, roll)`.
    fn angles(&self) -> Vec3;

    /// Sets the orientation of the current mode.
    fn set_angles(&mut self, angles: Vec3);

    /// Distance of the current mode.
    fn distance(&self) -> f32;

    /// Sets the distance; the setter bounds it to `[0, 512 × SceneScale]` (REQ-LUA-088).
    fn set_distance(&mut self, distance: f32);

    /// Mode: 0 = TPV, 1 = FPV.
    fn mode(&self) -> i32;

    /// Sets the mode.
    fn set_mode(&mut self, mode: i32);

    /// Aimed point in scene coordinates; `(128, 120, 0)` by default.
    fn look_at(&self) -> Vec3;

    /// Sets the aimed point.
    fn set_look_at(&mut self, target: Vec3);

    /// The default view of SPEC-06 REQ-SCN-012 (`Camera:ResetView()`).
    fn reset_view(&mut self);

    /// The upper bound of the distance, `512 × SceneScale`.
    fn max_distance(&self) -> f32 {
        512.0
    }
}

/// A camera that holds its state and nothing else: what the headless runtime, the acceptance
/// harness and the unit tests use.
#[derive(Debug, Clone)]
pub struct DetachedScriptCamera {
    angles: Vec3,
    distance: f32,
    mode: i32,
    look_at: Vec3,
    reset_count: u32,
}

impl DetachedScriptCamera {
    const DEFAULT_ANGLES: Vec3 = Vec3::new(0.81, 0.81, 0.0);
    const DEFAULT_LOOK_AT: Vec3 = Vec3::new(128.0, 120.0, 0.0);

    /// Creates the camera in its default view.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            angles: Self::DEFAULT_ANGLES,
            distance: 260.0,
            mode: 0,
            look_at: Self::DEFAULT_LOOK_AT,
            reset_count: 0,
        }
    }

    /// How many times [`ScriptCamera::reset_view`] was called.
    #[must_use]
    pub const fn reset_count(&self) -> u32 {
        self.reset_count
    }
}

impl Default for DetachedScriptCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptCamera for DetachedScriptCamera {
    fn angles(&self) -> Vec3 {
        self.angles
    }

    fn set_angles(&mut self, angles: Vec3) {
        self.angles = angles;
    }

    fn distance(&self) -> f32 {
        self.distance
    }

    fn set_distance(&mut self, distance: f32) {
        self.distance = distance.clamp(0.0, self.max_distance());
    }

    fn mode(&self) -> i32 {
        self.mode
    }

    fn set_mode(&mut self, mode: i32) {
        self.mode = mode;
    }

    fn look_at(&self) -> Vec3 {
        self.look_at
    }

    fn set_look_at(&mut self, target: Vec3) {
        self.look_at = target;
    }

    fn reset_view(&mut self) {
        self.reset_count += 1;
        self.angles = Self::DEFAULT_ANGLES;
        self.look_at = Self::DEFAULT_LOOK_AT;
    }
}

/// The backdrop as scripts see it (SPEC-05 REQ-LUA-090, SPEC-06 REQ-SCN-048).
pub trait ScriptBackdrop: Send {
    /// Whether the panel is displayed; false by default.
    fn enabled(&self) -> bool;

    /// Shows or hides the panel.
    fn set_enabled(&mut self, enabled: bool);

    /// Colour of the panel, opaque black by default.
    fn color(&self) -> LuaColor;

    /// Sets the colour of the panel.
    fn set_color(&mut self, color: LuaColor);

    /// Whether the panel receives shadows; true by default.
    fn receive_shadow(&self) -> bool;

    /// Sets whether the panel receives shadows.
    fn set_receive_shadow(&mut self, receive: bool);

    /// Size of the panel, `(256, 240, 1)` by default.
    fn size(&self) -> Vec3;

    /// Sets the size of the panel.
    fn set_size(&mut self, size: Vec3);

    /// Offset of the panel centre, zero by default.
    fn offset(&self) -> Vec3;

    /// Sets the offset of the panel centre.
    fn set_offset(&mut self, offset: Vec3);

    /// The end-of-game reset of REQ-LUA-090.
    fn reset(&mut self);
}

/// A backdrop that holds its own state (headless default).
#[derive(Debug, Clone)]
pub struct DetachedScriptBackdrop {
    enabled: bool,
    color: LuaColor,
    receive_shadow: bool,
    size: Vec3,
    offset: Vec3,
}

impl DetachedScriptBackdrop {
    /// Creates the backdrop in its reset state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            enabled: false,
            color: LuaColor::BLACK,
            receive_shadow: true,
            size: Vec3::new(256.0, 240.0, 1.0),
            offset: Vec3::ZERO,
        }
    }
}

impl Default for DetachedScriptBackdrop {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBackdrop for DetachedScriptBackdrop {
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn color(&self) -> LuaColor {
        self.color
    }

    fn set_color(&mut self, color: LuaColor) {
        self.color = color;
    }

    fn receive_shadow(&self) -> bool {
        self.receive_shadow
    }

    fn set_receive_shadow(&mut self, receive: bool) {
        self.receive_shadow = receive;
    }

    fn size(&self) -> Vec3 {
        self.size
    }

    fn set_size(&mut self, size: Vec3) {
        self.size = size;
    }

    fn offset(&self) -> Vec3 {
        self.offset
    }

    fn set_offset(&mut self, offset: Vec3) {
        self.offset = offset;
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

/// The inputs as scripts see them: the two players' action masks, the port bytes, the port
/// types and the Zapper aim offset (SPEC-05 REQ-LUA-091, 092, SPEC-08 §4.1).
pub trait ScriptInput: Send {
    /// Action mask of player `index` (0 or 1), SPEC-08 REQ-INP-002; 0 for any other index.
    fn player_state(&self, index: usize) -> i32;

    /// The global action mask: the or of both players' (SPEC-08).
    fn global_state(&self) -> i32;

    /// The NES controller byte of `port` (0 or 1); 0 for any other port.
    fn port_byte(&self, port: usize) -> i32;

    /// Device type of `port`: 0 controller, 1 Zapper; 0 for any other port.
    fn port_type(&self, port: usize) -> i32;

    /// Sets the device type of both ports (REQ-INP-060).
    fn set_port_types(&mut self, first: i32, second: i32);

    /// The Zapper aim offset in scene units (SPEC-08 REQ-INP-063).
    fn zapper_offset(&self) -> Vec3;

    /// Sets the Zapper aim offset.
    fn set_zapper_offset(&mut self, offset: Vec3);
}

/// Inputs that hold their own state (headless default): nothing is ever pressed.
#[derive(Debug, Clone, Default)]
pub struct DetachedScriptInput {
    players: [i32; 2],
    types: [i32; 2],
    zapper_offset: Vec3,
}

impl DetachedScriptInput {
    /// Sets the action mask a player reports.
    pub fn set_player_state(&mut self, index: usize, mask: i32) {
        self.players[index & 1] = mask;
    }
}

impl ScriptInput for DetachedScriptInput {
    fn player_state(&self, index: usize) -> i32 {
        self.players.get(index).copied().unwrap_or(0)
    }

    fn global_state(&self) -> i32 {
        self.players[0] | self.players[1]
    }

    fn port_byte(&self, port: usize) -> i32 {
        self.players.get(port).map_or(0, |mask| mask & 0xFF)
    }

    fn port_type(&self, port: usize) -> i32 {
        self.types.get(port).copied().unwrap_or(0)
    }

    fn set_port_types(&mut self, first: i32, second: i32) {
        self.types = [first, second];
    }

    fn zapper_offset(&self) -> Vec3 {
        self.zapper_offset
    }

    fn set_zapper_offset(&mut self, offset: Vec3) {
        self.zapper_offset = offset;
    }
}

/// Side-effect-free access to the core's CPU address space (SPEC-05 REQ-LUA-094, SPEC-02
/// REQ-CORE-080).
pub trait ScriptMemory: Send {
    /// Reads the byte at the 16-bit CPU address `address`.
    fn read(&self, address: u16) -> u8;

    /// Writes a byte at the 16-bit CPU address `address`.
    fn write(&mut self, address: u16, value: u8);
}

/// A flat 64 KiB scratch space: the headless default.
#[derive(Debug, Clone)]
pub struct DetachedScriptMemory {
    bytes: Box<[u8]>,
}

impl DetachedScriptMemory {
    /// Creates the zeroed memory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bytes: vec![0; 0x1_0000].into_boxed_slice(),
        }
    }
}

impl Default for DetachedScriptMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptMemory for DetachedScriptMemory {
    fn read(&self, address: u16) -> u8 {
        self.bytes[usize::from(address)]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.bytes[usize::from(address)] = value;
    }
}

/// The profile metadata a script may read and write for the session (SPEC-05 REQ-LUA-095,
/// SPEC-03 REQ-PROF-022). Writing has no effect on the library display.
#[derive(Debug, Clone, Default)]
pub struct ScriptProfileInfo {
    /// Title, field 8.
    pub title: String,
    /// Profile author, field 9.
    pub maker: String,
    /// Game publisher, field 10.
    pub game_dev: String,
    /// Description, field 11.
    pub description: String,
    /// Profile identifier, field 7; read only.
    pub id: String,
    /// Format version, field 2; read only.
    pub version: String,
}

impl ScriptProfileInfo {
    /// True when the version is "2" (SPEC-03).
    #[must_use]
    pub fn is_v2_format(&self) -> bool {
        self.version == "2"
    }

    /// Replaces the identifier with a fresh 12-character one (SPEC-03 REQ-PROF-052), without
    /// any other effect.
    ///
    /// # Errors
    ///
    /// The operating system could not supply random bytes.
    pub fn renew_id(&mut self) -> Result<(), getrandom::Error> {
        const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        // Rejection sampling keeps the draw uniform: 248 is the largest multiple of 62 that
        // fits in a byte.
        const LIMIT: u8 = 248;

        let mut id = String::with_capacity(12);
        while id.len() < 12 {
            let mut buffer = [0_u8; 12];
            getrandom::fill(&mut buffer)?;
            for byte in buffer.into_iter().filter(|byte| *byte < LIMIT) {
                if id.len() < 12 {
                    id.push(char::from(ALPHABET[usize::from(byte) % ALPHABET.len()]));
                }
            }
        }

        self.id = id;
        Ok(())
    }
}

/// The input and memory bridges of one machine. A host whose inputs and memory belong to the
/// same core implements both traits on one type, which a context can then lend as one value.
pub trait ScriptMachine: ScriptInput + ScriptMemory {}

impl<T: ScriptInput + ScriptMemory + ?Sized> ScriptMachine for T {}

/// What the host lends to the script API for the length of one hook call: the camera and the
/// backdrop of the scene, and the inputs and memory of the machine.
///
/// The runtime never keeps these: the borrow ends with the call, so the host stays the only
/// owner of the scene and the core and the compiler rules out conflicting access. See
/// [`lend_context`](crate::lend_context).
pub struct ScriptContext<'a> {
    /// The camera exposed as `Camera` (REQ-LUA-088).
    pub camera: &'a mut dyn ScriptCamera,
    /// The backdrop exposed as `Backdrop` (REQ-LUA-090).
    pub backdrop: &'a mut dyn ScriptBackdrop,
    /// The inputs and the memory exposed as `Gamepad`, `ScriptPad` and `Script:ReadMem`
    /// (REQ-LUA-091, 092, 094).
    pub machine: &'a mut dyn ScriptMachine,
}

/// The detached input and memory of an environment, seen as one machine.
struct HeadlessMachine<'a> {
    input: &'a mut dyn ScriptInput,
    memory: &'a mut dyn ScriptMemory,
}

impl ScriptInput for HeadlessMachine<'_> {
    fn player_state(&self, index: usize) -> i32 {
        self.input.player_state(index)
    }

    fn global_state(&self) -> i32 {
        self.input.global_state()
    }

    fn port_byte(&self, port: usize) -> i32 {
        self.input.port_byte(port)
    }

    fn port_type(&self, port: usize) -> i32 {
        self.input.port_type(port)
    }

    fn set_port_types(&mut self, first: i32, second: i32) {
        self.input.set_port_types(first, second);
    }

    fn zapper_offset(&self) -> Vec3 {
        self.input.zapper_offset()
    }

    fn set_zapper_offset(&mut self, offset: Vec3) {
        self.input.set_zapper_offset(offset);
    }
}

impl ScriptMemory for HeadlessMachine<'_> {
    fn read(&self, address: u16) -> u8 {
        self.memory.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.memory.write(address, value);
    }
}

/// Everything the Lua runtime needs from the host besides the conversion engine.
///
/// Every member has a working default, so a runtime can be created for the acceptance test of
/// REQ-ACC-011 and for the unit tests without a window, a renderer or a ROM. The camera,
/// backdrop, input and memory members are the headless bridges: the script API uses them only
/// when the host lends no [`ScriptContext`] to the call.
pub struct ScriptEnvironment {
    /// The settings model exposed as `Setting` (REQ-LUA-089).
    pub settings: ScriptSettings,
    /// The camera exposed as `Camera` (REQ-LUA-088).
    pub camera: Box<dyn ScriptCamera>,
    /// The backdrop exposed as `Backdrop` (REQ-LUA-090).
    pub backdrop: Box<dyn ScriptBackdrop>,
    /// The inputs exposed as `Gamepad` and `ScriptPad` (REQ-LUA-091, 092).
    pub input: Box<dyn ScriptInput>,
    /// The core memory exposed through `Script:ReadMem` (REQ-LUA-094).
    pub memory: Box<dyn ScriptMemory>,
    /// The profile metadata exposed as `Profile` (REQ-LUA-095).
    pub profile: ScriptProfileInfo,
    /// Where `print` and the error messages go (REQ-LUA-052, 061).
    pub log: Box<dyn ScriptLog>,
    /// What `NotiCenter.Notify` raises (REQ-LUA-093); ignored by default.
    pub notify: Option<Box<dyn FnMut(ScriptNotification) + Send>>,
    /// The optimised Lua paths (Rust-only setting `emu.lua_optimized`, `unsafe` C metamethods;
    /// see `binding::raw`). Off by default; `ZAXNES_FAST_LUA=1` turns them on for a run of the
    /// tests.
    pub optimized: bool,
}

impl ScriptEnvironment {
    /// Runs `operation` on the headless bridges.
    pub(crate) fn with_headless_context<R>(
        &mut self,
        operation: impl FnOnce(&mut ScriptContext<'_>) -> R,
    ) -> R {
        let mut machine = HeadlessMachine {
            input: &mut *self.input,
            memory: &mut *self.memory,
        };
        operation(&mut ScriptContext {
            camera: &mut *self.camera,
            backdrop: &mut *self.backdrop,
            machine: &mut machine,
        })
    }
}

impl Default for ScriptEnvironment {
    fn default() -> Self {
        Self {
            settings: ScriptSettings::default(),
            camera: Box::new(DetachedScriptCamera::new()),
            backdrop: Box::new(DetachedScriptBackdrop::new()),
            input: Box::new(DetachedScriptInput::default()),
            memory: Box::new(DetachedScriptMemory::new()),
            profile: ScriptProfileInfo::default(),
            log: Box::new(NullScriptLog),
            notify: None,
            optimized: std::env::var("ZAXNES_FAST_LUA").is_ok_and(|value| value.trim() == "1"),
        }
    }
}
