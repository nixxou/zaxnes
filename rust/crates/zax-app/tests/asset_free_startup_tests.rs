//! Port of `AssetFreeStartupTests.cs`: the publishable base of AMD-024 — the application must run
//! with no asset folder at all (REQ-PUB-001), ship no asset file (REQ-PUB-002), synthesise its
//! audio (REQ-PUB-005) and carry the tile key in code (REQ-PUB-007).
//!
//! Not ported here: the options window and profile chooser rows (T13, `zax-app` UI), the system
//! font lookup and the `--font` option (T14).
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code
#![allow(clippy::float_cmp)] // exact values by construction

mod common;

use std::path::Path;

use common::TemporaryRoot;
use walkdir::WalkDir;
use zax_app::audio::audio_mixer::BLOCK;
use zax_app::audio::synth_audio_library::SynthAudioLibrary;
use zax_app::audio::ui_sound_synth;
use zax_app::audio::{AudioMixer, TrackRandom, UiAudioLibrary, UiSound};
use zax_app::emulation::game_catalog::GameCatalog;
use zax_app::game_session::{GameSession, SessionRequest};
use zax_app::library_game_catalog::LibraryGameCatalog;
use zax_app::neutral_assets;
use zax_app::profile_resolver;
use zax_app::user_data_paths::UserDataPaths;
use zax_profiles::catalogue::ProfileImportOutcome;
use zax_profiles::catalogue::{
    built_in, DEFAULT_THREE_DIMENSIONAL_TITLE, TWO_DIMENSIONAL_MODE_TITLE,
};
use zax_profiles::layout::FieldPresence;
use zax_profiles::profile::GameProfile;
use zax_profiles::{export, format};
use zax_render::scene::SceneState;
use zax_scripting::NullScriptLog;
use zax_settings::catalogue as settings_catalogue;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

fn paths_of(root: &TemporaryRoot) -> UserDataPaths {
    UserDataPaths::resolve(Some(root.path()), None, None)
}

fn open_catalog(root: &TemporaryRoot) -> LibraryGameCatalog {
    LibraryGameCatalog::open(None, &paths_of(root), Box::new(|_| {}))
}

fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

// REQ-PUB-001 (Q-065)
#[test]
fn req_pub_001_the_library_without_an_asset_folder_holds_the_two_generic_entries() {
    let root = TemporaryRoot::new("zax-p7a");
    let catalog = open_catalog(&root);

    let entries = catalog.library().entries();
    assert_eq!(entries.len(), 2);

    // "2D Mode" keeps the identifier REQ-PROF-060 writes in clear, so a library.json from an
    // earlier version still finds its entry (REQ-PERS-035).
    assert_eq!(built_in::TWO_DIMENSIONAL_MODE_ID, "47");

    let two_d: Vec<_> = entries.iter().filter(|entry| entry.is_2d_mode()).collect();
    let three_d: Vec<_> = entries
        .iter()
        .filter(|entry| entry.is_default_three_dimensional())
        .collect();
    assert_eq!(two_d.len(), 1);
    assert_eq!(three_d.len(), 1);
    assert_eq!(two_d[0].display_title(), TWO_DIMENSIONAL_MODE_TITLE);
    assert_eq!(three_d[0].display_title(), DEFAULT_THREE_DIMENSIONAL_TITLE);
    assert_eq!(two_d[0].id(), built_in::TWO_DIMENSIONAL_MODE_ID);
    assert_eq!(three_d[0].id(), built_in::DEFAULT_THREE_DIMENSIONAL_ID);

    // Neither names a file, and neither can be deleted (REQ-UI-037).
    for entry in entries {
        assert_eq!(entry.profile_path(), Path::new(""));
        assert!(!entry.can_delete());
        assert!(entry.is_generic());
    }
}

// REQ-PUB-001 (Q-065)
#[test]
fn req_pub_001_both_generic_entries_resolve_to_the_empty_profile_built_in_code() {
    let root = TemporaryRoot::new("zax-p7a");
    let mut catalog = open_catalog(&root);

    for id in [
        built_in::TWO_DIMENSIONAL_MODE_ID,
        built_in::DEFAULT_THREE_DIMENSIONAL_ID,
    ] {
        assert!(catalog.exists(id));
        let game = catalog.resolve(id).expect("the entry resolves");

        // The profile has no pattern at all: the default geometry of SPEC-04 is what the 3D
        // entry converts with, and the 2D entry shows the pixel frame.
        assert!(game.profile.permanent_patterns.is_empty());
        assert!(game.profile.temporary_patterns.is_empty());
    }

    assert!(
        catalog
            .resolve(built_in::TWO_DIMENSIONAL_MODE_ID)
            .unwrap()
            .is_2d_mode
    );
    let three_d = catalog
        .resolve(built_in::DEFAULT_THREE_DIMENSIONAL_ID)
        .unwrap();
    assert!(!three_d.is_2d_mode);
    assert!(three_d.is_default_three_dimensional);
}

// REQ-PUB-001 (Q-065)
#[test]
fn req_pub_001_the_default_3d_entry_converts_in_3d_and_the_2d_entry_does_not() {
    // REQ-SCN-055 makes a profile with no pattern the 2D game; the default-3D entry asks for the
    // other reading of the same empty profile.
    let profile = GameProfile::create_empty();
    let session_of = |title: &str, two_dimensional_mode: Option<bool>| {
        let mut scene = SceneState::default();
        let request = SessionRequest {
            profile: &profile,
            title,
            scene: &mut scene,
            log: Box::new(NullScriptLog),
            different_game: true,
            two_dimensional_mode,
        };
        GameSession::create(request, &mut SettingsModel::new()).unwrap()
    };

    assert!(session_of("2D Mode", None).two_dimensional_mode());
    assert!(!session_of("3D", Some(false)).two_dimensional_mode());
}

// REQ-PUB-001
#[test]
fn req_pub_001_a_3dn_profile_still_imports_with_no_asset_folder() {
    let root = TemporaryRoot::new("zax-p7a");
    let mut catalog = open_catalog(&root);

    let mut profile = GameProfile::create_empty();
    profile.id = Some("verifier-1".to_owned());
    profile.title = Some("Un jeu importé".to_owned());
    export::normalize(&mut profile, None).unwrap();

    let file = root.path().join("imported.3dn");
    std::fs::write(&file, format::save(&profile, FieldPresence::All)).unwrap();

    let result = catalog.import_profile(&file);
    assert_eq!(result.outcome, ProfileImportOutcome::Added);

    // The entry is there, deletable, and resolves to the profile that was imported.
    let entry = catalog.library().find("verifier-1").unwrap();
    assert_eq!(entry.display_title(), "Un jeu importé");
    assert!(entry.can_delete());
    assert_eq!(catalog.resolve("verifier-1").unwrap().id, "verifier-1");

    // And the two generic entries are still there beside it.
    assert_eq!(catalog.library().entries().len(), 3);
}

// REQ-PUB-001
#[test]
fn req_pub_001_the_asset_folder_is_never_discovered_only_named() {
    // AMD-024: nothing in the repository or the release may depend on an asset folder, so the
    // folder is not looked for. Only --assets names one.
    let root = TemporaryRoot::new("zax-p7a");
    assert_eq!(neutral_assets::locate(None), None);
    assert_eq!(neutral_assets::locate(Some(Path::new("  "))), None);
    assert_eq!(
        neutral_assets::locate(Some(&root.path().join("does-not-exist"))),
        None
    );
    assert_eq!(
        neutral_assets::locate(Some(root.path())),
        Some(std::path::absolute(root.path()).unwrap())
    );
}

// REQ-PUB-001
#[test]
fn req_pub_001_the_command_line_resolves_the_generic_entries_without_assets() {
    // With nothing asked for, the 2D entry is what a launch falls back to (REQ-SCN-055).
    let resolved = profile_resolver::resolve(None, None, &|_| {}).unwrap();
    assert_eq!(resolved.title, TWO_DIMENSIONAL_MODE_TITLE);

    // And the 3D entry is reachable by title.
    let resolved =
        profile_resolver::resolve(Some(DEFAULT_THREE_DIMENSIONAL_TITLE), None, &|_| {}).unwrap();
    assert_eq!(resolved.title, DEFAULT_THREE_DIMENSIONAL_TITLE);

    // --list-profiles prints the two built-ins rather than nothing.
    assert_eq!(profile_resolver::list(None).unwrap().len(), 2);
}

// REQ-PUB-002
#[test]
fn req_pub_002_the_repository_carries_no_asset_file() {
    const FORBIDDEN: [&str; 12] = [
        "bytes", "3dn", "png", "jpg", "jpeg", "obj", "wav", "ogg", "mp3", "ttf", "otf", "ico",
    ];
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|folder| folder.join("rust").join("Cargo.toml").is_file())
        .expect("the repository root holds rust/Cargo.toml");

    let mut stowaways: Vec<String> = Vec::new();
    let mut branding = 0;
    for entry in WalkDir::new(repository)
        .into_iter()
        .filter_entry(|entry| {
            // Build output and the local corpora are not the repository: the public test ROMs
            // and the golden pictures the render tests compare with carry their own rules.
            let relative = entry.path().strip_prefix(repository).unwrap();
            !(relative.starts_with(".git")
                || relative.starts_with("rust/target")
                || relative.starts_with("rust/test-roms")
                || relative.starts_with("rust/goldens")
                || relative.starts_with("tests/roms")
                || relative.starts_with("artifacts")
                || relative.components().any(|part| {
                    matches!(
                        part.as_os_str().to_str(),
                        Some("obj" | "bin" | "TestResults")
                    )
                }))
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let extension = entry
            .path()
            .extension()
            .map(|extension| extension.to_string_lossy().to_lowercase());
        if extension.is_some_and(|extension| FORBIDDEN.contains(&extension.as_str())) {
            let relative = entry.path().strip_prefix(repository).unwrap();
            // REQ-PUB-002 excepts the files generated by the project and listed in
            // ASSETS-LICENSES.md: the logo and the icon made for Zaxnes, under branding/.
            if relative.starts_with("branding") {
                branding += 1;
            } else {
                stowaways.push(relative.display().to_string());
            }
        }
    }
    assert!(stowaways.is_empty(), "{stowaways:?}");

    if branding > 0 {
        let notices = std::fs::read_to_string(repository.join("ASSETS-LICENSES.md")).unwrap();
        assert!(notices.contains("branding/"));
        assert!(notices.contains("GPL-3.0"));
    }
}

// REQ-PUB-007
#[test]
fn req_pub_007_the_tile_encoding_key_is_a_code_constant() {
    use zax_profiles::wire::cipher;

    // The key the imported .3dn files of REQ-PROF-030 are read with is interoperability data,
    // not an asset: it lives in the code and no file is consulted.
    assert_eq!(cipher::KEY, 0x0007_08FA_CD88_4A5A);
    assert_eq!(cipher::ROTATION, 26);

    // And it is still the bijection a round trip needs.
    for plane in [0_u64, 1, 0xDEAD_BEEF_CAFE_F00D, u64::MAX] {
        assert_eq!(cipher::decode(cipher::encode(plane)), plane);
    }
}

// REQ-PUB-005
#[test]
fn req_pub_005_every_interface_event_yields_a_non_silent_synthesised_sound() {
    let library = SynthAudioLibrary::new(48_000);

    for sound in UiSound::ALL {
        let clip = library.sound(sound).expect("a clip");
        assert_eq!(clip.channels, 1);
        assert_eq!(clip.sample_rate, 48_000);

        // Non-silent, and roughly the length the table of SPEC-11 section 2 gives.
        assert!(peak(&clip.samples) > 0.05, "{sound:?} is silent");
        assert!((ui_sound_synth::duration(sound) - clip.duration()).abs() <= 0.002);
    }

    // The four are distinct: no two events share a waveform (REQ-AUD-041).
    for (index, first) in UiSound::ALL.iter().enumerate() {
        for second in &UiSound::ALL[index + 1..] {
            assert_ne!(
                library.sound(*first).unwrap().samples,
                library.sound(*second).unwrap().samples
            );
        }
    }
}

// REQ-PUB-005 (revised 2026-10-21)
#[test]
fn req_pub_005_the_publishable_build_carries_no_menu_music() {
    // The revision of REQ-PUB-005 removes the menu music entirely: the three WAV tracks of
    // SPEC-11 section 2 were copies taken from the original build and are not replaced.
    let library = SynthAudioLibrary::new(48_000);
    assert_eq!(library.track_count(), 0);
    assert!(library.load_track(0).is_none());
}

// REQ-PUB-005 (revised 2026-10-21)
#[test]
fn req_pub_005_nothing_plays_in_the_menus_but_the_interface_sounds() {
    let library = SynthAudioLibrary::new(48_000);
    let mut mixer = AudioMixer::new(48_000, Some(Box::new(library)), TrackRandom::new(11));
    let mut stereo = vec![0.0_f32; BLOCK * 2];

    // Not running, that is on the welcome screen or in the menus: the output stays silent,
    // whatever the music gain climbs to, because there is no track to start.
    for _ in 0..400 {
        mixer.fill(&mut stereo);
        assert_eq!(peak(&stereo), 0.0);
    }
    assert_eq!(mixer.current_track(), None);
    assert_eq!(mixer.track_changes, 0);

    // The interface sounds are still there, and still audible (REQ-AUD-040, REQ-AUD-041).
    mixer.play_sound(UiSound::Confirm);
    assert!(mixer.sound_playing());

    let mut audible = false;
    for _ in 0..20 {
        if audible {
            break;
        }
        mixer.fill(&mut stereo);
        audible = peak(&stereo) > 1e-5;
    }
    assert!(
        audible,
        "the synthesised interface sounds must still be audible"
    );
}

// REQ-PUB-005 (revised 2026-10-21)
#[test]
fn req_pub_005_the_background_music_setting_survives() {
    // The setting stays in the model so that an existing settings.json still reads — and Lua
    // still sees Setting.BGMusic. (Its absence from the options window is checked with the UI.)
    assert!(settings_catalogue::descriptors()
        .any(|descriptor| descriptor.id() == SettingId::AudioBgMusic));
}
