using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.Profiles.Tests;

/// <summary>
/// The second profile folder of AMD-033: <c>profiles</c> next to the executable, scanned in
/// every mode, read-only, and merged with the user folder by the rules of §2 (REQ-PERS-040,
/// REQ-PROF-062).
/// </summary>
public sealed class ProgramProfilesFolderTests
{
    // In-memory paths: nothing is read from or written to a real disk, and no path of any
    // machine appears here (lot PUB-1).
    private const string UserFolder = @"X:\fake\AppData\Local\Zaxnes\profiles";
    private const string ProgramFolder = @"X:\fake\Program Files\Zaxnes\profiles";

    private static byte[] MakeProfile(string? id, string? title = null)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        return ProfileFormat.Save(profile);
    }

    /// <summary>
    /// AMD-033 §1 and §2: both folders are shown, each profile tagged with where it was found.
    /// </summary>
    [Fact]
    public void AMD_033_both_folders_are_scanned_and_the_origin_is_kept()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Mine.3dn"), MakeProfile("mine", "Mine"));
        files.AddFile(Path.Combine(ProgramFolder, "Shipped.3dn"), MakeProfile("shipped", "Shipped"));

        RecordingProfileLog log = new();
        ProfileFolders folders = new(files, UserFolder, ProgramFolder, log);
        IReadOnlyList<UserProfile> profiles = folders.Scan();

        Assert.Equal(2, profiles.Count);
        UserProfile mine = profiles.Single(profile => profile.Id == "mine");
        UserProfile shipped = profiles.Single(profile => profile.Id == "shipped");
        Assert.Equal(ProfileOrigin.User, mine.Origin);
        Assert.Equal(ProfileOrigin.ProgramFolder, shipped.Origin);
        Assert.Equal(Path.Combine(ProgramFolder, "Shipped.3dn"), shipped.FilePath);
        Assert.Empty(log.Warnings);
    }

    /// <summary>
    /// AMD-033 §2: a file of the same name in the user folder hides the other, which is not even
    /// read — the comparison ignores case.
    /// </summary>
    [Fact]
    public void AMD_033_a_file_of_the_same_name_hides_the_one_of_the_program_folder()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Game.3dn"), MakeProfile("mine", "Mine"));

        // Not a profile at all: were it read, it would be reported as refused.
        string hidden = Path.Combine(ProgramFolder, "GAME.3dn");
        files.AddFile(hidden, [0x0a, 0x03, 0x33, 0x64, 0x78]);

        RecordingProfileLog log = new();
        UserProfile profile = Assert.Single(new ProfileFolders(files, UserFolder, ProgramFolder, log).Scan());

        Assert.Equal("mine", profile.Id);
        Assert.Equal(ProfileOrigin.User, profile.Origin);
        Assert.DoesNotContain(hidden, files.Reads);
        Assert.Empty(log.Warnings);
    }

    /// <summary>
    /// AMD-033 §2: two byte-identical files are the same profile twice and appear once, with
    /// nothing to report.
    /// </summary>
    [Fact]
    public void AMD_033_two_byte_identical_files_appear_once_and_silently()
    {
        byte[] bytes = MakeProfile("twice", "Twice");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Here.3dn"), bytes);
        files.AddFile(Path.Combine(ProgramFolder, "There.3dn"), bytes);

        RecordingProfileLog log = new();
        UserProfile profile = Assert.Single(new ProfileFolders(files, UserFolder, ProgramFolder, log).Scan());

        Assert.Equal(Path.Combine(UserFolder, "Here.3dn"), profile.FilePath);
        Assert.Equal(ProfileOrigin.User, profile.Origin);
        Assert.Empty(log.Warnings);
    }

    /// <summary>
    /// AMD-033 §2: for one identifier carried by two different files, the user folder wins and
    /// the other is reported once.
    /// </summary>
    [Fact]
    public void AMD_033_the_user_folder_wins_the_same_identifier_and_the_other_is_logged_once()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Mine.3dn"), MakeProfile("same", "Mine"));
        string loser = Path.Combine(ProgramFolder, "Theirs.3dn");
        files.AddFile(loser, MakeProfile("same", "Theirs"));

        RecordingProfileLog log = new();
        UserProfile profile = Assert.Single(new ProfileFolders(files, UserFolder, ProgramFolder, log).Scan());

        Assert.Equal("Mine", profile.Title);
        Assert.Equal(ProfileOrigin.User, profile.Origin);
        Assert.Single(log.Warnings);
        Assert.Contains(ProfileImportMessages.DuplicatedProfile(loser, "same"), log.Warnings);
    }

    /// <summary>
    /// AMD-033 §2: the rules of REQ-PERS-030 and REQ-PROF-062 still hold for the second folder —
    /// a built-in identifier wins, and an unreadable or foreign file is simply left out.
    /// </summary>
    [Fact]
    public void AMD_033_the_reading_and_rejection_rules_apply_to_the_second_folder()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(ProgramFolder, "a-builtin.3dn"), MakeProfile("1", "Not Arkanoid"));
        files.AddFile(Path.Combine(ProgramFolder, "b-noid.3dn"), MakeProfile(null, "No identifier"));
        files.AddFile(Path.Combine(ProgramFolder, "c-foreign.3dn"), [0x0a, 0x03, 0x33, 0x64, 0x78]);
        files.AddFile(Path.Combine(ProgramFolder, "d-good.3dn"), MakeProfile("good", "Good"));

        RecordingProfileLog log = new();
        UserProfile profile = Assert.Single(new ProfileFolders(files, UserFolder, ProgramFolder, log).Scan(["1"]));

        Assert.Equal("good", profile.Id);
        Assert.Equal(ProfileOrigin.ProgramFolder, profile.Origin);
        Assert.True(log.AnyContains("duplicated profile"));
        Assert.True(log.AnyContains("carries no identifier"));
        Assert.True(log.AnyContains(ProfileImportMessages.NotA3dnFile));
    }

    /// <summary>
    /// AMD-033 §1: a folder that is missing, that cannot be listed, or that is not given at all
    /// is not an error — the user profiles are still scanned.
    /// </summary>
    [Fact]
    public void AMD_033_a_missing_or_unreadable_folder_is_not_an_error()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Mine.3dn"), MakeProfile("mine", "Mine"));

        Assert.Single(new ProfileFolders(files, UserFolder, ProgramFolder).Scan());
        Assert.Single(new ProfileFolders(files, UserFolder, null).Scan());
        Assert.Single(new ProfileFolders(files, UserFolder, string.Empty).Scan());

        files.AddFile(Path.Combine(ProgramFolder, "Shipped.3dn"), MakeProfile("shipped"));
        files.Unreadable.Add(ProgramFolder);

        RecordingProfileLog log = new();
        UserProfile profile = Assert.Single(new ProfileFolders(files, UserFolder, ProgramFolder, log).Scan());
        Assert.Equal("mine", profile.Id);
        Assert.True(log.AnyContains("could not be listed"));
    }

    /// <summary>
    /// AMD-033 §1: with <c>--userdata</c> pointing at the folder of the executable, the two
    /// folders are the same one and its profiles are listed once, as the user's.
    /// </summary>
    [Fact]
    public void AMD_033_the_same_folder_under_both_names_is_scanned_once()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(ProgramFolder, "Mine.3dn"), MakeProfile("mine", "Mine"));

        RecordingProfileLog log = new();
        ProfileFolders folders = new(files, ProgramFolder, ProgramFolder + @"\", log);

        Assert.Null(folders.ProgramDirectory);
        UserProfile profile = Assert.Single(folders.Scan());
        Assert.Equal(ProfileOrigin.User, profile.Origin);
        Assert.Empty(log.Warnings);
    }

    /// <summary>
    /// AMD-033 §2: the library carries the origin, so the interface can show it; Q-074: a game
    /// whose file lives next to the executable is not deletable, since deleting it would delete
    /// a file of a folder Zaxnes only ever reads.
    /// </summary>
    // Q-074
    [Fact]
    public void AMD_033_the_library_carries_the_origin_and_protects_the_program_folder()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(UserFolder, "Mine.3dn"), MakeProfile("mine", "Mine"));
        files.AddFile(Path.Combine(ProgramFolder, "Shipped.3dn"), MakeProfile("shipped", "Shipped"));

        ProfileCatalogue catalogue = ProfileCatalogue.FromEntries(
            @"X:\fake\assets", [new CatalogueEntry("47", "2D Mode", "", "", "", "profiles/empty.bytes")]);
        GameLibrary library = GameLibrary.Build(
            catalogue, new ProfileFolders(files, UserFolder, ProgramFolder).Scan(catalogue.Ids));

        GameLibraryEntry mine = library.Find("mine")!;
        GameLibraryEntry shipped = library.Find("shipped")!;

        Assert.Equal(GameProfileSource.User, shipped.Source);
        Assert.Equal(ProfileOrigin.ProgramFolder, shipped.Origin);
        Assert.True(shipped.IsFromProgramFolder);
        Assert.False(shipped.CanDelete);
        Assert.False(library.Remove("shipped"));

        Assert.False(mine.IsFromProgramFolder);
        Assert.True(mine.CanDelete);
        Assert.True(library.Remove("mine"));
    }

    /// <summary>
    /// AMD-033 §1 and §2: an import always writes into the user folder, and the profile it
    /// brings takes over the library entry of the one of the same identifier found next to the
    /// executable, whose file is left alone.
    /// </summary>
    [Fact]
    public void AMD_033_an_import_writes_to_the_user_folder_and_takes_over_the_entry()
    {
        FakeProfileFileSystem files = new();
        string shipped = Path.Combine(ProgramFolder, "Shipped.3dn");
        files.AddFile(shipped, MakeProfile("same", "Shipped"));
        const string source = @"X:\fake\downloads\Better.3dn";
        files.AddFile(source, MakeProfile("same", "Better"));

        ProfileFolders folders = new(files, UserFolder, ProgramFolder);
        ProfileCatalogue catalogue = ProfileCatalogue.FromEntries(@"X:\fake\assets", []);
        GameLibrary library = GameLibrary.Build(catalogue, folders.Scan());
        Assert.Equal(ProfileOrigin.ProgramFolder, library.Find("same")!.Origin);

        // The import only ever knows the profiles of the user folder, so it has nothing to
        // delete; the file next to the executable stays where it is.
        UserProfileFolder user = new(files, UserFolder);
        ProfileImportResult result = user.Import(source, [], []);

        Assert.Equal(ProfileImportOutcome.Added, result.Outcome);
        Assert.True(files.FileExists(Path.Combine(UserFolder, "Better.3dn")));
        Assert.True(files.FileExists(shipped));

        library.AddUserProfile(result.Profile!);
        GameLibraryEntry entry = library.Find("same")!;
        Assert.Equal(Path.Combine(UserFolder, "Better.3dn"), entry.ProfilePath);
        Assert.Equal(ProfileOrigin.User, entry.Origin);
        Assert.True(entry.CanDelete);

        // And the next scan keeps the user's file, the program one now being a duplicate.
        UserProfile kept = Assert.Single(folders.Scan());
        Assert.Equal(ProfileOrigin.User, kept.Origin);
    }
}
