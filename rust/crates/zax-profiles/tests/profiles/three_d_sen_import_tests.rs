use std::io;
use std::path::{Path, PathBuf};

use rstest::rstest;
use zax_profiles::catalogue::ProfileFileSystem;
use zax_profiles::format;
use zax_profiles::import::{
    expected_layout, find_bundle, locate_bundle, ThreeDSenImporter, ThreeDSenProfileOutcome,
    UnityTextAsset, BUNDLE_EXTENSION, BUNDLE_PREFIX,
};
use zax_profiles::layout::FieldPresence;
use zax_profiles::profile::GameProfile;

use super::common::fake_fs::FakeProfileFileSystem;
use super::common::recording_log::RecordingProfileLog;
use super::common::synthetic_bundle::{self, SyntheticBundleOptions};

const DESTINATION: &str = "/zax/profiles";

fn destination(name: &str) -> PathBuf {
    Path::new(DESTINATION).join(name)
}

fn profile(id: &str, title: &str) -> Vec<u8> {
    let mut profile = GameProfile::create_empty();
    profile.id = Some(id.to_owned());
    profile.title = Some(title.to_owned());
    let bytes = format::save(&profile, FieldPresence::All);

    // The decoder of SPEC-03 is what the import checks with, so the fixture must pass it.
    assert!(format::load(&bytes, None, None).is_ok());
    bytes
}

fn asset(name: &str, script: Vec<u8>) -> UnityTextAsset {
    UnityTextAsset::new(name, script)
}

/// AMD-030 §5: "écrit sans aucune modification [...] sous le nom `<m_Name>.3dn`"
#[test]
fn a_valid_profile_is_written_unmodified() {
    let bytes = profile("ABCDEFGHIJKL", "Pinball");
    let files = FakeProfileFileSystem::new();
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None);

    let summary = importer
        .import(Path::new("bundle"), &[asset("Pinball", bytes.clone())])
        .unwrap();

    assert_eq!(1, summary.imported);
    assert_eq!(0, summary.already_present);
    assert_eq!(0, summary.failed);
    assert!(summary.is_complete());

    let path = destination("Pinball.3dn");
    assert!(files.file_exists(&path));
    assert_eq!(bytes, files.read_all_bytes(&path).unwrap());
    assert_eq!("Pinball.3dn", summary.results[0].file_name);
}

/// AMD-030 §5: a profile that does not decode is a failure and the import carries on.
#[test]
fn a_payload_that_is_not_a_profile_fails_without_stopping_the_import() {
    let files = FakeProfileFileSystem::new();
    let log = RecordingProfileLog::new();
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), Some(&log));

    let summary = importer
        .import(
            Path::new("bundle"),
            &[
                asset("broken", vec![1, 2, 3, 4, 5]),
                asset("good", profile("ABCDEFGHIJKL", "Good")),
            ],
        )
        .unwrap();

    assert_eq!(1, summary.imported);
    assert_eq!(1, summary.failed);
    assert!(!summary.is_complete());
    assert!(!files.file_exists(&destination("broken.3dn")));
    assert!(files.file_exists(&destination("good.3dn")));
    assert_eq!(ThreeDSenProfileOutcome::Failed, summary.results[0].outcome);
    assert!(summary.results[0].message.contains("not a usable profile"));
    assert!(log.any_contains("broken"));
}

/// AMD-030 §2: identical content is skipped.
#[test]
fn an_identical_profile_already_there_is_skipped() {
    let bytes = profile("ABCDEFGHIJKL", "Pinball");
    let files = FakeProfileFileSystem::new();
    files.add_file(&destination("Pinball.3dn"), &bytes);
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None);

    let summary = importer
        .import(Path::new("bundle"), &[asset("Pinball", bytes)])
        .unwrap();

    assert_eq!(0, summary.imported);
    assert_eq!(1, summary.already_present);
    assert_eq!(1, files.paths().len());
    assert_eq!("Pinball.3dn", summary.results[0].file_name);
}

/// AMD-030 §2: "le nouveau profil reçoit un suffixe ` (2)`, ` (3)`..."
#[test]
fn a_name_clash_with_other_bytes_is_suffixed() {
    let files = FakeProfileFileSystem::new();
    files.add_file(
        &destination("Pinball.3dn"),
        &profile("OTHER0000001", "Older"),
    );
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None);

    let first = importer
        .import(
            Path::new("bundle"),
            &[asset("Pinball", profile("ABCDEFGHIJKL", "Pinball"))],
        )
        .unwrap();
    let second = importer
        .import(
            Path::new("bundle"),
            &[asset("Pinball", profile("MNOPQRSTUVWX", "Pinball 2"))],
        )
        .unwrap();

    assert_eq!("Pinball (2).3dn", first.results[0].file_name);
    assert_eq!("Pinball (3).3dn", second.results[0].file_name);
    assert!(files.file_exists(&destination("Pinball (2).3dn")));
    assert!(files.file_exists(&destination("Pinball (3).3dn")));

    // The file that was already there is left alone.
    assert_eq!(
        profile("OTHER0000001", "Older"),
        files.read_all_bytes(&destination("Pinball.3dn")).unwrap()
    );
}

/// AMD-030 §2: a third import of the same bytes still finds them, under the suffix.
#[test]
fn a_suffixed_profile_is_recognised_on_the_next_import() {
    let mine = profile("ABCDEFGHIJKL", "Pinball");
    let files = FakeProfileFileSystem::new();
    files.add_file(
        &destination("Pinball.3dn"),
        &profile("OTHER0000001", "Older"),
    );
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None);

    importer
        .import(Path::new("bundle"), &[asset("Pinball", mine.clone())])
        .unwrap();
    let again = importer
        .import(Path::new("bundle"), &[asset("Pinball", mine)])
        .unwrap();

    assert_eq!(0, again.imported);
    assert_eq!(1, again.already_present);
    assert_eq!("Pinball (2).3dn", again.results[0].file_name);
}

/// AMD-030 §5: "les caractères interdits par le système de fichiers [...] par _"
#[rstest]
#[case("Blaster Master (U)", "Blaster Master (U).3dn")]
#[case("Mike Tyson's Punch-Out!!", "Mike Tyson's Punch-Out!!.3dn")]
#[case("A/B", "A_B.3dn")]
#[case("C:D*E?F\"G<H>I|J", "C_D_E_F_G_H_I_J.3dn")]
#[case("trailing. ", "trailing.3dn")]
#[case("", "profil.3dn")]
#[case("...", "profil.3dn")]
fn the_file_name_substitutes_what_the_file_system_forbids(
    #[case] name: &str,
    #[case] expected: &str,
) {
    assert_eq!(expected, ThreeDSenImporter::file_name_for(name));
}

/// A file system that fails the way a full disk does, on the temporary file of an import.
struct FailingFileSystem<'a> {
    inner: &'a FakeProfileFileSystem,
    fail_on: &'static str,
    on_move: bool,
}

impl ProfileFileSystem for FailingFileSystem<'_> {
    fn file_exists(&self, path: &Path) -> bool {
        self.inner.file_exists(path)
    }

    fn directory_exists(&self, path: &Path) -> bool {
        self.inner.directory_exists(path)
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        self.inner.create_directory(path)
    }

    fn list_files(&self, directory: &Path, extension: &str) -> io::Result<Vec<PathBuf>> {
        self.inner.list_files(directory, extension)
    }

    fn list_directories(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list_directories(directory)
    }

    fn read_all_bytes(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.inner.read_all_bytes(path)
    }

    fn read_all_text(&self, path: &Path) -> io::Result<String> {
        self.inner.read_all_text(path)
    }

    fn write_all_bytes(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        if !self.on_move && path.to_string_lossy().contains(self.fail_on) {
            return Err(io::Error::other(format!(
                "{}: the disk is full",
                path.display()
            )));
        }
        self.inner.write_all_bytes(path, contents)
    }

    fn write_all_text(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.inner.write_all_text(path, contents)
    }

    fn delete_file(&self, path: &Path) -> io::Result<()> {
        self.inner.delete_file(path)
    }

    fn copy_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
        self.inner.copy_file(source, destination, overwrite)
    }

    fn move_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
        if self.on_move && source.to_string_lossy().contains(self.fail_on) {
            return Err(io::Error::other(format!(
                "{}: the move failed",
                destination.display()
            )));
        }
        self.inner.move_file(source, destination, overwrite)
    }
}

/// AMD-030 §5: "n'écrit aucun fichier partiel"
#[test]
fn a_write_that_fails_leaves_no_partial_file() {
    let inner = FakeProfileFileSystem::new();
    let files = FailingFileSystem {
        inner: &inner,
        fail_on: ".part",
        on_move: false,
    };
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None);

    let summary = importer
        .import(
            Path::new("bundle"),
            &[asset("Pinball", profile("ABCDEFGHIJKL", "Pinball"))],
        )
        .unwrap();

    assert_eq!(0, summary.imported);
    assert_eq!(1, summary.failed);
    assert!(summary.results[0].message.contains("could not be written"));
    assert!(inner.paths().is_empty());
}

/// AMD-030 §5: the temporary file goes away when the move fails.
#[test]
fn a_move_that_fails_removes_the_temporary_file() {
    let inner = FakeProfileFileSystem::new();
    let files = FailingFileSystem {
        inner: &inner,
        fail_on: ".part",
        on_move: true,
    };
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None);

    let summary = importer
        .import(
            Path::new("bundle"),
            &[asset("Pinball", profile("ABCDEFGHIJKL", "Pinball"))],
        )
        .unwrap();

    assert_eq!(1, summary.failed);
    assert!(inner.paths().is_empty());
}

/// AMD-030 §2: the end-of-import message.
#[test]
fn the_summary_reports_the_three_counts() {
    let present = profile("ABCDEFGHIJKL", "Pinball");
    let files = FakeProfileFileSystem::new();
    files.add_file(&destination("Pinball.3dn"), &present);
    let importer = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None);

    let summary = importer
        .import(
            Path::new("bundle"),
            &[
                asset("Pinball", present),
                asset("Tetris", profile("MNOPQRSTUVWX", "Tetris")),
                asset("Dr Mario", profile("ZYXWVUTSRQPO", "Dr Mario")),
                asset("junk", vec![0xFF, 0xFF]),
            ],
        )
        .unwrap();

    assert_eq!(2, summary.imported);
    assert_eq!(1, summary.already_present);
    assert_eq!(1, summary.failed);
    assert_eq!(
        // AMD-036 §1: the summary of AMD-030 §5 is English here, so the command line and the
        // log always print English; the interface builds its own from the counts.
        "3dSen: 2 profile(s) imported, 1 already present, 1 failed.",
        summary.message()
    );
}

/// Lays out an installation in the fake file system and returns the bundle's path.
fn layout(
    files: &FakeProfileFileSystem,
    root: &Path,
    data_folder: &str,
    platform: &str,
    fingerprint: &str,
    contents: &[u8],
) -> PathBuf {
    let bundle = root
        .join(data_folder)
        .join("StreamingAssets")
        .join("aa")
        .join(platform)
        .join(format!("{BUNDLE_PREFIX}{fingerprint}{BUNDLE_EXTENSION}"));
    files.add_file(&bundle, contents);
    bundle
}

/// AMD-030 §4: a bundle that cannot be read writes nothing at all.
#[test]
fn an_unreadable_bundle_writes_nothing() {
    let installation = FakeProfileFileSystem::new();
    let bundle = layout(
        &installation,
        Path::new("/install"),
        "3dSen_Data",
        "StandaloneWindows",
        "dead",
        &[1, 2, 3, 4, 5, 6, 7, 8],
    );
    // The bundle and the destination are in the same file system here, so the importer is
    // pointed at the bundle file itself.
    let importer = ThreeDSenImporter::new(&installation, Path::new(DESTINATION), None);

    let before = installation.paths().len();
    assert!(importer.import_from(&bundle).is_err());
    assert_eq!(before, installation.paths().len());
}

/// AMD-030 §3: "Si rien n'est trouvé, l'interface le dit clairement et rappelle le chemin
/// attendu."
#[test]
fn an_installation_without_a_bundle_names_the_expected_path() {
    let files = FakeProfileFileSystem::new();
    files.create_directory(Path::new("/install")).unwrap();
    files
        .create_directory(&Path::new("/install").join("3dSen_Data"))
        .unwrap();

    let error = locate_bundle(&files, Path::new("/install")).unwrap_err();

    assert!(error
        .message()
        .contains("no 3dSen profile bundle was found"));
    assert!(error.message().contains(&expected_layout()));
    assert!(error.message().contains("StreamingAssets/aa"));
}

/// AMD-030 §3
#[test]
fn a_path_that_does_not_exist_is_refused() {
    let files = FakeProfileFileSystem::new();

    let error = locate_bundle(&files, Path::new("/zax-no-such-0123456789abcdef")).unwrap_err();

    assert!(error.message().contains("neither a file nor a folder"));
}

/// AMD-030 §3.1: the platform folders are tried in the order the amendment gives.
#[test]
fn the_first_platform_folder_of_the_list_wins() {
    let files = FakeProfileFileSystem::new();
    let root = Path::new("/install");
    let first = layout(&files, root, "3dSen_Data", "StandaloneWindows", "aaaa", &[]);
    let later = layout(&files, root, "3dSen_Data", "StandaloneLinux64", "0000", &[]);

    assert_eq!(Some(first), find_bundle(&files, root));
    assert!(files.file_exists(&later));
}

/// AMD-030 §3.1: the data folder's name is not assumed, only its suffix.
#[test]
fn any_data_folder_name_is_accepted() {
    let files = FakeProfileFileSystem::new();
    let root = Path::new("/install");
    let bundle = layout(
        &files,
        root,
        "SomeOtherName_Data",
        "StandaloneWindows64",
        "beef",
        &[],
    );

    assert_eq!(Some(bundle), find_bundle(&files, root));
}

/// AMD-030 §3.2: the macOS layout.
#[test]
fn the_bundle_is_found_inside_an_app_folder() {
    let files = FakeProfileFileSystem::new();
    let root = Path::new("/install");
    let bundle = layout(
        &files,
        root,
        "3dSen.app/Contents/Resources/Data",
        "StandaloneOSX",
        "cafe",
        &[],
    );

    assert_eq!(Some(bundle), find_bundle(&files, root));
}

/// AMD-030 §2: "ou directement le fichier bundle"
#[test]
fn a_bundle_file_can_be_named_directly() {
    let files = FakeProfileFileSystem::new();
    let bundle = layout(
        &files,
        Path::new("/install"),
        "3dSen_Data",
        "StandaloneWindows",
        "1234",
        &[],
    );

    assert_eq!(Some(bundle.clone()), find_bundle(&files, &bundle));
}

/// AMD-030 §5: the installation is only read: nothing is copied or changed.
#[test]
fn the_installation_is_left_untouched_by_an_import() {
    let files = FakeProfileFileSystem::new();
    let root = Path::new("/install");
    let contents = synthetic_bundle::build(
        &[("Pinball", profile("ABCDEFGHIJKL", "Pinball"))],
        &SyntheticBundleOptions::default(),
    );
    let bundle = layout(
        &files,
        root,
        "3dSen_Data",
        "StandaloneWindows",
        "1234",
        &contents,
    );
    let before = files.paths();

    let summary = ThreeDSenImporter::new(&files, Path::new(DESTINATION), None)
        .import_from(root)
        .unwrap();

    assert_eq!(1, summary.imported);
    assert_eq!(bundle, summary.bundle_path);
    assert_eq!(contents, files.read_all_bytes(&bundle).unwrap());
    let in_installation = files.list_files(bundle.parent().unwrap(), "").unwrap();
    assert_eq!(vec![bundle], in_installation);
    assert!(before.iter().all(|path| files.paths().contains(path)));
}
