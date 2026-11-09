//! The input word a frame is run with, and what is plugged into a controller port.
//!
//! Moved here from `zax_nes::input` by lot FCEUX-23 step 2: `Engine::run_frame` takes the word
//! and `Engine::port_type`/`set_port_type` take the type, so both engines have to mean the same
//! thing by them. The *ports themselves* — `zax_nes::input::InputPorts`, the shift registers,
//! the Zapper — are Zaxnes's own machine and stay there; FCEUX has its own.

/// The 16 button bits one frame is run with (REQ-CORE-020, REQ-CORE-022): **port 1 in bits 0 to
/// 7, port 2 in bits 8 to 15**, and for a controller the order inside a byte is A, B, Select,
/// Start, Up, Down, Left, Right (SPEC-02 §4.4).
///
/// An alias rather than a newtype, deliberately: the word crosses the FFI boundary to FCEUX as a
/// `uint16_t` (`zax_set_input_word`) and is built by the application's input routing from the
/// same eight [`button`] bits on both engines. Naming it is what matters — the two engines agreeing
/// on which bit is `A` is a correctness question the C API already answers, and `zax-fceux`'s
/// `Fceux::set_input_word` is tested against this layout.
pub type InputWord = u16;

/// The bit of each button inside one port's byte of an [`InputWord`] (SPEC-02 §4.4).
///
/// Port 1 uses these as they are; port 2 uses them shifted left by 8.
pub mod button {
    /// A.
    pub const A: u8 = 1 << 0;
    /// B.
    pub const B: u8 = 1 << 1;
    /// Select.
    pub const SELECT: u8 = 1 << 2;
    /// Start.
    pub const START: u8 = 1 << 3;
    /// Up.
    pub const UP: u8 = 1 << 4;
    /// Down.
    pub const DOWN: u8 = 1 << 5;
    /// Left.
    pub const LEFT: u8 = 1 << 6;
    /// Right.
    pub const RIGHT: u8 = 1 << 7;
}

/// How far port 2's byte sits inside an [`InputWord`].
pub const PORT_2_SHIFT: u32 = 8;

/// Controller ports a machine has (REQ-CORE-022): the two at `$4016` and `$4017`.
pub const PORT_COUNT: usize = 2;

/// What is plugged into a controller port (REQ-CORE-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControllerPortType {
    /// A standard pad.
    #[default]
    Controller = 0,
    /// A Zapper light gun.
    Zapper = 1,
}

/// A port number outside 0 to 1 (REQ-CORE-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPort(pub usize);

impl std::fmt::Display for InvalidPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "port {} is out of range (0 or 1)", self.0)
    }
}

impl std::error::Error for InvalidPort {}
