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
