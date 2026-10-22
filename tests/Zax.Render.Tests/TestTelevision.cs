using Zax.Render.Assets;

namespace Zax.Render.Tests;

/// <summary>
/// Locates the television model of AMD-025, <c>assets-free/tv/retro-crt-tv/retro_crt_tv.glb</c>,
/// from the test binaries.
/// </summary>
/// <remarks>
/// The file is not in the repository — it is a CC BY 4.0 model the release ships beside the
/// executable — so the tests that need the real thing check <see cref="Available"/> first and
/// return quietly when it is not there, the way the <c>assets-neutral</c> tests do. Everything
/// the loader itself has to do is covered by the synthetic files of
/// <see cref="TvModelTests"/>, which need nothing.
/// </remarks>
internal static class TestTelevision
{
    /// <summary>The model file, or null when it was not found.</summary>
    public static string? Path { get; } = Locate();

    /// <summary>Whether the model file is really there.</summary>
    public static bool Available => Path is not null;

    /// <summary>The loaded model, read once, or null when the file is missing.</summary>
    public static TvModel? Model { get; } = TvModel.TryLoad(Path);

    /// <summary>The raw glTF content of the model, read once, or null.</summary>
    public static GltfModel? Gltf { get; } = GltfBinary.TryLoad(Path);

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
