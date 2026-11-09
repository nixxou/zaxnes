namespace Zax.Profiles.Tests;

/// <summary>
/// The user's own dumps, named by an environment variable. No machine path is written anywhere
/// in this repository (AMD-024 REQ-PUB-001); <c>HOWTO-RUN.md</c> documents the variable. A test
/// that needs a dump is reported as <b>skipped</b> when it is not there.
/// </summary>
internal static class UserRoms
{
    /// <summary>Environment variable naming the folder the user keeps their dumps in.</summary>
    public const string Variable = "ZAXNES_ROMS";

    /// <summary>Where the user keeps their dumps, or null when the variable is not set.</summary>
    public static string? Folder { get; } =
        Environment.GetEnvironmentVariable(Variable) is { } value && value.Trim().Length > 0
            ? value.Trim()
            : null;

    /// <summary>Full path of a dump, or null when it is not on this machine.</summary>
    public static string? Path(string name)
    {
        if (Folder is null)
        {
            return null;
        }

        string candidate = System.IO.Path.Combine(Folder, name);
        return File.Exists(candidate) ? candidate : null;
    }

    /// <summary>True when every one of <paramref name="names"/> is on this machine.</summary>
    public static bool Available(params string[] names)
    {
        foreach (string name in names)
        {
            if (Path(name) is null)
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>The message a skipped test reports.</summary>
    public static string MissingMessage(params string[] names)
        => Folder is null
            ? $"no user ROM folder: set {Variable} to the folder holding your own dumps."
            : $"user ROM not available: {string.Join(", ", names)} under {Folder}.";
}

/// <summary>
/// A fact that is skipped when <c>assets-neutral/profiles-2.1</c> or one of the three dumps the
/// optional criteria of AMD-062 name is not available.
/// </summary>
public sealed class HashRomFactAttribute : FactAttribute
{
    /// <summary>The three dumps the optional criteria of REQ-PROF-078 and 080 use.</summary>
    internal static readonly string[] Roms =
    [
        "Super Mario Bros. (World).nes",
        "Legend of Zelda, The (USA).nes",
        "Mega Man 2 (USA).nes",
    ];

    /// <summary>Creates the attribute.</summary>
    public HashRomFactAttribute()
    {
        if (!NeutralAssets.HashProfilesAvailable)
        {
            Skip = NeutralAssets.MissingMessage;
        }
        else if (!UserRoms.Available(Roms))
        {
            Skip = UserRoms.MissingMessage(Roms);
        }
    }
}
