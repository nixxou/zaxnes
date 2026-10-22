namespace Zax.Profiles.Import;

/// <summary>
/// Reading the <c>TextAsset</c> profiles out of a 3dSen profile bundle (AMD-030 §4).
/// </summary>
/// <remarks>
/// This is the whole of the reader seen from outside: give it a bundle, get the names and the
/// unmodified payloads. Nothing is written and nothing of the installation is copied; what the
/// importer does with the payloads is <see cref="ThreeDSenImporter"/>'s business.
/// </remarks>
public static class ThreeDSenProfileBundle
{
    /// <summary>
    /// Node names that are resource side-files rather than serialized files, and are skipped.
    /// </summary>
    private static readonly string[] ResourceExtensions = [".resS", ".resource"];

    /// <summary>Reads the <c>TextAsset</c>s of the bundle at <paramref name="bundlePath"/>.</summary>
    /// <exception cref="UnityAssetFormatException">The bundle cannot be read.</exception>
    /// <exception cref="IOException">The file cannot be read.</exception>
    public static IReadOnlyList<UnityTextAsset> ReadTextAssets(string bundlePath)
    {
        ArgumentException.ThrowIfNullOrEmpty(bundlePath);
        return ReadTextAssets(File.ReadAllBytes(bundlePath));
    }

    /// <summary>
    /// Reads the <c>TextAsset</c>s of a bundle already in memory, in the order the serialized
    /// files list their objects.
    /// </summary>
    /// <exception cref="UnityAssetFormatException">The bundle cannot be read.</exception>
    public static IReadOnlyList<UnityTextAsset> ReadTextAssets(ReadOnlySpan<byte> bundle)
    {
        UnityBundle container = UnityBundle.Read(bundle);

        List<UnityTextAsset> assets = [];
        foreach (UnityBundleFile file in container.Files)
        {
            if (IsResourceNode(file.Path))
            {
                continue;
            }

            UnitySerializedFile serialized = UnitySerializedFile.Read(file.Data);
            foreach (UnityObject asset in serialized.Objects)
            {
                // AMD-030 §4.4: the AssetBundle object (142) and anything else are ignored.
                if (asset.ClassId == UnityTextAsset.ClassId)
                {
                    assets.Add(UnityTextAsset.Read(asset, serialized.IsBigEndian));
                }
            }
        }

        return assets;
    }

    private static bool IsResourceNode(string path)
    {
        foreach (string extension in ResourceExtensions)
        {
            if (path.EndsWith(extension, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }
}
