//! The command line of AMD-030 §2: `--import-3dsen <folder|file>` imports, prints the summary
//! and exits with 0 when it all worked (`ThreeDSenCliTests.cs`).

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use std::path::{Path, PathBuf};

use common::TemporaryRoot;
use ui_harness::bundle_support::synthetic_bundle::{self, SyntheticBundleOptions};
use zax_app::app_options::{AppOptions, OptionsError, USAGE};
use zax_app::three_d_sen_command::{self, EXIT_FAILURE, EXIT_SUCCESS};
use zax_profiles::format;
use zax_profiles::import::{expected_layout, UnityCompression, BUNDLE_EXTENSION, BUNDLE_PREFIX};
use zax_profiles::layout::FieldPresence;
use zax_profiles::profile::GameProfile;

/// A profile the decoder accepts, with the identifier and the title given.
fn profile(id: &str) -> Vec<u8> {
    let mut profile = GameProfile::create_empty();
    profile.id = Some(id.to_owned());
    profile.title = Some(id.to_owned());
    zax_profiles::export::normalize(&mut profile, None).expect("a valid profile");
    format::save(&profile, FieldPresence::All)
}

/// A temporary 3dSen installation and a temporary profile folder, both outside the repository:
/// no file of 3dSen and no imported profile ever lands in the tree.
struct Sandbox {
    root: TemporaryRoot,
    output: Vec<u8>,
    errors: Vec<u8>,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            root: TemporaryRoot::new("zax-p8-cli-"),
            output: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn install_root(&self) -> PathBuf {
        let install = self.root.path().join("3dSen");
        std::fs::create_dir_all(&install).expect("the installation folder can be created");
        install
    }

    fn profiles_directory(&self) -> PathBuf {
        self.root.path().join("userdata").join("profiles")
    }

    /// Writes a synthetic bundle in the layout of AMD-030 §3 and returns its path.
    fn write_bundle(&self, assets: &[(&str, Vec<u8>)]) -> PathBuf {
        let directory = self
            .install_root()
            .join("3dSen_Data")
            .join("StreamingAssets")
            .join("aa")
            .join("StandaloneWindows");
        std::fs::create_dir_all(&directory).expect("the bundle folder can be created");
        let bundle = directory.join(format!("{BUNDLE_PREFIX}0123456789abcdef{BUNDLE_EXTENSION}"));
        let options = SyntheticBundleOptions {
            block_compression: UnityCompression::Lz4,
            block_size: 128 * 1024,
            ..SyntheticBundleOptions::default()
        };
        std::fs::write(&bundle, synthetic_bundle::build(assets, &options))
            .expect("the bundle can be written");
        bundle
    }

    fn run(&mut self, source: &Path) -> i32 {
        three_d_sen_command::run_into(
            source,
            &self.profiles_directory(),
            &mut self.output,
            &mut self.errors,
        )
        .expect("the output can be written")
    }

    fn output(&self) -> String {
        String::from_utf8_lossy(&self.output).into_owned()
    }

    fn errors(&self) -> String {
        String::from_utf8_lossy(&self.errors).into_owned()
    }

    fn clear(&mut self) {
        self.output.clear();
        self.errors.clear();
    }

    /// The `.3dn` files of the profile folder.
    fn profile_files(&self) -> Vec<PathBuf> {
        std::fs::read_dir(self.profiles_directory()).map_or_else(
            |_| Vec::new(),
            |listing| {
                listing
                    .flatten()
                    .map(|item| item.path())
                    .filter(|path| path.extension().is_some_and(|e| e == "3dn"))
                    .collect()
            },
        )
    }
}

fn parse(arguments: &[&str]) -> AppOptions {
    AppOptions::parse(arguments).expect("the arguments parse")
}

#[test]
fn the_option_takes_a_folder_or_a_file() {
    let options = parse(&["--import-3dsen", r"C:\Games\3dSen"]);

    assert_eq!(
        options.import_three_d_sen_path.as_deref(),
        Some(r"C:\Games\3dSen")
    );
    assert!(!options.smoke_test);
    assert_eq!(options.rom_path, None);
}

/// AMD-030 §2: a malformed command line is a usage error, which the program maps to 2.
#[test]
fn the_option_without_a_value_is_a_usage_error() {
    let error = AppOptions::parse(&["--import-3dsen"]).expect_err("a value is required");

    assert_eq!(
        error,
        OptionsError::MissingValue {
            option: "--import-3dsen"
        }
    );
    assert!(error.to_string().contains("--import-3dsen"));
}

#[test]
fn the_help_names_the_option() {
    assert!(USAGE.contains("--import-3dsen"));
}

/// AMD-030 §2: "importe puis quitte avec le code 0 si tout s'est bien passé".
#[test]
fn an_import_that_works_exits_with_zero_and_prints_the_summary() {
    let mut sandbox = Sandbox::new();
    sandbox.write_bundle(&[
        ("Pinball", profile("ABCDEFGHIJKL")),
        ("Tetris", profile("MNOPQRSTUVWX")),
    ]);

    let code = sandbox.run(&sandbox.install_root());

    assert_eq!(code, EXIT_SUCCESS);
    assert!(sandbox.output().contains("2 profile(s) imported"));
    assert!(sandbox.output().contains("0 failed"));
    assert_eq!(sandbox.errors(), "");
    assert_eq!(sandbox.profile_files().len(), 2);
}

/// AMD-030 §2: the bundle may be named directly.
#[test]
fn the_bundle_file_can_be_named_on_the_command_line() {
    let mut sandbox = Sandbox::new();
    let bundle = sandbox.write_bundle(&[("Pinball", profile("ABCDEFGHIJKL"))]);

    assert_eq!(sandbox.run(&bundle), EXIT_SUCCESS);
    assert!(sandbox.profiles_directory().join("Pinball.3dn").is_file());
}

/// AMD-030 §2: a second import adds nothing and still succeeds.
#[test]
fn a_second_import_reports_the_profiles_as_already_present() {
    let mut sandbox = Sandbox::new();
    sandbox.write_bundle(&[("Pinball", profile("ABCDEFGHIJKL"))]);

    assert_eq!(sandbox.run(&sandbox.install_root()), EXIT_SUCCESS);
    sandbox.clear();
    assert_eq!(sandbox.run(&sandbox.install_root()), EXIT_SUCCESS);

    assert!(sandbox
        .output()
        .contains("0 profile(s) imported, 1 already present"));
    assert_eq!(sandbox.profile_files().len(), 1);
}

/// AMD-030 §5: one bad payload fails, the rest is imported, and the code is not 0.
#[test]
fn a_bundle_with_one_bad_payload_exits_with_one_and_imports_the_rest() {
    let mut sandbox = Sandbox::new();
    sandbox.write_bundle(&[
        ("junk", vec![1, 2, 3, 4]),
        ("Pinball", profile("ABCDEFGHIJKL")),
    ]);

    let code = sandbox.run(&sandbox.install_root());

    assert_eq!(code, EXIT_FAILURE);
    assert!(sandbox
        .output()
        .contains("1 profile(s) imported, 0 already present, 1 failed"));
    assert!(sandbox.errors().contains("junk"));
    assert_eq!(sandbox.profile_files().len(), 1);
}

/// AMD-030 §3: nothing found, a clear message and no file written.
#[test]
fn an_installation_without_a_bundle_exits_with_one() {
    let mut sandbox = Sandbox::new();
    std::fs::create_dir_all(sandbox.install_root().join("3dSen_Data"))
        .expect("the data folder can be created");

    let code = sandbox.run(&sandbox.install_root());

    assert_eq!(code, EXIT_FAILURE);
    assert!(sandbox
        .errors()
        .contains("no 3dSen profile bundle was found"));
    assert!(sandbox.errors().contains(&expected_layout()));
    assert_eq!(sandbox.output(), "");
    assert_eq!(sandbox.profile_files().len(), 0);
}

/// AMD-030 §5: a corrupt bundle never crashes and writes nothing.
#[test]
fn a_corrupt_bundle_exits_with_one_without_writing() {
    let mut sandbox = Sandbox::new();
    let bundle = sandbox.write_bundle(&[("Pinball", profile("ABCDEFGHIJKL"))]);
    let whole = std::fs::read(&bundle).expect("the bundle can be read");
    std::fs::write(&bundle, &whole[..whole.len() / 2]).expect("the bundle can be cut");

    let code = sandbox.run(&sandbox.install_root());

    assert_eq!(code, EXIT_FAILURE);
    assert_ne!(sandbox.errors(), "");
    assert!(!sandbox.output().contains("profile(s) imported"));
    assert_eq!(sandbox.profile_files().len(), 0);
}

#[test]
fn a_path_that_does_not_exist_exits_with_one() {
    let mut sandbox = Sandbox::new();
    let missing = sandbox.root.path().join("nowhere");

    let code = sandbox.run(&missing);

    assert_eq!(code, EXIT_FAILURE);
    assert!(sandbox.errors().contains("neither a file nor a folder"));
}

/// AMD-030 §5: the profile is written byte for byte as the bundle holds it.
#[test]
fn the_written_file_is_the_payload_unchanged() {
    let mut sandbox = Sandbox::new();
    let payload = profile("ABCDEFGHIJKL");
    sandbox.write_bundle(&[("Blaster Master (U)", payload.clone())]);

    assert_eq!(sandbox.run(&sandbox.install_root()), EXIT_SUCCESS);

    let written = sandbox.profiles_directory().join("Blaster Master (U).3dn");
    assert_eq!(
        std::fs::read(written).expect("the profile is written"),
        payload
    );
}
