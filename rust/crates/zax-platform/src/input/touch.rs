//! The touch pad (Rust-only, owner request): a transparent pad drawn over the picture of a
//! touch screen, for a tablet with no gamepad.
//!
//! Its fingers drive an SDL virtual game controller, so the interface and the game see one more
//! gamepad and need nothing of their own. The controller exists only while the pad is shown: in
//! the automatic mode a real gamepad hides the pad, and the virtual one goes away with it, so
//! that the real one gets the first player when it is pressed.

use std::ffi::CString;
use std::ptr;
use std::sync::OnceLock;

use sdl2::event::Event;
use sdl2::sys;

/// The name SDL gives the virtual controller.
pub const TOUCH_PAD_NAME: &str = "Zaxnes touch pad";

/// Whether this build offers the pad: on Android, or with `ZAXNES_TOUCH_PAD=1` to try it on a
/// desktop, where the mouse touches it.
#[must_use]
pub fn offered() -> bool {
    static OFFERED: OnceLock<bool> = OnceLock::new();
    *OFFERED.get_or_init(|| {
        cfg!(target_os = "android") || std::env::var("ZAXNES_TOUCH_PAD").is_ok_and(|v| v == "1")
    })
}

/// What the screen shows of the pad: where its controls are, and which are pressed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPadView {
    pub layout: TouchLayout,
    pressed: [bool; 9],
}

impl TouchPadView {
    /// Whether `control` is pressed.
    #[must_use]
    pub const fn is_pressed(&self, control: TouchControl) -> bool {
        self.pressed[control.index()]
    }
}

/// When the pad is shown (`input.touch_pad`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPadMode {
    /// Never.
    Off,
    /// Always.
    On,
    /// While no real gamepad is connected.
    Auto,
}

impl TouchPadMode {
    /// The mode of the setting's value: 0 off, 1 on, anything else automatic.
    #[must_use]
    pub const fn from_setting(value: i32) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::On,
            _ => Self::Auto,
        }
    }
}

/// A control of the pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchControl {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Select,
    Start,
    Menu,
}

impl TouchControl {
    /// The controls, in the order of [`TouchPad::pressed`].
    pub const ALL: [Self; 9] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::A,
        Self::B,
        Self::Select,
        Self::Start,
        Self::Menu,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    /// The game-controller button it presses: A and B by their labels, as on a handheld.
    const fn button(self) -> sys::SDL_GameControllerButton {
        use sys::SDL_GameControllerButton as B;
        match self {
            Self::Up => B::SDL_CONTROLLER_BUTTON_DPAD_UP,
            Self::Down => B::SDL_CONTROLLER_BUTTON_DPAD_DOWN,
            Self::Left => B::SDL_CONTROLLER_BUTTON_DPAD_LEFT,
            Self::Right => B::SDL_CONTROLLER_BUTTON_DPAD_RIGHT,
            Self::A => B::SDL_CONTROLLER_BUTTON_A,
            Self::B => B::SDL_CONTROLLER_BUTTON_B,
            Self::Select => B::SDL_CONTROLLER_BUTTON_BACK,
            Self::Start => B::SDL_CONTROLLER_BUTTON_START,
            Self::Menu => B::SDL_CONTROLLER_BUTTON_GUIDE,
        }
    }
}

/// A round control: its centre and radius, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: [f32; 2],
    pub radius: f32,
}

impl Circle {
    fn contains(&self, point: [f32; 2], scale: f32) -> bool {
        distance(point, self.center) <= self.radius * scale
    }
}

/// A rectangular control, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Rect {
    fn contains(&self, point: [f32; 2], margin: f32) -> bool {
        point[0] >= self.min[0] - margin
            && point[0] <= self.max[0] + margin
            && point[1] >= self.min[1] - margin
            && point[1] <= self.max[1] + margin
    }
}

/// Where the controls are on a view: the cross bottom left, B and A bottom right, Select and
/// Start small at the bottom centre, the menu top right (owner's choice). Everything is sized by
/// the view's height, so that the pad keeps its size in hand whatever the ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchLayout {
    /// The cross: its centre, and the radius of its arms.
    pub dpad: Circle,
    pub a: Circle,
    pub b: Circle,
    pub select: Rect,
    pub start: Rect,
    pub menu: Circle,
}

/// How far from its drawing a button still takes a finger, as a factor of its radius.
const BUTTON_REACH: f32 = 1.25;
/// How far from its centre the cross takes a new finger, as a factor of its radius.
const DPAD_REACH: f32 = 1.5;
/// The part of the cross's radius around its centre that presses nothing.
const DPAD_DEAD_ZONE: f32 = 0.2;
/// sin 22.5°: a direction is pressed within 67.5° of its axis, so that the diagonals take the
/// eighths between the arms.
const DPAD_AXIS: f32 = 0.383;

impl TouchLayout {
    /// The layout on a view of `width` by `height` pixels.
    #[must_use]
    pub fn for_view(width: f32, height: f32) -> Self {
        let unit = height.min(width * 0.75);
        let dpad_radius = unit * 0.16;
        let button = unit * 0.085;
        let small = [unit * 0.14, unit * 0.055];
        let bottom = height - unit * 0.05;
        let pill = |center_x: f32| Rect {
            min: [center_x - small[0] * 0.5, bottom - small[1]],
            max: [center_x + small[0] * 0.5, bottom],
        };
        Self {
            dpad: Circle {
                center: [unit * 0.07 + dpad_radius, height - unit * 0.08 - dpad_radius],
                radius: dpad_radius,
            },
            a: Circle {
                center: [width - unit * 0.13, height - unit * 0.28],
                radius: button,
            },
            b: Circle {
                center: [width - unit * 0.34, height - unit * 0.17],
                radius: button,
            },
            select: pill(width * 0.5 - unit * 0.1),
            start: pill(width * 0.5 + unit * 0.1),
            menu: Circle {
                center: [width - unit * 0.08, unit * 0.08],
                radius: unit * 0.045,
            },
        }
    }

    /// Whether a finger put down at `point` takes the cross.
    fn takes_dpad(&self, point: [f32; 2]) -> bool {
        self.dpad.contains(point, DPAD_REACH)
    }

    /// The directions a finger of the cross at `point` presses: up, down, left, right.
    fn directions(&self, point: [f32; 2]) -> [bool; 4] {
        let offset = [point[0] - self.dpad.center[0], point[1] - self.dpad.center[1]];
        let length = (offset[0] * offset[0] + offset[1] * offset[1]).sqrt();
        if length < self.dpad.radius * DPAD_DEAD_ZONE {
            return [false; 4];
        }
        let (x, y) = (offset[0] / length, offset[1] / length);
        [y < -DPAD_AXIS, y > DPAD_AXIS, x < -DPAD_AXIS, x > DPAD_AXIS]
    }

    /// The button under a finger at `point`, if any.
    fn button_at(&self, point: [f32; 2]) -> Option<TouchControl> {
        let margin = self.select.max[1] - self.select.min[1];
        if self.a.contains(point, BUTTON_REACH) {
            Some(TouchControl::A)
        } else if self.b.contains(point, BUTTON_REACH) {
            Some(TouchControl::B)
        } else if self.select.contains(point, margin * 0.4) {
            Some(TouchControl::Select)
        } else if self.start.contains(point, margin * 0.4) {
            Some(TouchControl::Start)
        } else if self.menu.contains(point, 1.6) {
            Some(TouchControl::Menu)
        } else {
            None
        }
    }
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    let (x, y) = (a[0] - b[0], a[1] - b[1]);
    (x * x + y * y).sqrt()
}

/// What a finger was put down on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grip {
    /// The cross: the finger steers it wherever it goes, until it is lifted.
    Dpad,
    /// A button: the finger presses the button under it, and slides from one to another.
    Buttons,
    /// Neither, or the pad hidden: a tap there is a click of the interface.
    Free,
}

/// A finger on the screen.
#[derive(Debug, Clone, Copy)]
struct Finger {
    id: i64,
    point: [f32; 2],
    start: [f32; 2],
    grip: Grip,
}

/// What the pad presses from where its fingers are.
#[derive(Debug, Default)]
struct Fingers {
    fingers: Vec<Finger>,
    /// How far free fingers moved down since the last [`Fingers::take_scroll`], in pixels.
    scroll: f32,
}

impl Fingers {
    /// A finger put down at `point`; `pad` is the layout of the pad when it is shown.
    fn down(&mut self, pad: Option<&TouchLayout>, id: i64, point: [f32; 2]) {
        self.fingers.retain(|finger| finger.id != id);
        let grip = match pad {
            Some(layout) if layout.takes_dpad(point) => Grip::Dpad,
            Some(layout) if layout.button_at(point).is_some() => Grip::Buttons,
            _ => Grip::Free,
        };
        self.fingers.push(Finger {
            id,
            point,
            start: point,
            grip,
        });
    }

    fn moved(&mut self, id: i64, point: [f32; 2]) {
        if let Some(finger) = self.fingers.iter_mut().find(|finger| finger.id == id) {
            if finger.grip == Grip::Free {
                self.scroll += point[1] - finger.point[1];
            }
            finger.point = point;
        }
    }

    fn take_scroll(&mut self) -> f32 {
        std::mem::take(&mut self.scroll)
    }

    /// Lifts a finger: where it tapped, when it was free and stayed within `slop` pixels of
    /// where it went down (farther, it swiped).
    fn up(&mut self, id: i64, slop: f32) -> Option<[f32; 2]> {
        let index = self.fingers.iter().position(|finger| finger.id == id)?;
        let finger = self.fingers.remove(index);
        (finger.grip == Grip::Free && distance(finger.point, finger.start) <= slop)
            .then_some(finger.start)
    }

    fn pressed(&self, layout: &TouchLayout) -> [bool; 9] {
        let mut pressed = [false; 9];
        for finger in &self.fingers {
            match finger.grip {
                Grip::Dpad => {
                    let directions = layout.directions(finger.point);
                    for (slot, on) in pressed.iter_mut().zip(directions) {
                        *slot |= on;
                    }
                }
                Grip::Buttons => {
                    if let Some(control) = layout.button_at(finger.point) {
                        pressed[control.index()] = true;
                    }
                }
                Grip::Free => {}
            }
        }
        pressed
    }
}

/// The pad: its fingers, and the virtual controller they press while it is shown.
#[derive(Debug)]
pub struct TouchPad {
    mode: TouchPadMode,
    /// A game is on the screen: the pad is shown only then, a menu takes taps and swipes.
    playing: bool,
    view: [f32; 2],
    /// Drawable pixels per window coordinate, for the mouse.
    scale: [f32; 2],
    layout: TouchLayout,
    fingers: Fingers,
    pressed: [bool; 9],
    /// A real gamepad is connected (the automatic mode hides the pad).
    physical: bool,
    /// The virtual controller, open while the pad is shown.
    joystick: *mut sys::SDL_Joystick,
    /// The controller's name, kept for SDL.
    name: CString,
    /// A mouse stands for a finger (`ZAXNES_TOUCH_PAD=1` on a desktop, to try the pad).
    mouse_finger: bool,
    /// Where free fingers tapped since the last [`TouchPad::take_taps`], in drawable pixels.
    taps: Vec<[f32; 2]>,
}

/// The finger the mouse stands for.
const MOUSE_FINGER: i64 = -1;

impl TouchPad {
    /// A pad, hidden until its mode and view are known. `mouse_finger` lets the left button of
    /// the mouse touch it.
    #[must_use]
    pub fn new(mouse_finger: bool) -> Self {
        Self {
            mode: TouchPadMode::Off,
            playing: false,
            view: [0.0, 0.0],
            scale: [1.0, 1.0],
            layout: TouchLayout::for_view(0.0, 0.0),
            fingers: Fingers::default(),
            pressed: [false; 9],
            physical: physical_gamepad_connected(),
            joystick: ptr::null_mut(),
            name: CString::new(TOUCH_PAD_NAME).unwrap_or_default(),
            mouse_finger,
            taps: Vec::new(),
        }
    }

    /// Whether the pad is on the screen now.
    #[must_use]
    pub fn shown(&self) -> bool {
        self.playing
            && match self.mode {
                TouchPadMode::Off => false,
                TouchPadMode::On => true,
                TouchPadMode::Auto => !self.physical,
            }
    }

    /// What to draw of the pad; none while it is hidden.
    #[must_use]
    pub fn view(&self) -> Option<TouchPadView> {
        (!self.joystick.is_null()).then_some(TouchPadView {
            layout: self.layout,
            pressed: self.pressed,
        })
    }

    /// Where fingers tapped outside the pad since the last call, in drawable pixels: clicks of
    /// the interface's rows and hints.
    pub fn take_taps(&mut self) -> Vec<[f32; 2]> {
        std::mem::take(&mut self.taps)
    }

    /// How far fingers off the pad slid down since the last call, in drawable pixels (up is
    /// negative): the scroll of a menu.
    pub fn take_scroll(&mut self) -> f32 {
        self.fingers.take_scroll()
    }

    /// Takes the mode of the setting, whether a game is on the screen, and the sizes of the
    /// window and of its drawable, for this frame; the pad is laid out and drawn in drawable
    /// pixels.
    pub fn update(
        &mut self,
        mode: TouchPadMode,
        playing: bool,
        window: [f32; 2],
        drawable: [f32; 2],
    ) {
        self.mode = mode;
        self.playing = playing;
        let [width, height] = drawable;
        if window[0] > 0.0 && window[1] > 0.0 {
            self.scale = [width / window[0], height / window[1]];
        }
        if self.view != [width, height] {
            self.view = [width, height];
            self.layout = TouchLayout::for_view(width, height);
        }
        self.follow_visibility();
    }

    /// Takes one SDL event: the fingers, and the gamepads plugged or unplugged.
    pub fn handle_event(&mut self, event: &Event) {
        let (view, scale) = (self.view, self.scale);
        let pad = (!self.joystick.is_null()).then_some(&self.layout);
        // How far a tap may slide: a little of the view's height.
        let slop = view[1] * 0.03;
        let at = |x: f32, y: f32| [x * view[0], y * view[1]];
        #[allow(clippy::cast_precision_loss)] // pixel coordinates are far below 2^24
        let mouse = |x: i32, y: i32| [x as f32 * scale[0], y as f32 * scale[1]];
        match *event {
            Event::FingerDown {
                finger_id, x, y, ..
            } => self.fingers.down(pad, finger_id, at(x, y)),
            Event::FingerMotion {
                finger_id, x, y, ..
            } => self.fingers.moved(finger_id, at(x, y)),
            Event::FingerUp { finger_id, .. } => {
                self.taps.extend(self.fingers.up(finger_id, slop));
            }
            Event::MouseButtonDown {
                mouse_btn: sdl2::mouse::MouseButton::Left,
                x,
                y,
                ..
            } if self.mouse_finger => self.fingers.down(pad, MOUSE_FINGER, mouse(x, y)),
            Event::MouseMotion { x, y, .. } if self.mouse_finger => {
                self.fingers.moved(MOUSE_FINGER, mouse(x, y));
            }
            Event::MouseButtonUp {
                mouse_btn: sdl2::mouse::MouseButton::Left,
                ..
            } if self.mouse_finger => self.taps.extend(self.fingers.up(MOUSE_FINGER, slop)),
            Event::JoyDeviceAdded { .. } | Event::JoyDeviceRemoved { .. } => {
                self.physical = physical_gamepad_connected();
                return;
            }
            _ => return,
        }
        self.press(self.fingers.pressed(&self.layout));
    }

    /// Attaches the virtual controller when the pad appears, detaches it when it goes.
    fn follow_visibility(&mut self) {
        let shown = self.shown() && self.view[1] > 0.0;
        if shown && self.joystick.is_null() {
            self.attach();
        } else if !shown && !self.joystick.is_null() {
            self.fingers = Fingers::default();
            self.press([false; 9]);
            self.detach();
        }
    }

    fn press(&mut self, pressed: [bool; 9]) {
        if self.joystick.is_null() {
            self.pressed = [false; 9];
            return;
        }
        for control in TouchControl::ALL {
            let on = pressed[control.index()];
            if on != self.pressed[control.index()] {
                // SAFETY: the joystick is the open virtual controller of this pad.
                unsafe {
                    sys::SDL_JoystickSetVirtualButton(
                        self.joystick,
                        control.button() as i32,
                        u8::from(on),
                    );
                }
            }
        }
        self.pressed = pressed;
    }

    fn attach(&mut self) {
        // SAFETY: the description is zeroed as SDL asks, then filled; SDL copies the name.
        unsafe {
            let mut description: sys::SDL_VirtualJoystickDesc = std::mem::zeroed();
            #[allow(clippy::cast_possible_truncation)] // the version is 1
            {
                description.version = sys::SDL_VIRTUAL_JOYSTICK_DESC_VERSION as u16;
            }
            description.type_ = sys::SDL_JoystickType::SDL_JOYSTICK_TYPE_GAMECONTROLLER as u16;
            description.nbuttons = sys::SDL_GameControllerButton::SDL_CONTROLLER_BUTTON_MAX as u16;
            description.name = self.name.as_ptr();
            let index = sys::SDL_JoystickAttachVirtualEx(&description);
            if index < 0 {
                return;
            }
            self.joystick = sys::SDL_JoystickOpen(index);
        }
        self.pressed = [false; 9];
    }

    fn detach(&mut self) {
        // SAFETY: the joystick is the open virtual controller of this pad; it is closed once,
        // then its device is found by its instance and detached.
        unsafe {
            let instance = sys::SDL_JoystickInstanceID(self.joystick);
            sys::SDL_JoystickClose(self.joystick);
            self.joystick = ptr::null_mut();
            for index in 0..sys::SDL_NumJoysticks() {
                if sys::SDL_JoystickGetDeviceInstanceID(index) == instance {
                    sys::SDL_JoystickDetachVirtual(index);
                    break;
                }
            }
        }
    }
}

impl Drop for TouchPad {
    fn drop(&mut self) {
        if !self.joystick.is_null() {
            self.detach();
        }
    }
}

/// Whether a gamepad other than a virtual one (this pad's) is connected.
fn physical_gamepad_connected() -> bool {
    // SAFETY: plain queries of SDL's joystick list.
    unsafe {
        (0..sys::SDL_NumJoysticks())
            .any(|index| sys::SDL_JoystickIsVirtual(index) == sys::SDL_bool::SDL_FALSE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> TouchLayout {
        TouchLayout::for_view(1920.0, 1080.0)
    }

    #[test]
    fn the_cross_presses_eight_directions() {
        let layout = layout();
        let center = layout.dpad.center;
        let reach = layout.dpad.radius;
        let at = |x: f32, y: f32| layout.directions([center[0] + x * reach, center[1] + y * reach]);
        assert_eq!([true, false, false, false], at(0.0, -1.0));
        assert_eq!([false, false, false, true], at(1.0, 0.1));
        assert_eq!([false, true, true, false], at(-0.7, 0.7));
        assert_eq!([false; 4], at(0.05, 0.05));
    }

    #[test]
    fn a_finger_on_the_cross_keeps_it_and_another_presses_a() {
        let layout = layout();
        let mut fingers = Fingers::default();
        let dpad = layout.dpad.center;
        fingers.down(Some(&layout), 1, [dpad[0], dpad[1] - layout.dpad.radius]);
        fingers.down(Some(&layout), 2, layout.a.center);
        let pressed = fingers.pressed(&layout);
        assert!(pressed[TouchControl::Up.index()]);
        assert!(pressed[TouchControl::A.index()]);
        // The cross's finger, far away to the right, still steers the cross.
        fingers.moved(1, [dpad[0] + layout.dpad.radius * 4.0, dpad[1]]);
        let pressed = fingers.pressed(&layout);
        assert!(pressed[TouchControl::Right.index()]);
        assert!(!pressed[TouchControl::Up.index()]);
        // A finger sliding from A onto B lets go of A.
        fingers.moved(2, layout.b.center);
        let pressed = fingers.pressed(&layout);
        assert!(pressed[TouchControl::B.index()] && !pressed[TouchControl::A.index()]);
        assert_eq!(None, fingers.up(1, 10.0));
        assert_eq!(None, fingers.up(2, 10.0));
        assert_eq!([false; 9], fingers.pressed(&layout));
    }

    #[test]
    fn a_finger_off_the_pad_taps_and_presses_nothing() {
        let layout = layout();
        let mut fingers = Fingers::default();
        fingers.down(Some(&layout), 1, [960.0, 400.0]);
        // Sliding onto A does not press it: the finger went down off the pad.
        fingers.moved(1, layout.a.center);
        assert_eq!([false; 9], fingers.pressed(&layout));
        assert_eq!(None, fingers.up(1, 10.0));
        fingers.take_scroll();
        fingers.down(Some(&layout), 2, [960.0, 400.0]);
        fingers.moved(2, [964.0, 403.0]);
        assert_eq!(Some([960.0, 400.0]), fingers.up(2, 10.0));
        // The free fingers' slides are the menu's scroll; a long one is no tap.
        assert!((fingers.take_scroll() - 3.0).abs() < 1e-3);
        fingers.down(None, 4, [960.0, 400.0]);
        fingers.moved(4, [960.0, 250.0]);
        assert_eq!(None, fingers.up(4, 10.0));
        assert!((fingers.take_scroll() + 150.0).abs() < 1e-3);
        // With the pad hidden, a finger on A's place is a tap too.
        fingers.down(None, 3, layout.a.center);
        assert_eq!(Some(layout.a.center), fingers.up(3, 10.0));
    }

    #[test]
    fn select_start_and_the_menu_are_where_they_are_drawn() {
        let layout = layout();
        let middle = |rect: Rect| {
            [
                (rect.min[0] + rect.max[0]) * 0.5,
                (rect.min[1] + rect.max[1]) * 0.5,
            ]
        };
        assert_eq!(Some(TouchControl::Select), layout.button_at(middle(layout.select)));
        assert_eq!(Some(TouchControl::Start), layout.button_at(middle(layout.start)));
        assert_eq!(Some(TouchControl::Menu), layout.button_at(layout.menu.center));
        assert_eq!(None, layout.button_at([960.0, 400.0]));
        assert!(layout.select.max[0] < layout.start.min[0]);
    }

    #[test]
    fn the_modes_of_the_setting() {
        assert_eq!(TouchPadMode::Off, TouchPadMode::from_setting(0));
        assert_eq!(TouchPadMode::On, TouchPadMode::from_setting(1));
        assert_eq!(TouchPadMode::Auto, TouchPadMode::from_setting(2));
    }
}
