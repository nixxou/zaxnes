//! First end-to-end checks of the runtime and of the binding layer.

use zax_engine3d::settings::ZLayer;

use super::support::{img, Batch, PatternSpec, ScriptProfileBuilder};

#[test]
fn a_chunk_runs_and_its_globals_survive() {
    let mut session = ScriptProfileBuilder::new()
        .script("a", "n = (n or 0) + 1\nfunction Start() started = n end")
        .start();

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_eq!(session.number("return started"), 1.0);
}

#[test]
fn globals_of_the_api_are_installed() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    assert_eq!(session.text("return type(Frame)"), "userdata");
    assert_eq!(session.text("return type(Setting)"), "userdata");
    assert_eq!(session.text("return type(TLayer)"), "table");
    assert_eq!(session.number("return TLayer.L2"), 3.0);
    assert_eq!(session.text("return type(io)"), "nil");
    assert_eq!(session.text("return type(unpack)"), "function");
}

#[test]
fn value_types_build_and_compare() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    assert_eq!(session.number("return Vector3(1, 2, 3).x"), 1.0);
    assert_eq!(
        session.number("if Vector3(1, 2) == Vector3(1, 2, 0) then return 1 else return 0 end"),
        1.0
    );
    assert_eq!(session.number("return Vector3(3, 4, 0).magnitude"), 5.0);
    assert_eq!(
        session.number("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).y"),
        -1.0
    );
    assert_eq!(session.number("return Color(1, 0, 0).a"), 1.0);
}

#[test]
fn settings_are_live() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    session.evaluate("Setting.EmptyTile = true");
    assert!(session.engine.settings().empty_tile);

    session.evaluate("Setting.Layer[3].CastShadow = true");
    assert!(
        session
            .engine
            .settings()
            .layer(ZLayer::from_raw(3))
            .cast_shadow
    );

    session.evaluate("Clipping[1] = 16");
    assert_eq!(session.engine.settings().clipping.y, 16.0);

    assert_eq!(
        session.number("if Light == Setting.Light then return 1 else return 0 end"),
        1.0
    );
}

/// SPEC-09 REQ-PERS-008 §4.1.10 with AMD-020 (Q-052): the three static members of
/// `app.engine_mode` are exposed in Lua, read only, and say that the engine always runs on a
/// profile — the ROM mode is the editor's.
#[test] // REQ-PERS-008, Q-052
fn the_engine_mode_members_of_paragraph_4_1_10_are_exposed_read_only() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    assert_eq!(session.text("return type(Setting.EngineMode)"), "number");
    assert_eq!(session.number("return Setting.EngineMode"), 1.0);
    assert_eq!(
        session.number("if Setting.IsProfileMode then return 1 else return 0 end"),
        1.0
    );
    assert_eq!(
        session.number("if Setting.IsRomMode then return 1 else return 0 end"),
        0.0
    );

    // Read only: an assignment is refused, and the value does not move.
    assert!(session.try_evaluate("Setting.EngineMode = 0").is_err());
    assert_eq!(session.number("return Setting.EngineMode"), 1.0);
}

#[test]
fn frame_hooks_see_the_instances() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script(
            "a",
            r"
            seen = 0
            function UpdateS()
              seen = seen + 1
              lastTag = shape.TagStr
              offsetX = shape.Offset.x
              shape.Offset.x = 5
              afterX = shape.Offset.x
            end
            ",
        )
        .start();

    session.frame(&Batch::new().bg_cell(img::a(), 10, 5), 0);

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_eq!(session.number("return seen"), 1.0);
    assert_eq!(session.text("return lastTag"), "t");
    assert_eq!(session.number("return offsetX"), 0.0);
    assert_eq!(session.number("return afterX"), 5.0);
}

#[test]
fn unknown_members_read_as_nil_and_writes_raise() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script(
            "a",
            r"
            function UpdateS()
              typo = shape.hiden
              ok, err = pcall(function () shape.NoSuch = 1 end)
            end
            ",
        )
        .start();

    session.frame(&Batch::new().bg_cell(img::a(), 10, 5), 0);

    assert_eq!(session.text("return type(typo)"), "nil");
    assert_eq!(session.number("if ok then return 1 else return 0 end"), 0.0);
    assert!(session
        .text("return err")
        .contains("cannot set NoSuch, no such field"));
}

#[test]
fn numbers_from_the_host_are_floats() {
    let mut session = ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start();
    session.runtime(|runtime, _| {
        zax_engine3d::script::ScriptHost::set_frame_globals(runtime, 120, 2.0).unwrap();
    });

    assert_eq!(session.text("return math.type(frame)"), "float");
    assert_eq!(session.text("return tostring(frame)"), "120");
    assert!(session
        .number("return math.fmod(5, Frame.shapeCount)")
        .is_nan());
}

#[test]
fn a_hook_error_disables_the_script_and_the_game_goes_on() {
    let mut session = ScriptProfileBuilder::new()
        .script("e", "function Update() local x = nil; return x.y end")
        .script("g", "count = 0\nfunction Update() count = count + 1 end")
        .start();

    session.frame(&Batch::new(), 1);
    session.frame(&Batch::new(), 2);
    session.frame(&Batch::new(), 3);

    let errors = session.log.errors();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("attempt to index"));
    assert!(errors[0].contains("'e'"));
    assert_eq!(session.number("return count"), 3.0);
    assert!(session.script_failed("e"));
}

#[test]
fn identity_holds_across_accesses() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script(
            "a",
            r"
            function UpdateS()
              sameShape = shape.Shape2D == shape.Shape2D
              sameFrame = Frame:GetShape(0) == Frame:GetShape(0)
              asKey = {}
              asKey[shape.Shape2D] = 7
              keyed = asKey[shape.Shape2D]
            end
            ",
        )
        .start();

    session.frame(&Batch::new().bg_cell(img::a(), 10, 5), 0);

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_eq!(
        session.number("if sameShape then return 1 else return 0 end"),
        1.0
    );
    assert_eq!(
        session.number("if sameFrame then return 1 else return 0 end"),
        1.0
    );
    assert_eq!(session.number("return keyed"), 7.0);
}

#[test]
fn print_writes_the_dialect_format() {
    let session = ScriptProfileBuilder::new()
        .script("a", r#"print("frame", 12.0, 0.4, true, nil)"#)
        .load();

    assert_eq!(session.log.prints(), ["frame\t12\t0.4\ttrue\tnil"]);
}

#[test]
fn build_pattern_runs_before_the_geometry() {
    let session = ScriptProfileBuilder::new()
        .pattern(&[img::a(), img::b()], PatternSpec::tagged("gear"))
        .script_with(
            "build",
            "function BuildPattern() pattern.fillrate = 0.05 end",
            "gear",
            true,
        )
        .load();

    let pattern = session.engine.patterns().permanent()[0];
    let fill_rate = session.engine.patterns().get(pattern).unwrap().instances()[0].fill_rate;
    assert!((fill_rate - 0.05).abs() < 1e-6);
}
