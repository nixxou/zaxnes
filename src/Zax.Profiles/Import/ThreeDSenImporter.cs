using Zax.Profiles.Catalogue;

namespace Zax.Profiles.Import;

/// <summary>What became of one profile of the bundle (AMD-030 §2, §5).</summary>
public enum ThreeDSenProfileOutcome
{
    /// <summary>Written into the folder of imported profiles.</summary>
    Imported,

    /// <summary>Byte-for-byte identical to a profile already there: nothing was written.</summary>
    AlreadyPresent,

    /// <summary>Refused by the SPEC-03 decoder, or unwritable: nothing was written.</summary>
    Failed,
}

/// <summary>One line of the import report.</summary>
/// <param name="Name">The <c>m_Name</c> of the <c>TextAsset</c>.</param>
/// <param name="Outcome">What was done.</param>
/// <param name="FileName">
/// The file the profile was written to, or the identical one that was already there; empty on a
/// failure.
/// </param>
/// <param name="Message">Why it failed, or an empty string.</param>
public sealed record ThreeDSenProfileResult(
    string Name,
    ThreeDSenProfileOutcome Outcome,
    string FileName,
    string Message);

/// <summary>
/// What a whole import did (AMD-030 §2: imported, already present, failed).
/// </summary>
public sealed class ThreeDSenImportSummary
{
    /// <summary>Creates a summary.</summary>
    public ThreeDSenImportSummary(string bundlePath, IReadOnlyList<ThreeDSenProfileResult> results)
    {
        ArgumentNullException.ThrowIfNull(bundlePath);
        ArgumentNullException.ThrowIfNull(results);

        BundlePath = bundlePath;
        Results = results;

        int imported = 0;
        int present = 0;
        int failed = 0;
        foreach (ThreeDSenProfileResult result in results)
        {
            switch (result.Outcome)
            {
                case ThreeDSenProfileOutcome.Imported:
                    imported++;
                    break;

                case ThreeDSenProfileOutcome.AlreadyPresent:
                    present++;
                    break;

                default:
                    failed++;
                    break;
            }
        }

        Imported = imported;
        AlreadyPresent = present;
        Failed = failed;
    }

    /// <summary>The bundle that was read.</summary>
    public string BundlePath { get; }

    /// <summary>One line per <c>TextAsset</c> of the bundle, in the bundle's own order.</summary>
    public IReadOnlyList<ThreeDSenProfileResult> Results { get; }

    /// <summary>How many profiles were written.</summary>
    public int Imported { get; }

    /// <summary>How many were already there, identical, and were skipped.</summary>
    public int AlreadyPresent { get; }

    /// <summary>How many were refused.</summary>
    public int Failed { get; }

    /// <summary>True when nothing failed.</summary>
    public bool IsComplete => Failed == 0;

    /// <summary>
    /// The end-of-import message of AMD-030 §2: how many profiles were imported, how many were
    /// already there and how many failed.
    /// </summary>
    /// <remarks>
    /// In English, which is what <c>--import-3dsen</c> prints and what the log records whatever
    /// the interface language is (AMD-036 §1, AMD-035 §2). The interface builds its own
    /// notification from the three counts instead, in the language in force.
    /// </remarks>
    public string Message =>
        $"3dSen: {Imported} profile(s) imported, {AlreadyPresent} already present, "
        + $"{Failed} failed.";
}

/// <summary>
/// The import of AMD-030 §5: every <c>TextAsset</c> of a 3dSen bundle is checked with the
/// SPEC-03 decoder and, when it decodes, written unmodified as <c>&lt;m_Name&gt;.3dn</c> into the
/// user's folder of imported profiles (SPEC-09 REQ-PERS-072).
/// </summary>
/// <remarks>
/// <para>
/// A profile that does not decode is counted as a failure and the import carries on. A profile
/// whose bytes are already there, identical, is skipped; one whose file name is taken by
/// different bytes gets a <c>" (2)"</c>, <c>" (3)"</c>… suffix.
/// </para>
/// <para>
/// Each file is written to a temporary name beside it and then moved into place, so a failure
/// never leaves a partial <c>.3dn</c> behind (AMD-030 §5, robustness).
/// </para>
/// </remarks>
public sealed class ThreeDSenImporter
{
    /// <summary>Suffix of the temporary file a write goes through.</summary>
    public const string PartialSuffix = ".part";

    /// <summary>How many suffixed names are tried before a profile is called a failure.</summary>
    public const int MaximumNameAttempts = 1000;

    private readonly IProfileFileSystem _files;
    private readonly IProfileLog _log;

    /// <summary>Creates an importer writing into <paramref name="destinationDirectory"/>.</summary>
    /// <param name="files">The file system to write through.</param>
    /// <param name="destinationDirectory">
    /// The folder of imported profiles, <c>&lt;root&gt;/profiles</c> (SPEC-09 REQ-PERS-072).
    /// </param>
    /// <param name="log">Where the refused profiles are noted.</param>
    public ThreeDSenImporter(
        IProfileFileSystem files,
        string destinationDirectory,
        IProfileLog? log = null)
    {
        ArgumentNullException.ThrowIfNull(files);
        ArgumentException.ThrowIfNullOrEmpty(destinationDirectory);

        _files = files;
        DestinationDirectory = destinationDirectory;
        _log = log ?? NullProfileLog.Instance;
    }

    /// <summary>The folder the profiles are written to.</summary>
    public string DestinationDirectory { get; }

    /// <summary>
    /// The file name a profile called <paramref name="assetName"/> is written under: the name
    /// itself with <c>.3dn</c> appended, every character the file system forbids replaced by
    /// <c>_</c> (AMD-030 §5).
    /// </summary>
    public static string FileNameFor(string assetName)
    {
        ArgumentNullException.ThrowIfNull(assetName);

        char[] invalid = Path.GetInvalidFileNameChars();
        char[] characters = assetName.ToCharArray();
        for (int index = 0; index < characters.Length; index++)
        {
            if (Array.IndexOf(invalid, characters[index]) >= 0)
            {
                characters[index] = '_';
            }
        }

        // Windows cannot create a name ending in a space or a dot at all; trimming them is the
        // same substitution rule carried to a character the list does not name.
        string name = new string(characters).TrimEnd(' ', '.');
        if (name.Length == 0)
        {
            name = "profil";
        }

        return name + UserProfileFolder.Extension;
    }

    /// <summary>
    /// Imports from an installation folder or a bundle file (AMD-030 §3 then §5).
    /// </summary>
    /// <param name="installationOrBundlePath">
    /// The folder holding <c>3dSen.exe</c>, or the bundle itself.
    /// </param>
    /// <exception cref="UnityAssetFormatException">
    /// No bundle was found, or the bundle cannot be read. Nothing is written in that case.
    /// </exception>
    /// <exception cref="IOException">The bundle cannot be read from the disk.</exception>
    public ThreeDSenImportSummary ImportFrom(string installationOrBundlePath)
    {
        ArgumentException.ThrowIfNullOrEmpty(installationOrBundlePath);

        string bundle = ThreeDSenInstallation.LocateBundle(installationOrBundlePath);
        IReadOnlyList<UnityTextAsset> assets = ThreeDSenProfileBundle.ReadTextAssets(bundle);
        return Import(bundle, assets);
    }

    /// <summary>
    /// Imports already-read <c>TextAsset</c>s.
    /// </summary>
    /// <param name="bundlePath">Where they came from, for the report.</param>
    /// <param name="assets">The assets, in the bundle's order.</param>
    public ThreeDSenImportSummary Import(string bundlePath, IReadOnlyList<UnityTextAsset> assets)
    {
        ArgumentNullException.ThrowIfNull(bundlePath);
        ArgumentNullException.ThrowIfNull(assets);

        _files.CreateDirectory(DestinationDirectory);

        List<ThreeDSenProfileResult> results = new(assets.Count);
        foreach (UnityTextAsset asset in assets)
        {
            results.Add(ImportOne(asset));
        }

        ThreeDSenImportSummary summary = new(bundlePath, results);
        _log.Information($"{bundlePath}: {summary.Message}");
        return summary;
    }

    private ThreeDSenProfileResult ImportOne(UnityTextAsset asset)
    {
        byte[] bytes = asset.Script.ToArray();

        // AMD-030 §5: the payload is checked with the decoder of SPEC-03; a profile that does
        // not decode is a failure and the import carries on.
        ProfileLoadResult decoded = ProfileFormat.Load(bytes, sourceName: asset.Name);
        if (!decoded.IsSuccess)
        {
            string message = $"\"{asset.Name}\" is not a usable profile: {decoded.Message}";
            _log.Warning(message);
            return new ThreeDSenProfileResult(
                asset.Name, ThreeDSenProfileOutcome.Failed, string.Empty, message);
        }

        string fileName = FileNameFor(asset.Name);
        try
        {
            return Write(asset.Name, fileName, bytes);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            string message = $"\"{asset.Name}\" could not be written: {exception.Message}";
            _log.Warning(message);
            return new ThreeDSenProfileResult(
                asset.Name, ThreeDSenProfileOutcome.Failed, string.Empty, message);
        }
    }

    private ThreeDSenProfileResult Write(string assetName, string fileName, byte[] bytes)
    {
        string candidate = fileName;
        for (int attempt = 1; attempt <= MaximumNameAttempts; attempt++)
        {
            string path = Path.Combine(DestinationDirectory, candidate);
            if (!_files.FileExists(path))
            {
                WriteWhole(path, bytes);
                return new ThreeDSenProfileResult(
                    assetName, ThreeDSenProfileOutcome.Imported, candidate, string.Empty);
            }

            if (SameBytes(path, bytes))
            {
                // AMD-030 §2: identical content is skipped, which is what makes a second import
                // of the same installation add nothing.
                return new ThreeDSenProfileResult(
                    assetName, ThreeDSenProfileOutcome.AlreadyPresent, candidate, string.Empty);
            }

            candidate = Suffixed(fileName, attempt + 1);
        }

        string message =
            $"\"{assetName}\" could not be imported: {MaximumNameAttempts} file names of the form "
            + $"\"{fileName}\" are already taken by other profiles.";
        _log.Warning(message);
        return new ThreeDSenProfileResult(
            assetName, ThreeDSenProfileOutcome.Failed, string.Empty, message);
    }

    /// <summary>
    /// Writes through a temporary file: either the <c>.3dn</c> appears whole, or it does not
    /// appear at all (AMD-030 §5).
    /// </summary>
    private void WriteWhole(string path, byte[] bytes)
    {
        string partial = path + PartialSuffix;
        try
        {
            _files.WriteAllBytes(partial, bytes);
            _files.MoveFile(partial, path, overwrite: false);
        }
        catch
        {
            TryDelete(partial);
            throw;
        }
    }

    private void TryDelete(string path)
    {
        try
        {
            _files.DeleteFile(path);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            _log.Warning($"{path}: the temporary file could not be removed ({exception.Message}).");
        }
    }

    private bool SameBytes(string path, byte[] bytes)
    {
        try
        {
            return _files.ReadAllBytes(path).AsSpan().SequenceEqual(bytes);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // An unreadable neighbour is treated as different, which sends the profile to the
            // next suffixed name rather than losing it.
            _log.Warning($"{path}: unreadable file, it is not compared ({exception.Message}).");
            return false;
        }
    }

    /// <summary>« Name.3dn » with the <c>" (n)"</c> suffix of AMD-030 §2.</summary>
    private static string Suffixed(string fileName, int number)
    {
        string stem = Path.GetFileNameWithoutExtension(fileName);
        string extension = Path.GetExtension(fileName);
        return $"{stem} ({number}){extension}";
    }
}
