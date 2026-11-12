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
    pressed: [bool; CONTROLS],
    /// The camera does not follow the fingers.
    pub camera_locked: bool,
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

/// The player's settings of the pad: its size and its vibrations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchOptions {
    /// The size of the controls, 1 the size they were made at (`input.touch_scale`).
    pub scale: f32,
    /// The vibration of the cross: 0 none, 1 light, 2 medium, 3 strong
    /// (`input.touch_haptic_dpad`).
    pub haptic_dpad: i32,
    /// The vibration of the buttons, likewise (`input.touch_haptic_buttons`).
    pub haptic_buttons: i32,
}

impl Default for TouchOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            haptic_dpad: 0,
            haptic_buttons: 0,
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
    /// The 2D picture or the 3D (Y in game).
    Flat,
    /// The camera back to its default view (X in game).
    Recenter,
    /// Locks the camera against the fingers, or frees it; not a button of the controller.
    Lock,
}

/// How many controls the pad has.
const CONTROLS: usize = 12;

impl TouchControl {
    /// The controls, in the order of [`TouchPad::pressed`].
    pub const ALL: [Self; CONTROLS] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::A,
        Self::B,
        Self::Select,
        Self::Start,
        Self::Menu,
        Self::Flat,
        Self::Recenter,
        Self::Lock,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    /// The game-controller button it presses: A and B by their labels, as on a handheld. The
    /// lock presses none.
    const fn button(self) -> Option<sys::SDL_GameControllerButton> {
        use sys::SDL_GameControllerButton as B;
        Some(match self {
            Self::Up => B::SDL_CONTROLLER_BUTTON_DPAD_UP,
            Self::Down => B::SDL_CONTROLLER_BUTTON_DPAD_DOWN,
            Self::Left => B::SDL_CONTROLLER_BUTTON_DPAD_LEFT,
            Self::Right => B::SDL_CONTROLLER_BUTTON_DPAD_RIGHT,
            Self::A => B::SDL_CONTROLLER_BUTTON_A,
            Self::B => B::SDL_CONTROLLER_BUTTON_B,
            Self::Select => B::SDL_CONTROLLER_BUTTON_BACK,
            Self::Start => B::SDL_CONTROLLER_BUTTON_START,
            Self::Menu => B::SDL_CONTROLLER_BUTTON_GUIDE,
            Self::Flat => B::SDL_CONTROLLER_BUTTON_Y,
            Self::Recenter => B::SDL_CONTROLLER_BUTTON_X,
            Self::Lock => return None,
        })
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

    /// The circle `scale` times as large, the point `anchor` (a corner or an edge of the view)
    /// staying where it is: the control keeps to its edge, its gap to it growing with it.
    fn scaled_from(self, anchor: [f32; 2], scale: f32) -> Self {
        Self {
            center: scale_point(self.center, anchor, scale),
            radius: self.radius * scale,
        }
    }
}

fn scale_point(point: [f32; 2], anchor: [f32; 2], scale: f32) -> [f32; 2] {
    [
        anchor[0] + (point[0] - anchor[0]) * scale,
        anchor[1] + (point[1] - anchor[1]) * scale,
    ]
}

/// A rectangular control, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Rect {
    fn scaled_from(self, anchor: [f32; 2], scale: f32) -> Self {
        Self {
            min: scale_point(self.min, anchor, scale),
            max: scale_point(self.max, anchor, scale),
        }
    }

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
    /// Top left: the 2D/3D switch, the camera's lock and its recentring.
    pub flat: Circle,
    pub lock: Circle,
    pub recenter: Circle,
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
                center: [
                    unit * 0.07 + dpad_radius,
                    height - unit * 0.08 - dpad_radius,
                ],
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
            flat: Circle {
                center: [unit * 0.08, unit * 0.08],
                radius: unit * 0.045,
            },
            lock: Circle {
                center: [unit * 0.2, unit * 0.08],
                radius: unit * 0.045,
            },
            recenter: Circle {
                center: [unit * 0.32, unit * 0.08],
                radius: unit * 0.045,
            },
        }
    }

    /// The layout on a view of `width` by `height` pixels at `scale` times the size: each control
    /// grows or shrinks from the corner or the edge of the view it keeps to — the cross from the
    /// bottom left corner, B and A from the bottom right one, Select and Start from the middle of
    /// the bottom edge, the menu from the top right corner, the three small buttons from the top
    /// left one.
    #[must_use]
    pub fn scaled(width: f32, height: f32, scale: f32) -> Self {
        let base = Self::for_view(width, height);
        let scale = scale.clamp(0.25, 3.0);
        let (top_left, top_right) = ([0.0, 0.0], [width, 0.0]);
        let (bottom_left, bottom_right) = ([0.0, height], [width, height]);
        let bottom_middle = [width * 0.5, height];
        Self {
            dpad: base.dpad.scaled_from(bottom_left, scale),
            a: base.a.scaled_from(bottom_right, scale),
            b: base.b.scaled_from(bottom_right, scale),
            select: base.select.scaled_from(bottom_middle, scale),
            start: base.start.scaled_from(bottom_middle, scale),
            menu: base.menu.scaled_from(top_right, scale),
            flat: base.flat.scaled_from(top_left, scale),
            lock: base.lock.scaled_from(top_left, scale),
            recenter: base.recenter.scaled_from(top_left, scale),
        }
    }

    /// Whether a finger put down at `point` takes the cross.
    fn takes_dpad(&self, point: [f32; 2]) -> bool {
        self.dpad.contains(point, DPAD_REACH)
    }

    /// The directions a finger of the cross at `point` presses: up, down, left, right.
    fn directions(&self, point: [f32; 2]) -> [bool; 4] {
        let offset = [
            point[0] - self.dpad.center[0],
            point[1] - self.dpad.center[1],
        ];
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
        } else if self.flat.contains(point, 1.3) {
            Some(TouchControl::Flat)
        } else if self.lock.contains(point, 1.3) {
            Some(TouchControl::Lock)
        } else if self.recenter.contains(point, 1.3) {
            Some(TouchControl::Recenter)
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

    /// Where the free fingers are, in the order they went down.
    fn free_points(&self) -> Vec<[f32; 2]> {
        self.fingers
            .iter()
            .filter(|finger| finger.grip == Grip::Free)
            .map(|finger| finger.point)
            .collect()
    }

    fn pressed(&self, layout: &TouchLayout) -> [bool; CONTROLS] {
        let mut pressed = [false; CONTROLS];
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
// Four independent facts of the frame (in game, locked, a real gamepad, the mouse as a finger).
#[allow(clippy::struct_excessive_bools)]
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
    pressed: [bool; CONTROLS],
    /// The camera does not follow the fingers while the pad is shown (on by default: the pad's
    /// fingers would turn it).
    camera_locked: bool,
    /// One free finger turning the camera, in drawable pixels.
    camera_finger: Option<[f32; 2]>,
    /// The distance between two free fingers pinching, last seen.
    pinch: Option<f32>,
    /// The zoom asked by pinching since the last [`TouchPad::take_zoom`], in wheel notches.
    zoom: f32,
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
    options: TouchOptions,
    /// The device's vibrator, opened at the first vibration asked for.
    vibrator: Vibrator,
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
            pressed: [false; CONTROLS],
            camera_locked: true,
            camera_finger: None,
            pinch: None,
            zoom: 0.0,
            physical: physical_gamepad_connected(),
            joystick: ptr::null_mut(),
            name: CString::new(TOUCH_PAD_NAME).unwrap_or_default(),
            mouse_finger,
            taps: Vec::new(),
            options: TouchOptions::default(),
            vibrator: Vibrator::default(),
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
            camera_locked: self.camera_locked,
        })
    }

    /// Whether this pad's fingers stand for the mouse's drag and wheel on the camera: on a touch
    /// screen, not on a desktop trying the pad with the mouse.
    #[must_use]
    pub const fn drives_pointer(&self) -> bool {
        !self.mouse_finger
    }

    /// The one free finger turning the camera, like a mouse drag; none while the camera is
    /// locked, in a menu, or with no finger or two.
    #[must_use]
    pub const fn camera_finger(&self) -> Option<[f32; 2]> {
        self.camera_finger
    }

    /// The zoom two free fingers asked by pinching since the last call, in wheel notches
    /// (positive: closer).
    pub fn take_zoom(&mut self) -> f32 {
        std::mem::take(&mut self.zoom)
    }

    /// The camera follows free fingers in game, unless the shown pad locks it.
    fn camera_free(&self) -> bool {
        self.playing && (self.joystick.is_null() || !self.camera_locked)
    }

    /// One free finger turns the camera; two pinch its zoom.
    fn follow_camera_fingers(&mut self) {
        let points = if self.camera_free() {
            self.fingers.free_points()
        } else {
            Vec::new()
        };
        self.camera_finger = (points.len() == 1).then(|| points[0]);
        if let [first, second] = points[..] {
            let spread = distance(first, second);
            if let Some(last) = self.pinch {
                // A notch per few hundredths of the view's height of spreading.
                self.zoom += (spread - last) / (self.view[1] * 0.04).max(1.0);
            }
            self.pinch = Some(spread);
        } else {
            self.pinch = None;
        }
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
        options: TouchOptions,
        playing: bool,
        window: [f32; 2],
        drawable: [f32; 2],
    ) {
        self.mode = mode;
        self.playing = playing;
        // Compared exactly: another size lays the pad out again.
        #[allow(clippy::float_cmp)]
        let rescaled = options.scale != self.options.scale;
        self.options = options;
        let [width, height] = drawable;
        if window[0] > 0.0 && window[1] > 0.0 {
            self.scale = [width / window[0], height / window[1]];
        }
        // The window's own sizes, compared exactly: a new size lays the pad out again.
        #[allow(clippy::float_cmp)]
        let resized = self.view != [width, height];
        if resized || rescaled {
            self.view = [width, height];
            self.layout = TouchLayout::scaled(width, height, self.options.scale);
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
        self.follow_camera_fingers();
    }

    /// Attaches the virtual controller when the pad appears, detaches it when it goes.
    fn follow_visibility(&mut self) {
        let shown = self.shown() && self.view[1] > 0.0;
        if shown && self.joystick.is_null() {
            self.attach();
        } else if !shown && !self.joystick.is_null() {
            self.fingers = Fingers::default();
            self.press([false; CONTROLS]);
            self.detach();
        }
        if !self.camera_free() {
            self.camera_finger = None;
            self.pinch = None;
        }
    }

    fn press(&mut self, pressed: [bool; CONTROLS]) {
        if self.joystick.is_null() {
            self.pressed = [false; CONTROLS];
            return;
        }
        let (dpad, buttons) = new_presses(&self.pressed, &pressed);
        if dpad {
            self.vibrator.pulse(self.options.haptic_dpad);
        }
        if buttons {
            self.vibrator.pulse(self.options.haptic_buttons);
        }
        for control in TouchControl::ALL {
            let on = pressed[control.index()];
            if on == self.pressed[control.index()] {
                continue;
            }
            match control.button() {
                // SAFETY: the joystick is the open virtual controller of this pad.
                Some(button) => unsafe {
                    sys::SDL_JoystickSetVirtualButton(self.joystick, button as i32, u8::from(on));
                },
                None if on => self.camera_locked = !self.camera_locked,
                None => {}
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
            let index = sys::SDL_JoystickAttachVirtualEx(&raw const description);
            if index < 0 {
                return;
            }
            self.joystick = sys::SDL_JoystickOpen(index);
        }
        self.pressed = [false; CONTROLS];
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

/// What a change of the pressed controls from `before` to `after` asks to vibrate: the cross
/// (another direction pressed, a diagonal included) and the buttons (one more pressed).
fn new_presses(before: &[bool; CONTROLS], after: &[bool; CONTROLS]) -> (bool, bool) {
    let directions = 4;
    let dpad = after[..directions].iter().any(|on| *on) && after[..directions] != before[..directions];
    let buttons = (directions..CONTROLS).any(|index| after[index] && !before[index]);
    (dpad, buttons)
}

/// The vibration of a level: its strength (0 to 1) and how long it lasts, in milliseconds.
const fn pulse_of(level: i32) -> Option<(f32, u32)> {
    match level {
        1 => Some((0.35, 12)),
        2 => Some((0.65, 20)),
        3 => Some((1.0, 30)),
        _ => None,
    }
}

/// The device's vibrator: SDL's first haptic device, which on Android is the phone's or the
/// tablet's own (SDL calls it `VIBRATOR_SERVICE`). Opened once, at the first pulse; none on a
/// device without one.
#[derive(Debug, Default)]
struct Vibrator {
    haptic: Option<*mut sys::SDL_Haptic>,
}

impl Vibrator {
    /// A short pulse at `level` (0 none, 1 light, 2 medium, 3 strong).
    fn pulse(&mut self, level: i32) {
        let Some((strength, length)) = pulse_of(level) else {
            return;
        };
        let haptic = *self.haptic.get_or_insert_with(open_vibrator);
        if haptic.is_null() {
            return;
        }
        // SAFETY: an open haptic device whose rumble was initialised.
        unsafe {
            sys::SDL_HapticRumblePlay(haptic, strength, length);
        }
    }
}

impl Drop for Vibrator {
    fn drop(&mut self) {
        if let Some(haptic) = self.haptic.filter(|haptic| !haptic.is_null()) {
            // SAFETY: the device this vibrator opened, closed once.
            unsafe { sys::SDL_HapticClose(haptic) };
        }
    }
}

/// Opens the first haptic device for rumbling; null when there is none or it cannot rumble.
fn open_vibrator() -> *mut sys::SDL_Haptic {
    // SAFETY: SDL's haptic subsystem, started then queried; the device is closed by `Vibrator`.
    unsafe {
        if sys::SDL_InitSubSystem(sys::SDL_INIT_HAPTIC) != 0 || sys::SDL_NumHaptics() < 1 {
            return ptr::null_mut();
        }
        let haptic = sys::SDL_HapticOpen(0);
        if haptic.is_null() {
            return haptic;
        }
        if sys::SDL_HapticRumbleInit(haptic) != 0 {
            sys::SDL_HapticClose(haptic);
            return ptr::null_mut();
        }
        haptic
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
        assert_eq!([false; CONTROLS], fingers.pressed(&layout));
    }

    #[test]
    fn a_finger_off_the_pad_taps_and_presses_nothing() {
        let layout = layout();
        let mut fingers = Fingers::default();
        fingers.down(Some(&layout), 1, [960.0, 400.0]);
        // Sliding onto A does not press it: the finger went down off the pad.
        fingers.moved(1, layout.a.center);
        assert_eq!([false; CONTROLS], fingers.pressed(&layout));
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
                f32::midpoint(rect.min[0], rect.max[0]),
                f32::midpoint(rect.min[1], rect.max[1]),
            ]
        };
        assert_eq!(
            Some(TouchControl::Select),
            layout.button_at(middle(layout.select))
        );
        assert_eq!(
            Some(TouchControl::Start),
            layout.button_at(middle(layout.start))
        );
        assert_eq!(
            Some(TouchControl::Menu),
            layout.button_at(layout.menu.center)
        );
        assert_eq!(
            Some(TouchControl::Flat),
            layout.button_at(layout.flat.center)
        );
        assert_eq!(
            Some(TouchControl::Lock),
            layout.button_at(layout.lock.center)
        );
        assert_eq!(
            Some(TouchControl::Recenter),
            layout.button_at(layout.recenter.center)
        );
        assert_eq!(None, layout.button_at([960.0, 400.0]));
        assert!(layout.select.max[0] < layout.start.min[0]);
    }

    #[test]
    fn a_larger_pad_keeps_each_control_to_its_corner() {
        let (width, height) = (1920.0, 1080.0);
        let normal = TouchLayout::scaled(width, height, 1.0);
        assert_eq!(normal, layout());
        let large = TouchLayout::scaled(width, height, 1.5);
        // The cross grows from the bottom left corner: its gaps to both edges grow with it.
        let gap = |circle: Circle| {
            [
                circle.center[0] - circle.radius,
                height - circle.center[1] - circle.radius,
            ]
        };
        let (before, after) = (gap(normal.dpad), gap(large.dpad));
        assert!((after[0] - before[0] * 1.5).abs() < 1e-3);
        assert!((after[1] - before[1] * 1.5).abs() < 1e-3);
        assert!((large.dpad.radius - normal.dpad.radius * 1.5).abs() < 1e-3);
        // A from the bottom right one, the menu from the top right, Select from the bottom middle.
        assert!(large.a.center[0] < normal.a.center[0] && large.a.center[1] < normal.a.center[1]);
        assert!(large.menu.center[0] < normal.menu.center[0]);
        assert!(large.menu.center[1] > normal.menu.center[1]);
        assert!(large.select.min[0] < normal.select.min[0]);
        assert!((large.select.max[1] - height).abs() > (normal.select.max[1] - height).abs());
        // Smaller, the other way; and the pad still answers where it is drawn.
        let small = TouchLayout::scaled(width, height, 0.6);
        assert!(small.dpad.center[1] > normal.dpad.center[1]);
        assert_eq!(Some(TouchControl::A), small.button_at(small.a.center));
        assert_eq!(Some(TouchControl::B), large.button_at(large.b.center));
    }

    #[test]
    fn a_new_press_vibrates_its_part_of_the_pad() {
        let mut before = [false; CONTROLS];
        let mut after = before;
        after[TouchControl::Up.index()] = true;
        assert_eq!((true, false), new_presses(&before, &after));
        before = after;
        // Held: nothing more; a diagonal is a new direction.
        assert_eq!((false, false), new_presses(&before, &after));
        after[TouchControl::Right.index()] = true;
        assert_eq!((true, false), new_presses(&before, &after));
        before = after;
        after[TouchControl::A.index()] = true;
        assert_eq!((false, true), new_presses(&before, &after));
        // Letting go vibrates nothing.
        assert_eq!((false, false), new_presses(&after, &[false; CONTROLS]));
        assert_eq!(None, pulse_of(0));
        assert!(pulse_of(3).is_some_and(|(strength, _)| (strength - 1.0).abs() < 1e-6));
    }

    #[test]
    fn the_modes_of_the_setting() {
        assert_eq!(TouchPadMode::Off, TouchPadMode::from_setting(0));
        assert_eq!(TouchPadMode::On, TouchPadMode::from_setting(1));
        assert_eq!(TouchPadMode::Auto, TouchPadMode::from_setting(2));
    }
}
