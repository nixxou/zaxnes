//! The library and the imports with both formats (AMD-062 REQ-PROF-062, REQ-PROF-063, and the
//! amended AMD-030 §5 and §6 / AMD-039 §4).

use std::path::{Path, PathBuf};

use zax_profiles::catalogue::{
    derived_id, import_messages, ProfileCatalogue, ProfileImportOutcome, ProfileOrigin,
    UserProfile, UserProfileFolder,
};
use zax_profiles::format::{self, ProfileLoadErrorKind};
use zax_profiles::hash::envelope::messages;
use zax_profiles::import::{read_text_assets, ThreeDSenImporter};
use zax_profiles::library::GameLibrary;
use zax_profiles::wire::hash_envelope;

use zax_profiles::catalogue::ProfileFileSystem;

use super::common::fake_fs::FakeProfileFileSystem;
use super::common::neutral_assets;
use super::common::synthetic_bundle::{self, SyntheticBundleOptions};
use super::common::synthetic_hash_profile::EnvelopeSpec;

/// The profiles folder the tests scan and import into.
fn destination() -> PathBuf {
    PathBuf::from("C:").join("user").join("profiles")
}

/// A file the user points at, outside the profiles folder.
fn source(name: &str) -> PathBuf {
    PathBuf::from("C:").join("downloads").join(name)
}

fn folder<'a>(files: &'a FakeProfileFileSystem, directory: &Path) -> UserProfileFolder<'a> {
    UserProfileFolder::new(files, directory, None, ProfileOrigin::User)
}

fn find<'a>(profiles: &'a [UserProfile], file_name: &str) -> &'a UserProfile {
    let found: Vec<&UserProfile> = profiles
        .iter()
        .filter(|profile| profile.file_path.ends_with(file_name))
        .collect();
    assert_eq!(1, found.len(), "one profile of {file_name}");
    found[0]
}

/// The `Id` of the envelope of a "3dh" file.
fn envelope_id(bytes: &[u8]) -> String {
    hash_envelope::decode(bytes).unwrap().id.unwrap_or_default()
}

// REQ-PROF-062: the user profiles folder is scanned for `.3dn` **and** `.3dh`, and the content
// decides the format.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_062_the_user_folder_scans_both_extensions() {
    let files = FakeProfileFileSystem::new();
    let hash_profile = neutral_assets::read_hash_profile("BalloonFight.bytes");
    let plain_profile = neutral_assets::read_profile("Mega Man 2.bytes");
    let zelda = neutral_assets::read_hash_profile("Legend Of Zelda.bytes");

    // One file of each format, under each extension. A "3dh" profile named `.3dn` is what the
    // import from an installation writes (AMD-030 §5 as AMD-062 amends it), and a 3dn profile
    // named `.3dh` is read just as well: the content decides (REQ-PROF-070).
    files.add_file(&destination().join("a-hash.3dh"), &hash_profile);
    files.add_file(&destination().join("b-plain.3dn"), &plain_profile);
    files.add_file(&destination().join("c-misnamed.3dn"), &zelda);
    files.add_file(
        &destination().join("d-misnamed.3dh"),
        &neutral_assets::read_profile("Bomberman.bytes"),
    );

    let profiles = folder(&files, &destination()).scan(&[]).unwrap();
    assert_eq!(4, profiles.len());

    // The identifier of a "3dh" entry is the one of its envelope (REQ-PROF-073, AMD-037).
    assert_eq!(envelope_id(&hash_profile), find(&profiles, "a-hash.3dh").id);
    assert_eq!(envelope_id(&zelda), find(&profiles, "c-misnamed.3dn").id);

    // A 3dn keeps its own, carried or derived (AMD-037 §2).
    let header = format::load_header(&plain_profile, None, None).unwrap();
    let expected = if header.has_id() {
        header.id.clone().unwrap_or_default()
    } else {
        derived_id(&plain_profile)
    };
    assert_eq!(expected, find(&profiles, "b-plain.3dn").id);
}

// REQ-PROF-063: an imported file is copied as it is, with no conversion.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_063_an_imported_hash_profile_is_copied_as_it_is() {
    let files = FakeProfileFileSystem::new();
    let bytes = neutral_assets::read_hash_profile("Super Mario Bros.bytes");
    let chosen = source("Super Mario Bros.3dh");
    files.add_file(&chosen, &bytes);

    let result = folder(&files, &destination())
        .import(&chosen, &[], &[])
        .unwrap();

    assert_eq!(ProfileImportOutcome::Added, result.outcome);
    assert_eq!(envelope_id(&bytes), result.profile.unwrap().id);

    let written = destination().join("Super Mario Bros.3dh");
    assert!(files.file_exists(&written));
    assert_eq!(bytes, files.read_all_bytes(&written).unwrap());
    assert!(zax_profiles::hash::envelope::is_hash_profile(
        &files.read_all_bytes(&written).unwrap()
    ));
}

// REQ-PROF-063 and AMD-056: the dialogues are unchanged, and the cause **shown** for a refused
// "3dh" is the third text of REQ-UI-084.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_063_a_refused_hash_profile_shows_the_obsolete_format_message() {
    let files = FakeProfileFileSystem::new();
    let envelope =
        hash_envelope::decode(&neutral_assets::read_hash_profile("BalloonFight.bytes")).unwrap();

    // An envelope version outside "1", "2" and "5" is a refusal of REQ-PROF-072.
    let bytes = EnvelopeSpec {
        version: Some("4".to_owned()),
        payload: envelope.payload.clone(),
        ..EnvelopeSpec::default()
    }
    .encode();
    let chosen = source("bad.3dh");
    files.add_file(&chosen, &bytes);

    let plan = folder(&files, &destination())
        .inspect(&chosen, &[], &[])
        .unwrap();

    assert_eq!(ProfileImportOutcome::Rejected, plan.outcome);
    assert_eq!(import_messages::obsolete_format(&chosen), plan.message);
    assert!(!plan.writes());

    // Loading it is what names the cause of REQ-PROF-072 on the log line.
    let error = format::load(&bytes, None, None).unwrap_err();
    assert_eq!(ProfileLoadErrorKind::UnsupportedHashEnvelope, error.kind);
    assert_eq!(messages::VERSION_4, error.message);
}

// REQ-PROF-072 and REQ-PROF-073: a "3dh" whose payload was damaged still appears in the list of
// games, with the `Id` and the `Title` of its envelope, and is refused at load time.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_073_a_damaged_hash_profile_still_enters_the_library() {
    let files = FakeProfileFileSystem::new();
    let original = neutral_assets::read_hash_profile("Super Mario Bros.bytes");
    let envelope = hash_envelope::decode(&original).unwrap();
    let mut payload = envelope.payload.clone().unwrap_or_default();
    payload[200] ^= 0x01;

    let bytes = EnvelopeSpec {
        payload: Some(payload),
        payload_hash_override: envelope.payload_hash.clone(),
        id: envelope.id.clone(),
        title: envelope.title.clone(),
        ..EnvelopeSpec::default()
    }
    .encode();
    files.add_file(&destination().join("damaged.3dh"), &bytes);

    let profiles = folder(&files, &destination()).scan(&[]).unwrap();
    assert_eq!(1, profiles.len());
    assert_eq!(envelope_id(&original), profiles[0].id);
    assert_eq!(envelope.title, profiles[0].header.title);

    let error = format::load(&bytes, None, None).unwrap_err();
    assert_eq!(ProfileLoadErrorKind::DamagedHashPayload, error.kind);
    assert_eq!(messages::DAMAGED_PAYLOAD, error.message);
}

// AMD-030 §5 as AMD-062 amends it: a synthetic bundle whose TextAsset is the "3dh"
// `BalloonFight.bytes` imports that file byte for byte, under the `.3dn` extension.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn amd_030_a_bundle_carrying_a_hash_profile_imports_it_byte_for_byte() {
    let bytes = neutral_assets::read_hash_profile("BalloonFight.bytes");
    let files = FakeProfileFileSystem::new();
    let bundle = synthetic_bundle::build(
        &[("BalloonFight", bytes.clone())],
        &SyntheticBundleOptions::three_d_sen_2(),
    );

    let assets = read_text_assets(&bundle).unwrap();
    let bundle_path = Path::new("/3dSen/bundle");
    let summary = ThreeDSenImporter::new(&files, &destination(), None)
        .import(bundle_path, &assets)
        .unwrap();

    assert_eq!(1, summary.imported);
    assert_eq!(0, summary.failed);

    // The file keeps the `.3dn` extension even though its content is a "3dh": the format is
    // recognised by the content (REQ-PROF-070).
    let written = destination().join("BalloonFight.3dn");
    assert!(files.file_exists(&written));
    assert_eq!(bytes, files.read_all_bytes(&written).unwrap());
    assert!(zax_profiles::hash::envelope::is_hash_profile(
        &files.read_all_bytes(&written).unwrap()
    ));

    // A second import of the same bundle adds nothing (AMD-030 §2).
    let again = ThreeDSenImporter::new(&files, &destination(), None)
        .import(bundle_path, &assets)
        .unwrap();
    assert_eq!(0, again.imported);
    assert_eq!(1, again.already_present);
}

// AMD-065 REQ-PROF-086, REQ-PROF-087, on the 101 pairs of the extracted assets: a 2.0 profile and
// a 2.1 one of the same game, side by side in the user folder with the names their installation
// gives them. Whichever of the two conditions pairs them, the library has **one** entry, the 2.1
// one, and the 3dn file stays on the disk.
//
// This replaces what AMD-039 §4 last bullet and AMD-062 §8 accepted — two entries for the 45 games
// whose files carry different identifiers (AMD-065 §4).
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn amd_065_a_two_zero_and_a_two_one_profile_of_the_same_game_make_one_entry() {
    // No built-in profile at all: the two user files are alone, which is AMD-065 §3's third
    // criterion.
    let catalogue = ProfileCatalogue::from_entries(Path::new(""), Vec::new());
    let mut same_id = 0;
    let mut same_name = 0;
    for file in neutral_assets::hash_profile_files() {
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        let stem = name.trim_end_matches(".bytes");
        let hash_bytes = std::fs::read(&file).unwrap();
        let plain_name = neutral_assets::counterpart_name(stem);
        let plain_bytes = neutral_assets::read_profile(&neutral_assets::counterpart_name(&name));

        let files = FakeProfileFileSystem::new();
        let plain_path = destination().join(format!("{plain_name}.3dn"));
        let hash_path = destination().join(format!("{stem}.3dh"));
        files.add_file(&plain_path, &plain_bytes);
        files.add_file(&hash_path, &hash_bytes);

        let profiles = folder(&files, &destination()).scan(&[]).unwrap();
        let envelope = envelope_id(&hash_bytes);
        let header = format::load_header(&plain_bytes, None, None).unwrap();

        if header.has_id() {
            // Both files carry the same identifier (REQ-PROF-086 point 1), so the scan already
            // yields one profile — and it must be the 2.1 one.
            assert_eq!(header.id.clone().unwrap_or_default(), envelope, "{name}");
            assert_eq!(1, profiles.len(), "{name}");
            assert_eq!(envelope, profiles[0].id);
            assert!(
                profiles[0].file_path.ends_with(format!("{stem}.3dh")),
                "{name}"
            );
            same_id += 1;
        } else {
            // The 3dn takes the derived identifier of AMD-037, the "3dh" the one of its envelope:
            // two profiles, which the name then pairs (REQ-PROF-086 point 2).
            assert_eq!(2, profiles.len(), "{name}");
            assert_eq!(
                derived_id(&plain_bytes),
                find(&profiles, &format!("{plain_name}.3dn")).id,
                "{name}"
            );
            assert_eq!(
                envelope,
                find(&profiles, &format!("{stem}.3dh")).id,
                "{name}"
            );
            same_name += 1;
        }

        // REQ-PROF-087: one entry, shown from the 2.1 profile, and nothing deleted or rewritten.
        let library = GameLibrary::build(&catalogue, &profiles, None, None);
        assert_eq!(1, library.entries().len(), "{name}");
        assert_eq!(envelope, library.entries()[0].id(), "{name}");
        assert!(
            library.entries()[0]
                .profile_path()
                .ends_with(format!("{stem}.3dh")),
            "{name}"
        );
        assert_eq!(plain_bytes, files.read_all_bytes(&plain_path).unwrap());
    }

    // REQ-PROF-073, notes: 56 of the 101 counterparts carry an identifier.
    assert_eq!(56, same_id);
    assert_eq!(45, same_name);
}
