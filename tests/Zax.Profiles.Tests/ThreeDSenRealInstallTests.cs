using System.Security.Cryptography;
using System.Text.Json;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Import;
using Zax.Profiles.Library;

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

    /// <summary>
    /// How many of them carry no <c>Id</c> and take an identifier derived from their content
    /// (AMD-037 §1).
    /// </summary>
    public const int WithoutIdentifierCount = 45;

    /// <summary>
    /// The environment variable naming an installed 3dSen, when the tests are to be run against
    /// one rather than against the stand-in of <c>cleanroom/verify/p8-input</c>.
    /// </summary>
    public const string InstallVariable = "ZAXNES_3DSEN_INSTALL";

    /// <summary>
    /// The environment variable naming the asset folder holding <c>profiles.json</c> and the 101
    /// built-in profiles, for the tests that need the catalogue in use (AMD-037 §4).
    /// </summary>
    public const string CatalogueVariable = "ZAXNES_PROFILES";

    /// <summary>The message a skipped test reports.</summary>
    public const string MissingMessage =
        "No 3dSen installation is available: neither " + InstallVariable
        + " nor the stand-in of cleanroom/verify/p8-input.";

    /// <summary>The message a test needing the catalogue assets reports when skipped.</summary>
    public const string MissingCatalogueMessage =
        "No catalogue asset folder is available: neither " + CatalogueVariable
        + " nor assets-neutral next to the solution.";

    /// <summary>The installation folder, or null when it is not there.</summary>
    public static string? Root { get; } = Locate();

    /// <summary>
    /// The asset folder of the catalogue in use, or null when there is none: what
    /// <see cref="CatalogueVariable"/> names, else <c>assets-neutral</c>.
    /// </summary>
    public static string? CatalogueRoot { get; } = LocateCatalogue();

    /// <summary>The catalogue file of <see cref="CatalogueRoot"/>, or null.</summary>
    public static string? CatalogueFile
        => CatalogueRoot is null ? null : Path.Combine(CatalogueRoot, "profiles.json");

    /// <summary>True when an installation is available at all.</summary>
    public static bool InstallAvailable => Root is not null;

    /// <summary>True when the catalogue assets are available too.</summary>
    public static bool CatalogueAvailable
        => Root is not null && CatalogueFile is not null && File.Exists(CatalogueFile);

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
        // An installed 3dSen when the variable names one; the stand-in otherwise, which holds
        // the same bundle and is the only one a clean-side session may read.
        string? named = Environment.GetEnvironmentVariable(InstallVariable);
        if (!string.IsNullOrWhiteSpace(named) && Directory.Exists(named))
        {
            return named;
        }

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

    private static string? LocateCatalogue()
    {
        string? named = Environment.GetEnvironmentVariable(CatalogueVariable);
        return !string.IsNullOrWhiteSpace(named) && Directory.Exists(named)
            ? named
            : NeutralAssets.Root;
    }
}

/// <summary>
/// A fact that is skipped when no 3dSen installation is available (AMD-037 §4, second bullet).
/// </summary>
public sealed class ThreeDSenInstallFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public ThreeDSenInstallFactAttribute()
    {
        if (!ThreeDSenInstall.InstallAvailable)
        {
            Skip = ThreeDSenInstall.MissingMessage;
        }
    }
}

/// <summary>
/// A fact that is skipped when the installation or the catalogue assets are not available
/// (AMD-037 §4, second bullet).
/// </summary>
public sealed class ThreeDSenCatalogueFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public ThreeDSenCatalogueFactAttribute()
    {
        if (!ThreeDSenInstall.InstallAvailable)
        {
            Skip = ThreeDSenInstall.MissingMessage;
        }
        else if (!ThreeDSenInstall.CatalogueAvailable)
        {
            Skip = ThreeDSenInstall.MissingCatalogueMessage;
        }
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

    /// <summary>
    /// AMD-037 §4, second bullet, first case: with no asset folder, the 101 imported profiles all
    /// appear in the library and not one is left out for want of an identifier. The 45 that carry
    /// none take the identifier derived from their content, and a second start finds the same
    /// ones since no file was rewritten.
    /// </summary>
    [ThreeDSenInstallFact] // REQ-PROF-062, REQ-PROF-064
    public void AMD_037_the_101_imported_profiles_all_appear_without_an_asset_folder()
    {
        string destination = Path.Combine(
            Path.GetTempPath(), "zax-fix7-" + Guid.NewGuid().ToString("N"), "profiles");
        try
        {
            ThreeDSenImportSummary summary =
                new ThreeDSenImporter(PhysicalProfileFileSystem.Instance, destination)
                    .ImportFrom(ThreeDSenInstall.Root!);
            Assert.Equal(ThreeDSenInstall.ProfileCount, summary.Imported);

            // No asset folder: the catalogue is the two generic entries written in code
            // (AMD-024 REQ-PUB-001, Q-065), so nothing of it can hide a user profile.
            ProfileCatalogue catalogue = BuiltInCatalogue.Create();
            RecordingProfileLog log = new();
            IReadOnlyList<UserProfile> profiles = new ProfileFolders(
                    PhysicalProfileFileSystem.Instance,
                    destination,
                    programDirectory: null,
                    log,
                    BuiltInProfileIndex.For(PhysicalProfileFileSystem.Instance, catalogue))
                .Scan(catalogue.Ids);

            Assert.Equal(ThreeDSenInstall.ProfileCount, profiles.Count);
            Assert.Equal(
                ThreeDSenInstall.ProfileCount,
                profiles.Select(profile => profile.Id).Distinct(StringComparer.Ordinal).Count());
            Assert.DoesNotContain(
                log.Warnings,
                line => line.Contains("no identifier", StringComparison.Ordinal));

            // AMD-037 §1: 56 of the profiles carry an Id, 45 do not.
            Assert.Equal(
                ThreeDSenInstall.WithoutIdentifierCount,
                profiles.Count(profile => profile.IdIsDerived));
            Assert.All(
                profiles.Where(profile => profile.IdIsDerived),
                profile => Assert.Equal(12, profile.Id.Length));

            GameLibrary library = GameLibrary.Build(catalogue, profiles);
            Assert.Equal(
                ThreeDSenInstall.ProfileCount + BuiltInCatalogue.Entries.Count,
                library.Entries.Count);

            // Nothing was rewritten, so the next start derives the very same identifiers
            // (AMD-037 §2, AMD-030 §5).
            IReadOnlyList<UserProfile> again = new ProfileFolders(
                PhysicalProfileFileSystem.Instance, destination).Scan(catalogue.Ids);
            Assert.Equal(
                profiles.Select(profile => profile.Id),
                again.Select(profile => profile.Id));
        }
        finally
        {
            Directory.Delete(Path.GetDirectoryName(destination)!, recursive: true);
        }
    }

    /// <summary>
    /// AMD-037 §4, second bullet, second case: with the asset folder of the catalogue, no game
    /// appears twice — every imported file is a built-in profile byte for byte, so each is
    /// reported once and adds nothing.
    /// </summary>
    [ThreeDSenCatalogueFact] // REQ-PROF-060, REQ-PROF-062
    public void AMD_037_with_the_catalogue_assets_no_game_appears_twice()
    {
        string destination = Path.Combine(
            Path.GetTempPath(), "zax-fix7-" + Guid.NewGuid().ToString("N"), "profiles");
        try
        {
            ThreeDSenImportSummary summary =
                new ThreeDSenImporter(PhysicalProfileFileSystem.Instance, destination)
                    .ImportFrom(ThreeDSenInstall.Root!);
            Assert.Equal(ThreeDSenInstall.ProfileCount, summary.Imported);

            ProfileCatalogue catalogue = ProfileCatalogue.Load(
                PhysicalProfileFileSystem.Instance, ThreeDSenInstall.CatalogueFile!);
            RecordingProfileLog log = new();
            IReadOnlyList<UserProfile> profiles = new ProfileFolders(
                    PhysicalProfileFileSystem.Instance,
                    destination,
                    programDirectory: null,
                    log,
                    BuiltInProfileIndex.For(PhysicalProfileFileSystem.Instance, catalogue))
                .Scan(catalogue.Ids);

            // Each of the 101 files is one of the catalogue's own profiles: none adds a game,
            // and each is reported once.
            Assert.Empty(profiles);
            Assert.Equal(
                ThreeDSenInstall.ProfileCount,
                log.Informations.Count(line => line.Contains(
                    "identical to the built-in profile", StringComparison.Ordinal)));

            GameLibrary library = GameLibrary.Build(catalogue, profiles);
            Assert.Equal(catalogue.Entries.Count, library.Entries.Count);
            Assert.Equal(
                library.Entries.Count,
                library.Entries.Select(entry => entry.Id).Distinct(StringComparer.Ordinal).Count());
            Assert.Equal(
                library.Entries.Count,
                library.Entries
                    .Select(entry => entry.DisplayTitle)
                    .Distinct(StringComparer.OrdinalIgnoreCase)
                    .Count());
        }
        finally
        {
            Directory.Delete(Path.GetDirectoryName(destination)!, recursive: true);
        }
    }
}
