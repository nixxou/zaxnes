//! Port of `GameSessionScriptTests.cs`: the Lua runtime seen from the session — the load order of
//! REQ-LUA-015, the game start and end of REQ-LUA-020/021, the savestate sequence of
//! REQ-LUA-071/072 (DEC-R2-T6), the libretro sequence of REQ-LUA-075 and the reset of
//! REQ-LUA-013.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code
#![allow(clippy::float_cmp)] // exact values by construction

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::SyntheticRom;
use zax_app::game_session::{GameSession, SessionRequest};
use zax_app::input::zapper::RELEASED_BYTE;
use zax_app::input::InputAction;
use zax_nes::ZapperPosition;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_render::scene::SceneState;
use zax_scripting::NullScriptLog;
use zax_settings::model::SettingsModel;

fn script(name: &str, source: &str) -> ProfileScript {
    ProfileScript::new(Some(name), true, Some(source), Some("*"))
}

fn prelude(source: &str) -> ProfileScript {
    ProfileScript::new(Some("!prelude"), true, Some(source), Some(""))
}

/// A session over the synthetic ROM and a profile built from the given scripts.
struct Session {
    game: GameSession,
    scene: Rc<RefCell<SceneState>>,
    model: SettingsModel,
}

impl Session {
    fn open(scripts: Vec<ProfileScript>) -> Self {
        let mut profile = GameProfile::create_empty();
        profile.scripts.extend(scripts);

        let scene = Rc::new(RefCell::new(SceneState::default()));
        let mut model = SettingsModel::new();
        let request = SessionRequest {
            profile: &profile,
            title: "Test",
            scene: Rc::clone(&scene),
            log: Box::new(NullScriptLog),
            different_game: true,
            two_dimensional_mode: None,
        };
        let mut game = GameSession::create(request, &mut model).unwrap();
        assert!(game.load_rom(&SyntheticRom::counter(), "test.nes").success);
        game.start(&mut model);
        Self { game, scene, model }
    }

    fn evaluate(&mut self, chunk: &str) {
        self.game
            .with_runtime(|runtime, state| runtime.evaluate(state, chunk))
            .expect("a runtime")
            .unwrap();
    }

    fn number(&mut self, chunk: &str) -> f64 {
        self.game
            .with_runtime(|runtime, state| runtime.evaluate_number(state, chunk))
            .expect("a runtime")
            .unwrap()
    }

    fn text(&mut self, chunk: &str) -> String {
        self.game
            .with_runtime(|runtime, state| runtime.evaluate_text(state, chunk))
            .expect("a runtime")
            .unwrap()
    }

    /// The global `log` table the scripts fill, as strings.
    fn log(&mut self) -> Vec<String> {
        // The table holds a handful of entries.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = self.number("return #log") as usize;
        (1..=count)
            .map(|index| self.text(&format!("return tostring(log[{index}])")))
            .collect()
    }

    fn run_frame(&mut self, player_one: InputAction) {
        self.game.run_frame(
            &mut self.model,
            player_one,
            InputAction::NONE,
            RELEASED_BYTE,
            ZapperPosition::new(0, 0),
        );
    }

    fn script_failed(&self, name: &str) -> bool {
        self.game
            .runtime()
            .unwrap()
            .find_script(name)
            .unwrap()
            .failed()
    }
}

/// REQ-LUA-020: starting the game calls `Start` of every script in script order.
#[test]
fn starting_the_game_calls_start_in_script_order_req_lua_020() {
    let mut session = Session::open(vec![
        script("b", "function Start() log[#log + 1] = 'b' end"),
        script("a", "function Start() log[#log + 1] = 'a' end"),
        prelude("log = {}"),
    ]);

    assert_eq!(session.log(), ["a", "b"]);
    assert_eq!(session.game.script_names(), ["!prelude", "a", "b"]);
}

/// REQ-LUA-021 and REQ-LUA-090: ending the game calls `End`, then the end-of-game resets put the
/// backdrop back to its default state.
#[test]
fn ending_the_game_calls_end_then_resets_the_backdrop_req_lua_021() {
    let mut session = Session::open(vec![
        script(
            "a",
            "function Start() Backdrop.Enable = true; Backdrop.OfsZ = 17.1 end\n\
             function End() log[#log + 1] = 'End'; log[#log + 1] = Backdrop.Enable end",
        ),
        prelude("log = {}"),
    ]);

    assert!(session.scene.borrow().backdrop.enabled);
    assert!((session.scene.borrow().backdrop.offset.z - 17.1).abs() < 1e-4);

    session.game.end(&mut session.model);

    // End saw the state it had set; the reset of REQ-LUA-090 runs after it.
    assert_eq!(session.log(), ["End", "true"]);
    assert!(!session.scene.borrow().backdrop.enabled);
    assert_eq!(session.scene.borrow().backdrop.offset.z, 0.0);
}

/// REQ-LUA-071 and REQ-LUA-072 with the order of DEC-R2-T6: `Save` then the block, then on the way
/// back `End`, the core, `PData`, `Start` and `Load`.
#[test]
fn the_savestate_sequence_follows_req_lua_072_and_dec_r2_t6() {
    let mut session = Session::open(vec![
        script(
            "_scene",
            "scene = 0\n\
             function Save() PData[6] = scene; log[#log + 1] = 'Save' end\n\
             function Start() log[#log + 1] = 'Start(' .. tostring(PData[6]) .. ')' end\n\
             function Load() scene = PData[6]; log[#log + 1] = 'Load' end\n\
             function End() log[#log + 1] = 'End' end",
        ),
        prelude("log = {}"),
    ]);

    session.evaluate("scene = 2");
    let block = session
        .game
        .capture_script_state(&mut session.model)
        .unwrap();

    // The savestate kept scene 2; the running game moves on to something else.
    session.evaluate("scene = 0; PData[6] = 0");
    session.run_frame(InputAction::NONE);

    // BeginScriptRestore is End; the caller restores the core in between.
    session.game.end(&mut session.model);
    session
        .game
        .complete_script_restore(&mut session.model, &block)
        .unwrap();

    assert_eq!(
        session.log(),
        ["Start(0)", "Save", "End", "Start(2)", "Load"]
    );

    // Start already saw the restored entry, and Load pushed it into the script's own state.
    assert_eq!(session.number("return scene"), 2.0);
    assert_eq!(session.game.frame_counter(), 0);
}

/// REQ-LUA-075 and DEC-R2-D5: the libretro path restores the entries and calls `Load` only — the
/// profile stays loaded, so no `End` and no `Start`.
#[test]
fn the_libretro_sequence_calls_load_alone_req_lua_075() {
    let mut session = Session::open(vec![
        script(
            "a",
            "function Save() PData[1] = 42; log[#log + 1] = 'Save' end\n\
             function Load() log[#log + 1] = 'Load(' .. tostring(PData[1]) .. ')' end\n\
             function Start() log[#log + 1] = 'Start' end\n\
             function End() log[#log + 1] = 'End' end",
        ),
        prelude("log = {}"),
    ]);

    let block = session
        .game
        .capture_script_state(&mut session.model)
        .unwrap();
    session.evaluate("PData[1] = 0");
    session
        .game
        .restore_script_state_in_place(&mut session.model, &block)
        .unwrap();

    assert_eq!(session.log(), ["Start", "Save", "Load(42)"]);
}

/// REQ-LUA-013 and REQ-LUA-062 (DEC-05-Q5): a reset of the same game calls `End` then `Start`
/// without re-running the chunks, and keeps the error flags.
#[test]
fn a_reset_keeps_the_lua_state_req_lua_013() {
    let mut session = Session::open(vec![
        script(
            "a",
            "n = (n or 0) + 1\n\
             function Start() log[#log + 1] = 'Start(' .. tostring(n) .. ')' end\n\
             function End() log[#log + 1] = 'End' end",
        ),
        script("bad", "function Update() local x = nil; return x.y end"),
        prelude("log = {}"),
    ]);

    session.run_frame(InputAction::NONE);
    assert!(session.script_failed("bad"));

    session.game.reset(&mut session.model);

    // The chunk did not run again (n stays 1) and the failed script got neither hook
    // (REQ-LUA-062).
    assert_eq!(session.log(), ["Start(1)", "End", "Start(1)"]);
    assert!(session.script_failed("bad"));
    assert_eq!(session.game.frame_counter(), 0);
}

/// REQ-LUA-032 with SPEC-08 REQ-INP-071: the script pad adds its bits to the player's mask before
/// the core runs the frame, and never masks a physical input.
#[test]
fn the_script_pad_is_additive_req_lua_032() {
    let mut session = Session::open(vec![
        script(
            "pad",
            "function Start() ScriptPad:Enable(true) end\n\
             function InputProcess(index) log[#log + 1] = index; return 8.9 end",
        ),
        prelude("log = {}"),
    ]);

    session.run_frame(InputAction::A);

    assert_eq!(session.log(), ["0"]);

    // 8.9 truncates to 8 (REQ-LUA-040) and joins the A button of the physical mask.
    assert_eq!(session.game.runtime().unwrap().script_pad_state(), 8);
}
