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
