namespace Zax.Profiles.Import;

/// <summary>
/// Finding the profile bundle inside an installation of 3dSen (AMD-030 §3).
/// </summary>
/// <remarks>
/// <para>
/// The user names the installation folder — the one holding <c>3dSen.exe</c> — or the bundle
/// file itself. Nothing is guessed about the <c>*</c> part of the bundle name: it is a
/// fingerprint that changes from one version to the next, so the folder is listed and the
/// matching file is taken.
/// </para>
/// <para>
/// The installation is only ever read. The bundle is never copied, moved or changed
/// (AMD-030 §5, external files).
/// </para>
/// </remarks>
public static class ThreeDSenInstallation
{
    /// <summary>What the bundle's file name starts with.</summary>
    public const string BundlePrefix = "defaultprofiles_assets_all_";

    /// <summary>What the bundle's file name ends with.</summary>
    public const string BundleExtension = ".bundle";

    /// <summary>The folder holding the addressable groups, under <c>&lt;name&gt;_Data</c>.</summary>
    private const string StreamingPath = "StreamingAssets/aa";

    /// <summary>The suffix of the data folder of a Unity player.</summary>
    private const string DataSuffix = "_Data";

    /// <summary>
    /// The player platform folders, in the order AMD-030 §3 gives them: the first bundle found
    /// wins.
    /// </summary>
    public static IReadOnlyList<string> Platforms { get; } =
    [
        "StandaloneWindows",
        "StandaloneWindows64",
        "StandaloneLinux64",
        "StandaloneOSX",
    ];

    /// <summary>
    /// The path the interface shows when nothing is found (AMD-030 §3, last sentence).
    /// </summary>
    public static string ExpectedLayout =>
        $"<installation>/<nom>{DataSuffix}/{StreamingPath}/<{string.Join('|', Platforms)}>/"
        + $"{BundlePrefix}*{BundleExtension}";

    /// <summary>
    /// Finds the bundle for <paramref name="path"/>: the file itself when a file is named, else
    /// the first bundle of the layout of AMD-030 §3 under that folder.
    /// </summary>
    /// <returns>The bundle's full path, or <see langword="null"/> when there is none.</returns>
    public static string? FindBundle(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);

        if (File.Exists(path))
        {
            return Path.GetFullPath(path);
        }

        if (!Directory.Exists(path))
        {
            return null;
        }

        string root = Path.GetFullPath(path);

        // 1. <folder>/<anything>_Data/StreamingAssets/aa/<platform>/
        if (FindUnder(DataFolders(root)) is { } windows)
        {
            return windows;
        }

        // 2. macOS: <folder>/*.app/Contents/Resources/Data/StreamingAssets/aa/<platform>/
        return FindUnder(MacDataFolders(root));
    }

    /// <summary>
    /// Finds the bundle, or explains where it was looked for.
    /// </summary>
    /// <exception cref="UnityAssetFormatException">Nothing was found.</exception>
    public static string LocateBundle(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);

        if (FindBundle(path) is { } bundle)
        {
            return bundle;
        }

        if (!Directory.Exists(path) && !File.Exists(path))
        {
            throw new UnityAssetFormatException(
                $"\"{path}\" is neither a file nor a folder.");
        }

        throw new UnityAssetFormatException(
            $"no 3dSen profile bundle was found under \"{path}\". "
            + $"Name the folder holding 3dSen.exe, or the bundle itself; the expected path is "
            + $"{ExpectedLayout}.");
    }

    /// <summary>Whether a file name is a profile bundle (AMD-030 §3).</summary>
    public static bool IsBundleName(string fileName)
    {
        ArgumentNullException.ThrowIfNull(fileName);
        return fileName.StartsWith(BundlePrefix, StringComparison.OrdinalIgnoreCase)
            && fileName.EndsWith(BundleExtension, StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>
    /// The platform folders are tried in order, every candidate data folder for each: the
    /// platform decides, not the order the file system happens to list the folders in.
    /// </summary>
    private static string? FindUnder(IReadOnlyList<string> dataFolders)
    {
        foreach (string platform in Platforms)
        {
            foreach (string data in dataFolders)
            {
                string folder = Path.Combine(
                    data,
                    StreamingPath.Replace('/', Path.DirectorySeparatorChar),
                    platform);

                if (FirstBundleIn(folder) is { } bundle)
                {
                    return bundle;
                }
            }
        }

        return null;
    }

    private static string? FirstBundleIn(string folder)
    {
        if (!Directory.Exists(folder))
        {
            return null;
        }

        List<string> bundles = [];
        foreach (string file in Listed(folder, Directory.EnumerateFiles))
        {
            if (IsBundleName(Path.GetFileName(file)))
            {
                bundles.Add(file);
            }
        }

        if (bundles.Count == 0)
        {
            return null;
        }

        // Several fingerprints in one folder is not expected; sorting keeps the choice stable.
        bundles.Sort(StringComparer.Ordinal);
        return bundles[0];
    }

    private static IReadOnlyList<string> DataFolders(string root)
    {
        List<string> folders = [];
        foreach (string directory in Listed(root, Directory.EnumerateDirectories))
        {
            if (Path.GetFileName(directory).EndsWith(DataSuffix, StringComparison.OrdinalIgnoreCase))
            {
                folders.Add(directory);
            }
        }

        folders.Sort(StringComparer.Ordinal);
        return folders;
    }

    private static IReadOnlyList<string> MacDataFolders(string root)
    {
        List<string> folders = [];
        foreach (string directory in Listed(root, Directory.EnumerateDirectories))
        {
            if (!Path.GetFileName(directory).EndsWith(".app", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            folders.Add(Path.Combine(directory, "Contents", "Resources", "Data"));
        }

        folders.Sort(StringComparer.Ordinal);
        return folders;
    }

    /// <summary>
    /// Lists a folder, treating an unreadable one as empty: pointing the import at a drive with
    /// no medium or at a folder without rights must not throw (AMD-030 §5).
    /// </summary>
    private static IReadOnlyList<string> Listed(string folder, Func<string, IEnumerable<string>> list)
    {
        try
        {
            return [.. list(folder)];
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException
            or ArgumentException or NotSupportedException)
        {
            return [];
        }
    }
}
