//! AMD-067 §3: the Lua members of 3dSen 2.2. All of them are dummies — present so that a script
//! written for 2.2 neither errors nor loses its hooks; Zaxnes has no turbo fire.

use super::support::{img, Batch, ScriptProfileBuilder, ScriptSession};

fn blank() -> ScriptSession {
    ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start()
}

// REQ-LUA-122: the version is "2.2", and the criterion of AMD-040 still holds.
#[test]
fn req_lua_122_the_version_is_two_two() {
    let mut session = blank();

    assert_eq!("2.2", session.text("return CS.Nes3D.Info.Version"));
    assert_eq!(
        1.0,
        session.truth(r#"tonumber(CS.Nes3D.Info.Version:match("^%d+")) >= 2"#)
    );
}

// REQ-LUA-123: `Setting.TurboRate` reads 15, keeps the nearest rung of {10, 15, 20, 30} (the
// smaller one at equal distance) and is back at 15 in a new session.
#[test]
fn req_lua_123_the_turbo_rate_is_a_session_dummy() {
    let mut session = blank();

    assert_eq!(15.0, session.number("return Setting.TurboRate"));
    for (written, kept) in [(22, 20), (12, 10), (100, 30), (0, 10), (25, 20), (13, 15), (18, 20)] {
        session.evaluate(&format!("Setting.TurboRate = {written}"));
        assert_eq!(
            f64::from(kept),
            session.number("return Setting.TurboRate"),
            "{written}"
        );
    }
    assert!(session
        .try_evaluate(r#"Setting.TurboRate = "fast""#)
        .is_err());
    assert_eq!(
        "TurboRateId",
        session.text("return CS.Nes3D.SettingManager.TurboRateId")
    );
    assert_eq!(2.0, session.number("return CS.Nes3D.SettingManager.DefaultMsaa"));

    let mut again = blank();
    assert_eq!(15.0, again.number("return Setting.TurboRate"));
}

// REQ-LUA-124: `Gamepad:GetPlayerState(i)` reads what `Gamepad[i].State` reads for 0 and 1 and 0
// for anything else, without an error; `CS.Nes3D.Gamepads` has the one key.
#[test]
fn req_lua_124_the_player_state_is_the_pad_state() {
    let mut session = blank();

    for player in 0..2 {
        assert_eq!(
            1.0,
            session.truth(&format!(
                "Gamepad:GetPlayerState({player}) == Gamepad[{player}].State"
            ))
        );
    }
    assert_eq!(0.0, session.number("return Gamepad:GetPlayerState(5)"));
    assert_eq!(0.0, session.number("return Gamepad:GetPlayerState(-1)"));
    assert_eq!(0.0, session.number("return Gamepad:GetPlayerState(0.5)"));
    assert_eq!(0.0, session.number("return Gamepad:GetPlayerState(nil)"));
    assert_eq!(0.0, session.number("return Gamepad:GetPlayerState()"));
    assert_eq!(0.0, session.number(r#"return Gamepad:GetPlayerState("x")"#));
    assert_eq!(
        "3dSen.TurboKeysMigrated",
        session.text("return CS.Nes3D.Gamepads.TurboKeysMigratedKey")
    );
    assert_eq!("nil", session.text("return type(CS.Nes3D.Gamepads.Autre)"));
}

// REQ-LUA-124, last criterion: a profile script that reads and writes every member once a frame
// keeps its hooks and logs no error.
#[test]
fn req_lua_124_a_script_using_every_member_keeps_its_hooks() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "a",
            r"
            frames = 0
            function Update()
                local v = CS.Nes3D.Info.Version
                Setting.TurboRate = Setting.TurboRate + 5
                local id = CS.Nes3D.SettingManager.TurboRateId
                local s = Gamepad:GetPlayerState(0) + Gamepad:GetPlayerState(1)
                local k = CS.Nes3D.Gamepads.TurboKeysMigratedKey
                frames = frames + 1
            end
            ",
        )
        .start();
    let batch = Batch::new().bg_cell(img::a(), 10, 10);
    for counter in 0..3 {
        session.frame(&batch, counter);
    }
    assert_eq!(3.0, session.number("return frames"));
    assert!(session.log.errors().is_empty(), "{:?}", session.log.errors());
}
