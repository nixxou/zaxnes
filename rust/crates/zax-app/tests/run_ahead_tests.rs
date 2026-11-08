//! Run-ahead through the loop (AMD-041): the setting of REQ-LOOP-064, what follows the real
//! frame (REQ-LOOP-066) and what sees the displayed one (REQ-LOOP-067).
//!
//! The core's own sequence is tested in `zax-nes/tests/run_ahead_tests.rs`; here the question is
//! the loop around it — the setting read once per real frame, the rollback history, the quick
//! saves, the frame counter the scripts read, and the interaction with the Rust-only "emulation
//! ahead" of `docs/PERFORMANCE-HISTORY.md` §5.5.

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

/// The vector ROM of REQ-LOOP-065, shared with the core tests so that there is one definition.
#[path = "../../zax-nes/tests/common/run_ahead_rom.rs"]
mod run_ahead_rom;

use common::LoopHarness;
use zax_app::emulation::commands::Command;
use zax_app::emulation::rollback_history::RollbackHistory;
use zax_app::game_session::{GameSession, SessionRequest};
use zax_app::input::InputAction;
use zax_app::persistence::app_settings::AppSettings as _;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_render::scene::SceneState;
use zax_scripting::NullScriptLog;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use run_ahead_rom as rom;

/// A harness playing the vector ROM at `run_ahead`, past the ROM's warm-up.
fn playing(run_ahead: i32) -> LoopHarness {
    let mut harness = LoopHarness::create();
    harness
        .host
        .update_settings(|model| model.set(SettingId::EmuRunAhead, SettingValue::Int(run_ahead)));
    harness.launch_rom_with_script("vector", &rom::build(), None);
    for _ in 0..8 {
        harness.tick_frame();
    }
    harness
}

fn counter(harness: &LoopHarness) -> u32 {
    let session = harness.host.session().expect("a session");
    u32::from(session.core().read_memory(rom::COUNTER_ADDRESS))
}

fn core_frames(harness: &LoopHarness) -> i64 {
    harness
        .host
        .session()
        .expect("a session")
        .core()
        .frame_count()
}

// ------------------------------------------------------------------ REQ-LOOP-064

/// REQ-LOOP-064, the acceptance criterion of the setting: an empty user root gives 1, a write
/// past the bound reads the bound (10, Rust-only, owner request; 3 in AMD-041), −1 reads 0.
#[test]
fn req_loop_064_the_setting_has_the_default_and_the_bounds_of_the_amendment() {
    let model = SettingsModel::new();
    assert_eq!(1, model.run_ahead(), "an empty user root gives 1");

    let mut model = SettingsModel::new();
    model.set(SettingId::EmuRunAhead, SettingValue::Int(17));
    assert_eq!(10, model.run_ahead());
    model.set(SettingId::EmuRunAhead, SettingValue::Int(-1));
    assert_eq!(0, model.run_ahead());
    for value in 0..=3 {
        model.set(SettingId::EmuRunAhead, SettingValue::Int(value));
        assert_eq!(value, model.run_ahead());
    }
}

/// REQ-LOOP-064: the loop reads the setting again at every real frame, so a change during the
/// game applies at the next one and never in the middle of one.
#[test]
fn req_loop_064_a_change_during_the_game_applies_at_the_next_real_frame() {
    let mut harness = playing(0);
    assert_eq!(0, harness.host.session().unwrap().run_ahead());

    harness
        .host
        .update_settings(|model| model.set(SettingId::EmuRunAhead, SettingValue::Int(3)));
    harness.host.dispatch_setting_changes();
    // Still 0 until a real frame runs: the value is not pushed by the dispatch.
    assert_eq!(0, harness.host.session().unwrap().run_ahead());

    harness.tick_frame();
    assert_eq!(3, harness.host.session().unwrap().run_ahead());

    // And the frame that just ran was a whole real frame: the core counted exactly one.
    let before = core_frames(&harness);
    harness.tick_frame();
    assert_eq!(before + 1, core_frames(&harness));
}

/// REQ-LOOP-064: the setting is offered in the desktop options window, as a **list** with the
/// four labels of AMD-041 §3 in both languages, and the value 1 is marked recommended.
#[test]
fn req_loop_064_the_setting_is_a_labelled_list_in_the_options_window() {
    assert!(
        !zax_app::ui::options::EXCLUDED.contains(&SettingId::EmuRunAhead),
        "the setting is still hidden from the options window"
    );
    let descriptor = zax_settings::catalogue::descriptor(SettingId::EmuRunAhead);
    assert!(zax_app::ui::options::is_shown(descriptor));

    // The domain publishes a list (0 to 10, Rust-only, owner request), so the window draws a
    // drop-down and not a slider.
    let options = descriptor.domain().options().expect("a published list");
    assert_eq!(11, options.len());
    let labels: Vec<&str> = options.iter().map(|option| option.label.as_str()).collect();
    assert_eq!(
        vec![
            "Run-ahead off",
            "1 frame (Recommended)",
            "2 frames",
            "3 frames",
            "4 frames",
            "5 frames",
            "6 frames",
            "7 frames",
            "8 frames",
            "9 frames",
            "10 frames"
        ],
        labels
    );
    for (index, option) in options.iter().enumerate() {
        assert!((option.value - f64::from(u8::try_from(index).unwrap())).abs() < 1e-9);
    }

    for (language, expected) in [
        (
            zax_app::ui::strings::UiLanguage::English,
            ["Run-ahead", "Off", "1 frame (Recommended)"],
        ),
        (
            zax_app::ui::strings::UiLanguage::French,
            ["Run-ahead", "Désactivé", "1 frame (recommandé)"],
        ),
    ] {
        for (key, text) in [
            "setting.emu.run_ahead",
            "option.Run-ahead off",
            "option.1 frame (Recommended)",
        ]
        .into_iter()
        .zip(expected)
        {
            assert_eq!(
                Some(text),
                zax_app::ui::strings::lookup(language, key),
                "{language:?} / {key}"
            );
        }
    }
}

/// REQ-LOOP-064 and the owner's decision for handhelds: the setting is offered among the
/// **advanced** options of the console's own pages, and its handheld default stays 0 — it costs a
/// whole extra emulated frame, so it is off there unless the player asks for it.
#[test]
fn req_loop_064_the_handheld_offers_it_as_an_advanced_option_but_off() {
    assert!(
        zax_app::ui::handheld::advanced::rows().iter().any(|row| {
            matches!(
                row,
                zax_app::ui::handheld::options::OptionRow::Setting(SettingId::EmuRunAhead)
            )
        }),
        "the handheld's advanced options do not offer run-ahead"
    );

    let mut handheld = SettingsModel::new();
    zax_app::ui::handheld::defaults::apply(&mut handheld);
    assert_eq!(0, handheld.run_ahead(), "the handheld default is not 0");
    assert_eq!(1, SettingsModel::new().run_ahead(), "the desktop default");
}

// ------------------------------------------------------------------ REQ-LOOP-066

/// REQ-LOOP-066: the core's frame counter follows the **real** frame, whatever N says.
#[test]
fn req_loop_066_the_core_counts_one_frame_per_real_frame() {
    for ahead in 0..=3 {
        let mut harness = playing(ahead);
        let start = core_frames(&harness);
        for _ in 0..60 {
            harness.tick_frame();
        }
        assert_eq!(
            start + 60,
            core_frames(&harness),
            "N = {ahead}: the core counted frames in advance"
        );
    }
}

/// REQ-LOOP-066: the session's own counter — the one the conversion stamps its frames with and
/// the Lua global `frame` reads — also advances by one per real frame.
#[test]
fn req_loop_066_the_displayed_frames_are_contiguous() {
    for ahead in 0..=3 {
        let mut harness = playing(ahead);
        let start = harness.host.session().unwrap().frame_counter();
        for _ in 0..40 {
            harness.tick_frame();
        }
        assert_eq!(
            start + 40,
            harness.host.session().unwrap().frame_counter(),
            "N = {ahead}: the displayed frames are not contiguous"
        );
    }
}

/// REQ-LOOP-066, the acceptance criterion: Run-ahead = 3, a quick save then a quick load → the
/// core's frame counter is the one of the **real** frame of the save.
#[test]
fn req_loop_066_a_quick_save_holds_the_real_frame() {
    let mut harness = playing(3);
    for _ in 0..20 {
        harness.tick_frame();
    }
    let saved_core = core_frames(&harness);
    let saved_counter = counter(&harness);
    harness.run(Command::QuickSave);

    for _ in 0..30 {
        harness.tick_frame();
    }
    assert!(core_frames(&harness) > saved_core);

    harness.run(Command::QuickLoad);
    assert_eq!(
        saved_core,
        core_frames(&harness),
        "the quick load did not come back to the real frame of the save"
    );
    assert_eq!(
        saved_counter,
        counter(&harness),
        "the game's own per-frame counter is not the real frame's"
    );
}

/// REQ-LOOP-066, second half of the criterion: the rollback history still captures a snapshot at
/// every multiple of 60 of the **real** counter.
#[test]
fn req_loop_066_the_history_still_snapshots_every_sixty_real_frames() {
    for ahead in [0, 2, 3] {
        let mut harness = playing(ahead);
        let before = harness.host.rollback.count();
        // Enough real frames for three multiples of 60 to go by.
        for _ in 0..200 {
            harness.tick_frame();
        }
        let taken = harness.host.rollback.count() - before;
        assert!(
            (3..=4).contains(&taken),
            "N = {ahead}: {taken} snapshots over 200 real frames"
        );
        // And every snapshot sits on a multiple of 60 of the real counter.
        assert!(RollbackHistory::is_snapshot_frame(0));
        assert!(RollbackHistory::is_snapshot_frame(60));
        assert!(!RollbackHistory::is_snapshot_frame(61));
    }
}

/// REQ-LOOP-066: a frame step shows the frame it steps to, so it runs with no run-ahead at all,
/// and the real state follows it.
#[test]
fn req_loop_066_a_frame_step_runs_without_run_ahead() {
    let mut harness = playing(3);
    for _ in 0..20 {
        harness.tick_frame();
    }
    harness.run(Command::Pause);
    let before = core_frames(&harness);
    let shown = counter(&harness);

    harness.run(Command::FrameAdvance);
    assert_eq!(before + 1, core_frames(&harness), "the step ran one frame");
    assert_eq!(
        0,
        harness.host.session().unwrap().run_ahead(),
        "the step ran with run-ahead on"
    );
    // The picture of the step is the frame stepped to, not one three frames on.
    let session = harness.host.session().unwrap();
    assert_eq!(
        shown,
        u32::from(session.core().read_memory(rom::COUNTER_ADDRESS)) - 1
    );
}

/// REQ-LOOP-066: the real states the loop leaves behind do not depend on N — the same run with
/// the same inputs gives the same core frames and the same work RAM at every N.
#[test]
fn req_arch_012_the_real_states_do_not_depend_on_n() {
    let inputs = [
        InputAction::NONE,
        InputAction::A,
        InputAction::NONE,
        InputAction::A | InputAction::RIGHT,
    ];

    let mut reference: Option<(i64, Vec<u8>)> = None;
    for ahead in 0..=3 {
        let mut harness = playing(ahead);
        for frame in 0..80 {
            let action = inputs[frame % inputs.len()];
            harness.tick_actions(1.0 / 60.0, action, InputAction::NONE);
        }
        let session = harness.host.session().unwrap();
        let state = (
            session.core().frame_count(),
            session.core().bus().ram().to_vec(),
        );
        match &reference {
            None => reference = Some(state),
            Some(expected) => assert_eq!(
                *expected, state,
                "N = {ahead}: the real state parted from N = 0's"
            ),
        }
    }
}

// ------------------------------------------------------------------ REQ-LOOP-067

/// REQ-LOOP-067, the acceptance criterion: a script that increments a counter in `Update`
/// increases it by 1 per **real** frame, whatever N is.
#[test]
fn req_loop_067_a_frame_hook_runs_once_per_real_frame() {
    for ahead in 0..=3 {
        let mut harness = LoopHarness::create();
        harness.host.update_settings(|model| {
            model.set(SettingId::EmuRunAhead, SettingValue::Int(ahead));
        });
        harness.launch_rom_with_script(
            "hooked",
            &rom::build(),
            Some(
                r"
                    hooks = 0
                    function Update()
                        hooks = hooks + 1
                    end
                    function LateUpdate()
                        print('hooks ' .. hooks)
                    end
                ",
            ),
        );
        for _ in 0..8 {
            harness.tick_frame();
        }
        harness.clear_script_lines();
        for _ in 0..30 {
            harness.tick_frame();
        }

        let lines: Vec<String> = harness
            .script_lines()
            .into_iter()
            .filter(|line| line.starts_with("hooks "))
            .collect();
        assert_eq!(
            30,
            lines.len(),
            "N = {ahead}: {} frame hooks for 30 real frames",
            lines.len()
        );
        // And the counter they print rises by exactly one between two real frames.
        let values: Vec<i64> = lines
            .iter()
            .filter_map(|line| line.trim_start_matches("hooks ").trim().parse().ok())
            .collect();
        assert_eq!(30, values.len());
        for pair in values.windows(2) {
            assert_eq!(
                pair[0] + 1,
                pair[1],
                "N = {ahead}: the hook counter jumped ({pair:?})"
            );
        }
    }
}

/// REQ-LOOP-067, second half of the criterion: with Run-ahead = 2, `Script:ReadMem` on the
/// counter of the vector ROM returns the **displayed** value, k + 2.
#[test]
fn req_loop_067_read_mem_in_a_frame_hook_reads_the_displayed_frame() {
    let mut harness = LoopHarness::create();
    harness
        .host
        .update_settings(|model| model.set(SettingId::EmuRunAhead, SettingValue::Int(2)));
    harness.launch_rom_with_script(
        "readmem",
        &rom::build(),
        Some(
            r"
                function Update()
                    print('shown ' .. Script:ReadMem(0x0010))
                end
            ",
        ),
    );
    for _ in 0..8 {
        harness.tick_frame();
    }

    for _ in 0..10 {
        let before = counter(&harness);
        harness.clear_script_lines();
        harness.tick_frame();
        let shown: u32 = harness
            .script_lines()
            .iter()
            .find_map(|line| line.strip_prefix("shown "))
            .and_then(|value| value.trim().trim_end_matches(".0").parse().ok())
            .expect("the hook printed what it read");
        // `$0010` holds the value the picture of the frame that drew it shows: the displayed
        // frame is two ahead of the real frame whose counter was `before`.
        assert_eq!(
            before + 2,
            shown,
            "the hook read the real frame instead of the displayed one"
        );
        // And the real frame that just ran is the one after `before`.
        assert_eq!(before + 1, counter(&harness));
    }
}

/// Q-103: a `Script:WriteMem` from a **frame** hook lands on the displayed state — the hook reads
/// its own write straight back — and does not reach the real one, because REQ-LOOP-066 makes the
/// core follow the real frame. At Run-ahead = 0 the same write persists, which is what makes the
/// loss a consequence of the amendment and not of the write itself.
///
/// R0's answer keeps that (the write is not replayed) and adds the guard-rail of the section
/// below, which runs this very profile at N = 0 through the loop. So the behaviour the answer
/// keeps is pinned here by **driving the session's run-ahead directly**
/// (`GameSession::set_run_ahead`, the raw setter) and running frames straight on the session,
/// which is the path the guard-rail deliberately leaves alone.
#[test]
fn req_loop_067_a_frame_hook_write_does_not_reach_the_real_state() {
    const SOURCE: &str = r"
        function Update()
            Script:WriteMem(0x0040, 0x5A)
            print('in hook ' .. Script:ReadMem(0x0040))
        end
    ";

    for (ahead, expected) in [(0, 0x5A_u8), (1, 0x00), (2, 0x00), (3, 0x00)] {
        let mut harness = LoopHarness::create();
        harness.launch_rom_with_script("writer", &rom::build(), Some(SOURCE));
        // Q-103's guard-rail holds this profile at 0 whatever the setting says, which is what the
        // next section tests; the run-ahead itself is therefore set on the session.
        assert!(harness
            .host
            .session()
            .expect("a session")
            .forces_run_ahead_to_zero());
        harness
            .host
            .session_mut()
            .expect("a session")
            .set_run_ahead(ahead);
        harness.clear_script_lines();
        harness.run_exact_frames(12, &[]);

        // The write did happen: the hook read its own byte back from the frame it ran on.
        assert!(
            harness
                .script_lines()
                .iter()
                .any(|line| line.starts_with("in hook 90")),
            "N = {ahead}: the hook never wrote (lines {:?})",
            harness.script_lines()
        );
        assert_eq!(
            expected,
            harness.host.session().unwrap().core().read_memory(0x0040),
            "N = {ahead}: Q-103 — the real state does not hold what this lot says it should"
        );
    }
}

// ------------------------------------------------------------ Q-103, the guard-rail

/// A frame hook that writes the core — the case Q-103's answer protects.
const WRITES_THE_CORE: &str = r"
    function Update()
        Script:WriteMem(0x0040, 0x5A)
    end
";

/// A frame hook that only reads it, which is not concerned.
const ONLY_READS_THE_CORE: &str = r"
    function Update()
        local seen = Script:ReadMem(0x0040)
    end
";

/// The run-ahead the loop really applied, read out of the vector ROM rather than asked of the
/// session: a real frame leaves the counter at k + 1 and the picture showing what the **displayed**
/// frame drew, k + N (the acceptance criterion of REQ-LOOP-065).
fn effective_run_ahead(harness: &LoopHarness) -> i64 {
    let session = harness.host.session().expect("a session");
    i64::from(rom::displayed(session.core())) + 1 - i64::from(counter(harness))
}

/// Q-103, point 2 of the answer: a profile whose scripts change the core from a frame hook runs
/// at an **effective** run-ahead of 0 while the setting says 1 — the core emulates one frame per
/// real frame and nothing of the hook's write is thrown away.
#[test]
fn q_103_a_profile_that_writes_the_core_runs_at_zero() {
    let mut harness = LoopHarness::create();
    harness
        .host
        .update_settings(|model| model.set(SettingId::EmuRunAhead, SettingValue::Int(1)));
    harness.launch_rom_with_script("writer", &rom::build(), Some(WRITES_THE_CORE));
    for _ in 0..8 {
        harness.tick_frame();
    }

    let session = harness.host.session().expect("a session");
    assert!(
        session.forces_run_ahead_to_zero(),
        "the guard-rail did not see the write"
    );
    assert_eq!(0, session.run_ahead(), "the effective run-ahead is not 0");
    assert_eq!(
        0,
        effective_run_ahead(&harness),
        "the picture still comes from a frame in advance"
    );

    // And the write reaches the real state, which is the whole point of the guard-rail.
    assert_eq!(
        0x5A,
        harness
            .host
            .session()
            .expect("a session")
            .core()
            .read_memory(0x0040)
    );
}

/// Q-103: `ReadMem` alone is not concerned — the members that force the run-ahead to 0 are those
/// that **change** the core. The profile runs at the setting.
#[test]
fn q_103_a_profile_that_only_reads_the_core_runs_at_the_setting() {
    let mut harness = LoopHarness::create();
    harness
        .host
        .update_settings(|model| model.set(SettingId::EmuRunAhead, SettingValue::Int(1)));
    harness.launch_rom_with_script("reader", &rom::build(), Some(ONLY_READS_THE_CORE));
    for _ in 0..8 {
        harness.tick_frame();
    }

    assert!(!harness
        .host
        .session()
        .expect("a session")
        .forces_run_ahead_to_zero());
    assert_eq!(1, harness.host.session().expect("a session").run_ahead());
    assert_eq!(
        1,
        effective_run_ahead(&harness),
        "the picture is not one frame in advance"
    );
}

/// Q-103, point 2 again: the user's setting is **kept untouched** — not written, not persisted —
/// so the next game gets it back.
#[test]
fn q_103_the_guard_leaves_the_user_setting_alone() {
    let mut harness = LoopHarness::create();
    harness
        .host
        .update_settings(|model| model.set(SettingId::EmuRunAhead, SettingValue::Int(2)));
    harness.launch_rom_with_script("writer", &rom::build(), Some(WRITES_THE_CORE));
    for _ in 0..8 {
        harness.tick_frame();
    }
    assert_eq!(0, harness.host.session().expect("a session").run_ahead());
    assert_eq!(0, effective_run_ahead(&harness));
    assert_eq!(
        2,
        harness.host.settings().run_ahead(),
        "the forced game changed the setting"
    );

    // The next game is not concerned, so it runs at the value the player chose.
    harness.launch_rom_with_script("quiet", &rom::build(), Some(ONLY_READS_THE_CORE));
    for _ in 0..8 {
        harness.tick_frame();
    }
    let session = harness.host.session().expect("a session");
    assert!(!session.forces_run_ahead_to_zero());
    assert_eq!(2, session.run_ahead(), "the next game lost the setting");
    assert_eq!(2, effective_run_ahead(&harness));
    assert_eq!(2, harness.host.settings().run_ahead());
}

/// A session over `source`, with no ROM: all the guard-rail needs is the profile.
fn session_of(source: &str) -> GameSession {
    let mut profile = GameProfile::create_empty();
    profile.scripts.push(ProfileScript::new(
        Some("aid"),
        true,
        Some(source),
        Some("*"),
    ));
    let mut scene = SceneState::default();
    let mut model = SettingsModel::new();
    GameSession::create(
        SessionRequest {
            profile: &profile,
            title: "Test",
            scene: &mut scene,
            log: Box::new(NullScriptLog),
            different_game: true,
            two_dimensional_mode: None,
        },
        &mut model,
    )
    .expect("a session")
}

/// Q-103, point 3 of the answer: the verdict is reached **when the profile loads**, before a
/// single frame has run, and nothing of it is in the frame loop. What the loop then calls
/// (`apply_run_ahead_setting`) honours it; the raw setter the frame steps and the tests use does
/// not, which is where the two differ.
#[test]
fn q_103_the_guard_is_decided_when_the_profile_loads() {
    let mut forced = session_of(WRITES_THE_CORE);
    assert!(forced.forces_run_ahead_to_zero());
    forced.apply_run_ahead_setting(3);
    assert_eq!(0, forced.run_ahead(), "the setting reached a forced game");
    forced.set_run_ahead(3);
    assert_eq!(3, forced.run_ahead(), "the raw setter is not the guard");

    let mut quiet = session_of(ONLY_READS_THE_CORE);
    assert!(!quiet.forces_run_ahead_to_zero());
    quiet.apply_run_ahead_setting(3);
    assert_eq!(3, quiet.run_ahead());
    // The bounds of REQ-LOOP-064 still hold through the guarded path.
    quiet.apply_run_ahead_setting(17);
    assert_eq!(10, quiet.run_ahead());
    quiet.apply_run_ahead_setting(-1);
    assert_eq!(0, quiet.run_ahead());
}

/// Q-103, point 4 of the answer: the **desktop options window** says so beside the Run-ahead row
/// while the running game forces the value to 0 — in both languages, on that row only, and never
/// when nothing forces it. The verdict comes from the session that is playing.
#[test]
fn q_103_the_options_window_notes_the_forced_run_ahead() {
    use zax_app::ui::options::forced_run_ahead_note;

    let mut harness = LoopHarness::create();
    harness.launch_rom_with_script("writer", &rom::build(), Some(WRITES_THE_CORE));
    let forced = harness
        .host
        .session()
        .expect("a session")
        .forces_run_ahead_to_zero();
    assert!(forced);

    assert!(
        forced_run_ahead_note(SettingId::EmuRunAhead, forced).is_some(),
        "the window says nothing while the game holds run-ahead at 0"
    );
    // No other row carries it, and a game that is not concerned carries none at all.
    assert!(forced_run_ahead_note(SettingId::EmuGameSpeed, forced).is_none());
    assert!(forced_run_ahead_note(SettingId::EmuRunAhead, false).is_none());

    for (language, expected) in [
        (
            zax_app::ui::strings::UiLanguage::English,
            "(forced to 0: this game's scripts write the core)",
        ),
        (
            zax_app::ui::strings::UiLanguage::French,
            "(forcé à 0 : les scripts de ce jeu écrivent dans le cœur)",
        ),
    ] {
        assert_eq!(
            Some(expected),
            zax_app::ui::strings::lookup(language, "hint.run_ahead_forced"),
            "{language:?}"
        );
    }
}

/// Q-103, point 4: the **console's** options page too, on the Run-ahead row of its advanced
/// options, with its own shorter note — a handheld row has no room beside it.
#[test]
fn q_103_the_handheld_page_notes_the_forced_run_ahead() {
    use zax_app::ui::handheld::options::{forced_run_ahead_note, OptionRow};

    let mut harness = LoopHarness::create();
    harness.launch_rom_with_script("writer", &rom::build(), Some(WRITES_THE_CORE));
    let forced = harness
        .host
        .session()
        .expect("a session")
        .forces_run_ahead_to_zero();
    assert!(forced);

    let row = OptionRow::Setting(SettingId::EmuRunAhead);
    assert!(
        zax_app::ui::handheld::advanced::rows().contains(&row),
        "the row the note hangs on is not on the page"
    );
    assert!(forced_run_ahead_note(row, forced).is_some());
    assert!(
        forced_run_ahead_note(OptionRow::Setting(SettingId::EmuCatchUp), forced).is_none(),
        "another advanced row carries the note"
    );
    assert!(forced_run_ahead_note(OptionRow::CatchUp, forced).is_none());
    assert!(forced_run_ahead_note(row, false).is_none());

    for (language, expected) in [
        (
            zax_app::ui::strings::UiLanguage::English,
            "(forced to 0: scripts)",
        ),
        (
            zax_app::ui::strings::UiLanguage::French,
            "(forcé à 0 : scripts)",
        ),
    ] {
        assert_eq!(
            Some(expected),
            zax_app::ui::strings::lookup(language, "handheld.options.run_ahead_forced"),
            "{language:?}"
        );
    }
}

// ------------------------------------------- run-ahead beside "emulation ahead"

/// Run-ahead and the Rust-only "emulation ahead" are orthogonal: with the core on its own thread
/// and the frames pipelined, the real states are still those of a serial run at the same N
/// (`docs/PERFORMANCE-HISTORY.md` §5.5).
#[test]
fn run_ahead_and_emulation_ahead_give_the_same_real_frames() {
    for ahead in [0, 1, 3] {
        let mut serial = LoopHarness::create();
        serial.host.separate_core = false;
        let mut threaded = LoopHarness::create();
        threaded.host.separate_core = true;
        threaded.host.core_ahead = true;
        for harness in [&mut serial, &mut threaded] {
            harness.host.update_settings(|model| {
                model.set(SettingId::EmuRunAhead, SettingValue::Int(ahead));
            });
            harness.launch_rom_with_script("both", &rom::build(), None);
            for _ in 0..8 {
                harness.tick_frame();
            }
        }

        for _ in 0..60 {
            serial.tick_actions(1.0 / 60.0, InputAction::A, InputAction::NONE);
            threaded.tick_actions(1.0 / 60.0, InputAction::A, InputAction::NONE);
        }
        // The threaded run owes one frame at any instant: it is finished before anything reads
        // the core.
        threaded.host.complete_frame_in_flight();

        let left = serial.host.session().unwrap();
        let right = threaded.host.session().unwrap();
        assert_eq!(
            left.core().bus().ram(),
            right.core().bus().ram(),
            "N = {ahead}: the work RAM of the two paths parted"
        );
    }
}
