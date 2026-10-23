namespace Zax.App.Persistence;

/// <summary>
/// The user root of SPEC-09 §3.8 and the layout of REQ-PERS-072.
/// </summary>
/// <remarks>
/// <para>
/// By default <c>%LOCALAPPDATA%\Zaxnes</c> (DEC-09-Q7, REQ-PERS-070). A file named
/// <c>portable.txt</c> next to the executable moves it to <c>userdata\</c> beside the
/// executable (REQ-PERS-071), and <c>--userdata &lt;dir&gt;</c> overrides both.
/// </para>
/// <para>
/// When the root cannot be created or written to, the object still exists and
/// <see cref="IsWritable"/> is false with <see cref="UnwritableMessage"/> holding the text of
/// REQ-PERS-073; the program runs with in-memory defaults and refuses write actions.
/// </para>
/// <para>
/// AMD-029 §2: the root moved from <c>%LOCALAPPDATA%\Sen</c> to <c>%LOCALAPPDATA%\Zaxnes</c>
/// with the rename. The first time the new root is needed and does not exist yet, the content
/// of the old one is <b>copied</b> into it and <see cref="MigratedFrom"/> says so, which the
/// host logs. The old root is never written to, moved or deleted.
/// </para>
/// </remarks>
public sealed class UserDataPaths
{
    /// <summary>Product name fixed by DEC-09-Q7, renamed by AMD-029 §1.</summary>
    public const string ProductName = "Zaxnes";

    /// <summary>
    /// The folder name the user root had under the « Sen » working name. Only ever read, and
    /// only to seed a new root once (AMD-029 §2).
    /// </summary>
    public const string LegacyProductName = "Sen";

    /// <summary>Marker that switches to the portable layout (REQ-PERS-071).</summary>
    public const string PortableMarkerName = "portable.txt";

    /// <summary>Folder used by the portable layout.</summary>
    public const string PortableFolderName = "userdata";

    /// <summary>
    /// Name of a profile folder, under the user root and next to the executable alike
    /// (REQ-PERS-072, AMD-033 §1).
    /// </summary>
    public const string ProfilesFolderName = "profiles";

    private UserDataPaths(string root, UserDataRootKind kind, string executableDirectory)
    {
        Root = root;
        Kind = kind;
        ExecutableDirectory = executableDirectory;
        SettingsFile = Path.Combine(root, "settings.json");
        InputFile = Path.Combine(root, "input.json");
        LibraryFile = Path.Combine(root, "library.json");
        SavesDirectory = Path.Combine(root, "saves");
        SaveIndexFile = Path.Combine(SavesDirectory, "index.json");
        BatteryDirectory = Path.Combine(root, "battery");
        ProfilesDirectory = Path.Combine(root, "profiles");
        ProgramProfilesDirectory = Path.Combine(executableDirectory, ProfilesFolderName);
        CoversDirectory = Path.Combine(root, "covers");
        LogsDirectory = Path.Combine(root, "logs");
    }

    /// <summary>Where the root came from.</summary>
    public enum UserDataRootKind
    {
        /// <summary><c>%LOCALAPPDATA%\Zaxnes</c>.</summary>
        LocalAppData,

        /// <summary><c>userdata\</c> beside the executable.</summary>
        Portable,

        /// <summary>Given by <c>--userdata</c>.</summary>
        Explicit,
    }

    /// <summary>The user root.</summary>
    public string Root { get; }

    /// <summary>Where the root came from.</summary>
    public UserDataRootKind Kind { get; }

    /// <summary>Settings file (REQ-PERS-020).</summary>
    public string SettingsFile { get; }

    /// <summary>Bindings file (REQ-PERS-065).</summary>
    public string InputFile { get; }

    /// <summary>Library file (SPEC-09 §4.5.2).</summary>
    public string LibraryFile { get; }

    /// <summary>Folder holding the savestates.</summary>
    public string SavesDirectory { get; }

    /// <summary>Slot metadata (SPEC-09 §4.5.3).</summary>
    public string SaveIndexFile { get; }

    /// <summary>Folder holding the battery-backed RAM files (REQ-PERS-060).</summary>
    public string BatteryDirectory { get; }

    /// <summary>Folder scanned for user profiles (REQ-PERS-040).</summary>
    public string ProfilesDirectory { get; }

    /// <summary>The folder the executable lives in. Read-only for Zaxnes (AMD-033 §1).</summary>
    public string ExecutableDirectory { get; }

    /// <summary>
    /// The second scanned profile folder, <c>profiles</c> next to the executable (AMD-033 §1).
    /// It is scanned in every mode, it is never created nor written to, and it being missing or
    /// unreadable is not an error.
    /// </summary>
    public string ProgramProfilesDirectory { get; }

    /// <summary>Folder holding the cover art (REQ-PERS-036).</summary>
    public string CoversDirectory { get; }

    /// <summary>Folder for run logs.</summary>
    public string LogsDirectory { get; }

    /// <summary>Whether the root exists and accepts writes.</summary>
    public bool IsWritable { get; private set; }

    /// <summary>
    /// The old root this one was seeded from at first run, or null when nothing was migrated
    /// (AMD-029 §2). The host logs it; it is never a failure not to have one.
    /// </summary>
    public string? MigratedFrom { get; private set; }

    /// <summary>
    /// The notification text of REQ-PERS-073 when the root is not writable, else null.
    /// </summary>
    public string? UnwritableMessage { get; private set; }

    /// <summary>The savestate file of a slot.</summary>
    public string SlotStateFile(string slotId) => Path.Combine(SavesDirectory, slotId + ".state");

    /// <summary>The thumbnail file of a slot (stored beside the state, SPEC-09 §4.5.3).</summary>
    public string SlotThumbnailFile(string slotId) => Path.Combine(SavesDirectory, slotId + ".png");

    /// <summary>The battery RAM file of a ROM, keyed by its SHA-1 (REQ-PERS-060).</summary>
    public string BatteryFile(string romSha1) => Path.Combine(BatteryDirectory, romSha1 + ".srm");

    /// <summary>
    /// Resolves the root and creates the folders of REQ-PERS-072.
    /// </summary>
    /// <param name="explicitRoot">The <c>--userdata</c> value, or null.</param>
    /// <param name="executableDirectory">
    /// Folder holding the executable, used to look for <c>portable.txt</c>; the base directory
    /// of the running application when omitted.
    /// </param>
    /// <param name="localAppDataDirectory">
    /// Stands in for <c>%LOCALAPPDATA%</c>, so that a test can exercise the default root and
    /// the migration of AMD-029 §2 in a temporary folder instead of the user's real one.
    /// </param>
    public static UserDataPaths Resolve(
        string? explicitRoot = null,
        string? executableDirectory = null,
        string? localAppDataDirectory = null)
    {
        string baseDirectory = executableDirectory ?? AppContext.BaseDirectory;
        UserDataPaths paths;

        if (!string.IsNullOrWhiteSpace(explicitRoot))
        {
            paths = new UserDataPaths(
                Path.GetFullPath(explicitRoot), UserDataRootKind.Explicit, baseDirectory);
        }
        else if (File.Exists(Path.Combine(baseDirectory, PortableMarkerName)))
        {
            paths = new UserDataPaths(
                Path.Combine(baseDirectory, PortableFolderName),
                UserDataRootKind.Portable,
                baseDirectory);
        }
        else
        {
            string local = localAppDataDirectory
                ?? Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            if (string.IsNullOrEmpty(local))
            {
                local = baseDirectory;
            }

            paths = new UserDataPaths(
                Path.Combine(local, ProductName), UserDataRootKind.LocalAppData, baseDirectory);

            // Before EnsureDirectories, which would create the new root and make it look as if
            // it had always been there (AMD-029 §2).
            paths.MigrateLegacyRoot(Path.Combine(local, LegacyProductName));
        }

        paths.EnsureDirectories();
        return paths;
    }

    /// <summary>
    /// Copies the content of <paramref name="legacyRoot"/> into <paramref name="newRoot"/> when
    /// the new root does not exist yet and the old one does (AMD-029 §2). The old root is only
    /// read from.
    /// </summary>
    /// <returns>
    /// Whether anything was copied: false when the new root is already there, when there is no
    /// old root, or when the copy failed — none of which is an error.
    /// </returns>
    public static bool TryCopyLegacyRoot(string legacyRoot, string newRoot)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(legacyRoot);
        ArgumentException.ThrowIfNullOrWhiteSpace(newRoot);

        if (Directory.Exists(newRoot) || !Directory.Exists(legacyRoot))
        {
            return false;
        }

        // The same folder under two spellings would copy a tree onto itself.
        if (string.Equals(
            Path.GetFullPath(legacyRoot).TrimEnd(Path.DirectorySeparatorChar),
            Path.GetFullPath(newRoot).TrimEnd(Path.DirectorySeparatorChar),
            StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        try
        {
            CopyTree(legacyRoot, newRoot);
            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException or NotSupportedException)
        {
            // A half-copied root is still usable: what is missing simply takes its defaults,
            // and the old root is untouched either way. The start-up must not fail here.
            return false;
        }
    }

    private static void CopyTree(string source, string destination)
    {
        Directory.CreateDirectory(destination);

        foreach (string file in Directory.EnumerateFiles(source))
        {
            File.Copy(file, Path.Combine(destination, Path.GetFileName(file)), overwrite: false);
        }

        foreach (string directory in Directory.EnumerateDirectories(source))
        {
            CopyTree(directory, Path.Combine(destination, Path.GetFileName(directory)));
        }
    }

    private void MigrateLegacyRoot(string legacyRoot)
    {
        if (TryCopyLegacyRoot(legacyRoot, Root))
        {
            MigratedFrom = legacyRoot;
        }
    }

    /// <summary>
    /// Creates whatever is missing and probes the root for writes. Sets
    /// <see cref="IsWritable"/> and <see cref="UnwritableMessage"/> instead of throwing
    /// (REQ-PERS-073).
    /// </summary>
    public void EnsureDirectories()
    {
        try
        {
            Directory.CreateDirectory(Root);
            Directory.CreateDirectory(SavesDirectory);
            Directory.CreateDirectory(BatteryDirectory);
            Directory.CreateDirectory(ProfilesDirectory);
            Directory.CreateDirectory(CoversDirectory);
            Directory.CreateDirectory(LogsDirectory);

            string probe = Path.Combine(Root, ".write-probe");
            File.WriteAllBytes(probe, []);
            File.Delete(probe);

            IsWritable = true;
            UnwritableMessage = null;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException or NotSupportedException)
        {
            IsWritable = false;
            UnwritableMessage = $"User data folder is not writable: {Root}";
        }
    }

    /// <summary>
    /// Writes <paramref name="contents"/> atomically: a temporary file in the same folder, then
    /// a replace (REQ-PERS-021).
    /// </summary>
    public static void WriteAtomic(string path, ReadOnlySpan<byte> contents)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);

        string directory = Path.GetDirectoryName(path) ?? ".";
        Directory.CreateDirectory(directory);
        string temporary = Path.Combine(directory, Path.GetFileName(path) + ".tmp");

        File.WriteAllBytes(temporary, contents);
        File.Move(temporary, path, overwrite: true);
    }

    /// <summary>Writes UTF-8 text without a BOM, atomically (SPEC-09 §4.5).</summary>
    public static void WriteAtomicText(string path, string contents)
        => WriteAtomic(path, new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: false).GetBytes(contents));

    /// <summary>
    /// Renames an unreadable file to <c>&lt;name&gt;.corrupt-&lt;yyyyMMdd-HHmmss&gt;</c>
    /// (REQ-PERS-022). Failures are swallowed: a read-only root must not stop the start-up.
    /// </summary>
    public static void SetAsideCorruptFile(string path, DateTimeOffset now)
    {
        try
        {
            if (File.Exists(path))
            {
                string stamp = now.ToString("yyyyMMdd-HHmmss", System.Globalization.CultureInfo.InvariantCulture);
                File.Move(path, $"{path}.corrupt-{stamp}", overwrite: true);
            }
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // Nothing to do: the caller carries on with defaults.
        }
    }
}
