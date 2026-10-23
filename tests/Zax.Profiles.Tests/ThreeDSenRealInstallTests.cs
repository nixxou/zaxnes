using System.Security.Cryptography;
using System.Text.Json;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Import;

namespace Zax.Profiles.Tests;

/// <summary>
/// Locates the stand-in 3dSen installation of <c>cleanroom/verify/p8-input</c> and the reference
/// inventory, both of which sit beside the solution rather than inside it.
/// </summary>
/// <remarks>
/// Nothing of the installation ever enters the repository: the bundle is read where it is and
/// the profiles are written to a temporary folder outside the tree (AMD-030 §6, licences).
/// </remarks>
internal static class ThreeDSenInstall
{
    /// <summary>How many profiles the installation holds (AMD-030 §6).</summary>
    public const int ProfileCount = 101;

    /// <summary>The message a skipped test reports.</summary>
    public const string MissingMessage =
        "The stand-in 3dSen installation of cleanroom/verify/p8-input is not available next to the solution.";

    /// <summary>The installation folder, or null when it is not there.</summary>
    public static string? Root { get; } = Locate();

    /// <summary>True when both the installation and the reference inventory are available.</summary>
    public static bool Available => Root is not null && NeutralAssets.Root is not null;

    /// <summary>The SHA-256 of every <c>Profile</c> entry of <c>assets-neutral/inventory.json</c>.</summary>
    public static HashSet<string> ReferenceHashes()
    {
        if (NeutralAssets.Root is null)
        {
            throw new InvalidOperationException(NeutralAssets.MissingMessage);
        }

        using FileStream stream = File.OpenRead(Path.Combine(NeutralAssets.Root, "inventory.json"));
        using JsonDocument document = JsonDocument.Parse(stream);

        HashSet<string> hashes = new(StringComparer.OrdinalIgnoreCase);
        foreach (JsonElement entry in document.RootElement.EnumerateArray())
        {
            if (entry.TryGetProperty("kind", out JsonElement kind)
                && string.Equals(kind.GetString(), "Profile", StringComparison.Ordinal)
                && entry.TryGetProperty("sha256", out JsonElement hash)
                && hash.GetString() is { Length: > 0 } text)
            {
                hashes.Add(text);
            }
        }

        return hashes;
    }

    /// <summary>The SHA-256 of a file, lower-case hexadecimal.</summary>
    public static string HashOf(string path)
        => Convert.ToHexStringLower(SHA256.HashData(File.ReadAllBytes(path)));

    private static string? Locate()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = Path.Combine(
                directory.FullName, "verify", "p8-input", "fake-3dsen-install");
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
/// A fact that is skipped, rather than failed, when the stand-in installation is not there.
/// </summary>
public sealed class ThreeDSenFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public ThreeDSenFactAttribute()
    {
        if (!ThreeDSenInstall.Available)
        {
            Skip = ThreeDSenInstall.MissingMessage;
        }
    }
}

/// <summary>
/// The acceptance criteria of AMD-030 §6 on a real installation: 101 profiles imported, each one
/// byte-for-byte a profile of the reference inventory, and a second import that adds nothing.
/// </summary>
/// <remarks>
/// Opt-in: the whole class carries <c>Requires=OriginalAssets</c> and is skipped when the
/// installation is absent, which is the state of a clone of this repository.
/// </remarks>
[Trait("Requires", "OriginalAssets")]
public sealed class ThreeDSenRealInstallTests
{
    [ThreeDSenFact] // AMD-030 §3
    public void The_bundle_is_found_from_the_installation_folder()
    {
        string? bundle = ThreeDSenInstallation.FindBundle(ThreeDSenInstall.Root!);

        Assert.NotNull(bundle);
        Assert.True(ThreeDSenInstallation.IsBundleName(Path.GetFileName(bundle)));
        Assert.Contains("StandaloneWindows", bundle, StringComparison.Ordinal);
    }

    [ThreeDSenFact] // AMD-030 §4
    public void The_bundle_holds_the_101_profiles_and_the_asset_bundle_object()
    {
        string bundle = ThreeDSenInstallation.LocateBundle(ThreeDSenInstall.Root!);
        UnityBundle container = UnityBundle.Read(File.ReadAllBytes(bundle));

        // §4.1 and §4.2: version 7, one CAB- node, blocks of 128 KiB.
        Assert.Equal(7u, container.FormatVersion);
        UnityBundleFile node = Assert.Single(container.Files);
        Assert.StartsWith("CAB-", node.Path, StringComparison.Ordinal);

        // §4.3: version 21, little-endian, 102 objects.
        UnitySerializedFile serialized = UnitySerializedFile.Read(node.Data);
        Assert.Equal(21u, serialized.Version);
        Assert.False(serialized.IsBigEndian);
        Assert.Equal(ThreeDSenInstall.ProfileCount + 1, serialized.Objects.Count);

        IReadOnlyList<UnityTextAsset> assets = ThreeDSenProfileBundle.ReadTextAssets(bundle);
        Assert.Equal(ThreeDSenInstall.ProfileCount, assets.Count);
        Assert.All(assets, asset => Assert.NotEqual(string.Empty, asset.Name));
    }

    [ThreeDSenFact] // AMD-030 §5, §6
    public void Every_payload_decodes_with_the_spec_03_decoder()
    {
        string bundle = ThreeDSenInstallation.LocateBundle(ThreeDSenInstall.Root!);

        foreach (UnityTextAsset asset in ThreeDSenProfileBundle.ReadTextAssets(bundle))
        {
            ProfileLoadResult result = ProfileFormat.Load(asset.Script.Span, sourceName: asset.Name);
            Assert.True(result.IsSuccess, $"{asset.Name}: {result.Message}");
        }
    }

    [ThreeDSenFact] // AMD-030 §6, first and second bullets
    public void The_import_writes_101_profiles_of_the_inventory_and_the_second_one_adds_nothing()
    {
        HashSet<string> reference = ThreeDSenInstall.ReferenceHashes();
        Assert.Equal(ThreeDSenInstall.ProfileCount, reference.Count);

        // Outside the repository on purpose: no file of 3dSen ever lands in the tree.
        string destination = Path.Combine(
            Path.GetTempPath(), "zax-p8-" + Guid.NewGuid().ToString("N"), "profiles");
        try
        {
            ThreeDSenImporter importer = new(PhysicalProfileFileSystem.Instance, destination);

            ThreeDSenImportSummary first = importer.ImportFrom(ThreeDSenInstall.Root!);

            Assert.Equal(ThreeDSenInstall.ProfileCount, first.Imported);
            Assert.Equal(0, first.AlreadyPresent);
            Assert.Equal(0, first.Failed);

            string[] written = Directory.GetFiles(destination);
            Assert.Equal(ThreeDSenInstall.ProfileCount, written.Length);
            Assert.All(written, path => Assert.EndsWith(
                UserProfileFolder.Extension, path, StringComparison.Ordinal));

            // Each produced file is, byte for byte, a profile of the reference inventory.
            foreach (string path in written)
            {
                Assert.Contains(ThreeDSenInstall.HashOf(path), reference);
            }

            // One profile per inventory entry: no duplicate, nothing missing.
            Assert.Equal(
                ThreeDSenInstall.ProfileCount,
                written.Select(ThreeDSenInstall.HashOf).Distinct(StringComparer.OrdinalIgnoreCase).Count());

            ThreeDSenImportSummary second = importer.ImportFrom(ThreeDSenInstall.Root!);

            Assert.Equal(0, second.Imported);
            Assert.Equal(ThreeDSenInstall.ProfileCount, second.AlreadyPresent);
            Assert.Equal(0, second.Failed);
            Assert.Equal(ThreeDSenInstall.ProfileCount, Directory.GetFiles(destination).Length);
        }
        finally
        {
            Directory.Delete(Path.GetDirectoryName(destination)!, recursive: true);
        }
    }
}
