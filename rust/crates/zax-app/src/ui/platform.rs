//! Feeds the SDL events of the window to Dear `ImGui`: the pointer, the wheel, the typed text and
//! the keys the widgets use (the C# `ImGuiController` read them from `Silk.NET`). The upstream
//! `imgui-sdl2-support` crate needs `sdl2` 0.37 and the application uses 0.38, whose native
//! `links` key makes the two impossible to build together.

use imgui::{BackendFlags, ConfigFlags, Io, Key, MouseButton};
use sdl2::controller::{Axis, Button};
use sdl2::event::Event;
use sdl2::keyboard::{Mod, Scancode};
use sdl2::mouse::MouseButton as SdlButton;

use super::handheld::pad::{PadButton, PadState};

/// The system clipboard as `ImGui` reads and writes it: without it, a text field can neither
/// copy nor paste (the account fields of RetroAchievements, owner request).
pub struct SdlClipboard(pub sdl2::clipboard::ClipboardUtil);

impl imgui::ClipboardBackend for SdlClipboard {
    fn get(&mut self) -> Option<String> {
        if self.0.has_clipboard_text() {
            self.0.clipboard_text().ok()
        } else {
            None
        }
    }

    fn set(&mut self, value: &str) {
        // A clipboard that refuses the text leaves it as it was: nothing else to do.
        let _ = self.0.set_clipboard_text(value);
    }
}

/// The state between events: how the window's coordinates scale to the drawable's.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // independent switches of each kind of platform
pub struct SdlPlatform {
    scale: [f32; 2],
    /// Whether the pointer events are dropped: `--ui-shots` photographs the interface as it is
    /// laid out, not as the host cursor happens to hover it (the C# capture has no pointer).
    ignore_pointer: bool,
    /// Whether the game-controller events drive the interface (the handheld mode): they feed
    /// `ImGui`'s gamepad keys and the [`PadState`] the handheld screens read.
    gamepad: bool,
    /// The [`PadState`] alone: the desktop's console mode reads it, `ImGui` does not.
    feeds_pad: bool,
    /// Whether the face buttons stand for the handheld's at the same places (the desktop: an
    /// `XInput` pad labels A the bottom button, a handheld the right one) rather than by label.
    positional: bool,
    pad: PadState,
}

impl Default for SdlPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl SdlPlatform {
    /// A platform whose coordinates are the window's.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            scale: [1.0, 1.0],
            ignore_pointer: false,
            gamepad: false,
            feeds_pad: true,
            positional: true,
            pad: PadState::new(),
        }
    }

    /// A platform for the screenshot capture: the cursor, the buttons and the wheel of the host
    /// never reach the interface.
    #[must_use]
    pub const fn for_capture() -> Self {
        Self {
            scale: [1.0, 1.0],
            ignore_pointer: true,
            gamepad: false,
            feeds_pad: false,
            positional: false,
            pad: PadState::new(),
        }
    }

    /// A platform for the handheld mode: no pointer, and the gamepad navigates the interface.
    #[must_use]
    pub const fn for_handheld() -> Self {
        Self {
            scale: [1.0, 1.0],
            ignore_pointer: true,
            gamepad: true,
            feeds_pad: true,
            positional: false,
            pad: PadState::new(),
        }
    }

    /// Declares the gamepad to `ImGui` when this platform feeds it: the navigation flag, and the
    /// backend flag that says the keys are driven.
    pub fn configure(&self, io: &mut Io) {
        if self.gamepad {
            io.config_flags |= ConfigFlags::NAV_ENABLE_GAMEPAD | ConfigFlags::NO_MOUSE;
            io.backend_flags |= BackendFlags::HAS_GAMEPAD;
        }
    }

    /// The gamepad as the handheld screens read it.
    #[must_use]
    pub const fn pad(&self) -> &PadState {
        &self.pad
    }

    /// The gamepad, to move its clock and clear its presses.
    pub const fn pad_mut(&mut self) -> &mut PadState {
        &mut self.pad
    }

    /// Passes one SDL event on.
    #[allow(clippy::cast_precision_loss)] // pixel coordinates are far below 2^24
    pub fn handle_event(&mut self, io: &mut Io, event: &Event) {
        if self.feeds_pad {
            self.handle_gamepad_event(io, event);
        }
        if self.ignore_pointer
            && matches!(
                event,
                Event::MouseMotion { .. }
                    | Event::MouseButtonDown { .. }
                    | Event::MouseButtonUp { .. }
                    | Event::MouseWheel { .. }
            )
        {
            return;
        }
        match event {
            Event::MouseMotion { x, y, .. } => {
                io.add_mouse_pos_event([*x as f32 * self.scale[0], *y as f32 * self.scale[1]]);
            }
            Event::MouseButtonDown { mouse_btn, .. } => button(io, *mouse_btn, true),
            Event::MouseButtonUp { mouse_btn, .. } => button(io, *mouse_btn, false),
            Event::MouseWheel {
                precise_x,
                precise_y,
                ..
            } => io.add_mouse_wheel_event([*precise_x, *precise_y]),
            Event::TextInput { text, .. } => {
                for character in text.chars() {
                    io.add_input_character(character);
                }
            }
            Event::KeyDown {
                scancode, keymod, ..
            } => key(io, *scancode, *keymod, true),
            Event::KeyUp {
                scancode, keymod, ..
            } => key(io, *scancode, *keymod, false),
            _ => {}
        }
    }

    /// A game-controller event: the button or the stick reaches the pad state and `ImGui`.
    fn handle_gamepad_event(&mut self, io: &mut Io, event: &Event) {
        match event {
            Event::ControllerButtonDown { button, .. } => self.press(io, *button, true),
            Event::ControllerButtonUp { button, .. } => self.press(io, *button, false),
            Event::ControllerAxisMotion { axis, value, .. } => {
                self.move_stick(io, *axis, *value);
            }
            _ => {}
        }
    }

    fn press(&mut self, io: &mut Io, button: Button, down: bool) {
        let Some((pad_button, key)) = pad_mapping(button) else {
            return;
        };
        let pad_button = if self.positional {
            at_handheld_place(pad_button)
        } else {
            pad_button
        };
        self.pad.set(pad_button, down);
        if let Some(key) = key.filter(|_| self.gamepad) {
            io.add_key_event(key, down);
        }
    }

    fn move_stick(&mut self, io: &mut Io, axis: Axis, value: i16) {
        let horizontal = match axis {
            Axis::LeftX => true,
            Axis::LeftY => false,
            Axis::TriggerLeft => {
                self.pad.set(PadButton::L2, value > i16::MAX / 2);
                return;
            }
            Axis::TriggerRight => {
                self.pad.set(PadButton::R2, value > i16::MAX / 2);
                return;
            }
            _ => return,
        };
        self.pad.set_axis(horizontal, value);
        if !self.gamepad {
            return;
        }
        let (negative, positive) = if horizontal {
            (Key::GamepadLStickLeft, Key::GamepadLStickRight)
        } else {
            (Key::GamepadLStickUp, Key::GamepadLStickDown)
        };
        let amount = f32::from(value) / f32::from(i16::MAX);
        io.add_key_analog_event(negative, amount < 0.0, (-amount).max(0.0));
        io.add_key_analog_event(positive, amount > 0.0, amount.max(0.0));
    }

    /// Sets the size of the frame to draw and the time since the previous one.
    #[allow(clippy::cast_precision_loss)] // window sizes are far below 2^24
    pub fn prepare_frame(
        &mut self,
        io: &mut Io,
        window: (u32, u32),
        drawable: (u32, u32),
        delta_seconds: f32,
    ) {
        io.display_size = [drawable.0 as f32, drawable.1 as f32];
        io.delta_time = delta_seconds.max(1.0e-5);
        if window.0 > 0 && window.1 > 0 {
            self.scale = [
                drawable.0 as f32 / window.0 as f32,
                drawable.1 as f32 / window.1 as f32,
            ];
        }
    }
}

/// The handheld button and the `ImGui` gamepad key a game-controller button stands for.
const fn pad_mapping(button: Button) -> Option<(PadButton, Option<Key>)> {
    Some(match button {
        Button::DPadUp => (PadButton::Up, Some(Key::GamepadDpadUp)),
        Button::DPadDown => (PadButton::Down, Some(Key::GamepadDpadDown)),
        Button::DPadLeft => (PadButton::Left, Some(Key::GamepadDpadLeft)),
        Button::DPadRight => (PadButton::Right, Some(Key::GamepadDpadRight)),
        Button::A => (PadButton::A, Some(Key::GamepadFaceDown)),
        Button::B => (PadButton::B, Some(Key::GamepadFaceRight)),
        Button::X => (PadButton::X, Some(Key::GamepadFaceLeft)),
        Button::Y => (PadButton::Y, Some(Key::GamepadFaceUp)),
        Button::LeftShoulder => (PadButton::L1, Some(Key::GamepadL1)),
        Button::RightShoulder => (PadButton::R1, Some(Key::GamepadR1)),
        Button::Back => (PadButton::Select, Some(Key::GamepadBack)),
        Button::Start => (PadButton::Start, Some(Key::GamepadStart)),
        Button::Guide => (PadButton::Menu, None),
        _ => return None,
    })
}

/// The handheld button at the place of a face button named `labelled` on an `XInput` pad: a
/// handheld has A on the right, B at the bottom, X on top and Y on the left.
const fn at_handheld_place(labelled: PadButton) -> PadButton {
    match labelled {
        PadButton::A => PadButton::B,
        PadButton::B => PadButton::A,
        PadButton::X => PadButton::Y,
        PadButton::Y => PadButton::X,
        other => other,
    }
}

fn button(io: &mut Io, button: SdlButton, down: bool) {
    let mapped = match button {
        SdlButton::Left => MouseButton::Left,
        SdlButton::Right => MouseButton::Right,
        SdlButton::Middle => MouseButton::Middle,
        SdlButton::X1 => MouseButton::Extra1,
        SdlButton::X2 => MouseButton::Extra2,
        SdlButton::Unknown => return,
    };
    io.add_mouse_button_event(mapped, down);
}

fn key(io: &mut Io, scancode: Option<Scancode>, modifiers: Mod, down: bool) {
    io.add_key_event(
        Key::ModCtrl,
        modifiers.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
    );
    io.add_key_event(
        Key::ModShift,
        modifiers.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
    );
    io.add_key_event(
        Key::ModAlt,
        modifiers.intersects(Mod::LALTMOD | Mod::RALTMOD),
    );
    io.add_key_event(
        Key::ModSuper,
        modifiers.intersects(Mod::LGUIMOD | Mod::RGUIMOD),
    );
    if let Some(mapped) = scancode.and_then(imgui_key) {
        io.add_key_event(mapped, down);
    }
}

/// The `ImGui` key for the keys the widgets read: navigation, editing and the clipboard letters.
fn imgui_key(scancode: Scancode) -> Option<Key> {
    Some(match scancode {
        Scancode::Tab => Key::Tab,
        Scancode::Left => Key::LeftArrow,
        Scancode::Right => Key::RightArrow,
        Scancode::Up => Key::UpArrow,
        Scancode::Down => Key::DownArrow,
        Scancode::PageUp => Key::PageUp,
        Scancode::PageDown => Key::PageDown,
        Scancode::Home => Key::Home,
        Scancode::End => Key::End,
        Scancode::Insert => Key::Insert,
        Scancode::Delete => Key::Delete,
        Scancode::Backspace => Key::Backspace,
        Scancode::Space => Key::Space,
        Scancode::Return => Key::Enter,
        Scancode::KpEnter => Key::KeypadEnter,
        Scancode::Escape => Key::Escape,
        Scancode::A => Key::A,
        Scancode::C => Key::C,
        Scancode::V => Key::V,
        Scancode::X => Key::X,
        Scancode::Y => Key::Y,
        Scancode::Z => Key::Z,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motion() -> Event {
        Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: sdl2::mouse::MouseState::from_sdl_state(0),
            x: 120,
            y: 80,
            xrel: 0,
            yrel: 0,
        }
    }

    fn press() -> Event {
        Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: SdlButton::Left,
            clicks: 1,
            x: 120,
            y: 80,
        }
    }

    /// The desktop's pad has the handheld's face buttons at their places (owner request): an
    /// `XInput` pad labels A the bottom button, a handheld the right one.
    #[test]
    fn the_desktop_face_buttons_are_the_handhelds_at_the_same_places() {
        assert_eq!(PadButton::A, at_handheld_place(PadButton::B));
        assert_eq!(PadButton::B, at_handheld_place(PadButton::A));
        assert_eq!(PadButton::X, at_handheld_place(PadButton::Y));
        assert_eq!(PadButton::Y, at_handheld_place(PadButton::X));
        assert_eq!(PadButton::Start, at_handheld_place(PadButton::Start));
        assert!(SdlPlatform::new().positional);
        assert!(!SdlPlatform::for_handheld().positional);
    }

    /// Dear `ImGui` allows one context per process, so the platforms are checked in turn.
    #[test]
    fn a_capture_platform_ignores_the_host_cursor_and_buttons() {
        let mut context = imgui::Context::create();
        context.io_mut().display_size = [800.0, 600.0];
        let _ = context.fonts().build_rgba32_texture();

        let mut capture = SdlPlatform::for_capture();
        capture.handle_event(context.io_mut(), &motion());
        capture.handle_event(context.io_mut(), &press());
        context.new_frame();
        // The pointer stays where Dear ImGui starts it: nowhere (far off screen).
        let position = context.io().mouse_pos;
        assert!(position[0] < -1.0e6 && position[1] < -1.0e6, "{position:?}");
        assert!(!context.io().mouse_down[0]);
        let _ = context.render();

        SdlPlatform::new().handle_event(context.io_mut(), &motion());
        context.new_frame();
        assert_eq!(context.io().mouse_pos, [120.0, 80.0]);
        let _ = context.render();

        // The handheld platform declares the gamepad to Dear ImGui and feeds the pad state; the
        // desktop one feeds the pad of its console mode only, and the capture one neither.
        let start = Event::ControllerButtonDown {
            timestamp: 0,
            which: 0,
            button: Button::Start,
        };
        let mut handheld = SdlPlatform::for_handheld();
        handheld.configure(context.io_mut());
        assert!(context
            .io()
            .config_flags
            .contains(ConfigFlags::NAV_ENABLE_GAMEPAD));
        assert!(context
            .io()
            .backend_flags
            .contains(BackendFlags::HAS_GAMEPAD));
        handheld.handle_event(context.io_mut(), &start);
        assert!(handheld.pad().is_held(PadButton::Start));
        context.io_mut().config_flags = ConfigFlags::empty();
        let mut desktop = SdlPlatform::new();
        desktop.configure(context.io_mut());
        assert!(!context
            .io()
            .config_flags
            .contains(ConfigFlags::NAV_ENABLE_GAMEPAD));
        desktop.handle_event(context.io_mut(), &start);
        assert!(desktop.pad().is_held(PadButton::Start));
        let mut capture = SdlPlatform::for_capture();
        capture.handle_event(context.io_mut(), &start);
        assert!(!capture.pad().is_held(PadButton::Start));
    }
    #[test]
    fn the_standard_buttons_reach_the_handheld_buttons_and_the_imgui_keys() {
        assert_eq!(
            Some((PadButton::A, Some(Key::GamepadFaceDown))),
            pad_mapping(Button::A)
        );
        assert_eq!(
            Some((PadButton::L1, Some(Key::GamepadL1))),
            pad_mapping(Button::LeftShoulder)
        );
        assert_eq!(Some((PadButton::Menu, None)), pad_mapping(Button::Guide));
        assert_eq!(None, pad_mapping(Button::LeftStick));
    }
}
