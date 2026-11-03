//! The settings the `Setting` members of 3dSen 2.0 stand for: the rounding of AMD-040 §3, the
//! style identifiers of AMD-055 REQ-SCN-078 and the defaults of the new keys
//! (AMD-040 REQ-LUA-114, AMD-043 REQ-SCN-066, AMD-059 REQ-LUA-116).

use rstest::rstest;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::model::SettingsModel;
use zax_settings::styles;
use zax_settings::values::SettingValue;

/// AMD-040 §3 as a **script** write reaches it (`styles::snap_write`): a value is bounded, then
/// put on the nearest rung of the setting's grid.
///
/// RO-1 point 7 widened the grid the interface and the settings file use and left a Lua write on
/// the 13 rungs of 30°, so that 15 gave 0 and 75 gave 60. **RO-2 part 3** (owner decision) takes
/// that apart: a Lua write snaps on the same grid as the menu, to the nearest rung, ties away
/// from zero. Every case the owner named, plus the bounds and the thirteen multiples of 30, which
/// are untouched — which is what makes a profile written for 3dSen behave exactly as it did.
#[rstest]
#[case(500.0, 180.0)]
#[case(-500.0, -180.0)]
#[case(40.0, 30.0)]
#[case(45.0, 60.0)]
#[case(75.0, 90.0)]
#[case(-45.0, -60.0)]
#[case(-75.0, -90.0)]
#[case(27.5, 30.0)]
#[case(12.5, 15.0)]
#[case(-12.5, -15.0)]
#[case(15.0, 15.0)]
#[case(-15.0, -15.0)]
#[case(0.0, 0.0)]
fn ro_2_a_curvature_value_from_a_script_snaps_on_the_menu_grid(
    #[case] written: f64,
    #[case] expected: f64,
) {
    assert!(
        (styles::snap_write(SettingId::SceneBendLr, written) - expected).abs() < 1e-9,
        "{written}"
    );
}

/// RO-2 part 3: the menu grid and the Lua grid are **one** grid, for every style parameter and
/// every value — not two rules that happen to agree on the cases above. `snap_write` is the
/// setting's own domain, so there is nowhere left for them to diverge.
#[test]
fn ro_2_the_menu_grid_and_the_lua_grid_are_the_same() {
    for (name, id) in styles::all_parameters() {
        let domain = catalogue::descriptor(*id).domain();
        let low = domain.minimum().unwrap_or(-10.0) - 5.0;
        let high = domain.maximum().unwrap_or(10.0) + 5.0;
        // A fine sweep over the domain and a little past each end.
        let mut value = low;
        while value <= high {
            assert!(
                (styles::snap_write(*id, value) - domain.snap(value)).abs() < 1e-9,
                "{name} at {value}"
            );
            value += 0.5;
        }
    }
}

/// And a script's three curvature angles all take the same grid: RO-1 point 7 named the three
/// together, and RO-2 part 3 takes the exception off all three at once.
#[rstest]
#[case(SettingId::SceneBendLr)]
#[case(SettingId::SceneBendTb)]
#[case(SettingId::SceneArcH)]
fn ro_2_the_three_angles_round_alike_from_a_script(#[case] id: SettingId) {
    for (written, expected) in [(45.0, 60.0), (75.0, 90.0), (12.5, 15.0), (15.0, 15.0)] {
        assert!(
            (styles::snap_write(id, written) - expected).abs() < 1e-9,
            "{id:?} at {written}"
        );
    }
    // The arc centre is not an angle and has a grid of its own
    // (`ro_5_the_arc_centre_snaps_on_the_quarters`), out of range to the same bounds as before.
    assert!((styles::snap_write(SettingId::SceneArcCenter, 7.0) - 1.0).abs() < 1e-9);
    assert!((styles::snap_write(SettingId::SceneArcCenter, -7.0) - 0.0).abs() < 1e-9);
}

/// RO-5 part 2 (owner decision): `spl.arc_center` takes the five rungs 0, 0.25, 0.5, 0.75 and 1,
/// and a value between two of them lands on the nearest, **ties away from zero** — the rule RO-2
/// part 3 gave the three angles, which is `Domain::grid`'s and not `Domain::stepped`'s.
///
/// What a script written for 3dSen does is the first thing checked: 0 and 1 are rungs, so they go
/// through untouched, which is the whole of the compatibility this lot owes.
#[test]
fn ro_5_the_arc_centre_snaps_on_the_quarters() {
    /// Exact equality: a rung is a published value, not the result of arithmetic.
    #[allow(clippy::float_cmp, reason = "published rungs, compared exactly")]
    fn assert_exact(actual: f64, expected: f64) {
        assert_eq!(actual, expected);
    }
    let snap = |value: f64| styles::snap_write(SettingId::SceneArcCenter, value);

    // A 3dSen profile can only ever write 0 or 1, and both are left exactly where they are.
    assert_exact(snap(0.0), 0.0);
    assert_exact(snap(1.0), 1.0);

    for (written, expected) in [
        // The five rungs themselves.
        (0.0, 0.0),
        (0.25, 0.25),
        (0.5, 0.5),
        (0.75, 0.75),
        (1.0, 1.0),
        // The nearest rung.
        (0.1, 0.0),
        (0.2, 0.25),
        (0.4, 0.5),
        (0.6, 0.5),
        (0.9, 1.0),
        // Ties: away from zero, so halfway between two rungs goes to the lower height of the two
        // (the one of greater magnitude on the 0-to-1 scale is the larger number).
        (0.125, 0.25),
        (0.375, 0.5),
        (0.625, 0.75),
        (0.875, 1.0),
        // Out of range, clamped to the bounds as before.
        (7.0, 1.0),
        (-7.0, 0.0),
        (-0.3, 0.0),
    ] {
        assert_exact(snap(written), expected);
    }

    // And the menu's grid and the script's grid are the same function, rung for rung — the
    // invariant RO-2 part 3 established for every style parameter.
    let domain = catalogue::descriptor(SettingId::SceneArcCenter).domain();
    let mut probe = -0.5;
    while probe <= 1.5 {
        assert_exact(snap(probe), domain.snap(probe));
        probe += 0.01;
    }
}

/// RO-1 point 7: the **domain** — which is what the options window, the console's advanced page
/// and the settings file go through — takes the finer grid: 5° between −30 and 30, 30° beyond.
/// A value off the grid lands on the nearest rung, a tie going to the one of greater magnitude,
/// which is what keeps REQ-SCN-066's criterion (45 gives 60) true.
#[rstest]
#[case(500.0, 180.0)]
#[case(-500.0, -180.0)]
#[case(45.0, 60.0)]
#[case(-45.0, -60.0)]
#[case(15.0, 15.0)]
#[case(-15.0, -15.0)]
#[case(7.0, 5.0)]
#[case(3.0, 5.0)]
#[case(2.0, 0.0)]
#[case(40.0, 30.0)]
#[case(31.0, 30.0)]
#[case(0.0, 0.0)]
fn a_curvature_value_lands_on_the_finer_grid_of_ro_1(#[case] written: f64, #[case] expected: f64) {
    let domain = catalogue::descriptor(SettingId::SceneBendLr).domain();
    assert!((domain.snap(written) - expected).abs() < 1e-9, "{written}");
}

/// The same rounding reached through the model, which is what a write from the options window and
/// a value read out of the settings file both do (AMD-043 REQ-SCN-066: writing 45 gives 60; RO-1
/// point 7: a value already on the 30° grid loads unchanged, and 15 is now a value of its own).
#[test]
fn a_model_write_of_a_curvature_value_is_rounded_too() {
    let mut model = SettingsModel::new();
    model.set(SettingId::SceneBendLr, SettingValue::Int(45));
    assert_eq!(60, model.get(SettingId::SceneBendLr).as_int());
    model.set(SettingId::SceneBendLr, SettingValue::Int(500));
    assert_eq!(180, model.get(SettingId::SceneBendLr).as_int());

    // Every rung of the amendment's grid loads unchanged.
    for coarse in (-180..=180).step_by(30) {
        model.set(SettingId::SceneBendLr, SettingValue::Int(coarse));
        assert_eq!(coarse, model.get(SettingId::SceneBendLr).as_int());
    }
    // And so does every rung of the finer one.
    for fine in [-30, -25, -20, -15, -10, -5, 0, 5, 10, 15, 20, 25, 30] {
        model.set(SettingId::SceneBendLr, SettingValue::Int(fine));
        assert_eq!(fine, model.get(SettingId::SceneBendLr).as_int());
    }
    // A value off the grid snaps to the nearest allowed one.
    model.set(SettingId::SceneBendLr, SettingValue::Int(13));
    assert_eq!(15, model.get(SettingId::SceneBendLr).as_int());
    model.set(SettingId::SceneBendLr, SettingValue::Int(-99));
    assert_eq!(-90, model.get(SettingId::SceneBendLr).as_int());
}

/// AMD-055 REQ-SCN-079, the normalisation of the graded parameters: 9 gives 5, 0 gives 1.
#[rstest]
#[case(SettingId::RenderStyleAoStrength, 9, 5)]
#[case(SettingId::RenderStyleAoStrength, 0, 1)]
#[case(SettingId::RenderStyleAoStrength, 3, 3)]
#[case(SettingId::RenderStyleVoxelGrid, 4, 3)]
#[case(SettingId::RenderStyleAo, 1, 1)]
#[case(SettingId::RenderStyleAo, 7, 2)]
fn a_style_parameter_is_bounded_by_its_ladder(
    #[case] id: SettingId,
    #[case] written: i32,
    #[case] expected: i32,
) {
    let mut model = SettingsModel::new();
    model.set(id, SettingValue::Int(written));
    assert_eq!(expected, model.get(id).as_int());
}

/// AMD-055 REQ-SCN-078: 1 is written for Enhanced and read back as 4; anything else gives 0,
/// wherever it comes from.
#[rstest]
#[case(4, 4)]
#[case(1, 4)]
#[case(0, 0)]
#[case(2, 0)]
#[case(7, 0)]
#[case(-1, 0)]
fn the_visual_style_keeps_four_and_takes_one_for_it(#[case] written: i32, #[case] expected: i32) {
    let mut model = SettingsModel::new();
    model.set(SettingId::RenderVisualStyle, SettingValue::Int(written));
    assert_eq!(expected, model.get(SettingId::RenderVisualStyle).as_int());
}

/// REQ-LUA-114: every parameter identifier of the Enhanced style names a setting, and the
/// identifiers of the other styles name none.
#[test]
fn every_parameter_identifier_of_enhanced_names_a_setting() {
    for (name, id) in styles::parameters_of(styles::ENHANCED_STYLE) {
        assert_eq!(
            Some(*id),
            styles::parameter(styles::ENHANCED_STYLE, name),
            "{name}"
        );
        assert_eq!(
            Some(*id),
            styles::parameter(styles::ENHANCED_STYLE_ALIAS, name),
            "{name} through the alias"
        );
    }
    assert_eq!(None, styles::parameter(styles::CLASSIC_STYLE, "spl.ao"));
    assert_eq!(None, styles::parameter(7, "spl.ao"));
}

/// The defaults of the new keys, as the amendments give them.
#[rstest]
#[case(SettingId::VideoMaxQueuedFrames, 2)]
#[case(SettingId::VideoMsaa20, 2)]
#[case(SettingId::EmuRunAhead, 1)]
#[case(SettingId::UiLibraryView, 0)]
#[case(SettingId::VrRefreshRate, 90)]
#[case(SettingId::RenderVisualStyle, 0)]
#[case(SettingId::SceneBendLr, 0)]
#[case(SettingId::SceneArcCenter, 0)]
#[case(SettingId::RenderStyleAoStrength, 3)]
#[case(SettingId::RenderStyleAoQuality, 1)]
#[case(SettingId::RenderStyleSsrQuality, 2)]
fn the_new_keys_start_at_the_default_of_their_amendment(
    #[case] id: SettingId,
    #[case] expected: i32,
) {
    assert_eq!(expected, SettingsModel::new().get(id).as_int());
}

/// AMD-059 REQ-LUA-116: `ui.library_view` has no domain at all — any value is kept.
#[test]
fn the_library_view_keeps_whatever_is_written() {
    let mut model = SettingsModel::new();
    model.set(SettingId::UiLibraryView, SettingValue::Int(7));
    assert_eq!(7, model.get(SettingId::UiLibraryView).as_int());
    model.set(SettingId::UiLibraryView, SettingValue::Int(-3));
    assert_eq!(-3, model.get(SettingId::UiLibraryView).as_int());
}

/// REQ-PERS-081: each new setting publishes the finite ordered list of its values, except the
/// library view, which has no domain.
#[test]
fn the_new_settings_publish_their_values() {
    for id in [
        SettingId::VideoMaxQueuedFrames,
        SettingId::VideoMsaa20,
        SettingId::EmuRunAhead,
        SettingId::VrRefreshRate,
        SettingId::RenderVisualStyle,
        SettingId::SceneBendLr,
        SettingId::RenderStyleAo,
    ] {
        let descriptor = catalogue::descriptor(id);
        assert!(
            descriptor.domain().options().is_some_and(|o| !o.is_empty()),
            "{}",
            descriptor.key()
        );
    }
    assert!(catalogue::descriptor(SettingId::UiLibraryView)
        .domain()
        .options()
        .is_none());
}
