//! The three members 3dSen 2.1 adds and the version it reports (AMD-063 REQ-LUA-118 to
//! REQ-LUA-121).

use zax_engine3d::engine::ConversionEngine;

use super::support::{img, Batch, PatternSpec, ScriptProfileBuilder, ScriptSession};

fn blank() -> ScriptSession {
    ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start()
}

// REQ-LUA-118: the version is "2.1", and the criterion of AMD-040 still holds.
#[test]
fn req_lua_118_the_version_is_two_one() {
    let mut session = blank();

    assert_eq!("2.1", session.text("return CS.Nes3D.Info.Version"));
    assert_eq!(
        1.0,
        session.truth(r#"tonumber(CS.Nes3D.Info.Version:match("^%d+")) >= 2"#)
    );
}

// REQ-LUA-119, acceptance criterion: writing `RotOverride` imposes the rotation of the current
// mode at once, reading it gives a `Vector3` copy, and `nil` gives the camera its behaviour back
// while the setpoint stays where it was left.
#[test]
fn req_lua_119_rot_override_imposes_the_rotation_of_the_current_mode() {
    let mut session = blank();

    assert_eq!(
        "nil",
        session.text("return type(CS.Nes3D.CameraControl.RotOverride)")
    );

    session.evaluate("CS.Nes3D.CameraControl.RotOverride = Vector3(-60, 10, 0)");

    // Camera.Rot is the orientation of the current mode (REQ-LUA-088): the forced value.
    assert_eq!(-60.0, session.number("return Camera.Rot.x"));
    assert_eq!(10.0, session.number("return Camera.Rot.y"));
    assert_eq!(0.0, session.number("return Camera.Rot.z"));

    // Reading gives a Vector3 copy, not a bound vector: writing into it steers nothing.
    assert_eq!(
        "Vector3",
        session.text("return CS.Nes3D.CameraControl.RotOverride.__name or \"Vector3\"")
    );
    assert_eq!(
        -60.0,
        session.number("return CS.Nes3D.CameraControl.RotOverride.x")
    );
    session.evaluate("local copy = CS.Nes3D.CameraControl.RotOverride; copy.x = 10");
    assert_eq!(
        -60.0,
        session.number("return CS.Nes3D.CameraControl.RotOverride.x")
    );

    // Back to nil: the field reads nil and the camera stays where it was left.
    session.evaluate("CS.Nes3D.CameraControl.RotOverride = nil");
    assert_eq!(
        "nil",
        session.text("return type(CS.Nes3D.CameraControl.RotOverride)")
    );
    assert_eq!(-60.0, session.number("return Camera.Rot.x"));
    assert_eq!(10.0, session.number("return Camera.Rot.y"));

    // REQ-LUA-117 is intact: the other field of the table still reads, and an unnamed one is nil.
    assert_eq!(
        512.0,
        session.number("return CS.Nes3D.CameraControl.DistanceMax")
    );
    assert_eq!(
        "nil",
        session.text("return type(CS.Nes3D.CameraControl.Autre)")
    );
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
}

// REQ-LUA-119 and Q-180: the write accepts what any `Vector3` write of the API accepts — a bound
// vector and a `Vector2` at z = 0 — and anything else is a Lua error. A write of another field of
// `CameraControl` is still dropped (REQ-LUA-117).
#[test]
fn req_lua_119_rot_override_accepts_what_a_vector3_write_accepts() {
    let mut session = blank();

    // A bound vector of the API: the camera's own rotation.
    session.evaluate("Camera.Rot.x = 12; CS.Nes3D.CameraControl.RotOverride = Camera.Rot");
    assert_eq!(12.0, session.number("return Camera.Rot.x"));

    // A Vector2 counts as z = 0.
    session.evaluate("CS.Nes3D.CameraControl.RotOverride = Vector2(-30, 5)");
    assert_eq!(-30.0, session.number("return Camera.Rot.x"));
    assert_eq!(5.0, session.number("return Camera.Rot.y"));
    assert_eq!(0.0, session.number("return Camera.Rot.z"));

    // Anything else raises.
    for value in ["7", "\"x\"", "{}", "true"] {
        assert!(
            session
                .try_evaluate(&format!("CS.Nes3D.CameraControl.RotOverride = {value}"))
                .is_err(),
            "{value} should be refused"
        );
    }

    // The forced value is the last one accepted, and the other fields are untouched.
    assert_eq!(
        -30.0,
        session.number("return CS.Nes3D.CameraControl.RotOverride.x")
    );
    session.evaluate("CS.Nes3D.CameraControl.DistanceMax = 10");
    assert_eq!(
        512.0,
        session.number("return CS.Nes3D.CameraControl.DistanceMax")
    );
}

// REQ-LUA-120: `HasPlaceholders` is false everywhere for a 3dn profile — the pattern of the
// registry, the frame pattern of a shape and the one `Pattern.GetPatternWithTag` gives.
#[test]
fn req_lua_120_has_placeholders_is_false_for_a_3dn_profile() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("water"))
        .script(
            "a",
            r"
            function Update()
              local shape = Frame:GetShape(0)
              reference = shape.refPattern.HasPlaceholders
              frame_pattern = shape.pattern.HasPlaceholders
              tagged = Pattern.GetPatternWithTag('water').pattern.HasPlaceholders
            end
            ",
        )
        .start();

    let batch = Batch::new().bg_cell(img::a(), 10, 10);
    session.frame(&batch, 0);

    assert_eq!("false", session.text("return tostring(reference)"));
    assert_eq!("false", session.text("return tostring(frame_pattern)"));
    assert_eq!("false", session.text("return tostring(tagged)"));
}

// REQ-LUA-121: `IsSolidTile` answers false for every argument, raises nothing, and the hooks of
// the script that calls it go on running (REQ-LUA-062).
#[test]
fn req_lua_121_is_solid_tile_is_always_false() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::default())
        .script(
            "a",
            r"
            updates = 0
            answers = ''
            function Update()
              updates = updates + 1
              local shape = Frame:GetShape(0)
              answers = tostring(shape:IsSolidTile(nil))
                .. ',' .. tostring(shape:IsSolidTile(3))
                .. ',' .. tostring(shape:IsSolidTile('x'))
            end
            ",
        )
        .start();

    let batch = Batch::new().bg_cell(img::a(), 10, 10);
    session.frame(&batch, 0);
    session.frame(&batch, 1);

    assert_eq!("false,false,false", session.text("return answers"));
    assert_eq!(2.0, session.number("return updates"));
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
}

// The engine side of REQ-LUA-120: a pattern of a 3dn profile never has a placeholder.
#[test]
fn req_lua_120_a_3dn_pattern_has_no_placeholder() {
    let session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::default())
        .start();
    let engine: &ConversionEngine = &session.engine;
    let id = engine.patterns().permanent()[0];
    assert!(!engine.patterns().get(id).unwrap().has_placeholders());
}
