//! The user data's import folder, on every platform the application runs (owner request,
//! 2026-11-20): the 3dSen bundles, the archives holding one and the 3D profiles put there are
//! imported by themselves, each then marked with a `.done` file (`zax_profiles::import::folder`).
//! The Android activity puts there the bundles opened with Zaxnes, and the web page the files
//! its import dialogue takes.

use zax_profiles::catalogue::ProfileImportOutcome;
use zax_profiles::import::folder::{self, WaitingKind};

use crate::application::AppHost;
use crate::web_bridge::ImportResult;

/// Imports what waits in the import folder; how many profiles joined the library. Creates the
/// folder and its README when they are missing.
pub fn process(host: &mut AppHost) -> usize {
    if !host.paths().is_writable {
        return 0;
    }
    let folder = host.paths().root.join(folder::FOLDER);
    folder::prepare(&folder);
    let waiting = folder::waiting(&folder);
    if waiting.is_empty() {
        return 0;
    }
    let mut imported = 0;
    // What came of each file, for the web page's dialogue.
    let mut results: Vec<ImportResult> = Vec::new();
    for (path, kind) in waiting {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let result = match kind {
            WaitingKind::ThreeDSen => {
                let report = host.games_mut().import_from_three_d_sen(&path);
                crate::app_log::line(&report.message);
                match report.summary {
                    Some(summary) => {
                        imported += summary.imported;
                        let text = crate::ui::notifications::three_d_sen_summary(
                            summary.imported,
                            summary.already_present,
                            summary.failed,
                        );
                        host.notifications.show_normal(&text);
                        ImportResult {
                            ok: summary.failed == 0,
                            id: None,
                            imported: summary.imported,
                            text: format!("{name}: {text}"),
                        }
                    }
                    None => {
                        host.notifications.show_error(&report.message);
                        ImportResult {
                            ok: false,
                            id: None,
                            imported: 0,
                            text: format!("{name}: {}", report.message),
                        }
                    }
                }
            }
            // A profile put there replaces the one of the same identifier: the player chose
            // it, there is no dialogue to ask in.
            WaitingKind::Profile => {
                let done = host.games_mut().import_profile(&path);
                match (done.outcome, done.profile) {
                    (
                        ProfileImportOutcome::Added | ProfileImportOutcome::Replaced,
                        Some(profile),
                    ) => {
                        imported += 1;
                        let text = format!("{name}: {}", profile.title);
                        crate::app_log::line(&format!("profile imported: {text}"));
                        ImportResult {
                            ok: true,
                            id: Some(profile.id),
                            imported: 1,
                            text,
                        }
                    }
                    (ProfileImportOutcome::AlreadyBuiltIn, _) => ImportResult {
                        ok: true,
                        id: None,
                        imported: 0,
                        text: format!("{name}: {}", done.message),
                    },
                    _ => {
                        crate::app_log::line(&format!("profile refused: {name}: {}", done.message));
                        ImportResult {
                            ok: false,
                            id: None,
                            imported: 0,
                            text: format!("{name}: {}", done.message),
                        }
                    }
                }
            }
        };
        folder::mark_done(
            &path,
            &format!(
                "{}: {}",
                if result.ok { "ok" } else { "error" },
                result.text
            ),
        );
        results.push(result);
    }
    crate::web_bridge::report_import(&results);
    imported
}
