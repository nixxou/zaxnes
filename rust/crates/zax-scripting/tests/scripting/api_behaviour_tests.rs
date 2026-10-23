//! Behaviour of the script API beyond the vectors of SPEC-05 §5.2: the members of §4.2 and
//! §4.3 that only the corpus exercised in the C# test suite, the error forms of REQ-LUA-041 and
//! REQ-LUA-042, and the safeguards of §3.8.

use std::time::Duration;

use zax_engine3d::settings::ZLayer;
use zax_scripting::{ScriptNotification, ScriptNotificationSeverity};

use super::support::{assert_close, img, Batch, PatternSpec, ScriptProfileBuilder, ScriptSession};

fn blank() -> ScriptSession {
    ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start()
}

/// A session with one tagged pattern of two cells, a script and one frame already run.
fn with_frame(script: &str) -> ScriptSession {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a(), img::b()], PatternSpec::tagged("t"))
        .script("p", script)
        .start();
    session.frame(
        &Batch::new().bg_cell(img::a(), 4, 4).bg_cell(img::b(), 5, 4),
        1,
    );
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
    session
}

fn error_of(session: &mut ScriptSession, chunk: &str) -> String {
    session.try_evaluate(chunk).unwrap_err().to_string()
}

// the base environment (REQ-LUA-050, REQ-LUA-052, REQ-LUA-101)

#[test]
fn the_sandbox_removes_the_libraries_req_lua_050() {
    let mut session = blank();

    for name in ["io", "debug", "package", "require", "dofile", "loadfile"] {
        assert_eq!(
            session.text(&format!("return type({name})")),
            "nil",
            "{name}"
        );
    }

    assert_eq!(session.text("return type(os.execute)"), "nil");
    assert_eq!(session.text("return type(os.clock)"), "function");
    assert_eq!(session.text("return type(os.time)"), "function");
    assert_eq!(session.text("return type(os.date)"), "function");
}

#[test]
fn the_lua_5_1_globals_are_added_back_req_lua_101() {
    let mut session = blank();

    assert_eq!(session.number("return table.getn({1, 2, 3})"), 3.0);
    assert_eq!(session.number("return math.pow(2, 10)"), 1024.0);
    assert_eq!(session.number("return math.ldexp(3, 2)"), 12.0);
    assert_eq!(session.number("return math.log10(1000)"), 3.0);
    assert_eq!(session.number("return (loadstring('return 7'))()"), 7.0);
    assert_eq!(session.number("return (unpack({4, 5}))"), 4.0);
}

#[test]
fn print_and_tostring_write_host_values_in_their_own_form_req_lua_052() {
    let mut session = blank();
    session.evaluate(
        "print(Vector3(1, 2.5, 3)); print(Vector2(1, 2)); print(Color(1, 0, 0)); \
         print(IntVector2(3, 4)); print(Rect(1, 2, 3, 4)); print(1.5, -0.0)",
    );

    assert_eq!(
        session.log.prints(),
        [
            "(1, 2.5, 3)",
            "(1, 2)",
            "(1, 0, 0, 1)",
            "(3, 4)",
            "(1, 2, 3, 4)",
            "1.5\t0",
        ]
    );
    assert_eq!(
        session.text("return tostring(Quaternion.identity)"),
        "(0, 0, 0, 1)"
    );
}

// the error forms (REQ-LUA-041, REQ-LUA-042, REQ-LUA-081)

#[test]
fn a_method_called_with_a_dot_raises_invalid_arguments_req_lua_081() {
    let mut session = with_frame("");

    assert!(error_of(&mut session, "Frame.GetShape(0)").contains("invalid arguments to GetShape"));
    assert!(error_of(&mut session, "Camera.SetTPV()").contains("invalid arguments to SetTPV"));
    assert!(error_of(&mut session, "Script.Get('p')").contains("invalid arguments to Get"));
}

#[test]
fn a_string_where_a_number_is_needed_is_refused_req_lua_041() {
    let mut session = with_frame("");

    assert!(error_of(&mut session, "Palette.FrameColorIndex('5')")
        .contains("invalid arguments to FrameColorIndex"));
    assert!(error_of(&mut session, "Frame:GetShape('0')").contains("invalid arguments to GetShape"));
    assert!(error_of(&mut session, "Vector3(1, 'x', 3)").contains("invalid arguments to Vector3"));
}

#[test]
fn a_wrong_assignment_names_the_member_req_lua_041() {
    let mut session = with_frame("");

    assert!(error_of(&mut session, "Frame:GetShape(0).Alpha = 'x'")
        .contains("invalid arguments to Alpha"));
    assert!(error_of(&mut session, "Setting.Fov = true").contains("invalid arguments to Fov"));
}

#[test]
fn a_read_only_member_refuses_a_write_req_lua_042() {
    let mut session = with_frame("");

    assert!(error_of(&mut session, "Frame:GetShape(0).Index = 3")
        .contains("cannot set Index, it is read only"));
    assert!(
        error_of(&mut session, "Script.Count = 3").contains("cannot set Count, it is read only")
    );
    assert!(error_of(&mut session, "Vector3.zero = Vector3.one")
        .contains("cannot set zero, it is read only"));
}

#[test]
fn an_error_carries_the_position_of_the_script_line_req_lua_061() {
    let mut session = blank();

    let message = session
        .try_evaluate("local a = 1\nlocal b = Frame.GetShape(0)")
        .unwrap_err()
        .to_string();
    assert!(
        message.contains(":2: invalid arguments to GetShape"),
        "{message}"
    );
}

#[test]
fn an_index_out_of_range_raises_req_lua_045() {
    let mut session = with_frame("");

    assert!(error_of(&mut session, "return Frame:GetShape(2)")
        .contains("index 2 is out of range for GetShape (count 1)"));
    assert!(error_of(&mut session, "return Setting.Layer[6]")
        .contains("index 6 is out of range for Layer (count 6)"));
    assert!(error_of(&mut session, "return Gamepad[2]").contains("is out of range for Gamepad"));
    assert!(error_of(&mut session, "return Clipping[4]").contains("is out of range for Clipping"));
    assert!(
        error_of(&mut session, "return Frame:GetShape(0).Shape2D[7]")
            .contains("index 7 is out of range for Shape")
    );
}

// the script model (REQ-LUA-003, REQ-LUA-060, REQ-LUA-062, REQ-LUA-064)

#[test]
fn a_duplicate_script_name_keeps_the_last_req_lua_003() {
    let session = ScriptProfileBuilder::new()
        .script("same", "first = true")
        .script("same", "second = true")
        .load();

    assert_eq!(session.log.warnings().len(), 1);
    assert!(session.log.warnings()[0].contains("two scripts named 'same'"));
}

#[test]
fn a_chunk_that_does_not_compile_is_logged_and_left_without_hooks_req_lua_060() {
    let mut session = ScriptProfileBuilder::new()
        .script("bad", "function Start() = end")
        .script("good", "function Start() started = true end")
        .start();

    let errors = session.log.errors();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("script 'bad' failed to compile"));
    assert!(session.script_failed("bad"));
    assert_eq!(session.truth("started"), 1.0);
}

#[test]
fn a_chunk_that_raises_keeps_the_globals_it_defined_req_lua_060() {
    let mut session = ScriptProfileBuilder::new()
        .script("bad", "before = 1\nerror('boom')\nafter = 1")
        .load();

    assert!(session.log.errors()[0].contains("script 'bad' failed to run"));
    assert!(session.log.errors()[0].contains("boom"));
    assert_eq!(session.number("return before"), 1.0);
    assert_eq!(session.text("return type(after)"), "nil");
}

#[test]
fn a_hook_that_never_returns_is_cut_off_req_lua_064() {
    let mut session = ScriptProfileBuilder::new()
        .script("loop", "function Update() while true do end end")
        .script("next", "function Update() ran = true end")
        .start();
    session.runtime(|runtime, _| runtime.set_hook_budget(Duration::from_millis(30)));

    session.frame(&Batch::new(), 1);

    let errors = session.log.errors();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].contains("the hook used up its execution budget"),
        "{errors:?}"
    );
    assert!(session.script_failed("loop"));
    assert_eq!(session.truth("ran"), 1.0);
}

#[test]
fn a_hook_may_not_define_an_unknown_hook_name_but_detect_shape_is_never_called_req_lua_031() {
    let mut session = ScriptProfileBuilder::new()
        .script("d", "function DetectShape() called = true end")
        .start();
    session.frame(&Batch::new(), 1);

    assert_eq!(session.text("return type(called)"), "nil");
}

// the notifications (REQ-LUA-093)

#[test]
fn notifications_reach_the_host_req_lua_093() {
    let mut session = blank();
    session.evaluate(
        "NotiCenter.Notify('first', 2.5, 1); NotiCenter.Notify('second'); \
         NotiCenter.Show('third', 2); NotiCenter.Show('fourth', 9)",
    );

    assert_eq!(
        session.notifications(),
        [
            ScriptNotification {
                message: "first".to_owned(),
                seconds: 2.5,
                severity: ScriptNotificationSeverity::Warning,
            },
            ScriptNotification {
                message: "second".to_owned(),
                seconds: 4.0,
                severity: ScriptNotificationSeverity::Normal,
            },
            ScriptNotification {
                message: "third".to_owned(),
                seconds: 4.0,
                severity: ScriptNotificationSeverity::Error,
            },
            ScriptNotification {
                message: "fourth".to_owned(),
                seconds: 4.0,
                severity: ScriptNotificationSeverity::Error,
            },
        ]
    );
}

// the camera, the backdrop and the light (REQ-LUA-088, REQ-LUA-090, §4.2)

#[test]
fn the_camera_properties_read_and_write_req_lua_088() {
    let mut session = blank();

    assert_eq!(session.number("return Camera.Mode"), 0.0);
    assert_eq!(session.truth("Camera.IsTPV"), 1.0);
    session.evaluate("Camera:SetFPV()");
    assert_eq!(session.number("return Camera.Mode"), 1.0);
    assert_eq!(session.truth("Camera.IsFPV"), 1.0);
    session.evaluate("Camera:ToggleMode()");
    assert_eq!(session.number("return Camera.Mode"), 0.0);
    session.evaluate("Camera.Mode = 7");
    assert_eq!(session.number("return Camera.Mode"), 1.0);

    session.evaluate("Camera.Distance = 100");
    assert_eq!(session.number("return Camera.Distance"), 100.0);
    session.evaluate("Camera.Distance = 100000");
    assert_eq!(session.number("return Camera.Distance"), 512.0);

    session.evaluate("Camera:SetTarget(1, 2, 3)");
    assert_eq!(session.number("return Camera:GetTarget().y"), 2.0);
    session.evaluate("Camera:SetTarget(Vector3(4, 5, 6))");
    assert_eq!(session.number("return Camera.LookAt.z"), 6.0);
    session.evaluate("Camera.LookAt = Vector3(7, 8, 9)");
    assert_eq!(session.number("return Camera.Target.x"), 7.0);
}

#[test]
fn the_camera_vectors_are_bound_req_lua_044() {
    let mut session = blank();

    session.evaluate("Camera.Rot.x = 15; Camera.Target.y = 33");
    assert_close(15.0, session.number("return Camera.Rot.x"), 4);
    assert_close(33.0, session.number("return Camera.LookAt.y"), 4);
    session.evaluate("local r = Camera.Rot; r[1] = 99");
    assert_close(99.0, session.number("return Camera.Rot.y"), 4);
}

#[test]
fn a_change_of_field_of_view_resets_the_view_req_pers_011() {
    let mut session = blank();

    session.evaluate("Camera.LookAt = Vector3(1, 2, 3)");
    session.evaluate("Setting.Fov = 90");

    assert_eq!(session.number("return Camera.LookAt.x"), 128.0);
    assert_eq!(session.number("return Camera.LookAt.y"), 120.0);
}

#[test]
fn a_host_without_view_reset_keeps_the_view_on_a_change_of_field_of_view() {
    let mut settings = zax_scripting::ScriptSettings::default();
    settings.write_real(zax_settings::catalogue::SettingId::CameraFov, 90.0);
    assert!(settings.take_view_reset());

    // The C# harness wires no `ViewReset`: the same write asks for nothing.
    settings.set_resets_view(false);
    settings.write_real(zax_settings::catalogue::SettingId::CameraFov, 60.0);
    settings.write_real(zax_settings::catalogue::SettingId::VrSceneRatio, 2.0);
    assert!(!settings.take_view_reset());
}

#[test]
fn camera_reset_view_restores_the_default_req_scn_012() {
    let mut session = blank();

    session.evaluate("Camera.LookAt = Vector3(1, 2, 3); Camera:ResetView()");

    assert_eq!(session.number("return Camera.LookAt.x"), 128.0);
}

#[test]
fn the_backdrop_reads_and_writes_req_lua_090() {
    let mut session = blank();

    assert_eq!(session.truth("Backdrop.Enable"), 0.0);
    assert_eq!(session.truth("Backdrop.RecvShadow"), 1.0);
    assert_eq!(session.number("return Backdrop.SizeX"), 256.0);
    assert_eq!(session.number("return Backdrop.SizeY"), 240.0);
    assert_eq!(session.number("return Backdrop.SizeZ"), 1.0);

    session.evaluate(
        "Backdrop.Enable = true; Backdrop.RecvShadow = false; Backdrop.Color = Color(1, 0, 0); \
         Backdrop.SizeX = 10; Backdrop.OfsY = 5; Backdrop.Offset.z = 7; Backdrop.Size[1] = 20",
    );
    assert_eq!(session.truth("Backdrop.Enable"), 1.0);
    assert_eq!(session.truth("Backdrop.RecvShadow"), 0.0);
    assert_eq!(session.number("return Backdrop.Color.r"), 1.0);
    assert_eq!(session.number("return Backdrop.Size.x"), 10.0);
    assert_eq!(session.number("return Backdrop.SizeY"), 20.0);
    assert_eq!(session.number("return Backdrop.OfsY"), 5.0);
    assert_eq!(session.number("return Backdrop.Offset.z"), 7.0);
    assert!(error_of(&mut session, "Backdrop.Color = 3").contains("invalid arguments to Color"));
}

#[test]
fn the_light_is_the_one_of_the_settings_req_lua_089() {
    let mut session = blank();

    assert_eq!(session.truth("Light == Setting.Light"), 1.0);
    assert_eq!(session.number("return Light.Direction.y"), -1.0);
    assert_eq!(session.number("return Light.Intensity"), 1.0);
    session.evaluate(
        "Light.Direction = Vector3(0, -1, 0); Light.Color = Color(0, 1, 0); Light.Intensity = 0.5",
    );
    assert_eq!(session.number("return Setting.Light.Direction.x"), 0.0);
    assert_eq!(session.number("return Setting.Light.Color.g"), 1.0);
    assert_eq!(session.number("return Setting.Light.Intensity"), 0.5);
}

// the settings (REQ-LUA-089, SPEC-09)

#[test]
fn the_settings_validate_and_alias_req_pers_003() {
    let mut session = blank();

    session.evaluate("Setting.SameBgPalette = true");
    assert_eq!(session.truth("Setting.SamePalette"), 1.0);
    assert!(session.engine.settings().same_background_palette);
    session.evaluate("Setting.TopDownCam = true");
    assert_eq!(session.truth("Setting.LocalCam"), 1.0);

    session.evaluate("Setting.GameSpeed = 100");
    assert_eq!(session.number("return Setting.GameSpeed"), 8.0);
    session.evaluate("Setting.Volume = 3");
    assert_eq!(session.number("return Setting.Volume"), 1.0);
    session.evaluate("Setting.SpriteTolerance = -4");
    assert_eq!(session.number("return Setting.SpriteTolerance"), 4.0);
    assert_eq!(session.engine.settings().sprite_tolerance, 4);
}

#[test]
fn the_tv_box_reads_zero_and_accepts_writes_amd_031() {
    let mut session = blank();

    session.evaluate("Setting.TVBox = 3");

    assert_eq!(session.number("return Setting.TVBox"), 0.0);
    assert!(error_of(&mut session, "Setting.TVBox = 'x'").contains("invalid arguments to TVBox"));
}

#[test]
fn the_vr_members_are_frozen_on_desktop_req_pers_011() {
    let mut session = blank();

    session.evaluate("Setting.VrRuntime = 2; Setting.SceneRatio = 3");

    assert_eq!(session.number("return Setting.VrRuntime"), 0.0);
    assert_eq!(session.number("return Setting.SceneRatio"), 1.0);
    assert_eq!(session.number("return Setting.SceneScale"), 1.0);
    assert_eq!(session.number("return Video.SceneScale"), 1.0);
}

#[test]
fn the_stored_members_are_accepted_and_read_nil_req_pers_016() {
    let mut session = blank();

    session.evaluate("Setting.SavedPattern3D = 'x'; Setting.TVData = 1");

    assert_eq!(session.text("return type(Setting.SavedPattern3D)"), "nil");
    assert_eq!(session.text("return type(Setting.TVData)"), "nil");
}

#[test]
fn the_notification_identifiers_are_strings_req_pers_014() {
    let mut session = blank();

    assert_eq!(session.text("return Setting.FovId"), "FovId");
    assert_eq!(
        session.text("return Setting.LightDirectionId"),
        "Light.DirectionId"
    );
    assert_eq!(
        session.text("return Setting.SavedPattern3DId"),
        "SavePatternId"
    );
    assert_close(0.4, session.number("return Setting.DefaultSmoothTime"), 5);
    assert_close(0.005, session.number("return Setting.VRScale"), 5);
    assert_eq!(session.number("return Setting.NonVRScale"), 1.0);
}

#[test]
fn the_layers_and_the_clipping_are_live_req_pers_007() {
    let mut session = blank();

    session.evaluate("Setting.Layer[2].Offset = 30; Setting.Layer[2].Aligned = 9");
    assert_eq!(session.number("return Setting.Layer[2].Offset"), 30.0);
    assert_eq!(session.number("return Layer[2].Aligned"), 2.0);
    assert_eq!(session.number("return Layer[2].Coef"), -1.0);
    assert_eq!(session.engine.settings().layer(ZLayer::L1).offset, 30.0);

    session.evaluate("Layer.DefLayer = 4; Layer.Auto = true");
    assert_eq!(session.number("return Layer.DefLayer"), 4.0);
    assert_eq!(session.engine.settings().default_layer, ZLayer::L3);
    assert!(error_of(&mut session, "Layer.DefLayer = 6").contains("layer 6 is out of range"));

    session.evaluate("Clipping.x = 1; Clipping.target = Color(2, 3, 4, 5)");
    assert_eq!(session.number("return Clipping[0]"), 2.0);
    assert_eq!(session.number("return Clipping.w"), 5.0);
    assert_eq!(session.engine.settings().clipping.z, 4.0);
    assert_eq!(session.number("return Clipping.target.a"), 5.0);
}

#[test]
fn a_write_in_build_pattern_reaches_the_pipeline_at_the_next_hook() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("g"))
        .script_with(
            "b",
            "function BuildPattern() Setting.EmptyTile = true end",
            "g",
            true,
        )
        .load();

    session.start();

    assert!(session.engine.settings().empty_tile);
}

#[test]
fn the_input_types_are_recorded_in_the_model_req_inp_060() {
    let mut session = blank();

    assert_eq!(session.truth("Setting.ZapperEnable"), 0.0);
    session.evaluate("Setting:SetInputType(0, 1)");
    assert_eq!(session.number("return Setting:GetInputType(0)"), 0.0);
    assert_eq!(session.number("return Setting:GetInputType(1)"), 1.0);
    assert_eq!(session.truth("Setting.ZapperEnable"), 1.0);
}

// the profile metadata (REQ-LUA-095) and the persistent identifier

#[test]
fn the_profile_metadata_is_session_local_req_lua_095() {
    let mut session = blank();

    session.evaluate(
        "Profile.Title = 'T'; Profile.Maker = 'M'; Profile.GameDev = 'G'; Profile.Des = 'D'",
    );
    assert_eq!(
        session.text("return Profile.Title .. Profile.Maker .. Profile.GameDev .. Profile.Des"),
        "TMGD"
    );
    assert_eq!(session.truth("Profile.IsV2Format"), 0.0);
    assert!(error_of(&mut session, "Profile.Id = 'x'").contains("it is read only"));
}

#[test]
fn renew_id_replaces_the_identifier_req_prof_052() {
    let mut session = blank();

    session.evaluate("Profile:RenewId()");
    let first = session.text("return Profile.Id");
    session.evaluate("RenewId()");
    let second = session.text("return Profile.Id");

    assert_eq!(first.len(), 12);
    assert_eq!(second.len(), 12);
    assert_ne!(first, second);
    assert!(first.chars().all(|c| c.is_ascii_alphanumeric()));
}
