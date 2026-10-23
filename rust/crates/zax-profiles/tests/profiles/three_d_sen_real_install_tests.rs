//! The import against a real (or stand-in) 3dSen installation. The tests need the built-in
//! profiles (`ZAXNES_PROFILES`, for `inventory.json`) and the installation folder
//! (`ZAXNES_3DSEN_INSTALL`), so they are `#[ignore]`d.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zax_profiles::catalogue::UserProfileFolder;
use zax_profiles::format;
use zax_profiles::import::{
    find_bundle, is_bundle_name, locate_bundle, read_text_assets, ThreeDSenImporter, UnityBundle,
    UnitySerializedFile,
};

use super::common::neutral_assets;
use super::common::physical_fs::PhysicalProfileFileSystem;

const PROFILE_COUNT: usize = 101;

fn install_root() -> PathBuf {
    PathBuf::from(env::var_os("ZAXNES_3DSEN_INSTALL").unwrap_or_else(|| {
        panic!(
            "ZAXNES_3DSEN_INSTALL is not set: it names a 3dSen installation (or a stand-in) folder"
        )
    }))
}

fn reference_hashes() -> HashSet<String> {
    let text = fs::read_to_string(neutral_assets::root().join("inventory.json")).unwrap();
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry.get("kind").and_then(serde_json::Value::as_str) == Some("Profile"))
        .filter_map(|entry| entry.get("sha256").and_then(serde_json::Value::as_str))
        .filter(|hash| !hash.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn hash_of(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

/// AMD-030 §3
#[test]
#[ignore = "needs ZAXNES_PROFILES and ZAXNES_3DSEN_INSTALL (a 3dSen installation)"]
fn the_bundle_is_found_from_the_installation_folder() {
    let bundle = find_bundle(&PhysicalProfileFileSystem, &install_root()).unwrap();

    assert!(is_bundle_name(
        &bundle.file_name().unwrap().to_string_lossy()
    ));
    assert!(bundle.to_string_lossy().contains("StandaloneWindows"));
}

/// AMD-030 §4
#[test]
#[ignore = "needs ZAXNES_PROFILES and ZAXNES_3DSEN_INSTALL (a 3dSen installation)"]
fn the_bundle_holds_the_101_profiles_and_the_asset_bundle_object() {
    let bundle = locate_bundle(&PhysicalProfileFileSystem, &install_root()).unwrap();
    let bytes = fs::read(bundle).unwrap();
    let container = UnityBundle::read(&bytes).unwrap();

    // §4.1 and §4.2: version 7, one CAB- node, blocks of 128 KiB.
    assert_eq!(7, container.format_version);
    assert_eq!(1, container.files.len());
    let node = &container.files[0];
    assert!(node.path.starts_with("CAB-"));

    // §4.3: version 21, little-endian, 102 objects.
    let serialized = UnitySerializedFile::read(&node.data).unwrap();
    assert_eq!(21, serialized.version);
    assert!(!serialized.is_big_endian);
    assert_eq!(PROFILE_COUNT + 1, serialized.objects.len());

    let assets = read_text_assets(&bytes).unwrap();
    assert_eq!(PROFILE_COUNT, assets.len());
    assert!(assets.iter().all(|asset| !asset.name.is_empty()));
}

/// AMD-030 §5, §6
#[test]
#[ignore = "needs ZAXNES_PROFILES and ZAXNES_3DSEN_INSTALL (a 3dSen installation)"]
fn every_payload_decodes_with_the_spec_03_decoder() {
    let bundle = locate_bundle(&PhysicalProfileFileSystem, &install_root()).unwrap();

    for asset in read_text_assets(&fs::read(bundle).unwrap()).unwrap() {
        if let Err(error) = format::load(&asset.script, None, Some(&asset.name)) {
            panic!("{}: {error}", asset.name);
        }
    }
}

/// AMD-030 §6, first and second bullets
#[test]
#[ignore = "needs ZAXNES_PROFILES and ZAXNES_3DSEN_INSTALL (a 3dSen installation)"]
fn the_import_writes_101_profiles_of_the_inventory_and_the_second_one_adds_nothing() {
    let reference = reference_hashes();
    assert_eq!(PROFILE_COUNT, reference.len());

    // Outside the repository on purpose: no file of 3dSen ever lands in the tree.
    let scratch = env::temp_dir().join(format!("zax-p8-{}", std::process::id()));
    let destination = scratch.join("profiles");
    let outcome = std::panic::catch_unwind(|| {
        let importer = ThreeDSenImporter::new(&PhysicalProfileFileSystem, &destination, None);

        let first = importer.import_from(&install_root()).unwrap();

        assert_eq!(PROFILE_COUNT, first.imported);
        assert_eq!(0, first.already_present);
        assert_eq!(0, first.failed);

        let written: Vec<PathBuf> = fs::read_dir(&destination)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(PROFILE_COUNT, written.len());
        assert!(written.iter().all(|path| path
            .to_string_lossy()
            .ends_with(UserProfileFolder::EXTENSION)));

        // Each produced file is, byte for byte, a profile of the reference inventory.
        for path in &written {
            assert!(reference.contains(&hash_of(path)), "{}", path.display());
        }

        // One profile per inventory entry: no duplicate, nothing missing.
        let distinct: HashSet<String> = written.iter().map(|path| hash_of(path)).collect();
        assert_eq!(PROFILE_COUNT, distinct.len());

        let second = importer.import_from(&install_root()).unwrap();

        assert_eq!(0, second.imported);
        assert_eq!(PROFILE_COUNT, second.already_present);
        assert_eq!(0, second.failed);
        assert_eq!(PROFILE_COUNT, fs::read_dir(&destination).unwrap().count());
    });
    fs::remove_dir_all(&scratch).unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
