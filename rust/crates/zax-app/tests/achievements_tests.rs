//! RetroAchievements in the host (Rust-only, owner request), against a fake server: the saved
//! token signs in, the game loads and an achievement unlocks on the emulated frames in
//! softcore, save states carry the progress, a password sign-in keeps only the token.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::LoopHarness;
use zax_achievements::{HttpRequest, HttpResponse, Transport};
use zax_app::achievements::{Account, AchievementSystem};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::persistence::save_state_file;
use zax_settings::catalogue::SettingId;

/// One achievement on the counter ROM: $10 is not zero for 120 frames.
const ACHIEVEMENT_SETS: &str = r#"{"Success":true,"GameId":77,"Title":"Counter","ConsoleId":7,"ImageIconUrl":"","RichPresenceGameId":77,"RichPresencePatch":"","Sets":[{"AchievementSetId":1,"GameId":77,"Title":null,"Type":"core","ImageIconUrl":"","Achievements":[{"ID":501,"Title":"Count on it","Description":"Count for two seconds","Flags":3,"Points":10,"MemAddr":"0xH0010!=0.120.","Author":"Test","BadgeName":"00501","Created":1,"Modified":1}],"Leaderboards":[]}]}"#;

#[derive(Default)]
struct FakeServer {
    forms: Mutex<Vec<String>>,
}

impl Transport for FakeServer {
    fn send(&self, request: &HttpRequest) -> HttpResponse {
        let form = request.post_data.clone().unwrap_or_default();
        self.forms.lock().unwrap().push(form.clone());
        let body = if form.starts_with("r=login2") {
            r#"{"Success":true,"User":"Player","Token":"token-from-server","Score":7,"SoftcoreScore":3,"Messages":0,"Permissions":1,"AccountType":"Registered"}"#
        } else if form.starts_with("r=achievementsets") {
            ACHIEVEMENT_SETS
        } else if form.starts_with("r=awardachievement") {
            r#"{"Success":true,"Score":7,"SoftcoreScore":13,"AchievementID":501,"AchievementsRemaining":0}"#
        } else {
            r#"{"Success":true}"#
        };
        HttpResponse {
            status: if request.post_data.is_some() {
                200
            } else {
                404
            },
            body: body.as_bytes().to_vec(),
        }
    }
}

impl FakeServer {
    fn saw(&self, prefix: &str) -> Option<String> {
        self.forms
            .lock()
            .unwrap()
            .iter()
            .find(|form| form.starts_with(prefix))
            .cloned()
    }
}

/// A host whose achievements reach `server`, with `token` saved for the account `Player`.
fn harness(server: &Arc<FakeServer>, token: &str) -> LoopHarness {
    let mut harness = LoopHarness::create();
    let transport: Arc<dyn Transport> = server.clone();
    harness.host.achievements = AchievementSystem::with_transport(transport, "Zaxnes/test");
    harness.host.update_settings(|settings| {
        settings.set(SettingId::AchievementsUsername, "Player");
        settings.set(SettingId::AchievementsToken, token);
        settings.set(SettingId::AchievementsEnabled, true);
    });
    harness.host.dispatch_setting_changes();
    harness
}

/// Ticks (the requests answer on their own threads) until `done` holds.
fn wait_for(harness: &mut LoopHarness, mut done: impl FnMut(&LoopHarness) -> bool) {
    for _ in 0..2000 {
        if done(harness) {
            return;
        }
        harness.tick(1.0 / 1000.0);
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("the fake server's answers never arrived");
}

#[test]
fn the_saved_token_signs_in_and_a_frame_unlocks_in_softcore() {
    let server = Arc::new(FakeServer::default());
    let mut harness = harness(&server, "saved-token");
    wait_for(&mut harness, |harness| {
        matches!(
            harness.host.achievements.account(),
            Account::SignedIn { .. }
        )
    });
    assert!(server
        .saw("r=login2")
        .is_some_and(|form| form.contains("t=saved-token")));

    harness.launch("g1");
    wait_for(&mut harness, |harness| {
        harness.host.achievements.game().is_some()
    });
    let game = harness.host.achievements.game().unwrap();
    assert_eq!("Counter", game.title);
    assert_eq!(1, game.summary.core);
    assert!(server
        .saw("r=startsession")
        .is_some_and(|form| form.contains("&h=0&")));

    harness.run_frames(150);
    wait_for(&mut harness, |_| server.saw("r=awardachievement").is_some());
    assert!(server
        .saw("r=awardachievement")
        .is_some_and(|form| form.contains("a=501") && form.contains("&h=0&")));
    let unlocked = harness
        .host
        .achievements
        .achievements()
        .into_iter()
        .find(|entry| entry.id == 501)
        .unwrap();
    assert!(unlocked.unlocked);
}

#[test]
fn a_save_state_carries_the_progress_of_the_achievements() {
    let server = Arc::new(FakeServer::default());
    let mut harness = harness(&server, "saved-token");
    harness.launch("g1");
    wait_for(&mut harness, |harness| {
        harness.host.achievements.game().is_some()
    });
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(3));
    let file = std::fs::read(harness.paths.slot_state_file("3")).unwrap();
    let content = save_state_file::try_read(&file).expect("a readable state");
    assert_ne!(Vec::<u8>::new(), content.achievements);

    // The same game again: the set stays, no second load from the server.
    let loads = || {
        server
            .forms
            .lock()
            .unwrap()
            .iter()
            .filter(|form| form.starts_with("r=achievementsets"))
            .count()
    };
    harness.run_with(Command::LoadSlot, CommandArgs::of_number(3));
    harness.run_frames(10);
    assert_eq!(1, loads());
    assert!(harness.host.achievements.game().is_some());
}

#[test]
fn a_state_file_reads_back_its_achievements_with_or_without_a_thumbnail() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    let mut content = harness.host.capture_save_state("1").unwrap();
    content.achievements = vec![1, 2, 3, 4];
    let read = save_state_file::try_read(&save_state_file::write(&content)).unwrap();
    assert_eq!(content, read);
    content.thumbnail.clear();
    let read = save_state_file::try_read(&save_state_file::write(&content)).unwrap();
    assert_eq!(content, read);
}

#[test]
fn a_password_sign_in_keeps_the_token_and_a_sign_out_forgets_it() {
    let server = Arc::new(FakeServer::default());
    let mut harness = harness(&server, "");
    assert_eq!(&Account::SignedOut, harness.host.achievements.account());
    harness.host.sign_in_to_achievements("Player", "secret");
    wait_for(&mut harness, |harness| {
        matches!(
            harness.host.achievements.account(),
            Account::SignedIn { .. }
        )
    });
    harness.tick_frame();
    let settings = harness.host.settings();
    assert_eq!(
        "token-from-server",
        settings.get(SettingId::AchievementsToken).as_text()
    );

    harness.host.sign_out_of_achievements();
    assert_eq!(
        "",
        harness
            .host
            .settings()
            .get(SettingId::AchievementsToken)
            .as_text()
    );
    assert_eq!(&Account::SignedOut, harness.host.achievements.account());
}

#[test]
fn turned_off_nothing_reaches_the_server() {
    let server = Arc::new(FakeServer::default());
    let mut harness = LoopHarness::create();
    let transport: Arc<dyn Transport> = server.clone();
    harness.host.achievements = AchievementSystem::with_transport(transport, "Zaxnes/test");
    harness.launch("g1");
    harness.run_frames(30);
    std::thread::sleep(Duration::from_millis(20));
    harness.tick_frame();
    assert!(server.forms.lock().unwrap().is_empty());
    assert!(harness.host.achievements.popup().is_none());
    assert!(!harness.host.achievements.is_enabled());
}
