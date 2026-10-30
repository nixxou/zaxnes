//! The gamepad as the handheld interface reads it: which buttons are down, which went down this
//! frame, and the auto-repeat of the directions. Pure data, fed by the platform layer from the
//! SDL game-controller events.

/// A button of the standard gamepad the handheld interface uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadButton {
    /// D-pad or left stick up.
    Up,
    /// D-pad or left stick down.
    Down,
    /// D-pad or left stick left.
    Left,
    /// D-pad or left stick right.
    Right,
    /// South face button: accept.
    A,
    /// East face button: back.
    B,
    /// West face button: secondary action.
    X,
    /// North face button: secondary action.
    Y,
    /// Left shoulder.
    L1,
    /// Right shoulder.
    R1,
    /// Select / Back.
    Select,
    /// Start.
    Start,
    /// The menu (guide) button.
    Menu,
    /// The left trigger, pressed past half way.
    L2,
    /// The right trigger, pressed past half way.
    R2,
}

impl PadButton {
    const COUNT: usize = 15;

    pub const ALL: [Self; Self::COUNT] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
        Self::L1,
        Self::R1,
        Self::Select,
        Self::Start,
        Self::Menu,
        Self::L2,
        Self::R2,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    const fn repeats(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }
}

/// Seconds a direction is held before it starts to repeat.
pub const REPEAT_DELAY: f32 = 0.4;
/// Seconds between two repeats.
pub const REPEAT_INTERVAL: f32 = 0.09;
/// Seconds a direction is held before its repeats speed up (owner request): from then on they
/// come faster and faster, several a frame in the end, to go through a long list.
pub const ACCELERATE_AFTER: f32 = 4.0;
/// How much faster the repeats come for each second past [`ACCELERATE_AFTER`].
const ACCELERATION: f32 = 2.0;
/// The fastest they come: this many times [`REPEAT_INTERVAL`]'s rate.
const MAX_SPEED: f32 = 40.0;
/// Stick deflection (of 32767) that counts as a direction pressed.
pub const STICK_PRESS: i16 = 16_000;
/// Stick deflection under which a direction held by the stick is released.
pub const STICK_RELEASE: i16 = 8_000;

/// The buttons held and pressed.
#[derive(Debug, Clone, Copy, Default)]
pub struct PadState {
    held: [bool; PadButton::COUNT],
    /// How many times each button went down or repeated this frame.
    pressed: [u8; PadButton::COUNT],
    held_for: [f32; PadButton::COUNT],
    /// When, in seconds held, each direction repeats next.
    next_repeat: [f32; PadButton::COUNT],
    stick: [Option<PadButton>; 2],
}

impl PadState {
    /// A pad with nothing pressed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            held: [false; PadButton::COUNT],
            pressed: [0; PadButton::COUNT],
            held_for: [0.0; PadButton::COUNT],
            next_repeat: [REPEAT_DELAY; PadButton::COUNT],
            stick: [None; 2],
        }
    }

    /// A button goes down or up.
    pub const fn set(&mut self, button: PadButton, down: bool) {
        let index = button.index();
        if down && !self.held[index] {
            self.pressed[index] = self.pressed[index].saturating_add(1);
            self.held_for[index] = 0.0;
            self.next_repeat[index] = REPEAT_DELAY;
        }
        self.held[index] = down;
    }

    /// A stick axis moves: the horizontal one when `horizontal`, the vertical one otherwise. The
    /// stick presses a direction past [`STICK_PRESS`] and lets it go under [`STICK_RELEASE`], so
    /// a stick resting near the threshold does not flicker.
    pub fn set_axis(&mut self, horizontal: bool, value: i16) {
        let slot = usize::from(!horizontal);
        let magnitude = value.saturating_abs();
        let direction = match (horizontal, value < 0) {
            (true, true) => PadButton::Left,
            (true, false) => PadButton::Right,
            (false, true) => PadButton::Up,
            (false, false) => PadButton::Down,
        };
        if let Some(active) = self.stick[slot] {
            if magnitude < STICK_RELEASE || active != direction {
                self.set(active, false);
                self.stick[slot] = None;
            }
        }
        if self.stick[slot].is_none() && magnitude >= STICK_PRESS {
            self.set(direction, true);
            self.stick[slot] = Some(direction);
        }
    }

    /// Whether `button` is down.
    #[must_use]
    pub const fn is_held(&self, button: PadButton) -> bool {
        self.held[button.index()]
    }

    /// Whether `button` went down this frame, or repeats on this frame.
    #[must_use]
    pub const fn was_pressed(&self, button: PadButton) -> bool {
        self.pressed[button.index()] > 0
    }

    /// The buttons that went down (or repeat) this frame, in the order of [`PadButton`]: a
    /// direction held long enough repeats several times a frame, once for each.
    pub fn presses(&self) -> impl Iterator<Item = PadButton> + '_ {
        PadButton::ALL.into_iter().flat_map(|button| {
            std::iter::repeat_n(button, usize::from(self.pressed[button.index()]))
        })
    }

    /// Moves the clock of the held directions on by `seconds`, which makes them repeat: every
    /// [`REPEAT_INTERVAL`] after [`REPEAT_DELAY`], faster and faster past [`ACCELERATE_AFTER`].
    pub fn advance(&mut self, seconds: f32) {
        for index in 0..PadButton::COUNT {
            if !self.held[index] {
                continue;
            }
            self.held_for[index] += seconds;
            if !PadButton::ALL[index].repeats() {
                continue;
            }
            while self.held_for[index] >= self.next_repeat[index] {
                self.pressed[index] = self.pressed[index].saturating_add(1);
                self.next_repeat[index] += repeat_interval(self.next_repeat[index]);
            }
        }
    }

    /// Forgets the presses of the frame that ends.
    pub const fn end_frame(&mut self) {
        self.pressed = [0; PadButton::COUNT];
    }
}

/// The time to the next repeat of a direction held `held` seconds.
fn repeat_interval(held: f32) -> f32 {
    let speed = (1.0 + (held - ACCELERATE_AFTER).max(0.0) * ACCELERATION).min(MAX_SPEED);
    REPEAT_INTERVAL / speed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_is_seen_for_one_frame_only() {
        let mut pad = PadState::new();
        pad.set(PadButton::A, true);
        assert!(pad.was_pressed(PadButton::A) && pad.is_held(PadButton::A));
        pad.end_frame();
        assert!(!pad.was_pressed(PadButton::A) && pad.is_held(PadButton::A));
        pad.set(PadButton::A, true);
        assert!(
            !pad.was_pressed(PadButton::A),
            "a held button is not pressed again"
        );
    }

    #[test]
    fn a_direction_repeats_after_the_delay_and_a_button_does_not() {
        let mut pad = PadState::new();
        pad.set(PadButton::Down, true);
        pad.set(PadButton::A, true);
        pad.end_frame();
        let mut downs = 0;
        let mut accepts = 0;
        for _ in 0..100 {
            pad.advance(0.01);
            downs += u32::from(pad.was_pressed(PadButton::Down));
            accepts += u32::from(pad.was_pressed(PadButton::A));
            pad.end_frame();
        }
        // One second held: the first repeat at 0.4 s, then one every 0.09 s.
        assert!((6..=8).contains(&downs), "{downs}");
        assert_eq!(0, accepts);
    }

    #[test]
    fn a_direction_held_long_speeds_up() {
        let mut pad = PadState::new();
        pad.set(PadButton::Down, true);
        pad.end_frame();
        let mut presses_in = |seconds: f32| {
            let mut count = 0;
            let mut left = seconds;
            while left > 0.0 {
                pad.advance(1.0 / 60.0);
                count += pad.presses().count();
                pad.end_frame();
                left -= 1.0 / 60.0;
            }
            count
        };
        // The first seconds at the steady rate (11 a second), then faster and faster.
        let steady = presses_in(ACCELERATE_AFTER);
        let faster = presses_in(2.0);
        let fastest = presses_in(2.0);
        assert!(steady > 35 && steady < 45, "{steady}");
        assert!(faster > 22 * 2, "{faster}");
        assert!(fastest > faster, "{fastest} {faster}");
    }

    #[test]
    fn the_stick_presses_with_hysteresis() {
        let mut pad = PadState::new();
        pad.set_axis(false, -20_000);
        assert!(pad.is_held(PadButton::Up));
        pad.set_axis(false, -10_000);
        assert!(
            pad.is_held(PadButton::Up),
            "between the two thresholds it holds"
        );
        pad.set_axis(false, -4_000);
        assert!(!pad.is_held(PadButton::Up));
        pad.set_axis(true, 30_000);
        assert!(pad.is_held(PadButton::Right));
        pad.set_axis(true, -30_000);
        assert!(!pad.is_held(PadButton::Right) && pad.is_held(PadButton::Left));
    }
}
