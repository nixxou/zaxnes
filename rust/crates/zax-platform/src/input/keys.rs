//! Physical key positions, numbered like GLFW (and so like Silk.NET's `Key`) because the
//! persisted bindings of SPEC-09 REQ-PERS-065 store those numbers (DEC-08-5).
//!
//! SDL reports scancodes, which are physical positions too: the key labelled `W` on an AZERTY
//! keyboard reports the position of `Z`, the behaviour DEC-08-5 asks for.

use sdl2::keyboard::Scancode;

macro_rules! keys {
    ($($name:ident = $code:literal => $scancode:expr,)*) => {
        /// A physical key position; the discriminant is the persisted key code.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(i32)]
        pub enum Key {
            $(
                #[doc = concat!("Key code ", stringify!($code), ".")]
                $name = $code,
            )*
        }

        impl Key {
            /// Every known key, by increasing code.
            pub const ALL: &'static [Key] = &[$(Key::$name),*];

            /// The name of the key, as Silk.NET's `Key` enumeration spells it.
            #[must_use]
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$name => stringify!($name),)*
                }
            }

            fn scancode(self) -> Option<Scancode> {
                match self {
                    $(Key::$name => $scancode,)*
                }
            }
        }
    };
}

keys! {
    Space = 32 => Some(Scancode::Space),
    Apostrophe = 39 => Some(Scancode::Apostrophe),
    Comma = 44 => Some(Scancode::Comma),
    Minus = 45 => Some(Scancode::Minus),
    Period = 46 => Some(Scancode::Period),
    Slash = 47 => Some(Scancode::Slash),
    Number0 = 48 => Some(Scancode::Num0),
    Number1 = 49 => Some(Scancode::Num1),
    Number2 = 50 => Some(Scancode::Num2),
    Number3 = 51 => Some(Scancode::Num3),
    Number4 = 52 => Some(Scancode::Num4),
    Number5 = 53 => Some(Scancode::Num5),
    Number6 = 54 => Some(Scancode::Num6),
    Number7 = 55 => Some(Scancode::Num7),
    Number8 = 56 => Some(Scancode::Num8),
    Number9 = 57 => Some(Scancode::Num9),
    Semicolon = 59 => Some(Scancode::Semicolon),
    Equal = 61 => Some(Scancode::Equals),
    A = 65 => Some(Scancode::A),
    B = 66 => Some(Scancode::B),
    C = 67 => Some(Scancode::C),
    D = 68 => Some(Scancode::D),
    E = 69 => Some(Scancode::E),
    F = 70 => Some(Scancode::F),
    G = 71 => Some(Scancode::G),
    H = 72 => Some(Scancode::H),
    I = 73 => Some(Scancode::I),
    J = 74 => Some(Scancode::J),
    K = 75 => Some(Scancode::K),
    L = 76 => Some(Scancode::L),
    M = 77 => Some(Scancode::M),
    N = 78 => Some(Scancode::N),
    O = 79 => Some(Scancode::O),
    P = 80 => Some(Scancode::P),
    Q = 81 => Some(Scancode::Q),
    R = 82 => Some(Scancode::R),
    S = 83 => Some(Scancode::S),
    T = 84 => Some(Scancode::T),
    U = 85 => Some(Scancode::U),
    V = 86 => Some(Scancode::V),
    W = 87 => Some(Scancode::W),
    X = 88 => Some(Scancode::X),
    Y = 89 => Some(Scancode::Y),
    Z = 90 => Some(Scancode::Z),
    LeftBracket = 91 => Some(Scancode::LeftBracket),
    BackSlash = 92 => Some(Scancode::Backslash),
    RightBracket = 93 => Some(Scancode::RightBracket),
    GraveAccent = 96 => Some(Scancode::Grave),
    World1 = 161 => Some(Scancode::NonUsBackslash),
    World2 = 162 => None,
    Escape = 256 => Some(Scancode::Escape),
    Enter = 257 => Some(Scancode::Return),
    Tab = 258 => Some(Scancode::Tab),
    Backspace = 259 => Some(Scancode::Backspace),
    Insert = 260 => Some(Scancode::Insert),
    Delete = 261 => Some(Scancode::Delete),
    Right = 262 => Some(Scancode::Right),
    Left = 263 => Some(Scancode::Left),
    Down = 264 => Some(Scancode::Down),
    Up = 265 => Some(Scancode::Up),
    PageUp = 266 => Some(Scancode::PageUp),
    PageDown = 267 => Some(Scancode::PageDown),
    Home = 268 => Some(Scancode::Home),
    End = 269 => Some(Scancode::End),
    CapsLock = 280 => Some(Scancode::CapsLock),
    ScrollLock = 281 => Some(Scancode::ScrollLock),
    NumLock = 282 => Some(Scancode::NumLockClear),
    PrintScreen = 283 => Some(Scancode::PrintScreen),
    Pause = 284 => Some(Scancode::Pause),
    F1 = 290 => Some(Scancode::F1),
    F2 = 291 => Some(Scancode::F2),
    F3 = 292 => Some(Scancode::F3),
    F4 = 293 => Some(Scancode::F4),
    F5 = 294 => Some(Scancode::F5),
    F6 = 295 => Some(Scancode::F6),
    F7 = 296 => Some(Scancode::F7),
    F8 = 297 => Some(Scancode::F8),
    F9 = 298 => Some(Scancode::F9),
    F10 = 299 => Some(Scancode::F10),
    F11 = 300 => Some(Scancode::F11),
    F12 = 301 => Some(Scancode::F12),
    F13 = 302 => Some(Scancode::F13),
    F14 = 303 => Some(Scancode::F14),
    F15 = 304 => Some(Scancode::F15),
    F16 = 305 => Some(Scancode::F16),
    F17 = 306 => Some(Scancode::F17),
    F18 = 307 => Some(Scancode::F18),
    F19 = 308 => Some(Scancode::F19),
    F20 = 309 => Some(Scancode::F20),
    F21 = 310 => Some(Scancode::F21),
    F22 = 311 => Some(Scancode::F22),
    F23 = 312 => Some(Scancode::F23),
    F24 = 313 => Some(Scancode::F24),
    F25 = 314 => None,
    Keypad0 = 320 => Some(Scancode::Kp0),
    Keypad1 = 321 => Some(Scancode::Kp1),
    Keypad2 = 322 => Some(Scancode::Kp2),
    Keypad3 = 323 => Some(Scancode::Kp3),
    Keypad4 = 324 => Some(Scancode::Kp4),
    Keypad5 = 325 => Some(Scancode::Kp5),
    Keypad6 = 326 => Some(Scancode::Kp6),
    Keypad7 = 327 => Some(Scancode::Kp7),
    Keypad8 = 328 => Some(Scancode::Kp8),
    Keypad9 = 329 => Some(Scancode::Kp9),
    KeypadDecimal = 330 => Some(Scancode::KpPeriod),
    KeypadDivide = 331 => Some(Scancode::KpDivide),
    KeypadMultiply = 332 => Some(Scancode::KpMultiply),
    KeypadSubtract = 333 => Some(Scancode::KpMinus),
    KeypadAdd = 334 => Some(Scancode::KpPlus),
    KeypadEnter = 335 => Some(Scancode::KpEnter),
    KeypadEqual = 336 => Some(Scancode::KpEquals),
    ShiftLeft = 340 => Some(Scancode::LShift),
    ControlLeft = 341 => Some(Scancode::LCtrl),
    AltLeft = 342 => Some(Scancode::LAlt),
    SuperLeft = 343 => Some(Scancode::LGui),
    ShiftRight = 344 => Some(Scancode::RShift),
    ControlRight = 345 => Some(Scancode::RCtrl),
    AltRight = 346 => Some(Scancode::RAlt),
    SuperRight = 347 => Some(Scancode::RGui),
    Menu = 348 => Some(Scancode::Application),
}

impl Key {
    /// The persisted code of this key, for the default tables of §4.2.
    #[must_use]
    pub fn code(self) -> i32 {
        self as i32
    }

    /// The key with `code`, or `None` for a code this backend does not know — a binding file
    /// written on another backend must not fail (REQ-INP-052).
    #[must_use]
    pub fn from_code(code: i32) -> Option<Key> {
        Key::ALL
            .binary_search_by_key(&code, |key| key.code())
            .ok()
            .map(|index| Key::ALL[index])
    }

    /// The key at the physical position `scancode`, if it has a code.
    #[must_use]
    pub fn from_scancode(scancode: Scancode) -> Option<Key> {
        Key::ALL
            .iter()
            .copied()
            .find(|key| key.scancode() == Some(scancode))
    }
}
