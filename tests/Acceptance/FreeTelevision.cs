using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Locates the television model of AMD-025, <c>assets-free/tv/retro-crt-tv/retro_crt_tv.glb</c>,
/// from the test binaries.
/// </summary>
/// <remarks>
/// The file is a CC BY 4.0 model the release ships beside the executable; it is not in the
/// repository, so like <see cref="NeutralAssets"/> this may simply find nothing. The tests that
/// need it return quietly then — the unit tests of <c>Zax.Render.Tests</c> cover the loader on
/// files they build themselves.
/// </remarks>
public static class FreeTelevision
{
    /// <summary>The model file, or null when it was not found.</summary>
    public static string? Path { get; } = Locate();

    /// <summary>Whether the model file is really there.</summary>
    public static bool Available => Path is not null;

    /// <summary>The loaded model, read once, or null when the file is missing.</summary>
    public static TvModel? Model { get; } = TvModel.TryLoad(Path);

    private static string? Locate()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = System.IO.Path.Combine(
                directory.FullName, "assets-free", "tv", "retro-crt-tv", "retro_crt_tv.glb");
            if (File.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}
