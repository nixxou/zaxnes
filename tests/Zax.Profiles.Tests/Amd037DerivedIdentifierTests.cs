using System.Security.Cryptography;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.Profiles.Tests;

/// <summary>
/// AMD-037: a user profile whose <c>Id</c> is absent or empty takes the identifier derived from
/// its content instead of being ignored (SPEC-03 REQ-PROF-062, AMD-030 §5, AMD-033).
/// </summary>
/// <remarks>
/// Synthetic profiles only: every file here is built by <see cref="ProfileFormat.Save"/> from an
/// empty profile, and the file system is the in-memory one — no commercial data and no path of
/// any machine (lot PUB-1).
/// </remarks>
public sealed class Amd037DerivedIdentifierTests
{
    private const string UserFolder = @"X:\fake\AppData\Local\Zaxnes\profiles";
    private const string Assets = @"X:\fake\assets-neutral";

    private static byte[] MakeProfile(string? id, string? title = null)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        return ProfileFormat.Save(profile);
    }

    /// <summary>The identifier of AMD-037 §2, computed here rather than taken from the code.</summary>
    private static string ExpectedIdentifier(byte[] bytes)
        => Convert.ToBase64String(SHA256.HashData(bytes), 0, 9);

    /// <summary>
    /// AMD-037 §4, first criterion: a profile without <c>Id</c> dropped in the user folder
    /// appears in the library, with the twelve base64 characters of the first nine bytes of the
    /// SHA-256 of its bytes; read again at the next start it keeps the same identifier.
    /// </summary>
    [Fact] // REQ-PROF-062, REQ-PROF-064
    public void AMD_037_a_profile_without_an_identifier_appears_under_one_derived_from_its_content()
    {
        byte[] bytes = MakeProfile(id: null, title: "Sans identifiant");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Sans.3dn"), bytes);

        RecordingProfileLog log = new();
        UserProfileFolder folder = new(files, UserFolder, log);
        UserProfile profile = Assert.Single(folder.Scan());

        string expected = ExpectedIdentifier(bytes);
        Assert.Equal(expected, profile.Id);
        Assert.Equal(12, profile.Id.Length);
        Assert.All(profile.Id, character => Assert.True(
            char.IsAsciiLetterOrDigit(character) || character is '+' or '/',
            $"'{character}' is not in the standard base64 alphabet"));
        Assert.True(profile.IdIsDerived);
        Assert.Equal("Sans identifiant", profile.Title);

        // The game is in the library under that identifier, which is the key of its ROM path,
        // its cover and its saves (REQ-PROF-064).
        ProfileCatalogue catalogue = ProfileCatalogue.FromEntries(Assets, []);
        GameLibrary library = GameLibrary.Build(catalogue, [profile]);
        GameLibraryEntry entry = Assert.Single(library.Entries);
        Assert.Equal(expected, entry.Id);
        Assert.True(library.SetRomPath(expected, @"X:\fake\roms\jeu.nes"));

        // AMD-037 §2, second rule: nothing was written back, so the next start reads the very
        // same bytes and derives the very same identifier (AMD-030 §5 stays true).
        Assert.Equal(bytes, files.ReadAllBytes(Path.Combine(UserFolder, "Sans.3dn")));
        UserProfile reread = Assert.Single(new UserProfileFolder(files, UserFolder, log).Scan());
        Assert.Equal(expected, reread.Id);
        Assert.True(log.AnyContains(ProfileImportMessages.DerivedIdentifier(
            Path.Combine(UserFolder, "Sans.3dn"), expected)));
    }

    /// <summary>
    /// AMD-037 §4, second criterion: the same profile without <c>Id</c> copied under another
    /// name is the same content, so it adds one game, not two.
    /// </summary>
    [Fact] // REQ-PROF-062
    public void AMD_037_the_same_profile_under_two_names_appears_once()
    {
        byte[] bytes = MakeProfile(id: null, title: "Sans identifiant");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Aaa.3dn"), bytes);
        files.AddFile(Path.Combine(UserFolder, "Bbb.3dn"), bytes);

        RecordingProfileLog log = new();
        UserProfile profile = Assert.Single(new UserProfileFolder(files, UserFolder, log).Scan());

        // The folder is walked in ordinal order of the names, so the winner is fixed.
        Assert.Equal(Path.Combine(UserFolder, "Aaa.3dn"), profile.FilePath);
        Assert.Equal(ExpectedIdentifier(bytes), profile.Id);
        Assert.True(log.AnyContains("duplicated profile"));
    }

    /// <summary>
    /// AMD-037 §4, third criterion: a user profile whose bytes are those of a built-in profile
    /// of the catalogue in use adds no game — the built-in one is shown once, with its catalogue
    /// metadata — and the file is reported once. True with an <c>Id</c> as without.
    /// </summary>
    [Theory] // REQ-PROF-060, REQ-PROF-062
    [InlineData(null)]
    [InlineData("")]
    [InlineData("1")]
    [InlineData("otherid12345")]
    public void AMD_037_a_profile_identical_to_a_built_in_one_adds_no_game(string? id)
    {
        byte[] bytes = MakeProfile(id, title: "Pas Arkanoid");
        string builtInFile = Path.Combine(Assets, "profiles", "Arkanoid (U).bytes");

        FakeProfileFileSystem files = new();
        files.AddFile(builtInFile, bytes);
        string userFile = Path.Combine(UserFolder, "Arkanoid.3dn");
        files.AddFile(userFile, bytes);

        ProfileCatalogue catalogue = ProfileCatalogue.FromEntries(
            Assets,
            [new CatalogueEntry("1", "Arkanoid", "Taito", "", "", "profiles/Arkanoid (U).bytes")]);
        BuiltInProfileIndex index = Assert.IsType<BuiltInProfileIndex>(
            BuiltInProfileIndex.For(files, catalogue));

        RecordingProfileLog log = new();
        ProfileFolders folders = new(files, UserFolder, null, log, index);
        Assert.Empty(folders.Scan(catalogue.Ids));

        GameLibrary library = GameLibrary.Build(catalogue, folders.Scan(catalogue.Ids));
        GameLibraryEntry entry = Assert.Single(library.Entries);
        Assert.Equal("1", entry.Id);
        Assert.Equal("Arkanoid", entry.Title);
        Assert.Equal(GameProfileSource.BuiltIn, entry.Source);
        Assert.Contains(ProfileImportMessages.SameAsBuiltIn(userFile, "1"), log.Informations);

        // A profile of other bytes is not mistaken for the built-in one.
        files.AddFile(Path.Combine(UserFolder, "Autre.3dn"), MakeProfile(null, title: "Autre jeu"));
        Assert.Single(new ProfileFolders(files, UserFolder, null, log, index).Scan(catalogue.Ids));
    }

    /// <summary>
    /// AMD-037 §4, fourth criterion, and §2 last bullet: importing a chosen <c>.3dn</c> that
    /// carries no identifier is still refused, as in 3dSen. The rule only concerns files already
    /// sitting in a profile folder.
    /// </summary>
    [Fact] // REQ-PROF-063
    public void AMD_037_importing_a_single_file_without_an_identifier_is_still_refused()
    {
        FakeProfileFileSystem files = new();
        const string source = @"X:\fake\downloads\Sans.3dn";
        files.AddFile(source, MakeProfile(id: null, title: "Sans identifiant"));

        RecordingProfileLog log = new();
        UserProfileFolder folder = new(files, UserFolder, log);
        ProfileImportResult result = folder.Import(source);

        Assert.Equal(ProfileImportOutcome.Rejected, result.Outcome);
        Assert.Equal(ProfileImportMessages.ObsoleteFormat(source), result.Message);
        Assert.Null(result.Profile);
        Assert.False(files.FileExists(Path.Combine(UserFolder, "Sans.3dn")));
        Assert.Empty(folder.Scan());
    }

    /// <summary>
    /// AMD-037 §3: a derived identifier cannot be taken for a catalogue one, and a profile that
    /// does carry an <c>Id</c> keeps it untouched.
    /// </summary>
    [Fact] // REQ-PROF-062
    public void AMD_037_a_carried_identifier_is_kept_and_a_derived_one_never_looks_like_a_catalogue_id()
    {
        byte[] carried = MakeProfile("abc123def456", "Avec");
        byte[] derived = MakeProfile(null, "Sans");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Avec.3dn"), carried);
        files.AddFile(Path.Combine(UserFolder, "Sans.3dn"), derived);

        IReadOnlyList<UserProfile> scanned = new UserProfileFolder(files, UserFolder).Scan();

        Assert.Equal(2, scanned.Count);
        UserProfile withId = scanned.Single(profile => !profile.IdIsDerived);
        UserProfile withoutId = scanned.Single(profile => profile.IdIsDerived);
        Assert.Equal("abc123def456", withId.Id);
        Assert.Equal(12, withoutId.Id.Length);

        // Catalogue identifiers are "1" to "101"; none of them is twelve characters long.
        for (int number = 1; number <= 101; number++)
        {
            Assert.NotEqual(number.ToString(System.Globalization.CultureInfo.InvariantCulture), withoutId.Id);
        }
    }

    /// <summary>
    /// AMD-037 §2 and §3, with the two folders of AMD-033: a derived identifier takes part in
    /// the merge like any other, and at equal identifier the user's folder wins.
    /// </summary>
    [Fact] // REQ-PROF-062
    public void AMD_037_a_derived_identifier_follows_the_merge_rules_of_the_two_folders()
    {
        byte[] bytes = MakeProfile(id: null, title: "Sans identifiant");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Mine.3dn"), bytes);
        const string programFolder = @"X:\fake\Program Files\Zaxnes\profiles";
        files.AddFile(Path.Combine(programFolder, "Theirs.3dn"), bytes);

        RecordingProfileLog log = new();
        UserProfile profile = Assert.Single(
            new ProfileFolders(files, UserFolder, programFolder, log).Scan());

        Assert.Equal(ExpectedIdentifier(bytes), profile.Id);
        Assert.Equal(ProfileOrigin.User, profile.Origin);

        // Byte-identical files are the same profile twice: they appear once and silently
        // (AMD-033 §2).
        Assert.Empty(log.Warnings);
    }
}
