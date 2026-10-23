//! The test vectors of SPEC-05 §5.2, TV-LUA-01 to TV-LUA-16.

use zax_scripting::persistent;

use super::support::{assert_close, img, Batch, PatternSpec, ScriptProfileBuilder, ScriptSession};

fn truth(session: &mut ScriptSession, expression: &str) -> f64 {
    session.truth(expression)
}

fn assert_vector(session: &mut ScriptSession, name: &str, x: f64, y: f64, z: f64) {
    assert_close(x, session.number(&format!("return {name}.x")), 4);
    assert_close(y, session.number(&format!("return {name}.y")), 4);
    assert_close(z, session.number(&format!("return {name}.z")), 4);
}

/// TV-LUA-01: two scripts `b` (tags `t`) and `a` (tags `*`) in that file order, a frame with two
/// scriptable instances `i1` (tag `t`) and `i2` (tag `u`). The chunks run in file order, the
/// hooks in script order, and the `UpdateS` calls are grouped per script.
#[test]
fn tv_lua_01_hook_order() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .pattern(&[img::b()], PatternSpec::tagged("u"))
        .script_with(
            "b",
            r#"
            log[#log + 1] = "b.chunk"
            function Start() log[#log + 1] = "b.Start" end
            function Update() log[#log + 1] = "b.Update" end
            function UpdateS() log[#log + 1] = "b.UpdateS(" .. shape.TagStr .. ")" end
            function LateUpdate() log[#log + 1] = "b.LateUpdate" end
            "#,
            "t",
            true,
        )
        .script_with(
            "a",
            r#"
            log[#log + 1] = "a.chunk"
            function Start() log[#log + 1] = "a.Start" end
            function Update() log[#log + 1] = "a.Update" end
            function UpdateS() log[#log + 1] = "a.UpdateS(" .. shape.TagStr .. ")" end
            function LateUpdate() log[#log + 1] = "a.LateUpdate" end
            "#,
            "*",
            true,
        )
        .prelude("log = {}")
        .load();

    session.start();
    session.frame(
        &Batch::new()
            .bg_cell(img::a(), 4, 4)
            .bg_cell(img::b(), 12, 4),
        1,
    );

    assert_eq!(
        session.log_entries(),
        [
            "b.chunk",
            "a.chunk",
            "a.Start",
            "b.Start",
            "a.Update",
            "b.Update",
            "a.UpdateS(t)",
            "a.UpdateS(u)",
            "b.UpdateS(t)",
            "a.LateUpdate",
            "b.LateUpdate",
        ]
    );
}

/// REQ-LUA-002: a script whose tags match no instance still gets `Start`, `Update` and
/// `LateUpdate`, and never `UpdateS`.
#[test]
fn a_script_linked_to_nothing_still_gets_the_frame_hooks_req_lua_002() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script_with(
            "only",
            r#"
            function Start() log[#log + 1] = "Start" end
            function Update() log[#log + 1] = "Update" end
            function UpdateS() log[#log + 1] = "UpdateS" end
            function LateUpdate() log[#log + 1] = "LateUpdate" end
            "#,
            "nothing",
            true,
        )
        .prelude("log = {}")
        .load();

    session.start();
    session.frame(&Batch::new().bg_cell(img::a(), 4, 4), 1);

    assert_eq!(session.log_entries(), ["Start", "Update", "LateUpdate"]);
}

/// TV-LUA-02 and REQ-LUA-012: two scripts that define the same non-reserved global function
/// share the last definition, and the first script's hook calls that one.
#[test]
fn tv_lua_02_function_collision() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "bg",
            r#"
            function Connect() return "bg" end
            function Start() picked = Connect() end
            "#,
        )
        .script(
            "foot_body",
            r#"function Connect(foot, body) return "foot_body" end"#,
        )
        .start();

    assert_eq!(session.text("return picked"), "foot_body");
}

/// TV-LUA-03 and REQ-LUA-007: a disabled script gets `Start` and `End` but no frame hook, and
/// what its `Start` did stays in place.
#[test]
fn tv_lua_03_disabled_script() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script_with(
            "hud",
            r#"
            function Start()
              Setting.Layer[3].CastShadow = true
              log[#log + 1] = "Start"
            end
            function Update() log[#log + 1] = "Update" end
            function UpdateS() log[#log + 1] = "UpdateS" end
            function End() log[#log + 1] = "End" end
            "#,
            "*",
            false,
        )
        .prelude("log = {}")
        .load();

    session.start();
    session.frame(&Batch::new().bg_cell(img::a(), 4, 4), 1);
    session.end();

    assert!(
        session
            .engine
            .settings()
            .layer(zax_engine3d::settings::ZLayer::from_raw(3))
            .cast_shadow
    );
    assert_eq!(session.log_entries(), ["Start", "End"]);
}

/// REQ-LUA-007: a script may switch another on; the change applies from the next hook call.
#[test]
fn a_script_can_switch_another_on_req_lua_007() {
    let mut session = ScriptProfileBuilder::new()
        .script_with(
            "ov",
            "function Update() log[#log + 1] = 'ov' end",
            "*",
            false,
        )
        .script(
            "scene",
            r#"
            function Start() sOv = Script:Get("ov") end
            function Update()
              log[#log + 1] = "scene"
              sOv.Enable = scene == 1
            end
            "#,
        )
        .prelude("log = {}\nscene = 0")
        .start();

    session.frame(&Batch::new(), 1);
    session.evaluate("scene = 1");
    session.frame(&Batch::new(), 2);
    session.frame(&Batch::new(), 3);

    assert_eq!(session.log_entries(), ["scene", "scene", "ov", "scene"]);
}

/// TV-LUA-04: `Save` captures `PData[1]` to `PData[64]` as single-precision reals, with a
/// numeric string converted and anything else stored as 0; `PData[0]` is neither saved nor
/// restored.
#[test]
fn tv_lua_04_persistent_data() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "p",
            r#"
            function Save()
              PData[1] = 0.1
              PData[0] = 7
              PData[2] = "12"
              PData[3] = true
              PData[4] = nil
            end
            function Load() loaded = 1 end
            "#,
        )
        .start();

    session.save();
    let block = session.runtime(|runtime, _| runtime.capture_persistent_data().unwrap());
    assert_eq!(block.len(), persistent::BLOCK_SIZE);

    session.evaluate("PData[0] = 99; PData[1] = 5");
    session.runtime(|runtime, _| runtime.restore_persistent_data(&block).unwrap());
    session.load();

    let read = |session: &mut ScriptSession, index: i64| {
        session.runtime(|runtime, _| runtime.read_persistent_data(index).unwrap())
    };
    assert_close(0.100_000_001_490_116, read(&mut session, 1), 12);
    assert_eq!(read(&mut session, 2), 12.0);
    assert_eq!(read(&mut session, 3), 0.0);
    assert_eq!(read(&mut session, 4), 0.0);
    assert_eq!(read(&mut session, 0), 99.0);
    assert_eq!(session.text("return math.type(PData[2])"), "float");
    assert_eq!(session.number("return loaded"), 1.0);
}

/// REQ-LUA-070: the shape of `PData` and the two helper functions.
#[test]
fn persistent_data_helpers_req_lua_070() {
    let mut session = ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start();

    assert_eq!(session.number("return #PData"), 64.0);
    assert_eq!(session.number("return pdata_count"), 64.0);
    session.evaluate("PData[1] = 11; SetPData(64, 5)");
    assert_eq!(session.number("return GetPData(0)"), 5.0);
    assert_eq!(session.number("return PData[1]"), 5.0);
}

/// TV-LUA-05: the positions of a background instance of one tile at `pStart = (16, 32)` with
/// depth 8, layer L1 and centred pivots.
#[test]
fn tv_lua_05_positions() {
    let mut session = ScriptProfileBuilder::new()
        .grid(
            2,
            2,
            &[img::a(), img::b(), img::c(), img::d()],
            PatternSpec {
                tags: Some("t".to_owned()),
                depth: 8,
                ..PatternSpec::default()
            },
        )
        .script(
            "p",
            r"
            function UpdateS()
              bl = shape.BottomLeft
              tr = shape.TopRight
              bl2 = shape.BL
              tr2 = shape.TR
              size = shape.size
              ori = shape.OriPos
              pos = shape:GetPos()
            end
            ",
        )
        .start();

    // The four cells of the pattern at screen (16, 32) to (31, 47), the box TV-LUA-05
    // describes.
    session.frame(
        &Batch::new()
            .bg_cell(img::a(), 2, 4)
            .bg_cell(img::b(), 3, 4)
            .bg_cell(img::c(), 2, 5)
            .bg_cell(img::d(), 3, 5),
        1,
    );

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_vector(&mut session, "bl", 16.0, 201.0, -4.0);
    assert_vector(&mut session, "tr", 31.0, 216.0, 4.0);
    assert_vector(&mut session, "bl2", 16.0, 200.0, -4.0);
    assert_vector(&mut session, "tr2", 31.0, 215.0, 4.0);
    assert_vector(&mut session, "size", 16.0, 16.0, 8.0);
    assert_vector(&mut session, "ori", 23.5, 208.5, 0.0);
    assert_vector(&mut session, "pos", 24.0, 208.0, 4.0);
}

/// TV-LUA-06: the strict overlap test and its slack.
#[test]
fn tv_lua_06_overlap() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("a"))
        .pattern(&[img::b()], PatternSpec::tagged("b"))
        .script(
            "p",
            r#"
            function LateUpdate()
              local ia = Frame:GetShapeWithTag("a")
              local ib = Frame:GetShapeWithTag("b")
              plain = ia:Overlap(ib)
              slack = ia:Overlap(ib, 1)
            end
            "#,
        )
        .start();

    // A at pStart (0, 0) pEnd (7, 7); B at pStart (7, 0) pEnd (14, 7).
    session.frame(
        &Batch::new().sprite(img::a(), 0, 0).sprite(img::b(), 7, 0),
        1,
    );
    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_ne!(session.truth("plain"), 1.0);
    assert_eq!(session.truth("slack"), 1.0);

    // B moved to pStart (9, 0): even with the slack the boxes no longer touch.
    session.frame(
        &Batch::new().sprite(img::a(), 0, 0).sprite(img::b(), 9, 0),
        2,
    );
    assert_ne!(session.truth("slack"), 1.0);
}

/// TV-LUA-07: the deformation a script predicts through `GetDeform`.
#[test]
fn tv_lua_07_deformation() {
    let mut session = ScriptProfileBuilder::new()
        .grid(
            2,
            2,
            &[img::a(), img::b(), img::c(), img::d()],
            PatternSpec {
                tags: Some("t".to_owned()),
                depth: 8,
                ..PatternSpec::default()
            },
        )
        .script(
            "p",
            r"
            function UpdateS()
              shape.DeformAmp.content = Vector3(0, 0, 1)
              for r = 0, 2 do
                for c = 0, 3 do
                  shape.DeformSpeed[r + 4 * c] = 0
                end
              end
              shape.DeformSpeed.m23 = 2
              time = 0.25
              d = shape:GetDeform(shape.OriPos)
            end
            ",
        )
        .start();

    session.frame(
        &Batch::new()
            .bg_cell(img::a(), 2, 4)
            .bg_cell(img::b(), 3, 4)
            .bg_cell(img::c(), 2, 5)
            .bg_cell(img::d(), 3, 5),
        1,
    );

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_close(0.0, session.number("return d.x"), 4);
    assert_close(0.0, session.number("return d.y"), 4);
    assert_close(7.02066, session.number("return d.z"), 4);
}

/// TV-LUA-08: the vector type.
#[test]
fn tv_lua_08_vectors() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    assert_eq!(
        truth(&mut session, "Vector3(1, 2) == Vector3(1, 2, 0)"),
        1.0
    );
    assert_eq!(
        session.number("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).x"),
        0.0
    );
    assert_eq!(
        session.number("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).y"),
        -1.0
    );
    assert_eq!(
        session.number("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).z"),
        0.0
    );
    assert_eq!(session.number("return Vector3(3, 4, 0).magnitude"), 5.0);
    assert_eq!(session.number("return (Vector3(1, 1, 1) * 2).y"), 2.0);
}

/// TV-LUA-09: the quaternion type.
#[test]
fn tv_lua_09_quaternions() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    assert_close(
        0.0,
        session.number("return Quaternion.Euler(0, 90, 0).eulerAngles.x"),
        4,
    );
    assert_close(
        90.0,
        session.number("return Quaternion.Euler(0, 90, 0).eulerAngles.y"),
        4,
    );
    assert_close(
        0.0,
        session.number("return Quaternion.Euler(0, 90, 0).eulerAngles.z"),
        4,
    );
    assert_close(
        90.0,
        session.number(
            "return (Quaternion.Euler(0, 0, 30) * Quaternion.Euler(0, 0, 60)).eulerAngles.z",
        ),
        4,
    );
    assert_close(
        330.0,
        session.number("return Quaternion.Euler(-30, 0, 0).eulerAngles.x"),
        4,
    );
    assert_close(
        0.0,
        session.number("return (Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0)).x"),
        4,
    );
    assert_close(
        -1.0,
        session.number("return (Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0)).z"),
        4,
    );
    assert_close(
        90.0,
        session.number(
            "return Quaternion.Lerp(Quaternion.identity, Quaternion.Euler(0, 90, 0), 2).eulerAngles.y",
        ),
        4,
    );
}

/// TV-LUA-10: a colour and the palette of the published frame. The colour of NES index `$16`
/// is `#FC3800` (SPEC-04 REQ-E3D-251).
#[test]
fn tv_lua_10_colours_and_palette() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script(
            "p",
            r"
            function LateUpdate()
              alpha = Color(1, 0, 0).a
              published = Palette.FrameColorIndex(5)
              current = Palette.FrameColorIndex(5, 1)
              colour = Palette.FrameColor(5)
            end
            ",
        )
        .start();

    // Entry 5 of the frame palette is PPU byte 5 (REQ-E3D-250).
    let batch = Batch::new().bg_cell(img::a(), 4, 4).palette(5, 0x16);
    batch.convert_and_publish(&mut session.engine, 1);

    // The first frame has nothing published yet, so the lagging read gives 0; the second frame
    // sees the palette of the first (REQ-LUA-087, DEC-05-Q4).
    batch.convert_and_publish(&mut session.engine, 2);

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_eq!(session.number("return alpha"), 1.0);
    assert_eq!(session.number("return published"), f64::from(0x16));
    assert_eq!(session.number("return current"), f64::from(0x16));
    assert_close(252.0 / 255.0, session.number("return colour.r"), 5);
    assert_close(56.0 / 255.0, session.number("return colour.g"), 5);
    assert_close(0.0, session.number("return colour.b"), 5);
    assert_eq!(session.number("return colour.a"), 1.0);
}

/// TV-LUA-11: every number the host hands over is a Lua float.
#[test]
fn tv_lua_11_numbers() {
    let mut session = ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start();
    session.runtime(|runtime, _| runtime.environment_mut().memory.write(0x6C, 42));
    session.set_frame_globals(120, 2.0);

    assert_eq!(session.text("return math.type(frame)"), "float");
    assert_eq!(session.text("return tostring(frame)"), "120");
    assert!(session
        .number("return math.fmod(5, Frame.shapeCount)")
        .is_nan());
    assert_eq!(session.number("return Script:ReadMem(0x6C + 0.5)"), 42.0);
    assert_eq!(session.number("return Script:ReadMem(65536 + 0x6C)"), 42.0);
    assert_eq!(
        session.text("return math.type(Script:ReadMem(0x6C))"),
        "float"
    );
}

/// TV-LUA-12: a hook that raises is logged once, the script stops receiving hooks, the game
/// goes on, and a relaunch of the same game calls neither `End` nor `Start` of it.
#[test]
fn tv_lua_12_errors() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "e",
            r#"function Update() local x = nil; return x.y end
            function Start() log[#log + 1] = "e.Start" end
            function End() log[#log + 1] = "e.End" end
            function LateUpdate() log[#log + 1] = "e.LateUpdate" end
            "#,
        )
        .script("g", "function Update() log[#log + 1] = 'g.Update' end")
        .prelude("log = {}")
        .load();

    session.start();
    session.frame(&Batch::new(), 1);
    session.frame(&Batch::new(), 2);

    let errors = session.log.errors();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("'e'"));
    assert!(errors[0].contains("Update"));
    assert!(errors[0].contains("attempt to index"));
    assert!(errors[0].contains("e:1:"));

    // The relaunch of the same game: End then Start, with the error flag kept.
    session.end();
    session.start();

    assert_eq!(session.log_entries(), ["e.Start", "g.Update", "g.Update"]);
}

/// TV-LUA-13: unknown members.
#[test]
fn tv_lua_13_unknown_members() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script(
            "p",
            r"
            function UpdateS()
              typo1 = shape.hiden
              typo2 = shape2D.Hiden
              okSet, errSet = pcall(function () shape.NoSuch = 1 end)
              okPos, errPos = pcall(function () return Frame:GetShape(Frame.shapeCount) end)
              okRead, errRead = pcall(function () return shape.Pos end)
              okWrite, errWrite = pcall(function () shape.Pos = Vector3.zero end)
            end
            ",
        )
        .start();

    session.frame(&Batch::new().bg_cell(img::a(), 4, 4), 1);

    assert_eq!(session.text("return type(typo1)"), "nil");
    assert_eq!(session.text("return type(typo2)"), "nil");
    assert_eq!(truth(&mut session, "okSet"), 0.0);
    assert!(session
        .text("return errSet")
        .contains("cannot set NoSuch, no such field"));
    assert_eq!(truth(&mut session, "okPos"), 0.0);
    assert_eq!(truth(&mut session, "okRead"), 1.0);
    assert_eq!(truth(&mut session, "okWrite"), 0.0);
}

/// TV-LUA-14: the shared list of the tag queries.
#[test]
fn tv_lua_14_shared_list() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("a"))
        .pattern(&[img::b()], PatternSpec::tagged("b"))
        .script(
            "p",
            r#"
            function LateUpdate()
              local x = Frame:GetShapesWithTag("a")
              n = x.Count
              local y = Frame:GetShapesWithTag("b")
              afterwards = x.Count
              same = x == y
            end
            "#,
        )
        .start();

    session.frame(
        &Batch::new()
            .sprite(img::a(), 8, 16)
            .sprite(img::a(), 40, 16)
            .sprite(img::a(), 72, 16)
            .sprite(img::b(), 104, 16),
        1,
    );

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert_eq!(session.number("return n"), 3.0);
    assert_eq!(session.number("return afterwards"), 1.0);
    assert_eq!(truth(&mut session, "same"), 1.0);
}

/// TV-LUA-15: the profile's settings are applied after the chunks, so a chunk-level write is
/// overwritten; the same write in `Start` survives (REQ-LUA-015).
#[test]
fn tv_lua_15_load_order() {
    let mut builder = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::default())
        .script("chunk", "Setting.EmptyTile = true");
    builder.settings().empty_tile = false;

    let session = builder.load();
    assert!(!session.engine.settings().empty_tile);

    let mut other = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::default())
        .script("start", "function Start() Setting.EmptyTile = true end");
    other.settings().empty_tile = false;

    let second = other.start();
    assert!(second.engine.settings().empty_tile);
}

/// TV-LUA-16: the savestate sequence, `End` then `PData` then `Start` then `Load`, in script
/// order (REQ-LUA-072, DEC-R2-T6).
#[test]
fn tv_lua_16_savestate_sequence() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "_scene",
            r#"
            scene = 0
            function Save() PData[6] = scene; log[#log + 1] = "Save" end
            function Start() log[#log + 1] = "Start(" .. tostring(PData[6]) .. ")" end
            function Load() scene = PData[6]; log[#log + 1] = "Load" end
            function End() log[#log + 1] = "End" end
            "#,
        )
        .prelude("log = {}")
        .start();

    session.evaluate("scene = 2");
    session.save();
    let block = session.runtime(|runtime, _| runtime.capture_persistent_data().unwrap());

    session.evaluate("scene = 0; PData[6] = 0");
    session.end();
    session.runtime(|runtime, _| runtime.restore_persistent_data(&block).unwrap());
    session.start();
    session.load();

    assert_eq!(
        session.log_entries(),
        ["Start(0)", "Save", "End", "Start(2)", "Load"]
    );
    assert_eq!(session.number("return scene"), 2.0);
}

/// REQ-LUA-073 and §4.6: the persistent block is 256 bytes of little-endian singles.
#[test]
fn persistent_block_layout_req_lua_073() {
    let mut session = ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start();
    session.evaluate("PData[1] = 1.5; PData[64] = -2");

    let block = session.runtime(|runtime, _| runtime.capture_persistent_data().unwrap());
    assert_eq!(block.len(), 256);
    assert_eq!(f32::from_le_bytes(block[0..4].try_into().unwrap()), 1.5);
    assert_eq!(
        f32::from_le_bytes(block[252..256].try_into().unwrap()),
        -2.0
    );
}
