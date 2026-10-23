using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.App.Tests;

/// <summary>
/// AMD-033: the <c>profiles</c> folder next to the executable is scanned in every mode, is only
/// ever read, and its profiles are merged with the user's by the rules of §2.
/// </summary>
public sealed class ProgramProfilesTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), $"zax-amd033-{Guid.NewGuid():N}");

    public ProgramProfilesTests() => Directory.CreateDirectory(_root);

    /// <inheritdoc/>
    public void Dispose()
    {
        try
        {
            Directory.Delete(_root, recursive: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A throw-away folder that will not go is not a test failure.
        }
    }

    /// <summary>
    /// AMD-033 §1: the second folder is <c>profiles</c> beside the executable in the three
    /// modes — default root, portable and <c>--userdata</c> — and a profile dropped in it shows
    /// up in the library in each of them.
    /// </summary>
    [Theory]
    [InlineData(UserDataPaths.UserDataRootKind.LocalAppData)]
    [InlineData(UserDataPaths.UserDataRootKind.Portable)]
    [InlineData(UserDataPaths.UserDataRootKind.Explicit)]
    public void AMD_033_the_folder_beside_the_executable_is_scanned_in_every_mode(
        UserDataPaths.UserDataRootKind mode)
    {
        string executable = Path.Combine(_root, "bin");
        string programProfiles = Path.Combine(executable, "profiles");
        Directory.CreateDirectory(programProfiles);
        WriteProfile(Path.Combine(programProfiles, "Shipped.3dn"), "shipped", "Shipped");

        if (mode == UserDataPaths.UserDataRootKind.Portable)
        {
            File.WriteAllText(Path.Combine(executable, UserDataPaths.PortableMarkerName), string.Empty);
        }

        UserDataPaths paths = UserDataPaths.Resolve(
            explicitRoot: mode == UserDataPaths.UserDataRootKind.Explicit
                ? Path.Combine(_root, "elsewhere")
                : null,
            executableDirectory: executable,
            localAppDataDirectory: Path.Combine(_root, "local"));

        Assert.Equal(mode, paths.Kind);
        Assert.Equal(programProfiles, paths.ProgramProfilesDirectory);
        Assert.NotEqual(paths.ProfilesDirectory, paths.ProgramProfilesDirectory);

        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, paths);
        GameLibraryEntry entry = catalog.Library.Find("shipped")!;

        Assert.Equal("Shipped", entry.DisplayTitle);
        Assert.Equal(GameProfileSource.User, entry.Source);
        Assert.Equal(ProfileOrigin.ProgramFolder, entry.Origin);
        Assert.Equal("shipped", catalog.Resolve("shipped")!.Id);

        // The folder of the executable is read-only: the start-up writes nothing into it.
        Assert.Single(Directory.GetFiles(programProfiles));
    }

    /// <summary>
    /// AMD-033 §1: a missing folder is neither an error nor created, and an unreadable one lets
    /// the program start all the same.
    /// </summary>
    [Fact]
    public void AMD_033_a_missing_folder_is_not_created_and_is_not_an_error()
    {
        string executable = Path.Combine(_root, "bin");
        Directory.CreateDirectory(executable);

        UserDataPaths paths = UserDataPaths.Resolve(
            Path.Combine(_root, "userdata"), executable, Path.Combine(_root, "local"));

        Assert.True(paths.IsWritable);
        Assert.False(Directory.Exists(paths.ProgramProfilesDirectory));

        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, paths);
        Assert.Equal(2, catalog.Library.Entries.Count);
        Assert.False(Directory.Exists(paths.ProgramProfilesDirectory));

        // A file where the folder is expected makes it unreadable as a folder; the start-up
        // carries on with the user profiles alone.
        string other = Path.Combine(_root, "bin2");
        Directory.CreateDirectory(other);
        File.WriteAllText(Path.Combine(other, "profiles"), "not a folder");
        UserDataPaths blocked = UserDataPaths.Resolve(
            Path.Combine(_root, "userdata2"), other, Path.Combine(_root, "local"));

        Assert.Equal(2, LibraryGameCatalog.Open(assetRoot: null, blocked).Library.Entries.Count);
    }

    /// <summary>
    /// AMD-033 §2: a file of the same name in the user folder hides the one beside the
    /// executable, and the merge rules apply to the real folders as well.
    /// </summary>
    [Fact]
    public void AMD_033_the_user_folder_hides_the_file_of_the_same_name()
    {
        string executable = Path.Combine(_root, "bin");
        string programProfiles = Path.Combine(executable, "profiles");
        Directory.CreateDirectory(programProfiles);

        UserDataPaths paths = UserDataPaths.Resolve(
            Path.Combine(_root, "userdata"), executable, Path.Combine(_root, "local"));

        WriteProfile(Path.Combine(paths.ProfilesDirectory, "Game.3dn"), "both", "Mine");
        WriteProfile(Path.Combine(programProfiles, "GAME.3dn"), "other", "Theirs");
        WriteProfile(Path.Combine(programProfiles, "Only.3dn"), "only", "Only there");

        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, paths);

        Assert.Equal("Mine", catalog.Library.Find("both")!.DisplayTitle);
        Assert.Null(catalog.Library.Find("other"));
        Assert.Equal(ProfileOrigin.ProgramFolder, catalog.Library.Find("only")!.Origin);
    }

    /// <summary>
    /// AMD-033 §1: an import writes into the user folder only, takes over the library entry of
    /// the profile of the same identifier found beside the executable, and leaves that file
    /// where it is. Q-074: a game of the program folder cannot be deleted either.
    /// </summary>
    // Q-074
    [Fact]
    public void AMD_033_an_import_writes_to_the_user_folder_and_never_touches_the_other()
    {
        string executable = Path.Combine(_root, "bin");
        string programProfiles = Path.Combine(executable, "profiles");
        Directory.CreateDirectory(programProfiles);
        string shipped = Path.Combine(programProfiles, "Shipped.3dn");
        WriteProfile(shipped, "same", "Shipped");

        UserDataPaths paths = UserDataPaths.Resolve(
            Path.Combine(_root, "userdata"), executable, Path.Combine(_root, "local"));
        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, paths);

        GameLibraryEntry fromProgram = catalog.Library.Find("same")!;
        Assert.True(fromProgram.IsFromProgramFolder);
        Assert.False(fromProgram.CanDelete);
        Assert.False(catalog.DeleteGame("same"));
        Assert.True(File.Exists(shipped));

        string source = Path.Combine(_root, "Better.3dn");
        WriteProfile(source, "same", "Better");
        ProfileImportResult result = catalog.ImportProfile(source);

        Assert.Equal(ProfileImportOutcome.Added, result.Outcome);
        Assert.True(File.Exists(Path.Combine(paths.ProfilesDirectory, "Better.3dn")));
        Assert.True(File.Exists(shipped));
        Assert.Single(Directory.GetFiles(programProfiles));

        GameLibraryEntry entry = catalog.Library.Find("same")!;
        Assert.Equal(Path.Combine(paths.ProfilesDirectory, "Better.3dn"), entry.ProfilePath);
        Assert.Equal(ProfileOrigin.User, entry.Origin);
        Assert.True(entry.CanDelete);
    }

    /// <summary>
    /// AMD-033 §2: the origin is visible in the interface, where the source of a profile is
    /// already shown — the line under the title in the profile chooser.
    /// </summary>
    [Fact]
    public void AMD_033_the_chooser_shows_which_folder_a_profile_comes_from()
    {
        string executable = Path.Combine(_root, "bin");
        string programProfiles = Path.Combine(executable, "profiles");
        Directory.CreateDirectory(programProfiles);
        WriteProfile(Path.Combine(programProfiles, "Shipped.3dn"), "shipped", "Shipped");

        UserDataPaths paths = UserDataPaths.Resolve(
            Path.Combine(_root, "userdata"), executable, Path.Combine(_root, "local"));
        WriteProfile(Path.Combine(paths.ProfilesDirectory, "Mine.3dn"), "mine", "Mine");

        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, paths);
        ProfileChooserModel chooser = new(catalog.Library);

        Assert.Equal(
            "profile from the program folder",
            chooser.All.Single(choice => choice.Id == "shipped").SourceLabel);
        Assert.Equal(
            "user profile",
            chooser.All.Single(choice => choice.Id == "mine").SourceLabel);
    }

    private static void WriteProfile(string path, string id, string title)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        ProfileExport.Normalize(profile);

        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllBytes(path, ProfileFormat.Save(profile));
    }
}
