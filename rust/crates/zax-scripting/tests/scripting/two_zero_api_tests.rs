//! The Lua API of 3dSen 2.0: the `CS` table, the framed distance, the new members of `Setting`
//! and the export methods of `Instance` (AMD-040 REQ-LUA-112 to REQ-LUA-115, AMD-059
//! REQ-LUA-116 and REQ-LUA-117).

use super::support::{assert_close, ScriptProfileBuilder, ScriptSession};

fn blank() -> ScriptSession {
    ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start()
}

// REQ-LUA-112: the table CS

/// The two acceptance criteria of REQ-LUA-112: the leading integer of the version is at least 2
/// and the title is not the profile editor's.
#[test]
fn req_lua_112_a_two_zero_script_sees_a_version_of_at_least_two() {
    let mut session = blank();

    assert_eq!(
        2.0,
        session.number(r#"return tonumber(CS.Nes3D.Info.Version:match("^%d+"))"#)
    );
    assert_eq!(1.0, session.truth(r#"CS.Nes3D.Info.Title ~= "3dSenMaker""#));
    assert_eq!("2.0", session.text("return CS.Nes3D.Info.Version"));
    assert_eq!("Zaxnes", session.text("return CS.Nes3D.Info.Title"));
}

/// REQ-LUA-112: any other field of the three tables is nil, without an error.
#[test]
fn req_lua_112_an_unnamed_field_of_cs_is_nil_without_an_error() {
    let mut session = blank();

    for path in [
        "CS.Nes3D.Info.AppId",
        "CS.Nes3D.Autre",
        "CS.Autre",
        "CS.Nes3D.Info.Version2",
    ] {
        assert_eq!(
            "nil",
            session.text(&format!("return type({path})")),
            "{path}"
        );
    }
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
}

/// REQ-LUA-112: a write anywhere in the three tables has no effect on the program, and none on
/// what the next read gives either.
#[test]
fn req_lua_112_a_write_into_cs_is_dropped() {
    let mut session = blank();

    session.evaluate(
        r#"
        CS.Nes3D.Info.Version = "1.0"
        CS.Nes3D.Info.Title = "3dSenMaker"
        CS.Nes3D.Info.Nouveau = 7
        CS.Nes3D = nil
        CS.Autre = 1
        "#,
    );

    assert_eq!("2.0", session.text("return CS.Nes3D.Info.Version"));
    assert_eq!("Zaxnes", session.text("return CS.Nes3D.Info.Title"));
    assert_eq!("nil", session.text("return type(CS.Nes3D.Info.Nouveau)"));
    assert_eq!("nil", session.text("return type(CS.Autre)"));
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
}

/// REQ-LUA-112: the metatable stays hidden, so a script cannot take the seal off.
#[test]
fn req_lua_112_the_metatable_of_cs_is_hidden() {
    let mut session = blank();

    assert_eq!("CS", session.text("return getmetatable(CS)"));
    assert_eq!("CS", session.text("return getmetatable(CS.Nes3D)"));
    assert_eq!("CS", session.text("return getmetatable(CS.Nes3D.Info)"));
    assert!(session
        .try_evaluate("setmetatable(CS.Nes3D.Info, {})")
        .is_err());
    assert_eq!("2.0", session.text("return CS.Nes3D.Info.Version"));
}

// REQ-LUA-113: Camera.ResetDistance

/// The acceptance criterion of REQ-LUA-113: 189.65 at 70°, unchanged by a write of
/// `Camera.Distance`, and 320.60 at 45°.
#[test]
fn req_lua_113_the_framed_distance_follows_the_field_of_view() {
    let mut session = blank();

    assert_close(189.65, session.number("return Camera.ResetDistance"), 2);

    session.evaluate("Camera.Distance = 50");
    assert_close(189.65, session.number("return Camera.ResetDistance"), 2);
    assert_close(50.0, session.number("return Camera.Distance"), 3);

    session.evaluate("Setting.Fov = 45");
    assert_close(320.60, session.number("return Camera.ResetDistance"), 2);

    session.evaluate("Setting.Fov = 95");
    assert_close(121.69, session.number("return Camera.ResetDistance"), 2);
}

/// REQ-LUA-113: it is read at any time, with no view reset needed, and it is read only — a write
/// is what SPEC-05 does with a write on a read-only member (REQ-LUA-041).
#[test]
fn req_lua_113_the_framed_distance_is_read_only() {
    let mut session = blank();

    assert!(session.try_evaluate("Camera.ResetDistance = 10").is_err());
    assert_close(189.65, session.number("return Camera.ResetDistance"), 2);
}

/// The `punchout` profile of the 2.0 reads the three members together, in a function it calls at
/// the start of a game. The shape of that read, with no error and a framing that follows
/// (REQ-LUA-112 second criterion, checked here without the profile).
#[test]
fn req_lua_112_the_shape_a_two_zero_profile_reads_runs_without_error() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "punchout-like",
            r#"
            distance = 0
            function Start()
              local version = tonumber(CS.Nes3D.Info.Version:match("^%d+"))
              if version and version >= 2 then
                distance = Camera.ResetDistance
              elseif CS.Nes3D.Info.Title == "3dSenMaker" then
                Setting.Fov = 60
              else
                Setting.Fov = 50
              end
            end
            "#,
        )
        .start();

    session.start();

    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
    assert_close(189.65, session.number("return distance"), 2);
    // The 2.0 branch was taken: the field of view was left alone.
    assert_close(70.0, session.number("return Setting.Fov"), 3);
}

// REQ-LUA-114: the new members of Setting

/// The criteria of REQ-LUA-114 on the two scalar members: a run-ahead of 7 comes back as 3, a
/// style of 5 as 0.
#[test]
fn req_lua_114_run_ahead_and_visual_style_are_brought_into_their_domain() {
    let mut session = blank();

    assert_eq!(1.0, session.number("return Setting.RunAheadFrames"));
    session.evaluate("Setting.RunAheadFrames = 7");
    assert_eq!(3.0, session.number("return Setting.RunAheadFrames"));
    session.evaluate("Setting.RunAheadFrames = -2");
    assert_eq!(0.0, session.number("return Setting.RunAheadFrames"));
    session.evaluate("Setting.RunAheadFrames = 2");
    assert_eq!(2.0, session.number("return Setting.RunAheadFrames"));

    assert_eq!(0.0, session.number("return Setting.VisualStyle"));
    session.evaluate("Setting.VisualStyle = 5");
    assert_eq!(0.0, session.number("return Setting.VisualStyle"));
    // AMD-055 REQ-SCN-078: 4 and 1 are both Enhanced, and 4 is what a read gives.
    session.evaluate("Setting.VisualStyle = 4");
    assert_eq!(4.0, session.number("return Setting.VisualStyle"));
    session.evaluate("Setting.VisualStyle = 0");
    session.evaluate("Setting.VisualStyle = 1");
    assert_eq!(4.0, session.number("return Setting.VisualStyle"));
}

/// The criterion of REQ-LUA-114 on the style parameters, value by value.
///
/// RO-1 point 7 gave the three curvature angles a finer grid for the interface and the settings
/// file (5° between −30 and 30), and left this rule exactly where it was: a **script** write still
/// rounds on the 30° grid of REQ-SCN-066, so 15 reads back 0 and 45 reads back 60, and a profile
/// written for 3dSen behaves as it always did.
#[test]
fn req_lua_114_a_style_parameter_is_normalised_as_amd_040_requires() {
    let mut session = blank();

    assert_eq!(
        1.0,
        session.truth(r#"Setting:SetVisualStyleParameter(1, "spl.bend_zx", 500)"#)
    );
    assert_eq!(
        180.0,
        session.number(r#"return Setting:GetVisualStyleParameter(1, "spl.bend_zx")"#)
    );

    for (written, expected) in [(40.0, 30.0), (45.0, 60.0), (15.0, 0.0)] {
        session.evaluate(&format!(
            r#"Setting:SetVisualStyleParameter(1, "spl.bend_zx", {written})"#
        ));
        assert_eq!(
            expected,
            session.number(r#"return Setting:GetVisualStyleParameter(1, "spl.bend_zx")"#),
            "{written}"
        );
    }
}

/// The criterion of REQ-LUA-114 on an unknown style or identifier: false, nothing recorded, and
/// a read of 0.
#[test]
fn req_lua_114_an_unknown_style_or_identifier_answers_false() {
    let mut session = blank();

    assert_eq!(
        0.0,
        session.truth(r#"Setting:SetVisualStyleParameter(0, "spl.bend_zx", 30)"#)
    );
    assert_eq!(
        0.0,
        session.truth(r#"Setting:SetVisualStyleParameter(7, "spl.bend_zx", 30)"#)
    );
    assert_eq!(
        0.0,
        session.number(r#"return Setting:GetVisualStyleParameter(1, "inconnu")"#)
    );
    assert_eq!(
        0.0,
        session.number(r#"return Setting:GetVisualStyleParameter(0, "spl.bend_zx")"#)
    );
    // Nothing was recorded by the two refused writes.
    assert_eq!(
        0.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.bend_zx")"#)
    );
}

/// REQ-LUA-114: the reset takes every parameter of the style back to its default, and leaves an
/// unknown style alone.
#[test]
fn req_lua_114_the_reset_takes_every_parameter_of_the_style_back() {
    let mut session = blank();

    session.evaluate(
        r#"
        Setting:SetVisualStyleParameter(4, "spl.bend_zx", 60)
        Setting:SetVisualStyleParameter(4, "ao.strength", 5)
        "#,
    );
    assert_eq!(
        60.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.bend_zx")"#)
    );
    assert_eq!(
        5.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "ao.strength")"#)
    );

    // An unknown style changes nothing.
    session.evaluate("Setting:ResetVisualStyleParameters(7)");
    assert_eq!(
        60.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.bend_zx")"#)
    );

    session.evaluate("Setting:ResetVisualStyleParameters(4)");
    assert_eq!(
        0.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.bend_zx")"#)
    );
    // Back to the default of its ladder, which is 3 and not 0 (AMD-055 REQ-SCN-079).
    assert_eq!(
        3.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "ao.strength")"#)
    );
}

/// REQ-LUA-114: the three methods are methods — called with `.` the receiver is missing and the
/// call raises, as REQ-LUA-081 asks of every method of `Setting`.
#[test]
fn req_lua_114_the_style_methods_are_methods() {
    let mut session = blank();

    for chunk in [
        r#"Setting.GetVisualStyleParameter(4, "spl.bend_zx")"#,
        r#"Setting.SetVisualStyleParameter(4, "spl.bend_zx", 30)"#,
        "Setting.ResetVisualStyleParameters(4)",
    ] {
        assert!(session.try_evaluate(chunk).is_err(), "{chunk}");
    }
}

// REQ-LUA-115: the export methods of Instance

/// The criterion of REQ-LUA-115: the two methods exist, do nothing and raise nothing.
#[test]
fn req_lua_115_the_export_methods_exist_and_do_nothing() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(
            &[super::support::img::a()],
            super::support::PatternSpec::tagged("t"),
        )
        .script("p", "-- nothing")
        .start();

    session.evaluate(
        r#"
        local p = Pattern.GetPatternWithTag("t")
        p:BuildExportMeshes()
        p:ReleaseExportMeshes()
        "#,
    );

    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
}

// REQ-LUA-116: the four scalar members and the list

/// The four acceptance criteria of REQ-LUA-116 on the scalar members.
#[test]
fn req_lua_116_the_four_scalar_members_normalise_what_is_written() {
    let mut session = blank();

    // MaxQueuedFrames = 5 -> 2 ; = 0 -> 1, and it is the setting of AMD-052 itself.
    assert_eq!(2.0, session.number("return Setting.MaxQueuedFrames"));
    session.evaluate("Setting.MaxQueuedFrames = 5");
    assert_eq!(2.0, session.number("return Setting.MaxQueuedFrames"));
    session.evaluate("Setting.MaxQueuedFrames = 0");
    assert_eq!(1.0, session.number("return Setting.MaxQueuedFrames"));

    // RefreshRate: the nearest of {72, 90, 120}, the smaller one at equal distance.
    assert_eq!(90.0, session.number("return Setting.RefreshRate"));
    for (written, expected) in [(100, 90.0), (81, 72.0), (105, 90.0), (200, 120.0)] {
        session.evaluate(&format!("Setting.RefreshRate = {written}"));
        assert_eq!(
            expected,
            session.number("return Setting.RefreshRate"),
            "{written}"
        );
    }

    // Msaa: v <= 1 -> 0, 2 or 3 -> 2, >= 4 -> 4, and the anti-aliasing is left alone.
    assert_eq!(2.0, session.number("return Setting.Msaa"));
    let anti_alias = session.number("return Setting.AntiAlias");
    for (written, expected) in [(3, 2.0), (1, 0.0), (8, 4.0), (-7, 0.0)] {
        session.evaluate(&format!("Setting.Msaa = {written}"));
        assert_eq!(expected, session.number("return Setting.Msaa"), "{written}");
    }
    assert_eq!(anti_alias, session.number("return Setting.AntiAlias"));

    // LibraryView = 7 -> 7.
    assert_eq!(0.0, session.number("return Setting.LibraryView"));
    session.evaluate("Setting.LibraryView = 7");
    assert_eq!(7.0, session.number("return Setting.LibraryView"));
}

/// REQ-LUA-116: no numeric write of the four members raises, whatever the number.
#[test]
fn req_lua_116_no_numeric_write_of_the_new_members_raises() {
    let mut session = blank();

    for member in [
        "MaxQueuedFrames",
        "RefreshRate",
        "Msaa",
        "LibraryView",
        "RunAheadFrames",
        "VisualStyle",
    ] {
        for value in ["0", "-1", "1e9", "-1e9", "2.5", "-0.5", "1/0", "-1/0"] {
            let chunk = format!("Setting.{member} = {value}");
            assert!(session.try_evaluate(&chunk).is_ok(), "{chunk}");
        }
    }
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
}

/// The acceptance criterion of REQ-LUA-116 on the list: two parameters written, then the list of
/// two with the style, the identifier and the normalised value of element 0; then nil empties it
/// and takes the parameters back to their default.
#[test]
fn req_lua_116_the_list_holds_the_parameters_written_since_the_last_reset() {
    let mut session = blank();

    session.evaluate(
        r#"
        Setting:ResetVisualStyleParameters(4)
        Setting:SetVisualStyleParameter(4, "spl.bend_zx", 40)
        Setting:SetVisualStyleParameter(4, "spl.ao", 2)
        "#,
    );

    assert_eq!(
        2.0,
        session.number("return Setting.VisualStyleParameters.Count")
    );
    assert_eq!(
        4.0,
        session.number("return Setting.VisualStyleParameters[0].Style")
    );
    assert_eq!(
        "spl.bend_zx",
        session.text("return Setting.VisualStyleParameters[0].ParameterId")
    );
    assert_eq!(
        30.0,
        session.number("return Setting.VisualStyleParameters[0].Value")
    );
    assert_eq!(
        "spl.ao",
        session.text("return Setting.VisualStyleParameters[1].ParameterId")
    );
    assert_eq!(
        2.0,
        session.number("return Setting.VisualStyleParameters[1].Value")
    );

    session.evaluate("Setting.VisualStyleParameters = nil");
    assert_eq!(
        0.0,
        session.number("return Setting.VisualStyleParameters.Count")
    );
    assert_eq!(
        0.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.bend_zx")"#)
    );
    assert_eq!(
        0.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.ao")"#)
    );
}

/// REQ-LUA-116: a list a read handed out is a snapshot, and writing it back puts exactly its
/// elements back, every other parameter going to its default.
#[test]
fn req_lua_116_a_list_read_before_puts_exactly_its_elements_back() {
    let mut session = blank();

    session.evaluate(
        r#"
        Setting:SetVisualStyleParameter(4, "spl.bend_zx", 60)
        Setting:SetVisualStyleParameter(4, "ao.strength", 5)
        saved = Setting.VisualStyleParameters
        -- and now the state moves on
        Setting:SetVisualStyleParameter(4, "spl.bend_zx", -30)
        Setting:SetVisualStyleParameter(4, "spl.voxel_grid", 3)
        "#,
    );

    // The list that was read did not follow the two writes that came after it.
    assert_eq!(2.0, session.number("return saved.Count"));
    assert_eq!(60.0, session.number("return saved[0].Value"));
    assert_eq!(
        3.0,
        session.number("return Setting.VisualStyleParameters.Count")
    );

    session.evaluate("Setting.VisualStyleParameters = saved");

    assert_eq!(
        2.0,
        session.number("return Setting.VisualStyleParameters.Count")
    );
    assert_eq!(
        60.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.bend_zx")"#)
    );
    assert_eq!(
        5.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "ao.strength")"#)
    );
    // The parameter that was not in the list went back to its default.
    assert_eq!(
        0.0,
        session.number(r#"return Setting:GetVisualStyleParameter(4, "spl.voxel_grid")"#)
    );
}

/// REQ-LUA-116: the order is that of the first write, and a second write of the same parameter
/// updates its value where it stands.
#[test]
fn req_lua_116_the_order_of_the_list_is_that_of_the_first_write() {
    let mut session = blank();

    session.evaluate(
        r#"
        Setting:SetVisualStyleParameter(4, "ao.strength", 5)
        Setting:SetVisualStyleParameter(4, "spl.bend_zx", 30)
        Setting:SetVisualStyleParameter(4, "ao.strength", 1)
        "#,
    );

    assert_eq!(
        2.0,
        session.number("return Setting.VisualStyleParameters.Count")
    );
    assert_eq!(
        "ao.strength",
        session.text("return Setting.VisualStyleParameters[0].ParameterId")
    );
    assert_eq!(
        1.0,
        session.number("return Setting.VisualStyleParameters[0].Value")
    );
    assert_eq!(
        "spl.bend_zx",
        session.text("return Setting.VisualStyleParameters[1].ParameterId")
    );
}

/// REQ-LUA-116: a parameter written through the alias 1 shows up under the style 4, the
/// identifier REQ-SCN-078 reads back.
#[test]
fn req_lua_116_a_parameter_written_through_the_alias_shows_up_under_four() {
    let mut session = blank();

    session.evaluate(r#"Setting:SetVisualStyleParameter(1, "spl.arc_yx", 90)"#);

    assert_eq!(
        1.0,
        session.number("return Setting.VisualStyleParameters.Count")
    );
    assert_eq!(
        4.0,
        session.number("return Setting.VisualStyleParameters[0].Style")
    );
    assert_eq!(
        "spl.arc_yx",
        session.text("return Setting.VisualStyleParameters[0].ParameterId")
    );
}

/// REQ-LUA-116: a write that is neither nil nor a list raises, the one write of the requirement
/// that may.
#[test]
fn req_lua_116_any_other_write_of_the_list_raises() {
    let mut session = blank();

    for value in ["7", r#""x""#, "{}", "Setting", "true"] {
        let chunk = format!("Setting.VisualStyleParameters = {value}");
        assert!(session.try_evaluate(&chunk).is_err(), "{chunk}");
    }
}

/// REQ-LUA-043 for the list: two reads with nothing in between give the same value, and a write
/// in between gives another.
#[test]
fn req_lua_043_two_reads_of_the_list_give_the_same_value() {
    let mut session = blank();

    session.evaluate(r#"Setting:SetVisualStyleParameter(4, "spl.bend_zx", 30)"#);
    assert_eq!(
        1.0,
        session.truth("Setting.VisualStyleParameters == Setting.VisualStyleParameters")
    );

    session.evaluate("a = Setting.VisualStyleParameters");
    session.evaluate(r#"Setting:SetVisualStyleParameter(4, "spl.bend_zx", 60)"#);
    assert_eq!(0.0, session.truth("a == Setting.VisualStyleParameters"));
}

/// REQ-LUA-117: the three tables AMD-059 adds to `CS.Nes3D`, and a style written through the
/// constant the table publishes.
#[test]
fn req_lua_117_the_three_tables_of_cs_nes3d() {
    let mut session = blank();

    assert_eq!(
        512.0,
        session.number("return CS.Nes3D.CameraControl.DistanceMax")
    );
    assert_eq!(
        2.0,
        session.number("return CS.Nes3D.SettingManager.DefaultMsaa")
    );
    assert_eq!(0.0, session.number("return CS.Nes3D.VisualStyle.Classic"));
    assert_eq!(
        4.0,
        session.number("return CS.Nes3D.VisualStyle.SoftPixelLight")
    );

    session.evaluate("Setting.VisualStyle = CS.Nes3D.VisualStyle.SoftPixelLight");
    assert_eq!(
        1.0,
        session.truth("Setting.VisualStyle == CS.Nes3D.VisualStyle.SoftPixelLight")
    );

    for path in [
        "CS.Nes3D.CameraControl.Autre",
        "CS.Nes3D.SettingManager.Autre",
        "CS.Nes3D.VisualStyle.Autre",
    ] {
        assert_eq!(
            "nil",
            session.text(&format!("return type({path})")),
            "{path}"
        );
    }
}

/// REQ-LUA-117: a write into the three new tables is dropped, as in the two of REQ-LUA-112.
#[test]
fn req_lua_117_a_write_into_the_new_tables_is_dropped() {
    let mut session = blank();

    session.evaluate(
        r"
        CS.Nes3D.CameraControl.DistanceMax = 10
        CS.Nes3D.SettingManager.DefaultMsaa = 8
        CS.Nes3D.VisualStyle.SoftPixelLight = 1
        ",
    );

    assert_eq!(
        512.0,
        session.number("return CS.Nes3D.CameraControl.DistanceMax")
    );
    assert_eq!(
        2.0,
        session.number("return CS.Nes3D.SettingManager.DefaultMsaa")
    );
    assert_eq!(
        4.0,
        session.number("return CS.Nes3D.VisualStyle.SoftPixelLight")
    );
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
}

/// End to end: a script that writes every member AMD-059 adds keeps all its hooks over the
/// frames that follow, which is the point of the amendment (REQ-LUA-062).
#[test]
fn req_lua_116_a_script_that_writes_every_new_member_keeps_its_hooks() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "writes-everything",
            r#"
            updates = 0
            function Start()
              Setting.LibraryView = 3
              Setting.MaxQueuedFrames = 5
              Setting.RefreshRate = 100
              Setting.Msaa = 3
              Setting.RunAheadFrames = 7
              Setting.VisualStyle = CS.Nes3D.VisualStyle.SoftPixelLight
              Setting:SetVisualStyleParameter(4, "spl.bend_zx", 40)
              local saved = Setting.VisualStyleParameters
              Setting.VisualStyleParameters = saved
              local _ = CS.Nes3D.CameraControl.DistanceMax
                + CS.Nes3D.SettingManager.DefaultMsaa
                + Camera.ResetDistance
            end
            function Update()
              updates = updates + 1
            end
            "#,
        )
        .start();

    session.start();
    for counter in 1..=5 {
        session.frame(&super::support::Batch::new(), counter);
    }

    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
    assert_eq!(5.0, session.number("return updates"));
}
