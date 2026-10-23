//! Turns raw stick and trigger values into the 0-to-1 values the digital threshold of
//! SPEC-08 REQ-INP-030 is applied to, with the dead zones of REQ-INP-031 (DEC-08-8).
//!
//! Sticks get a **radial** dead zone: the magnitude of the two axes together decides whether
//! the stick is at rest, so a diagonal push is treated the same as an axis-aligned one.
//!
//! AMD-021 R2-011 settles what the 0.34 of REQ-INP-030 is measured on, and **replaces AMD-020
//! for Q-048**: the threshold applies to the **shaped** value of REQ-INP-031 — radial dead zone
//! 0.2 and saturation 0.9 for a stick, dead zone 0.2 and saturation 0.9 for a trigger, with a
//! linear re-spread between the two. The press decision therefore reads exactly the same number
//! an analog consumer reads, which is what closes Q-056. R2-011 also amends the trigger dead
//! zone of REQ-INP-031 from 0.1 to 0.2. None of these values is a user setting.

/// The digital threshold of REQ-INP-030: an analog command bound to an action is pressed from
/// 0.34 of its **shaped** travel (AMD-021 R2-011).
pub const PRESS_THRESHOLD: f32 = 0.34;
/// Radial dead zone of a stick, below which the value is zero (REQ-INP-031).
pub const STICK_DEAD_ZONE: f32 = 0.2;
/// Radial saturation of a stick, above which the value is one (REQ-INP-031).
pub const STICK_SATURATION: f32 = 0.9;
/// Dead zone of a trigger (REQ-INP-031 amended by AMD-021 R2-011).
pub const TRIGGER_DEAD_ZONE: f32 = 0.2;
/// Saturation of a trigger (AMD-021 R2-011).
pub const TRIGGER_SATURATION: f32 = 0.9;

/// The travel of the half-axis `direction` (+1 positive, −1 negative) of a stick whose two raw
/// axes are `raw` and `other` (−1 to 1), in 0 to 1: the shaped travel of that half (R2-011),
/// or 0 when the stick is inside its radial dead zone.
// R2-011 (supersedes AMD-020 / Q-048), Q-054
#[must_use]
pub fn half_axis(raw: f32, other: f32, direction: i32) -> f32 {
    let shaped = stick(raw, other);
    if direction < 0 {
        (-shaped).max(0.0)
    } else {
        shaped.max(0.0)
    }
}

/// The **shaped** value of one axis of a stick, −1 to 1, after the radial dead zone and the
/// saturation of REQ-INP-031: the magnitude is re-spread linearly between the two bounds and
/// the direction is kept.
#[must_use]
pub fn stick(raw: f32, other: f32) -> f32 {
    let magnitude = magnitude(raw, other);
    if magnitude <= STICK_DEAD_ZONE {
        return 0.0;
    }

    if magnitude >= STICK_SATURATION {
        // At or past saturation the direction is kept but the length becomes exactly 1.
        return raw / magnitude;
    }

    let rescaled = (magnitude - STICK_DEAD_ZONE) / (STICK_SATURATION - STICK_DEAD_ZONE);
    raw / magnitude * rescaled
}

/// The shaped value of the half-axis `direction`, 0 to 1. Since R2-011 this is the value the
/// press decision reads, so it is [`half_axis`] itself.
#[must_use]
pub fn shaped_half_axis(raw: f32, other: f32, direction: i32) -> f32 {
    half_axis(raw, other, direction)
}

/// The travel of a trigger, 0 to 1: its shaped travel, the dead zone and the saturation of
/// R2-011 re-spread linearly over the whole range. `raw` is the raw travel, 0 to 1.
// R2-011 (supersedes AMD-020 / Q-048), Q-054
#[must_use]
pub fn trigger(raw: f32) -> f32 {
    if raw <= TRIGGER_DEAD_ZONE {
        return 0.0;
    }

    if raw >= TRIGGER_SATURATION {
        return 1.0;
    }

    (raw - TRIGGER_DEAD_ZONE) / (TRIGGER_SATURATION - TRIGGER_DEAD_ZONE)
}

/// The shaped value of a trigger, 0 to 1. Since R2-011 this is [`trigger`] itself.
#[must_use]
pub fn shaped_trigger(raw: f32) -> f32 {
    trigger(raw)
}

/// Whether an analog travel counts as pressed (REQ-INP-030): 0.34 or more of the shaped value
/// (AMD-021 R2-011).
#[must_use]
pub fn is_pressed(travel: f32) -> bool {
    travel >= PRESS_THRESHOLD
}

fn magnitude(raw: f32, other: f32) -> f32 {
    ((raw * raw) + (other * other)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(actual: f32, expected: f32, tolerance: f32) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} vs {expected}"
        );
    }

    /// REQ-INP-030 — AMD-021 R2-011 (Q-048, Q-058): the threshold is 0.34 of the shaped value.
    #[test]
    fn req_inp_030_the_digital_threshold_is_zero_point_thirty_four() {
        assert!((PRESS_THRESHOLD - 0.34).abs() < f32::EPSILON);
        assert!(!is_pressed(0.339_99));
        assert!(is_pressed(0.34));

        near(half_axis(-0.30, 0.0, -1), 0.142_857, 1e-5);
        near(half_axis(-0.40, 0.0, -1), 0.285_714, 1e-5);

        // The exact bound is 0.438 of raw travel (0.2 + 0.34 × 0.7).
        for travel in [0.30, 0.34, 0.40, 0.4379] {
            assert!(
                !is_pressed(half_axis(-travel, 0.0, -1)),
                "{travel} of travel must stay released"
            );
        }
        assert!(is_pressed(half_axis(-0.4381, 0.0, -1)));
        assert!(is_pressed(half_axis(-0.60, 0.0, -1)));

        // A trigger at 0.5 of travel shapes to 0.429, so the action stays pressed.
        assert!(is_pressed(trigger(0.5)));
    }

    /// AMD-021 R2-011, third vector: a stick held at 45° with a travel of 0.6 shapes to
    /// components of 0.404, so both directions are pressed.
    #[test]
    fn req_inp_030_a_diagonal_push_of_zero_point_six_presses_both_directions() {
        let component = 0.6 / 2f32.sqrt();
        near(half_axis(component, component, 1), 0.404, 1e-3);
        assert!(is_pressed(half_axis(component, component, 1)));
    }

    /// REQ-INP-031 as amended by AMD-021 R2-011: radial dead zone 0.2, saturation 0.9, linear
    /// re-spread in between, and a trigger dead zone of 0.2.
    #[test]
    fn req_inp_031_the_dead_zones_are_the_documented_ones() {
        assert!((STICK_DEAD_ZONE - 0.2).abs() < f32::EPSILON);
        assert!((STICK_SATURATION - 0.9).abs() < f32::EPSILON);
        assert!((TRIGGER_DEAD_ZONE - 0.2).abs() < f32::EPSILON);
        assert!((TRIGGER_SATURATION - 0.9).abs() < f32::EPSILON);

        assert!(trigger(0.2).abs() < f32::EPSILON);
        near(trigger(0.5), 0.428_571, 1e-5);
        near(trigger(0.55), 0.5, 1e-4);

        // A drift of 0.15 is inside the dead zone: no action at all.
        assert!(stick(0.15, 0.0).abs() < f32::EPSILON);
        assert!(half_axis(0.15, 0.0, 1).abs() < f32::EPSILON);

        // At saturation the value is exactly 1.
        near(stick(0.9, 0.0), 1.0, 1e-5);
        near(stick(1.0, 0.0), 1.0, 1e-5);

        // Half-way between the two bounds the re-spread gives one half.
        near(stick(0.55, 0.0), 0.5, 1e-4);

        // The dead zone is radial: a diagonal push of the same magnitude behaves the same.
        let diagonal = 0.55 / 2f32.sqrt();
        let shaped = stick(diagonal, diagonal);
        near(((shaped * shaped) + (shaped * shaped)).sqrt(), 0.5, 1e-3);

        assert!(trigger(0.1).abs() < f32::EPSILON);
        near(trigger(1.0), 1.0, 1e-5);
    }

    #[test]
    fn the_shaped_values_are_the_press_values() {
        assert!((shaped_half_axis(0.5, 0.0, 1) - half_axis(0.5, 0.0, 1)).abs() < f32::EPSILON);
        assert!((shaped_trigger(0.5) - trigger(0.5)).abs() < f32::EPSILON);
    }
}
