namespace Zax.Profiles.Tests;

/// <summary>
/// Locates the extracted assets of <c>cleanroom/assets-neutral</c>, which hold the 101 built-in
/// profiles, the catalogue <c>profiles.json</c> and the key file <c>format-data.json</c>.
/// </summary>
/// <remarks>
/// They sit beside the solution rather than inside it, so the folder is found by walking up
/// from the test binary. A test that needs them is skipped, with a message, when they are not
/// there.
/// </remarks>
internal static class NeutralAssets
{
    /// <summary>Absolute path of <c>assets-neutral</c>, or null when it cannot be found.</summary>
    public static string? Root { get; } = Locate();

    /// <summary>Absolute path of <c>assets-neutral/profiles</c>, or null.</summary>
    public static string? ProfilesDirectory
        => Root is null ? null : Path.Combine(Root, "profiles");

    /// <summary>
    /// Absolute path of <c>assets-neutral/profiles-2.1</c>, the 101 « 3dh » profiles of 3dSen
    /// 2.1 (AMD-062 §1), or null.
    /// </summary>
    public static string? HashProfilesDirectory
        => Root is null ? null : Path.Combine(Root, "profiles-2.1");

    /// <summary>Absolute path of <c>assets-neutral/profiles.json</c>, or null.</summary>
    public static string? CatalogueFile
        => Root is null ? null : Path.Combine(Root, "profiles.json");

    /// <summary>True when the profiles folder is available.</summary>
    public static bool Available => ProfilesDirectory is not null && Directory.Exists(ProfilesDirectory);

    /// <summary>True when the « 3dh » profiles folder is available.</summary>
    public static bool HashProfilesAvailable
        => HashProfilesDirectory is not null && Directory.Exists(HashProfilesDirectory);

    /// <summary>The 101 « 3dh » profile files, ordered by file name (ordinal).</summary>
    public static IReadOnlyList<string> HashProfileFiles()
    {
        if (HashProfilesDirectory is null || !Directory.Exists(HashProfilesDirectory))
        {
            return [];
        }

        string[] files = Directory.GetFiles(HashProfilesDirectory, "*.bytes");
        Array.Sort(files, StringComparer.Ordinal);
        return files;
    }

    /// <summary>Reads one « 3dh » profile by file name, for example <c>BalloonFight.bytes</c>.</summary>
    public static byte[] ReadHashProfile(string fileName)
    {
        if (HashProfilesDirectory is null)
        {
            throw new InvalidOperationException(MissingMessage);
        }

        return File.ReadAllBytes(Path.Combine(HashProfilesDirectory, fileName));
    }

    /// <summary>
    /// The 3dn counterpart of a « 3dh » file name. Two profiles keep the apostrophe in
    /// <c>profiles-2.1</c> that <c>profiles</c> writes <c>_</c> (AMD-062 §1).
    /// </summary>
    public static string CounterpartName(string hashFileName)
        => hashFileName.Replace('\'', '_');

    /// <summary>The message a skipped test reports.</summary>
    public const string MissingMessage =
        "The extracted assets of cleanroom/assets-neutral are not available next to the solution.";

    /// <summary>The 101 built-in profile files, ordered by file name (ordinal).</summary>
    public static IReadOnlyList<string> ProfileFiles()
    {
        if (ProfilesDirectory is null || !Directory.Exists(ProfilesDirectory))
        {
            return [];
        }

        string[] files = Directory.GetFiles(ProfilesDirectory, "*.bytes");
        Array.Sort(files, StringComparer.Ordinal);
        return files;
    }

    /// <summary>Reads one built-in profile by file name, for example <c>empty.bytes</c>.</summary>
    public static byte[] ReadProfile(string fileName)
    {
        if (ProfilesDirectory is null)
        {
            throw new InvalidOperationException(MissingMessage);
        }

        return File.ReadAllBytes(Path.Combine(ProfilesDirectory, fileName));
    }

    /// <summary>Loads one built-in profile, asserting that it loaded.</summary>
    public static ProfileDocument LoadProfile(string fileName)
    {
        ProfileLoadResult result = ProfileFormat.Load(ReadProfile(fileName), sourceName: fileName);
        Assert.True(result.IsSuccess, $"{fileName}: {result.Message}");
        return result.Document!;
    }

    private static string? Locate()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName, "assets-neutral");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}

/// <summary>
/// A fact that is skipped, rather than failed, when <c>assets-neutral</c> is not available.
/// </summary>
public sealed class AssetFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public AssetFactAttribute()
    {
        if (!NeutralAssets.Available)
        {
            Skip = NeutralAssets.MissingMessage;
        }
    }
}

/// <summary>
/// A theory that is skipped, rather than failed, when <c>assets-neutral</c> is not available.
/// </summary>
public sealed class AssetTheoryAttribute : TheoryAttribute
{
    /// <summary>Creates the attribute.</summary>
    public AssetTheoryAttribute()
    {
        if (!NeutralAssets.Available)
        {
            Skip = NeutralAssets.MissingMessage;
        }
    }
}

/// <summary>
/// A fact that is skipped when the « 3dh » profiles of <c>assets-neutral/profiles-2.1</c> are
/// not available.
/// </summary>
public sealed class HashAssetFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public HashAssetFactAttribute()
    {
        if (!NeutralAssets.Available || !NeutralAssets.HashProfilesAvailable)
        {
            Skip = NeutralAssets.MissingMessage;
        }
    }
}
