//! The command line of AMD-030 §2: `--import-3dsen <folder|file>` imports the profiles of an
//! installed 3dSen, prints the summary and exits with 0 when it all worked
//! (`ThreeDSenImportCommand.cs`).

use std::io::{self, Write};
use std::path::Path;

use zax_profiles::import::{ThreeDSenImportError, ThreeDSenImporter, ThreeDSenProfileOutcome};

use crate::app_options::AppOptions;
use crate::physical_file_system::PhysicalProfileFileSystem;
use crate::user_data_paths::UserDataPaths;

/// Exit code when every profile of the bundle was dealt with.
pub const EXIT_SUCCESS: i32 = 0;

/// Exit code when no bundle was found, or at least one profile failed.
pub const EXIT_FAILURE: i32 = 1;

/// Runs the import named by `options`, into the profile folder of `--userdata`. The summary goes
/// to `output`, the failures to `error`.
///
/// # Errors
///
/// Writing to `output` or `error` fails.
pub fn run(
    options: &AppOptions,
    output: &mut impl Write,
    error: &mut impl Write,
) -> io::Result<i32> {
    let Some(source) = options
        .import_three_d_sen_path
        .as_deref()
        .filter(|source| !source.is_empty())
    else {
        writeln!(error, "--import-3dsen requires a folder or a file.")?;
        return Ok(EXIT_FAILURE);
    };
    let paths =
        UserDataPaths::resolve(options.user_data_root.as_deref().map(Path::new), None, None);
    // SPEC-09 REQ-PERS-073: with no writable root there is nowhere to put the profiles.
    if !paths.is_writable {
        let message = paths
            .unwritable_message
            .clone()
            .unwrap_or_else(|| format!("{} does not accept writes.", paths.root.display()));
        writeln!(error, "{message}")?;
        return Ok(EXIT_FAILURE);
    }
    run_into(Path::new(source), &paths.profiles_directory, output, error)
}

/// Runs the import of `source` into the folder `profiles_directory`.
///
/// # Errors
///
/// Writing to `output` or `error` fails.
pub fn run_into(
    source: &Path,
    profiles_directory: &Path,
    output: &mut impl Write,
    error: &mut impl Write,
) -> io::Result<i32> {
    let files = PhysicalProfileFileSystem;
    let importer = ThreeDSenImporter::new(&files, profiles_directory, None);
    let summary = match importer.import_from(source) {
        Ok(summary) => summary,
        Err(failure) => {
            // AMD-030 §5: a clear message, never a stack trace, and no file written.
            let message = match &failure {
                ThreeDSenImportError::Format(format) => format.message().to_owned(),
                ThreeDSenImportError::Io(io) => io.to_string(),
            };
            writeln!(error, "{message}")?;
            return Ok(EXIT_FAILURE);
        }
    };

    writeln!(output, "3dSen: {}", summary.bundle_path.display())?;
    writeln!(
        output,
        "3dSen: profiles go to {}",
        profiles_directory.display()
    )?;
    writeln!(output, "{}", summary.message())?;
    for result in summary
        .results
        .iter()
        .filter(|result| result.outcome == ThreeDSenProfileOutcome::Failed)
    {
        writeln!(error, "{}", result.message)?;
    }

    Ok(if summary.is_complete() {
        EXIT_SUCCESS
    } else {
        EXIT_FAILURE
    })
}
