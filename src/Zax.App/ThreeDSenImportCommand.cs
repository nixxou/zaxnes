using Zax.App.Persistence;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Import;

namespace Zax.App;

/// <summary>
/// The command line of AMD-030 §2: <c>--import-3dsen &lt;folder|file&gt;</c> imports the profiles
/// of an installed 3dSen, prints the summary and exits.
/// </summary>
/// <remarks>
/// It runs without a window, without the audio device and without the single-instance guard,
/// like <c>--list-profiles</c>: it only reads the installation and writes <c>.3dn</c> files into
/// the user's profile folder, which the next launch — or the running interface, after its own
/// import — picks up by scanning that folder.
/// </remarks>
internal static class ThreeDSenImportCommand
{
    /// <summary>Exit code when every profile of the bundle was dealt with.</summary>
    public const int ExitSuccess = 0;

    /// <summary>Exit code when no bundle was found, or at least one profile failed.</summary>
    public const int ExitFailure = 1;

    /// <summary>
    /// Runs the import.
    /// </summary>
    /// <param name="options">The parsed command line; <c>--userdata</c> is honoured.</param>
    /// <param name="output">Where the summary goes.</param>
    /// <param name="error">Where the failures go.</param>
    /// <returns>
    /// <see cref="ExitSuccess"/> when the bundle was read and nothing failed, else
    /// <see cref="ExitFailure"/>.
    /// </returns>
    public static int Run(AppOptions options, TextWriter output, TextWriter error)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(output);
        ArgumentNullException.ThrowIfNull(error);

        if (options.ImportThreeDSenPath is not { Length: > 0 } source)
        {
            error.WriteLine("--import-3dsen requires a folder or a file.");
            return ExitFailure;
        }

        UserDataPaths paths = UserDataPaths.Resolve(options.UserDataRoot);
        if (!paths.IsWritable)
        {
            error.WriteLine(paths.UnwritableMessage ?? $"{paths.Root} does not accept writes.");
            return ExitFailure;
        }

        return Run(source, paths.ProfilesDirectory, output, error);
    }

    /// <summary>
    /// Runs the import into a named folder, which is what the tests drive.
    /// </summary>
    public static int Run(
        string installationOrBundlePath,
        string profilesDirectory,
        TextWriter output,
        TextWriter error)
    {
        ArgumentException.ThrowIfNullOrEmpty(installationOrBundlePath);
        ArgumentException.ThrowIfNullOrEmpty(profilesDirectory);

        ThreeDSenImportSummary summary;
        try
        {
            ThreeDSenImporter importer = new(PhysicalProfileFileSystem.Instance, profilesDirectory);
            summary = importer.ImportFrom(installationOrBundlePath);
        }
        catch (Exception exception) when (exception is UnityAssetFormatException
            or IOException or UnauthorizedAccessException)
        {
            // AMD-030 §5: a clear message, never a stack trace, and no file written.
            error.WriteLine(exception.Message);
            return ExitFailure;
        }

        output.WriteLine($"3dSen: {summary.BundlePath}");
        output.WriteLine($"3dSen: profiles go to {profilesDirectory}");
        output.WriteLine(summary.Message);

        foreach (ThreeDSenProfileResult result in summary.Results)
        {
            if (result.Outcome == ThreeDSenProfileOutcome.Failed)
            {
                error.WriteLine(result.Message);
            }
        }

        return summary.IsComplete ? ExitSuccess : ExitFailure;
    }
}
