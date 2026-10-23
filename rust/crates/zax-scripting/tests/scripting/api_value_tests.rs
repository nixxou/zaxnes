//! The value types of SPEC-05 §4.3: construction, operators, equality tolerances and the
//! errors of their misuse (REQ-LUA-044, REQ-LUA-096, REQ-LUA-102).

use super::support::{assert_close, ScriptProfileBuilder, ScriptSession};

fn blank() -> ScriptSession {
    ScriptProfileBuilder::new().script("a", "-- nothing").load()
}

fn error_of(session: &mut ScriptSession, chunk: &str) -> String {
    session.try_evaluate(chunk).unwrap_err().to_string()
}

#[test]
fn vector3_arithmetic_follows_the_type_table_req_lua_096() {
    let mut session = blank();

    assert_eq!(
        session.number("return (Vector3(1, 2, 3) + Vector3(4, 5, 6)).z"),
        9.0
    );
    assert_eq!(
        session.number("return (Vector3(4, 5, 6) - Vector3(1, 2, 3)).y"),
        3.0
    );
    assert_eq!(session.number("return (-Vector3(1, 2, 3)).x"), -1.0);
    assert_eq!(session.number("return (2 * Vector3(1, 2, 3)).y"), 4.0);
    assert_eq!(session.number("return (Vector3(1, 2, 3) * 3).z"), 9.0);
    assert_eq!(session.number("return (Vector3(2, 4, 6) / 2).y"), 2.0);
    assert_eq!(session.number("return Vector3(1, 2, 2).sqrMagnitude"), 9.0);
    assert_eq!(
        session.number("return Vector3(0, 3, 4).normalized.z"),
        f64::from(0.8_f32)
    );
    assert_eq!(session.number("return Vector3.zero.normalized.x"), 0.0);
    assert_eq!(
        session.number("return Vector3.Dot(Vector3(1, 2, 3), Vector3(4, 5, 6))"),
        32.0
    );
    assert_eq!(
        session.number("return Vector3.Distance(Vector3(0, 0, 0), Vector3(0, 3, 4))"),
        5.0
    );
    assert_eq!(
        session.number("return Vector3.Lerp(Vector3.zero, Vector3(10, 0, 0), 0.25).x"),
        2.5
    );
    assert_eq!(
        session.number("return Vector3.Lerp(Vector3.zero, Vector3(10, 0, 0), 5).x"),
        10.0
    );
    assert_eq!(
        session.number("return Vector3.up.y + Vector3.down.y + Vector3.left.x + Vector3.right.x"),
        0.0
    );
    assert_eq!(
        session.number("return Vector3.forward.z + Vector3.back.z + Vector3.one.x"),
        1.0
    );
}

#[test]
fn vector3_errors_name_the_operation_req_lua_041() {
    let mut session = blank();

    assert!(error_of(&mut session, "return Vector3(1, 2, 3) + 1")
        .contains("invalid arguments to Vector3 +"));
    assert!(
        error_of(&mut session, "return Vector3(1, 2, 3) * Vector3(1, 2, 3)")
            .contains("invalid arguments to Vector3 *")
    );
    assert!(
        error_of(&mut session, "return Vector3(1, 2, 3) / Vector3(1, 2, 3)")
            .contains("invalid arguments to Vector3 /")
    );
    assert!(
        error_of(&mut session, "return Vector3.Cross(1, 2)").contains("invalid arguments to Cross")
    );
    assert!(error_of(&mut session, "return Vector3(1)").contains("invalid arguments to Vector3"));
    assert!(
        error_of(&mut session, "local v = Vector3(1, 2, 3); v.x = 'a'")
            .contains("invalid arguments to x")
    );
}

#[test]
fn a_vector_equality_has_a_tolerance_req_lua_102() {
    let mut session = blank();

    assert_eq!(
        session.truth("Vector3(1, 2, 3) == Vector3(1, 2, 3.000001)"),
        1.0
    );
    assert_eq!(session.truth("Vector3(1, 2, 3) == Vector3(1, 2, 3.1)"), 0.0);
    assert_eq!(session.truth("Vector2(1, 2) == Vector2(1, 2)"), 1.0);
    assert_eq!(session.truth("Vector2(1, 2) ~= Vector2(1, 3)"), 1.0);
    assert_eq!(session.truth("Color(1, 0, 0) == Color(1, 0, 0, 1)"), 1.0);
    assert_eq!(session.truth("Color(1, 0, 0) == Color(1, 0, 0.0001)"), 0.0);
    assert_eq!(session.truth("IntVector2(1, 2) == IntVector2(1, 2)"), 1.0);
    assert_eq!(
        session.truth("IntVector3(1, 2, 3) == IntVector3(1, 2, 3)"),
        1.0
    );
    assert_eq!(session.truth("Rect(1, 2, 3, 4) == Rect(1, 2, 3, 4)"), 1.0);
    assert_eq!(session.truth("Rect(1, 2, 3, 4) == Rect(1, 2, 3, 5)"), 0.0);
    assert_eq!(
        session.truth("Quaternion.Euler(0, 90, 0) == Quaternion.AngleAxis(90, Vector3(0, 1, 0))"),
        1.0
    );
}

#[test]
fn a_value_is_a_copy_of_what_it_was_read_from_req_lua_044() {
    let mut session = blank();

    session.evaluate(
        "local v = Vector3(1, 2, 3); local w = Vector3(v.x, v.y, v.z); w.x = 9; a, b = v.x, w.x",
    );

    assert_eq!(session.number("return a"), 1.0);
    assert_eq!(session.number("return b"), 9.0);
}

#[test]
fn vector2_arithmetic_and_conversions() {
    let mut session = blank();

    assert_eq!(
        session.number("return (Vector2(1, 2) + Vector2(3, 4)).y"),
        6.0
    );
    assert_eq!(
        session.number("return (Vector2(1, 2) - Vector2(3, 4)).x"),
        -2.0
    );
    assert_eq!(session.number("return (-Vector2(1, 2)).y"), -2.0);
    assert_eq!(session.number("return (3 * Vector2(1, 2)).y"), 6.0);
    assert_eq!(session.number("return (Vector2(1, 2) * 3).x"), 3.0);
    assert_eq!(session.number("return (Vector2(4, 2) / 2).x"), 2.0);
    assert_eq!(session.number("return Vector2(3, 4).magnitude"), 5.0);
    assert_eq!(session.number("return Vector2.one.x + Vector2.zero.y"), 1.0);
    assert_eq!(session.number("return Vector2().x"), 0.0);
    assert_eq!(
        session.number("return (Vector2(1, 2) + Vector3(3, 4, 5)).x"),
        4.0
    );
    assert!(
        error_of(&mut session, "return Vector2(1, 2) * Vector2(1, 2)")
            .contains("invalid arguments to Vector2 *")
    );
    assert!(error_of(&mut session, "return Vector2(1, 2) / 'a'")
        .contains("invalid arguments to Vector2 /"));
}

#[test]
fn quaternions_compose_and_interpolate_req_lua_096() {
    let mut session = blank();

    assert_eq!(session.number("return Quaternion.identity.w"), 1.0);
    assert_close(
        90.0,
        session.number("return Quaternion.Euler(Vector3(0, 90, 0)).eulerAngles.y"),
        4,
    );
    assert_close(
        45.0,
        session.number(
            "return Quaternion.Slerp(Quaternion.identity, Quaternion.Euler(0, 90, 0), 0.5).eulerAngles.y",
        ),
        3,
    );
    assert_close(
        45.0,
        session.number(
            "return Quaternion.Lerp(Quaternion.identity, Quaternion.Euler(0, 90, 0), 0.5).eulerAngles.y",
        ),
        1,
    );
    assert_eq!(
        session.number("return Quaternion.AngleAxis(30, Vector3.zero).w"),
        1.0
    );
    session.evaluate("local q = Quaternion.identity; q.x = 0.5; changed = q.x");
    assert_eq!(session.number("return changed"), 0.5);
    assert!(error_of(&mut session, "return Quaternion.identity * 2")
        .contains("invalid arguments to Quaternion *"));
    assert!(error_of(&mut session, "return Quaternion.Slerp(1, 2, 3)")
        .contains("invalid arguments to Slerp"));
}

#[test]
fn colours_add_scale_and_multiply() {
    let mut session = blank();

    assert_eq!(
        session.number("return (Color(1, 0, 0) + Color(0, 1, 0)).g"),
        1.0
    );
    assert_eq!(
        session.number("return (Color(1, 1, 1) - Color(0, 1, 0)).g"),
        0.0
    );
    assert_eq!(session.number("return (Color(1, 1, 1) * 0.5).b"), 0.5);
    assert_eq!(session.number("return (0.5 * Color(1, 1, 1)).a"), 0.5);
    assert_eq!(
        session.number("return (Color(0.5, 1, 1) * Color(0.5, 1, 1)).r"),
        0.25
    );
    assert_eq!(session.number("return Color.black.a + Color.white.r"), 2.0);
    session.evaluate("local c = Color(0.1, 0.2, 0.3); c.r = 1; c.a = 0.5; r, a = c.r, c.a");
    assert_eq!(session.number("return r"), 1.0);
    assert_eq!(session.number("return a"), 0.5);
}

#[test]
fn integer_vectors_truncate_and_divide() {
    let mut session = blank();

    assert_eq!(
        session.number("return (IntVector2(1, 2) + IntVector2(3, 4)).y"),
        6.0
    );
    assert_eq!(
        session.number("return (IntVector2(5, 5) - IntVector2(3, 4)).y"),
        1.0
    );
    assert_eq!(session.number("return (IntVector2(7, -7) / 2).y"), -3.0);
    assert_eq!(
        session.number("return IntVector2(1.9, -1.9).x + IntVector2(1.9, -1.9).y"),
        0.0
    );
    assert_eq!(
        session.number("return IntVector2.one.x + IntVector2.zero.y"),
        1.0
    );
    assert_eq!(session.number("return IntVector2().x"), 0.0);
    assert_eq!(session.number("return IntVector3(1, 2, 3).z"), 3.0);
    session.evaluate("local v = IntVector2(1, 2); v.x = 7.9; got = v.x");
    assert_eq!(session.number("return got"), 7.0);
    session.evaluate("local v = IntVector3(1, 2, 3); v.z = 9; got3 = v.z");
    assert_eq!(session.number("return got3"), 9.0);
    assert!(error_of(&mut session, "return IntVector2(1, 2) / 0").contains("IntVector2 / 0"));
    assert!(error_of(&mut session, "return IntVector2(1, 2) / 'a'")
        .contains("invalid arguments to IntVector2 /"));
    assert_eq!(session.text("return tostring(IntVector2(3, 4))"), "(3, 4)");
}

#[test]
fn rectangles_overlap_strictly() {
    let mut session = blank();

    assert_eq!(
        session.truth("Rect(0, 0, 10, 10):Overlaps(Rect(5, 5, 10, 10))"),
        1.0
    );
    assert_eq!(
        session.truth("Rect(0, 0, 10, 10):Overlaps(Rect(20, 20, 5, 5))"),
        0.0
    );
    session.evaluate("local r = Rect(1, 2, 3, 4); r.width = 30; w = r.width");
    assert_eq!(session.number("return w"), 30.0);
    assert!(
        error_of(&mut session, "return Rect(1, 2, 3, 4):Overlaps(5)")
            .contains("invalid arguments to Overlaps")
    );
    assert!(error_of(&mut session, "return Rect(1, 2, 3)").contains("invalid arguments to Rect"));
}

#[test]
fn an_unknown_member_of_a_value_reads_nil_and_a_write_raises_req_lua_042() {
    let mut session = blank();

    assert_eq!(session.text("return type(Vector3(1, 2, 3).w)"), "nil");
    assert!(
        error_of(&mut session, "local v = Vector3(1, 2, 3); v.w = 1")
            .contains("cannot set w, no such field")
    );
    assert!(
        error_of(&mut session, "local v = Vector3(1, 2, 3); v.magnitude = 1")
            .contains("cannot set magnitude, it is read only")
    );
    assert!(
        error_of(&mut session, "return Vector3(1, 2, 3) < Vector3(1, 2, 3)")
            .contains("attempt to compare")
    );
    assert!(error_of(&mut session, "return Frame + 1")
        .contains("attempt to perform arithmetic on a userdata value"));
    assert!(error_of(&mut session, "return Frame()").contains("attempt to call a userdata value"));
}

#[test]
fn a_bound_vector_is_accepted_wherever_a_vector_is_expected_req_lua_044() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    session.evaluate("Camera.Rot.x = 1; Camera.Rot.y = 2; Camera.Rot.z = 3");
    assert_eq!(session.number("return (Camera.Rot + Vector3.one).z"), 4.0);
    assert_eq!(
        session.number("return Vector3.Dot(Camera.Rot, Vector3.one)"),
        6.0
    );
    assert_eq!(session.number("return Vector2(Camera.Rot.x, 0).x"), 1.0);
    assert_eq!(session.truth("Camera.Rot.content == Vector3(1, 2, 3)"), 1.0);
    assert_eq!(session.text("return tostring(Camera.Rot)"), "(1, 2, 3)");
    assert_eq!(session.truth("Camera.Rot == Camera.Rot"), 1.0);
    assert!(error_of(&mut session, "return Camera.Rot[3]")
        .contains("index 3 is out of range for Vec3R"));
    assert!(
        error_of(&mut session, "Camera.Rot[3] = 1").contains("index 3 is out of range for Vec3R")
    );
}
