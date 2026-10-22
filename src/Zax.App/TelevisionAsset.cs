namespace Zax.App;

/// <summary>
/// Where the television model of AMD-025 is found: the CC BY 4.0 glTF file
/// <c>assets/tv/retro_crt_tv.glb</c> the release ships beside the executable.
/// </summary>
/// <remarks>
/// <para>
/// This is the one asset file the publishable build carries, and it is <b>not</b> an
/// exception to AMD-024 REQ-PUB-002: it comes from no original build, it is shipped unmodified
/// under its own licence and <c>ASSETS-LICENSES.md</c> carries the attribution CC BY 4.0 asks
/// for. AMD-025 also requires that the application still run without it — the search simply
/// comes back empty and the TV box option is disabled.
/// </para>
/// <para>
/// Unlike the asset folder of <see cref="NeutralAssets"/>, this one is discovered: it is part
/// of the build, next to the executable, and never somewhere up the user's directory tree.
/// <c>--tv-model &lt;file&gt;</c> names another one.
/// </para>
/// </remarks>
internal static class TelevisionAsset
{
    /// <summary>The folder of the release the model lives in.</summary>
    public static string FolderName { get; } = Path.Combine("assets", "tv");

    /// <summary>The file name of the model AMD-025 names.</summary>
    public const string FileName = "retro_crt_tv.glb";

    /// <summary>
    /// The model file, or null when there is none. <paramref name="hint"/> is the
    /// <c>--tv-model</c> option: when it is given and does not exist, nothing else is tried.
    /// </summary>
    public static string? Locate(string? hint = null)
    {
        if (!string.IsNullOrWhiteSpace(hint))
        {
            return File.Exists(hint) ? Path.GetFullPath(hint) : null;
        }

        string folder = Path.Combine(AppContext.BaseDirectory, FolderName);
        string named = Path.Combine(folder, FileName);
        if (File.Exists(named))
        {
            return named;
        }

        if (!Directory.Exists(folder))
        {
            return null;
        }

        // A renamed or updated model is still read, as long as it is the only one there.
        string[] candidates = Directory.GetFiles(folder, "*.glb");
        Array.Sort(candidates, StringComparer.Ordinal);
        return candidates.Length > 0 ? candidates[0] : null;
    }
}
