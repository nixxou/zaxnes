//! Port of `InputBindingTests.cs`: the desktop host layer of SPEC-08 §3.2 to §3.6 — the default
//! tables, the dead zones and the 0.34 threshold, the gamepad assignment, the remapping rules
//! and the persistence of the bindings (REQ-INP-121).
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code
#![allow(clippy::float_cmp)] // exact values by construction

mod common;

use common::{FakeGamepad, FakeGamepadHub, FakeKeyboard, FakePointer, TemporaryRoot};
use rstest::rstest;
use time::OffsetDateTime;
use zax_app::emulation::commands::Command;
use zax_app::input::binding_target::{BindingDeviceKind, BindingTarget};
use zax_app::input::gamepad_assignment::{GamepadAssignment, STARTUP_DELAY_SECONDS};
use zax_app::input::input_action::{self, InputAction};
use zax_app::input::rebinding_session::{PadAccess, RebindingSession};
use zax_app::input::{
    default_bindings, DesktopInputSource, DeviceRefs, InputBindings, InputSnapshot,
};
use zax_app::persistence::input_binding_store::{
    FileInputBindingStore, InputBindingStore, UNREADABLE_MESSAGE,
};
use zax_app::user_data_paths::UserDataPaths;
use zax_platform::input::devices::is_virtual_reality;
use zax_platform::input::{calibration, standard_gamepad as pad_map};
use zax_platform::input::{GamepadDevice, GamepadHub, Key, PhysicalControl};

fn key(key: Key) -> PhysicalControl {
    PhysicalControl::key(key.code())
}

fn sample(
    source: &mut DesktopInputSource,
    keyboard: Option<&FakeKeyboard>,
    hub: &mut FakeGamepadHub,
) -> InputSnapshot {
    let mut pointer = FakePointer::default();
    source.sample(
        0.016,
        DeviceRefs {
            keyboard: keyboard.map(|keyboard| keyboard as &dyn zax_platform::input::KeyboardDevice),
            pointer: Some(&mut pointer),
            hub,
        },
    )
}

fn device_identity(
    pads: &GamepadAssignment,
    hub: &dyn GamepadHub,
    player: usize,
) -> Option<String> {
    pads.device(hub, player)
        .map(|device| device.identity().to_owned())
}

/// REQ-INP-011 and §4.2: the default keyboard of player 1 is the table of §4.2 and the one of
/// player 2 is empty.
#[test]
fn req_inp_011_the_default_keyboards_are_the_table_and_an_empty_set() {
    let bindings = InputBindings::new();
    let first = bindings.keyboard(0);

    for (action, expected) in [
        (InputAction::LEFT, Key::Left),
        (InputAction::RIGHT, Key::Right),
        (InputAction::UP, Key::Up),
        (InputAction::DOWN, Key::Down),
        (InputAction::A, Key::X),
        (InputAction::B, Key::Z),
        (InputAction::SELECT, Key::Space),
        (InputAction::START, Key::Enter),
        (InputAction::ROT_LEFT, Key::F5),
        (InputAction::ROT_RIGHT, Key::F6),
        (InputAction::ROT_UP, Key::F7),
        (InputAction::ROT_DOWN, Key::F8),
        (InputAction::ZOOM_IN, Key::F9),
        (InputAction::ZOOM_OUT, Key::F10),
        (InputAction::MENU, Key::Escape),
        (InputAction::ROLL_BACK, Key::Comma),
        (InputAction::FAST_FORWARD, Key::Period),
    ] {
        assert_eq!(first.get_action(action), key(expected), "{action:?}");
    }

    // SPEC-08 §4.2: QuickSave and QuickLoad have no default key.
    assert!(!first.get_action(InputAction::QUICK_SAVE).is_bound());
    assert!(!first.get_action(InputAction::QUICK_LOAD).is_bound());

    assert_eq!(bindings.keyboard(1).count(), 0);
}

/// REQ-INP-020 and §4.3: the default gamepad table, with the hat on the directions, A on the
/// south button, B on the west one, `RollBack` on north, `FastForward` on east and the right stick
/// on the rotations.
#[test]
fn req_inp_020_the_default_gamepad_set_is_the_table_of_section_4_3() {
    let pad = default_bindings::create_gamepad(true);
    let button = PhysicalControl::button;

    for (action, expected) in [
        (InputAction::LEFT, button(pad_map::D_PAD_LEFT)),
        (InputAction::RIGHT, button(pad_map::D_PAD_RIGHT)),
        (InputAction::UP, button(pad_map::D_PAD_UP)),
        (InputAction::DOWN, button(pad_map::D_PAD_DOWN)),
        (InputAction::A, button(pad_map::SOUTH)),
        (InputAction::B, button(pad_map::WEST)),
        (InputAction::ROLL_BACK, button(pad_map::NORTH)),
        (InputAction::FAST_FORWARD, button(pad_map::EAST)),
        (InputAction::SELECT, button(pad_map::BACK)),
        (InputAction::START, button(pad_map::START)),
        (InputAction::MENU, button(pad_map::RIGHT_STICK)),
        (
            InputAction::ROT_LEFT,
            PhysicalControl::axis_negative(pad_map::RIGHT_STICK_X),
        ),
        (
            InputAction::ROT_RIGHT,
            PhysicalControl::axis_positive(pad_map::RIGHT_STICK_X),
        ),
        (
            InputAction::ROT_UP,
            PhysicalControl::axis_positive(pad_map::RIGHT_STICK_Y),
        ),
        (
            InputAction::ROT_DOWN,
            PhysicalControl::axis_negative(pad_map::RIGHT_STICK_Y),
        ),
        (
            InputAction::ZOOM_IN,
            PhysicalControl::trigger(pad_map::LEFT_TRIGGER),
        ),
        (
            InputAction::ZOOM_OUT,
            PhysicalControl::trigger(pad_map::RIGHT_TRIGGER),
        ),
    ] {
        assert_eq!(pad.get_action(action), expected, "{action:?}");
    }

    // REQ-INP-032 and §4.3: the left stick, both clicks but R3, the bumpers and Guide are
    // unbound, and QuickSave / QuickLoad have no default binding either.
    let bound: Vec<PhysicalControl> = pad.bindings().iter().map(|(_, control)| *control).collect();
    for control in [
        PhysicalControl::axis_positive(pad_map::LEFT_STICK_X),
        PhysicalControl::axis_negative(pad_map::LEFT_STICK_X),
        PhysicalControl::axis_positive(pad_map::LEFT_STICK_Y),
        PhysicalControl::axis_negative(pad_map::LEFT_STICK_Y),
        button(pad_map::LEFT_STICK),
        button(pad_map::LEFT_BUMPER),
        button(pad_map::RIGHT_BUMPER),
        button(pad_map::GUIDE),
    ] {
        assert!(!bound.contains(&control), "{control:?}");
    }
    assert!(!pad.get_action(InputAction::QUICK_SAVE).is_bound());
    assert!(!pad.get_action(InputAction::QUICK_LOAD).is_bound());
}

/// REQ-INP-021: a pad with no standard mapping starts with no useful default and works once its
/// raw controls are remapped.
#[test]
fn req_inp_021_an_unrecognised_pad_has_no_default_binding() {
    let generic = FakeGamepad::with_layout("Generic USB Joystick", false);
    let mut hub = FakeGamepadHub::default();
    hub.plug(&generic);
    let mut bindings = InputBindings::new();
    let mut pads = GamepadAssignment::new();

    pads.assign(&mut bindings, 0, Some(&generic));
    let set = pads.gamepad_bindings(&hub, &mut bindings, 0).unwrap();
    assert_eq!(set.count(), 0);

    set.set_action(InputAction::A, PhysicalControl::button(7));
    generic.press(7);
    assert_eq!(
        set.sample(|control| calibration::is_pressed(generic.value(control))),
        InputAction::A
    );
}

/// REQ-INP-030 and REQ-INP-031, as amended by AMD-021 R2-011 (which replaces AMD-020 for
/// Q-048): the 0.34 threshold is measured on the shaped value — radial dead zone 0.2 and
/// saturation 0.9 for a stick, 0.2 and 0.9 for a trigger.
#[test]
fn req_inp_030_the_digital_threshold_is_zero_point_three_four() {
    let pad = FakeGamepad::new("Pad");
    let right = PhysicalControl::axis_positive(pad_map::RIGHT_STICK_X);
    let left = PhysicalControl::axis_negative(pad_map::RIGHT_STICK_X);

    // REQ-INP-031: a resting stick drifting by 0.15 produces nothing.
    pad.axis(pad_map::RIGHT_STICK_X, 0.15);
    assert_eq!(pad.value(right), 0.0);

    // §5.1: 20 % to the right is released, 60 % is pressed.
    pad.axis(pad_map::RIGHT_STICK_X, 0.2);
    assert!(!calibration::is_pressed(pad.value(right)));
    pad.axis(pad_map::RIGHT_STICK_X, 0.6);
    assert!(calibration::is_pressed(pad.value(right)));

    // R2-011: 0.40 of travel shapes to 0.286, below the threshold; 0.45 shapes to 0.357, above.
    pad.axis(pad_map::RIGHT_STICK_X, -0.40);
    assert!((pad.value(left) - 0.2857).abs() < 1e-3);
    assert!(!calibration::is_pressed(pad.value(left)));
    pad.axis(pad_map::RIGHT_STICK_X, -0.45);
    assert!((pad.value(left) - 0.3571).abs() < 1e-3);
    assert!(calibration::is_pressed(pad.value(left)));

    // REQ-INP-031: the shaped value of a stick at 0.9 is exactly 1 (R2-011 closes Q-056).
    pad.axis(pad_map::RIGHT_STICK_X, 0.9);
    assert!((pad.value(right) - 1.0).abs() < 1e-3);

    // REQ-INP-030: a half-pressed trigger is pressed; 0.5 shapes to 0.429.
    let trigger = PhysicalControl::trigger(pad_map::LEFT_TRIGGER);
    pad.trigger(pad_map::LEFT_TRIGGER, 0.5);
    assert!((pad.value(trigger) - 0.4286).abs() < 1e-3);
    assert!(calibration::is_pressed(pad.value(trigger)));

    // R2-011: the trigger dead zone is 0.2, not the 0.1 of REQ-INP-031, and it saturates at 0.9.
    pad.trigger(pad_map::LEFT_TRIGGER, 0.15);
    assert_eq!(pad.value(trigger), 0.0);
    pad.trigger(pad_map::LEFT_TRIGGER, 0.9);
    assert!((pad.value(trigger) - 1.0).abs() < 1e-4);
}

#[test]
fn amd_021_r2_011_the_threshold_applies_to_the_shaped_value() {
    let pad = FakeGamepad::new("Pad");
    let up = PhysicalControl::axis_negative(pad_map::RIGHT_STICK_Y);
    let right_half = PhysicalControl::axis_positive(pad_map::RIGHT_STICK_X);

    // Straight up at 0.40 of travel: the shaped value is (0.40 - 0.2) / 0.7 = 0.286, no press.
    pad.axis(pad_map::RIGHT_STICK_Y, -0.40)
        .axis(pad_map::RIGHT_STICK_X, 0.0);
    assert!((pad.value(up) - 0.2857).abs() < 1e-3);
    assert!(!calibration::is_pressed(pad.value(up)));

    // At 0.45 the shaped value is 0.357: pressed.
    pad.axis(pad_map::RIGHT_STICK_Y, -0.45);
    assert!((pad.value(up) - 0.3571).abs() < 1e-3);
    assert!(calibration::is_pressed(pad.value(up)));

    // At 45 degrees with a travel of 0.6: the shaped magnitude is 0.571 and each component
    // 0.404, so Up and Right are both pressed.
    let diagonal = 0.6_f32 / 2.0_f32.sqrt();
    pad.axis(pad_map::RIGHT_STICK_Y, -diagonal)
        .axis(pad_map::RIGHT_STICK_X, diagonal);
    assert!((pad.value(up) - 0.404).abs() < 1e-3);
    assert!((pad.value(right_half) - 0.404).abs() < 1e-3);
    assert!(calibration::is_pressed(pad.value(up)));
    assert!(calibration::is_pressed(pad.value(right_half)));
}

#[test]
fn amd_021_r2_011_the_trigger_dead_zone_is_zero_point_two() {
    assert_eq!(calibration::PRESS_THRESHOLD, 0.34);
    assert_eq!(calibration::STICK_DEAD_ZONE, 0.2);
    assert_eq!(calibration::STICK_SATURATION, 0.9);
    assert_eq!(calibration::TRIGGER_DEAD_ZONE, 0.2);
    assert_eq!(calibration::TRIGGER_SATURATION, 0.9);

    // The press decision and an analog consumer read the same number (Q-056 is moot).
    assert_eq!(calibration::shaped_trigger(0.5), calibration::trigger(0.5));
    assert_eq!(
        calibration::shaped_half_axis(0.5, 0.0, 1),
        calibration::half_axis(0.5, 0.0, 1)
    );
}

/// REQ-INP-031: the stick dead zone is radial, so a diagonal push past the zone produces a value
/// on both halves.
#[test]
fn req_inp_031_the_stick_dead_zone_is_radial() {
    let pad = FakeGamepad::new("Pad");
    let right = PhysicalControl::axis_positive(pad_map::RIGHT_STICK_X);

    // Each axis alone is inside the dead zone, but the magnitude (0.212) is just past it.
    pad.axis(pad_map::RIGHT_STICK_X, 0.15)
        .axis(pad_map::RIGHT_STICK_Y, 0.15);
    assert!(pad.value(right) > 0.0);

    // At 0.1 each the magnitude is 0.141 and nothing is produced.
    pad.axis(pad_map::RIGHT_STICK_X, 0.1)
        .axis(pad_map::RIGHT_STICK_Y, 0.1);
    assert_eq!(pad.value(right), 0.0);
}

/// REQ-INP-032: with the default gamepad set, pushing the left stick fully produces neither a
/// NES direction nor a camera action.
#[test]
fn req_inp_032_the_left_stick_is_unbound_by_default() {
    let pad = FakeGamepad::new("Pad");
    pad.axis(pad_map::LEFT_STICK_X, 1.0)
        .axis(pad_map::LEFT_STICK_Y, 1.0);

    let set = default_bindings::create_gamepad(true);
    assert_eq!(
        set.sample(|control| calibration::is_pressed(pad.value(control))),
        InputAction::NONE
    );
}

/// REQ-INP-010: the same key may drive an action of J1 and another of J2; binding W to Up for J2
/// leaves J1 untouched.
#[test]
fn req_inp_010_a_key_can_drive_both_players() {
    let mut source = DesktopInputSource::unsaved();
    let mut hub = FakeGamepadHub::default();
    let mut keyboard = FakeKeyboard::default();

    source.edit(|bindings, _, _| {
        bindings
            .keyboard_mut(1)
            .set_action(InputAction::UP, key(Key::W));
    });
    keyboard.press(Key::W.code());

    let snapshot = sample(&mut source, Some(&keyboard), &mut hub);
    assert_eq!(snapshot.player_one, InputAction::NONE);
    assert_eq!(snapshot.player_two, InputAction::UP);

    // The right arrow is J1's: both players are then driven by one keyboard.
    keyboard.press(Key::Right.code());
    let snapshot = sample(&mut source, Some(&keyboard), &mut hub);
    assert_eq!(snapshot.player_one, InputAction::RIGHT);
    assert_eq!(snapshot.player_two, InputAction::UP);
}

/// REQ-INP-002 and REQ-INP-044: a key bound to two actions raises both bits, and the keyboard
/// and the pad are OR-merged.
#[test]
fn req_inp_044_one_control_can_drive_several_actions() {
    let mut source = DesktopInputSource::unsaved();
    let mut keyboard = FakeKeyboard::default();
    let pad = FakeGamepad::new("Pad");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&pad);

    source.edit(|bindings, pads, _| {
        pads.skip_startup_delay(&hub, bindings);
        pads.assign(bindings, 0, Some(&pad));
        // Bind X to both A and B: pressing it gives 0x03 (REQ-INP-044).
        bindings
            .keyboard_mut(0)
            .set_action(InputAction::A, key(Key::X));
        bindings
            .keyboard_mut(0)
            .set_action(InputAction::B, key(Key::X));
    });
    keyboard.press(Key::X.code());

    assert_eq!(
        sample(&mut source, Some(&keyboard), &mut hub).player_one,
        InputAction::A | InputAction::B
    );

    // REQ-INP-002: the pad's hat adds its bit; releasing the key leaves it set.
    pad.press(pad_map::D_PAD_RIGHT);
    assert_eq!(
        sample(&mut source, Some(&keyboard), &mut hub).player_one,
        InputAction::A | InputAction::B | InputAction::RIGHT
    );

    keyboard.release(Key::X.code());
    assert_eq!(
        sample(&mut source, Some(&keyboard), &mut hub).player_one,
        InputAction::RIGHT
    );
}

/// REQ-INP-023 rule 2: the first unassigned pad to become active goes to J1, the next one to J2 —
/// press M2 then M1 and M2 drives J1.
#[test]
fn req_inp_023_the_first_pad_used_goes_to_player_one() {
    let m1 = FakeGamepad::new("M1");
    let m2 = FakeGamepad::new("M2");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&m1).plug(&m2);
    let mut bindings = InputBindings::new();
    let mut pads = GamepadAssignment::new();

    pads.skip_startup_delay(&hub, &mut bindings);
    assert_eq!(device_identity(&pads, &hub, 0), None);
    assert_eq!(device_identity(&pads, &hub, 1), None);

    m2.press(pad_map::SOUTH);
    pads.update(&hub, &mut bindings, 0.016);
    assert_eq!(device_identity(&pads, &hub, 0).as_deref(), Some("M2"));
    assert_eq!(device_identity(&pads, &hub, 1), None);

    m2.release_all();
    m1.press(pad_map::SOUTH);
    pads.update(&hub, &mut bindings, 0.016);
    assert_eq!(device_identity(&pads, &hub, 0).as_deref(), Some("M2"));
    assert_eq!(device_identity(&pads, &hub, 1).as_deref(), Some("M1"));

    // REQ-INP-050: the assignment is remembered, so the same pads come back next launch.
    assert_eq!(bindings.last_gamepad(0), Some("M2"));
    assert_eq!(bindings.last_gamepad(1), Some("M1"));

    let mut reloaded = InputBindings::new();
    reloaded.set_last_gamepad(0, Some("M2"));
    reloaded.set_last_gamepad(1, Some("M1"));
    let mut next = GamepadAssignment::new();
    next.update(&hub, &mut reloaded, STARTUP_DELAY_SECONDS);
    assert_eq!(device_identity(&next, &hub, 0).as_deref(), Some("M2"));
    assert_eq!(device_identity(&next, &hub, 1).as_deref(), Some("M1"));
}

/// REQ-INP-023 rule 1: the reconnection waits about 0.5 s after launch before taking the
/// remembered pads back.
#[test]
fn req_inp_023_the_remembered_pads_come_back_after_half_a_second() {
    let pad = FakeGamepad::new("M1");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&pad);
    let mut bindings = InputBindings::new();
    bindings.set_last_gamepad(0, Some("M1"));
    let mut pads = GamepadAssignment::new();

    pads.update(&hub, &mut bindings, 0.2);
    assert!(!pads.startup_complete());
    assert_eq!(device_identity(&pads, &hub, 0), None);

    pads.update(&hub, &mut bindings, 0.4);
    assert!(pads.startup_complete());
    assert_eq!(device_identity(&pads, &hub, 0).as_deref(), Some("M1"));
}

/// REQ-INP-024: unplugging an assigned pad leaves the player with none and its bits fall to zero
/// at the next sample; plugging it back and pressing a button gives it back.
#[test]
fn req_inp_024_unplugging_drops_the_pad_bits_at_the_next_sample() {
    let pad = FakeGamepad::new("M1");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&pad);
    let mut source = DesktopInputSource::unsaved();

    source.edit(|bindings, pads, _| pads.skip_startup_delay(&hub, bindings));
    pad.press(pad_map::D_PAD_RIGHT);
    assert_eq!(
        sample(&mut source, None, &mut hub).player_one,
        InputAction::RIGHT
    );

    hub.unplug("M1");
    assert_eq!(
        sample(&mut source, None, &mut hub).player_one,
        InputAction::NONE
    );
    assert_eq!(device_identity(source.pads(), &hub, 0), None);

    // Plugged back in: not assigned until it is used again.
    hub.plug(&pad);
    pad.release_all();
    assert_eq!(
        sample(&mut source, None, &mut hub).player_one,
        InputAction::NONE
    );

    pad.press(pad_map::D_PAD_RIGHT);
    assert_eq!(
        sample(&mut source, None, &mut hub).player_one,
        InputAction::RIGHT
    );
}

/// REQ-INP-022: a VR controller never appears in the device list and is never assigned
/// automatically.
#[rstest]
#[case("Oculus Touch Controller")]
#[case("HTC Vive Controller")]
#[case("Valve Index Knuckles")]
#[case("Spatial Controller")]
fn req_inp_022_vr_devices_are_never_offered(#[case] identity: &str) {
    assert!(is_virtual_reality(identity));

    let pad = FakeGamepad::new(identity);
    pad.press(pad_map::SOUTH);
    let mut hub = FakeGamepadHub::default();
    hub.plug(&pad);
    assert!(hub.devices().is_empty());

    let mut bindings = InputBindings::new();
    let mut pads = GamepadAssignment::new();
    pads.update(&hub, &mut bindings, 1.0);
    assert_eq!(device_identity(&pads, &hub, 0), None);
}

/// DEC-08-1 (divergence D) and REQ-INP-041: assigning a pad to a player removes it from the
/// other one.
#[test]
fn dec_08_1_a_pad_assigned_to_one_player_is_removed_from_the_other() {
    let pad = FakeGamepad::new("M1");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&pad);
    let mut bindings = InputBindings::new();
    let mut pads = GamepadAssignment::new();

    pads.skip_startup_delay(&hub, &mut bindings);
    pads.assign(&mut bindings, 0, Some(&pad));
    assert_eq!(device_identity(&pads, &hub, 0).as_deref(), Some("M1"));

    pads.assign(&mut bindings, 1, Some(&pad));
    assert_eq!(device_identity(&pads, &hub, 0), None);
    assert_eq!(device_identity(&pads, &hub, 1).as_deref(), Some("M1"));
    assert_eq!(bindings.last_gamepad(0), None);

    // "none" takes the pad away (REQ-INP-041).
    pads.assign(&mut bindings, 1, None);
    assert_eq!(device_identity(&pads, &hub, 1), None);
}

/// REQ-INP-025: a gamepad set belongs to the device identity, survives an unplug and follows the
/// pad from one player to the other; another model keeps its own set.
#[test]
fn req_inp_025_gamepad_bindings_belong_to_the_device_identity() {
    let model_x = FakeGamepad::new("Model X");
    let model_y = FakeGamepad::new("Model Y");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&model_x).plug(&model_y);
    let mut bindings = InputBindings::new();
    let mut pads = GamepadAssignment::new();
    let north = PhysicalControl::button(pad_map::NORTH);

    pads.skip_startup_delay(&hub, &mut bindings);
    pads.assign(&mut bindings, 0, Some(&model_x));

    // Remap the north button of Model X onto QuickSave.
    pads.gamepad_bindings(&hub, &mut bindings, 0)
        .unwrap()
        .set_action(InputAction::QUICK_SAVE, north);

    // Unplug and plug back: the remapping is still there.
    hub.unplug("Model X");
    hub.plug(&model_x);
    assert_eq!(
        bindings
            .find_gamepad("Model X")
            .unwrap()
            .get_action(InputAction::QUICK_SAVE),
        north
    );

    // Model Y keeps the §4.3 defaults.
    pads.assign(&mut bindings, 1, Some(&model_y));
    let set = pads.gamepad_bindings(&hub, &mut bindings, 1).unwrap();
    assert!(!set.get_action(InputAction::QUICK_SAVE).is_bound());
    assert_eq!(set.get_action(InputAction::ROLL_BACK), north);

    // The set follows the pad from J1 to J2.
    pads.assign(&mut bindings, 1, Some(&model_x));
    let set = pads.gamepad_bindings(&hub, &mut bindings, 1).unwrap();
    assert_eq!(set.get_action(InputAction::QUICK_SAVE), north);
}

/// REQ-INP-042 and REQ-INP-043: activating a field clears the binding and starts listening, the
/// first newly pressed eligible key becomes the binding, and a second capture replaces the first.
#[test]
fn req_inp_042_a_capture_clears_then_takes_the_first_eligible_key() {
    let mut bindings = InputBindings::new();
    let mut keyboard = FakeKeyboard::default();
    let mut session = RebindingSession::new();

    assert!(session.begin_action(&mut bindings, None, InputAction::B));
    assert!(session.is_listening());

    // REQ-INP-042: the old binding is already gone while the listening is on.
    assert!(!bindings.keyboard(0).get_action(InputAction::B).is_bound());

    // Q-033: the first sample only records what is already held.
    session.capture(&mut bindings, None, Some(&keyboard));

    keyboard.press(Key::C.code());
    assert_eq!(
        session.capture(&mut bindings, None, Some(&keyboard)),
        key(Key::C)
    );
    assert!(!session.is_listening());
    assert_eq!(bindings.keyboard(0).get_action(InputAction::B), key(Key::C));

    // REQ-INP-043: one binding per target per device kind; the second capture replaces it.
    keyboard.release(Key::C.code());
    session.begin_action(&mut bindings, None, InputAction::B);
    session.capture(&mut bindings, None, Some(&keyboard));
    keyboard.press(Key::K.code());
    session.capture(&mut bindings, None, Some(&keyboard));

    assert_eq!(bindings.keyboard(0).get_action(InputAction::B), key(Key::K));
    let target = BindingTarget::of(InputAction::B);
    assert_eq!(
        bindings
            .keyboard(0)
            .bindings()
            .iter()
            .filter(|(candidate, _)| *candidate == target)
            .count(),
        1
    );
}

/// REQ-INP-042 and REQ-INP-012: a keyboard capture only accepts keys, a gamepad capture only pad
/// controls, and a mouse button can never be bound — the vocabulary has no such control.
#[test]
fn req_inp_042_only_the_right_device_kind_is_eligible() {
    let mut bindings = InputBindings::new();
    let mut keyboard = FakeKeyboard::default();
    let pad = FakeGamepad::new("Pad");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&pad);
    let mut pads = GamepadAssignment::new();
    pads.skip_startup_delay(&hub, &mut bindings);
    pads.assign(&mut bindings, 0, Some(&pad));
    let mut session = RebindingSession::new();
    let access = || {
        Some(PadAccess {
            pads: &pads,
            hub: &hub,
        })
    };

    // A keyboard capture ignores a pad button.
    session.begin_action(&mut bindings, access(), InputAction::A);
    session.capture(&mut bindings, access(), Some(&keyboard));
    pad.press(pad_map::SOUTH);
    assert!(!session
        .capture(&mut bindings, access(), Some(&keyboard))
        .is_bound());
    assert!(session.is_listening());

    // A gamepad capture ignores a key and takes the stick half-axis.
    session.select_device_kind(BindingDeviceKind::Gamepad);
    pad.release_all();
    session.begin_action(&mut bindings, access(), InputAction::A);
    session.capture(&mut bindings, access(), Some(&keyboard));

    keyboard.press(Key::J.code());
    assert!(!session
        .capture(&mut bindings, access(), Some(&keyboard))
        .is_bound());

    pad.axis(pad_map::LEFT_STICK_Y, 1.0);
    let half = PhysicalControl::axis_positive(pad_map::LEFT_STICK_Y);
    assert_eq!(
        session.capture(&mut bindings, access(), Some(&keyboard)),
        half
    );
    assert_eq!(
        pads.gamepad_bindings(&hub, &mut bindings, 0)
            .unwrap()
            .get_action(InputAction::A),
        half
    );

    // REQ-INP-012: a keyboard set refuses anything that is not a key.
    assert!(!bindings
        .keyboard_mut(0)
        .set_action(InputAction::A, PhysicalControl::button(0)));
    assert!(!bindings
        .gamepad_mut("Pad", true)
        .set_action(InputAction::A, key(Key::A)));
}

/// REQ-INP-045: stopping the listening, changing player, changing device kind or a pad change
/// all leave the action empty, because the field was cleared when it was activated.
#[test]
fn req_inp_045_stopping_the_listening_leaves_the_action_empty() {
    let mut bindings = InputBindings::new();
    let mut session = RebindingSession::new();

    session.begin_action(&mut bindings, None, InputAction::START);
    session.stop();
    assert!(!session.is_listening());
    assert!(!bindings
        .keyboard(0)
        .get_action(InputAction::START)
        .is_bound());

    session.begin_action(&mut bindings, None, InputAction::A);
    session.select_player(1);
    assert!(!session.is_listening());
    assert!(!bindings.keyboard(0).get_action(InputAction::A).is_bound());

    session.select_player(0);
    session.begin_action(&mut bindings, None, InputAction::UP);
    session.select_device_kind(BindingDeviceKind::Gamepad);
    assert!(!session.is_listening());
    assert!(!bindings.keyboard(0).get_action(InputAction::UP).is_bound());

    session.select_device_kind(BindingDeviceKind::Keyboard);
    session.begin_action(&mut bindings, None, InputAction::DOWN);
    session.notify_gamepad_changed();
    assert!(!session.is_listening());

    // Activating another field stops the first one.
    session.begin_action(&mut bindings, None, InputAction::LEFT);
    session.begin_action(&mut bindings, None, InputAction::RIGHT);
    assert_eq!(session.target(), &BindingTarget::of(InputAction::RIGHT));
    assert!(!bindings
        .keyboard(0)
        .get_action(InputAction::LEFT)
        .is_bound());

    session.close();
    assert!(!session.is_listening());
}

/// REQ-INP-046: in Gamepad mode with no pad assigned, there is nothing to show and activating a
/// field starts no capture.
#[test]
fn req_inp_046_gamepad_mode_without_a_pad_shows_nothing() {
    let mut bindings = InputBindings::new();
    let pads = GamepadAssignment::new();
    let hub = FakeGamepadHub::default();
    let mut session = RebindingSession::new();
    let access = || {
        Some(PadAccess {
            pads: &pads,
            hub: &hub,
        })
    };

    session.select_player(1);
    session.select_device_kind(BindingDeviceKind::Gamepad);

    assert!(session.active_set(&mut bindings, access()).is_none());
    assert!(!session.begin_action(&mut bindings, access(), InputAction::A));
    assert!(!session.is_listening());
}

/// DEC-08-2 and REQ-INP-047: Escape can be captured like any other key, and while a capture is
/// listening — and in the sample it completes in — the host produces no action at all, so the
/// same press cannot also be seen as Menu.
#[test]
fn dec_08_2_capturing_escape_does_not_also_open_the_menu() {
    let mut source = DesktopInputSource::unsaved();
    let mut hub = FakeGamepadHub::default();
    let mut keyboard = FakeKeyboard::default();

    source.edit(|bindings, _, session| session.begin_action(bindings, None, InputAction::B));
    assert!(source.rebinding().is_listening());

    // The sample that starts the listening produces nothing.
    assert_eq!(
        sample(&mut source, Some(&keyboard), &mut hub).player_one,
        InputAction::NONE
    );
    source.end_sample();

    keyboard.press(Key::Escape.code());
    let captured = sample(&mut source, Some(&keyboard), &mut hub);

    // Escape became the binding of B, and the mask stays empty: no Menu this sample.
    assert_eq!(
        source.bindings().keyboard(0).get_action(InputAction::B),
        key(Key::Escape)
    );
    assert_eq!(captured.player_one, InputAction::NONE);
    assert!(source.rebinding().suppresses_actions());

    source.end_sample();

    // From the next sample on, Escape is B.
    assert!(sample(&mut source, Some(&keyboard), &mut hub)
        .player_one
        .contains(InputAction::B));
}

/// REQ-INP-049: resetting a set puts the table of §4.2 back for J1, an empty set for J2 and the
/// table of §4.3 for a pad.
#[test]
fn req_inp_049_a_set_can_be_reset_to_its_defaults() {
    let mut bindings = InputBindings::new();

    bindings.keyboard_mut(0).clear_all();
    bindings
        .keyboard_mut(0)
        .set_action(InputAction::A, key(Key::Q));
    bindings.reset_keyboard(0);
    assert_eq!(bindings.keyboard(0).get_action(InputAction::A), key(Key::X));
    assert_eq!(bindings.keyboard(0).count(), 17);

    bindings
        .keyboard_mut(1)
        .set_action(InputAction::A, key(Key::Q));
    bindings.reset_keyboard(1);
    assert_eq!(bindings.keyboard(1).count(), 0);

    bindings
        .gamepad_mut("Pad", true)
        .set_action(InputAction::A, PhysicalControl::button(pad_map::NORTH));
    bindings.reset_gamepad("Pad", true);
    assert_eq!(
        bindings
            .find_gamepad("Pad")
            .unwrap()
            .get_action(InputAction::A),
        PhysicalControl::button(pad_map::SOUTH)
    );
}

/// AMD-010 REQ-UI-203: a menu command is bindable like an action and runs when its control is
/// pressed, once per press.
#[test]
fn req_ui_203_a_menu_command_can_be_bound_to_a_pad_button() {
    let reset = Command::Reset.name();
    let pad = FakeGamepad::new("Pad");
    let mut hub = FakeGamepadHub::default();
    hub.plug(&pad);
    let mut source = DesktopInputSource::unsaved();

    source.edit(|bindings, pads, _| {
        pads.skip_startup_delay(&hub, bindings);
        pads.assign(bindings, 0, Some(&pad));
    });

    // A command is a target like any other: in Keyboard mode the field edits the keyboard set
    // of the shown player.
    source.edit(|bindings, pads, session| {
        let access = Some(PadAccess { pads, hub: &hub });
        assert!(session.begin_command(bindings, access, reset));
        let set = session.active_set(bindings, access).unwrap();
        assert_eq!(set.kind(), BindingDeviceKind::Keyboard);
    });

    source.edit(|bindings, pads, session| {
        session.select_device_kind(BindingDeviceKind::Gamepad);
        let access = Some(PadAccess { pads, hub: &hub });
        assert!(session.begin_command(bindings, access, reset));
        session.capture(bindings, access, None);

        pad.press(pad_map::LEFT_BUMPER);
        assert_eq!(
            session.capture(bindings, access, None),
            PhysicalControl::button(pad_map::LEFT_BUMPER)
        );
        session.end_sample();
    });

    // The command fires once on the rising edge, not on every frame it is held.
    pad.release_all();
    sample(&mut source, None, &mut hub);
    source.end_sample();

    pad.press(pad_map::LEFT_BUMPER);
    sample(&mut source, None, &mut hub);
    assert_eq!(source.triggered_commands(), [reset.to_owned()]);

    source.end_sample();
    sample(&mut source, None, &mut hub);
    assert_eq!(source.triggered_commands(), &[] as &[String]);
}

fn open_store(paths: &UserDataPaths) -> FileInputBindingStore {
    FileInputBindingStore::new(paths.clone(), OffsetDateTime::now_utc)
}

/// REQ-INP-050, REQ-INP-052 and SPEC-09 REQ-PERS-065: the keyboard sets, the per-identity
/// gamepad sets and the last pad of each player survive a round trip through `input.json`, and
/// an unreadable file falls back to the defaults.
#[test]
fn req_pers_065_the_bindings_survive_a_round_trip_through_input_json() {
    let root = TemporaryRoot::new("zax-input");
    let paths = UserDataPaths::resolve(Some(root.path()), None, None);
    let mut store = open_store(&paths);
    let reset = BindingTarget::of_command(Command::Reset.name());

    let mut written = InputBindings::new();
    written
        .keyboard_mut(0)
        .set_action(InputAction::A, key(Key::K));
    written
        .keyboard_mut(1)
        .set_action(InputAction::UP, key(Key::W));
    let model = written.gamepad_mut("Model X", true);
    model.set_action(
        InputAction::QUICK_SAVE,
        PhysicalControl::button(pad_map::NORTH),
    );
    model.set_action(
        InputAction::ROT_LEFT,
        PhysicalControl::axis_negative(pad_map::LEFT_STICK_X),
    );
    model.set(
        reset.clone(),
        PhysicalControl::trigger(pad_map::RIGHT_TRIGGER),
    );
    written.set_last_gamepad(1, Some("Model X"));
    store.save(&written);

    assert!(paths.input_file.exists());

    let mut read = InputBindings::new();
    open_store(&paths).load(&mut read);

    assert_eq!(read.keyboard(0).get_action(InputAction::A), key(Key::K));
    assert_eq!(read.keyboard(1).get_action(InputAction::UP), key(Key::W));
    let model = read.find_gamepad("Model X").unwrap();
    assert_eq!(
        model.get_action(InputAction::QUICK_SAVE),
        PhysicalControl::button(pad_map::NORTH)
    );
    assert_eq!(
        model.get_action(InputAction::ROT_LEFT),
        PhysicalControl::axis_negative(pad_map::LEFT_STICK_X)
    );
    assert_eq!(
        model.get(&reset),
        PhysicalControl::trigger(pad_map::RIGHT_TRIGGER)
    );
    assert_eq!(read.last_gamepad(1), Some("Model X"));
    assert_eq!(read.last_gamepad(0), None);

    // REQ-INP-052: a corrupted file starts on the defaults, without a blocking error, and is
    // set aside.
    std::fs::write(&paths.input_file, "{ not json").unwrap();
    let mut recovered = InputBindings::new();
    let mut recovery = open_store(&paths);
    recovery.load(&mut recovered);

    assert_eq!(recovery.message(), Some(UNREADABLE_MESSAGE));
    assert_eq!(
        recovered.keyboard(0).get_action(InputAction::A),
        key(Key::X)
    );
    assert_eq!(recovered.keyboard(1).count(), 0);
    let set_aside = std::fs::read_dir(&paths.root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("input.json.corrupt-")
        })
        .count();
    assert!(set_aside > 0);
}

/// REQ-INP-051: a binding change is written at once, so nothing is lost on a hard kill.
#[test]
fn req_inp_051_a_change_is_written_immediately() {
    let root = TemporaryRoot::new("zax-input");
    let paths = UserDataPaths::resolve(Some(root.path()), None, None);
    let mut source = DesktopInputSource::new(InputBindings::new(), Box::new(open_store(&paths)));
    source.load();

    assert!(!paths.input_file.exists());

    source.edit(|bindings, _, _| {
        bindings
            .keyboard_mut(0)
            .set_action(InputAction::A, key(Key::K));
    });
    assert!(paths.input_file.exists());

    let mut reread = InputBindings::new();
    open_store(&paths).load(&mut reread);
    assert_eq!(reread.keyboard(0).get_action(InputAction::A), key(Key::K));
}

/// SPEC-09 REQ-PERS-065: the persisted form of a physical command is stable and round-trips.
#[rstest]
#[case("key:88")]
#[case("button:3")]
#[case("axis:2+")]
#[case("axis:3-")]
#[case("trigger:1")]
fn req_pers_065_a_control_round_trips_through_its_text(#[case] text: &str) {
    let control = PhysicalControl::parse(text);
    assert!(control.is_bound());
    assert_eq!(control.to_string(), text);
}

/// A malformed entry degrades to "unbound".
#[rstest]
#[case("")]
#[case("axis:2")]
#[case("mouse:0")]
#[case("key:")]
#[case("key:abc")]
#[case("nonsense")]
fn req_inp_052_a_malformed_control_is_unbound(#[case] text: &str) {
    assert!(!PhysicalControl::parse(text).is_bound());
}

/// REQ-INP-048: the bindings are global; nothing in the model is keyed by game.
#[test]
fn req_inp_048_bindings_are_global() {
    let mut bindings = InputBindings::new();
    bindings
        .keyboard_mut(0)
        .set_action(InputAction::A, key(Key::K));

    // There is one keyboard set per player and one per pad identity; no per-game dimension
    // exists in the model, so a remapping made in one game is active in every other.
    assert_eq!(bindings.keyboard(0).get_action(InputAction::A), key(Key::K));
    assert_eq!(input_action::PLAYER_COUNT, 2);
}

/// A handheld console (Rust-only, owner request): NES A and B on the buttons labelled A and B,
/// zoom on the shoulders, fast forward on R2, no rewind; X, Y and L2 stay free for the handheld
/// interface.
#[test]
fn a_handheld_pad_gets_the_handheld_defaults() {
    let mut bindings = InputBindings::new();
    bindings.use_handheld_defaults();
    let set = bindings.gamepad_mut("handheld pad", true);

    assert_eq!(
        set.get_action(InputAction::A),
        PhysicalControl::button(pad_map::SOUTH)
    );
    assert_eq!(
        set.get_action(InputAction::B),
        PhysicalControl::button(pad_map::EAST)
    );
    assert_eq!(
        set.get_action(InputAction::ZOOM_OUT),
        PhysicalControl::button(pad_map::LEFT_BUMPER)
    );
    assert_eq!(
        set.get_action(InputAction::ZOOM_IN),
        PhysicalControl::button(pad_map::RIGHT_BUMPER)
    );
    assert_eq!(
        set.get_action(InputAction::FAST_FORWARD),
        PhysicalControl::trigger(pad_map::RIGHT_TRIGGER)
    );
    assert!(!set.get_action(InputAction::ROLL_BACK).is_bound());
    for free in [pad_map::WEST, pad_map::NORTH] {
        assert!(set
            .bindings()
            .iter()
            .all(|(_, control)| *control != PhysicalControl::button(free)));
    }
    assert!(set
        .bindings()
        .iter()
        .all(|(_, control)| *control != PhysicalControl::trigger(pad_map::LEFT_TRIGGER)));
}

/// A pad an earlier version saved with the desktop defaults moves to the handheld defaults; one
/// the player changed is kept.
#[test]
fn a_saved_desktop_default_pad_moves_to_the_handheld_defaults() {
    let mut bindings = InputBindings::new();
    bindings.gamepad_mut("untouched", true);
    bindings
        .gamepad_mut("changed", true)
        .set_action(InputAction::A, PhysicalControl::button(pad_map::NORTH));

    bindings.use_handheld_defaults();

    assert_eq!(
        bindings
            .gamepad_mut("untouched", true)
            .get_action(InputAction::B),
        PhysicalControl::button(pad_map::EAST)
    );
    let changed = bindings.gamepad_mut("changed", true);
    assert_eq!(
        changed.get_action(InputAction::A),
        PhysicalControl::button(pad_map::NORTH)
    );
    assert_eq!(
        changed.get_action(InputAction::B),
        PhysicalControl::button(pad_map::WEST)
    );
}

/// The desktop plays as its console mode (owner request): NES A and B on the pad's A and B, the
/// rollback on the right stick's click. A pad saved with an earlier default (the desktop's, the
/// handheld's, or the console's with the face buttons at a handheld's places) moves to it; one
/// the player changed is kept.
#[test]
fn a_desktop_pad_gets_the_console_defaults() {
    let mut bindings = InputBindings::new();
    bindings.gamepad_mut("desktop default", true);
    let mut labelled = InputBindings::new();
    labelled.use_handheld_defaults();
    let by_labels = labelled.gamepad_mut("any", true).clone();
    bindings
        .gamepad_mut("handheld default", true)
        .replace(by_labels.bindings().iter().cloned());
    let former = zax_app::input::default_bindings::create_former_desktop_console_gamepad(true);
    bindings
        .gamepad_mut("former console default", true)
        .replace(former.bindings().iter().cloned());
    bindings
        .gamepad_mut("changed", true)
        .set_action(InputAction::A, PhysicalControl::button(pad_map::NORTH));

    bindings.use_desktop_console_defaults();

    for name in [
        "desktop default",
        "handheld default",
        "former console default",
        "new pad",
    ] {
        let set = bindings.gamepad_mut(name, true);
        assert_eq!(
            set.get_action(InputAction::A),
            PhysicalControl::button(pad_map::SOUTH),
            "{name}"
        );
        assert_eq!(
            set.get_action(InputAction::B),
            PhysicalControl::button(pad_map::EAST),
            "{name}"
        );
        assert_eq!(
            set.get_action(InputAction::ROLL_BACK),
            PhysicalControl::button(pad_map::RIGHT_STICK),
            "{name}"
        );
        assert_eq!(
            set.get_action(InputAction::FAST_FORWARD),
            PhysicalControl::trigger(pad_map::RIGHT_TRIGGER),
            "{name}"
        );
    }
    assert_eq!(
        bindings
            .gamepad_mut("changed", true)
            .get_action(InputAction::A),
        PhysicalControl::button(pad_map::NORTH)
    );
}
