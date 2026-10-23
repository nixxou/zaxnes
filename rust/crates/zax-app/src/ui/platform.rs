//! Feeds the SDL events of the window to Dear `ImGui`: the pointer, the wheel, the typed text and
//! the keys the widgets use (the C# `ImGuiController` read them from `Silk.NET`). The upstream
//! `imgui-sdl2-support` crate needs `sdl2` 0.37 and the application uses 0.38, whose native
//! `links` key makes the two impossible to build together.

use imgui::{Io, Key, MouseButton};
use sdl2::event::Event;
use sdl2::keyboard::{Mod, Scancode};
use sdl2::mouse::MouseButton as SdlButton;

/// The state between events: how the window's coordinates scale to the drawable's.
#[derive(Debug, Clone, Copy)]
pub struct SdlPlatform {
    scale: [f32; 2],
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
        Self { scale: [1.0, 1.0] }
    }

    /// Passes one SDL event on.
    #[allow(clippy::cast_precision_loss)] // pixel coordinates are far below 2^24
    pub fn handle_event(&self, io: &mut Io, event: &Event) {
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
