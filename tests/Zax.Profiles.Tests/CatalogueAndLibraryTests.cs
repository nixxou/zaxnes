using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.Profiles.Tests;

/// <summary>
/// The catalogue of built-in profiles, the user profile folder and the game library:
/// REQ-PROF-060 to REQ-PROF-065, with the persistence of SPEC-09 §3.3 and §3.4.
/// </summary>
public sealed class CatalogueAndLibraryTests
{
    // Two roots of the in-memory file system of these tests (FakeProfileFileSystem): nothing is
    // read from or written to a real disk, and no path of any machine appears here (lot PUB-1).
    private const string Root = @"X:\fake\AppData\Local\Zaxnes";
    private const string Assets = @"X:\fake\assets-neutral";

    private static byte[] MakeProfile(string? id, string? title = null)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        return ProfileFormat.Save(profile);
    }

    private static string MakeCatalogueJson(params (string Id, string Title, string File)[] entries)
    {
        IEnumerable<string> full = entries.Select(entry =>
            $$"""
              {
               "id": "{{entry.Id}}",
               "title": "{{entry.Title}}",
               "publisher": "",
               "description": "",
               "author": "",
               "file": "{{entry.File}}"
              }
              """);

        return $$"""
                 {
                  "comment": "test",
                  "editions": {
                   "full": [{{string.Join(",", full)}}],
                   "demo": []
                  }
                 }
                 """;
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_060_the_full_edition_of_the_catalogue_holds_the_101_built_in_games()
    {
        ProfileCatalogue catalogue = ProfileCatalogue.Load(
            PhysicalProfileFileSystem.Instance, NeutralAssets.CatalogueFile!);

        Assert.Equal(101, catalogue.Entries.Count);
        Assert.Equal(101, catalogue.Ids.Count);

        CatalogueEntry arkanoid = catalogue.Find("1")!;
        Assert.Equal("Arkanoid", arkanoid.Title);
        Assert.Equal("profiles/Arkanoid (U).bytes", arkanoid.File);
        Assert.True(PhysicalProfileFileSystem.Instance.FileExists(catalogue.ProfilePath(arkanoid)));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_061_the_game_of_identifier_47_is_the_2D_Mode_entry()
    {
        ProfileCatalogue catalogue = ProfileCatalogue.Load(
            PhysicalProfileFileSystem.Instance, NeutralAssets.CatalogueFile!);

        CatalogueEntry twoD = catalogue.Find("47")!;
        Assert.Equal("2D Mode", twoD.Title);
        Assert.Equal("profiles/empty.bytes", twoD.File);
        Assert.True(twoD.Is2DMode);
        Assert.Same(twoD, catalogue.TwoDimensionalMode);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_060_every_catalogue_entry_points_at_a_profile_that_loads()
    {
        ProfileCatalogue catalogue = ProfileCatalogue.Load(
            PhysicalProfileFileSystem.Instance, NeutralAssets.CatalogueFile!);

        foreach (CatalogueEntry entry in catalogue.Entries)
        {
            ProfileLoadResult result = catalogue.LoadProfile(PhysicalProfileFileSystem.Instance, entry);
            Assert.True(result.IsSuccess, $"{entry.Id} {entry.Title}: {result.Message}");
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_060_the_demo_edition_is_data_the_application_does_not_use()
    {
        // DEC-03-Q04: only the full edition is used. The demo edition is still readable, and
        // its identifiers deliberately overlap those of the full edition.
        ProfileCatalogue demo = ProfileCatalogue.Load(
            PhysicalProfileFileSystem.Instance, NeutralAssets.CatalogueFile!, ProfileCatalogue.DemoEdition);

        Assert.Equal(20, demo.Entries.Count);
        Assert.NotEqual("Arkanoid", demo.Find("1")!.Title);
    }

    [Fact]
    public void REQ_PROF_060_an_unreadable_catalogue_is_reported_not_swallowed()
    {
        FakeProfileFileSystem files = new();
        files.AddText(@"C:\assets\profiles.json", "{{{");

        Assert.Throws<ProfileCatalogueException>(
            () => ProfileCatalogue.Load(files, @"C:\assets\profiles.json"));
        Assert.Throws<ProfileCatalogueException>(
            () => ProfileCatalogue.Load(files, @"C:\assets\missing.json"));
        Assert.Throws<ProfileCatalogueException>(
            () => ProfileCatalogue.Parse("{}", @"C:\assets"));
    }

    [Fact]
    public void REQ_PROF_062_a_user_profile_without_a_title_is_named_after_its_file()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        files.AddFile(Path.Combine(layout.ProfilesDirectory, "Foo.3dn"), MakeProfile("abc123def456"));

        RecordingProfileLog log = new();
        UserProfileFolder folder = new(files, layout.ProfilesDirectory, log);
        UserProfile profile = Assert.Single(folder.Scan());

        Assert.Equal("abc123def456", profile.Id);
        Assert.Equal("Foo", profile.Title);
    }

    [Fact]
    public void REQ_PROF_062_a_second_profile_with_the_same_identifier_is_ignored()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        files.AddFile(Path.Combine(layout.ProfilesDirectory, "Aaa.3dn"), MakeProfile("dup", "First"));
        files.AddFile(Path.Combine(layout.ProfilesDirectory, "Bbb.3dn"), MakeProfile("dup", "Second"));

        RecordingProfileLog log = new();
        UserProfileFolder folder = new(files, layout.ProfilesDirectory, log);
        UserProfile profile = Assert.Single(folder.Scan());

        // The folder is walked in ordinal order of the file names, so the winner does not
        // depend on the file system (REQ-PROF-062, notes).
        Assert.Equal("First", profile.Title);
        Assert.True(log.AnyContains("duplicated profile"));
    }

    [Fact]
    public void REQ_PROF_062_an_identifier_that_belongs_to_a_built_in_profile_is_ignored()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        files.AddFile(Path.Combine(layout.ProfilesDirectory, "Clash.3dn"), MakeProfile("1", "Mine"));

        RecordingProfileLog log = new();
        UserProfileFolder folder = new(files, layout.ProfilesDirectory, log);

        Assert.Empty(folder.Scan(["1"]));
        Assert.True(log.AnyContains("duplicated profile"));
    }

    [Fact]
    public void REQ_PROF_062_an_unreadable_or_foreign_file_is_ignored_and_the_others_are_kept()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        byte[] good = MakeProfile("good", "Good");
        files.AddFile(Path.Combine(layout.ProfilesDirectory, "a-good.3dn"), good);
        files.AddFile(
            Path.Combine(layout.ProfilesDirectory, "b-truncated.3dn"),
            good.AsSpan(0, good.Length / 2).ToArray());
        files.AddFile(Path.Combine(layout.ProfilesDirectory, "c-foreign.3dn"), [0x0a, 0x03, 0x33, 0x64, 0x78]);
        files.AddFile(Path.Combine(layout.ProfilesDirectory, "d-noid.3dn"), MakeProfile(null, "No identifier"));

        RecordingProfileLog log = new();
        UserProfileFolder folder = new(files, layout.ProfilesDirectory, log);
        UserProfile profile = Assert.Single(folder.Scan());

        Assert.Equal("good", profile.Id);
        Assert.True(log.AnyContains(ProfileImportMessages.NotA3dnFile));
        Assert.True(log.AnyContains("carries no identifier"));
    }

    [Fact]
    public void REQ_PROF_063_importing_a_profile_without_an_identifier_is_refused()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        const string source = @"D:\downloads\Broken.3dn";
        files.AddFile(source, MakeProfile(null));

        UserProfileFolder folder = new(files, layout.ProfilesDirectory);
        ProfileImportResult result = folder.Import(source);

        Assert.Equal(ProfileImportOutcome.Rejected, result.Outcome);
        Assert.Equal(
            $"{source}has an obsolate or wrong profile format. "
            + "This profile is not compatible with the current version of Zaxnes.",
            result.Message);
        Assert.False(files.FileExists(Path.Combine(layout.ProfilesDirectory, "Broken.3dn")));
    }

    [Fact]
    public void REQ_PROF_063_importing_a_new_identifier_copies_the_file_and_adds_a_game()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        const string source = @"D:\downloads\Mine.3dn";
        files.AddFile(source, MakeProfile("mine01", "My Game"));

        UserProfileFolder folder = new(files, layout.ProfilesDirectory);
        ProfileImportResult result = folder.Import(source, ["1", "47"]);

        Assert.Equal(ProfileImportOutcome.Added, result.Outcome);
        Assert.Equal("My Game", result.Profile!.Title);
        Assert.True(files.FileExists(Path.Combine(layout.ProfilesDirectory, "Mine.3dn")));

        // Importing the same file twice creates one game, not two.
        IReadOnlyList<UserProfile> known = folder.Scan();
        ProfileImportResult again = folder.Import(source, ["1", "47"], known);
        Assert.Equal(ProfileImportOutcome.Replaced, again.Outcome);
        Assert.Single(folder.Scan());
    }

    [Fact]
    public void REQ_PROF_063_importing_over_a_user_profile_replaces_its_file()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        string installed = Path.Combine(layout.ProfilesDirectory, "Old.3dn");
        files.AddFile(installed, MakeProfile("same", "Old title"));

        const string source = @"D:\downloads\New.3dn";
        files.AddFile(source, MakeProfile("same", "New title"));

        UserProfileFolder folder = new(files, layout.ProfilesDirectory);
        IReadOnlyList<UserProfile> known = folder.Scan();
        ProfileImportResult result = folder.Import(source, [], known);

        Assert.Equal(ProfileImportOutcome.Replaced, result.Outcome);
        Assert.False(files.FileExists(installed));
        Assert.True(files.FileExists(Path.Combine(layout.ProfilesDirectory, "New.3dn")));
        Assert.Equal("New title", Assert.Single(folder.Scan()).Title);
    }

    [Fact]
    public void REQ_PROF_063_importing_the_identifier_of_a_built_in_profile_does_nothing()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        const string source = @"D:\downloads\Clash.3dn";
        files.AddFile(source, MakeProfile("1", "Not Arkanoid"));

        UserProfileFolder folder = new(files, layout.ProfilesDirectory);
        ProfileImportResult result = folder.Import(source, ["1"]);

        Assert.Equal(ProfileImportOutcome.AlreadyBuiltIn, result.Outcome);
        Assert.Empty(files.ListFiles(layout.ProfilesDirectory, UserProfileFolder.Extension));
    }

    [Fact]
    public void REQ_PROF_063_importing_a_file_that_is_not_a_profile_is_refused()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        const string source = @"D:\downloads\Foreign.3dn";
        files.AddFile(source, [0x0a, 0x03, 0x33, 0x64, 0x78]);

        UserProfileFolder folder = new(files, layout.ProfilesDirectory);
        ProfileImportResult result = folder.Import(source);

        Assert.Equal(ProfileImportOutcome.Rejected, result.Outcome);
        Assert.Equal(ProfileImportMessages.NotA3dnFile, result.Message);
    }

    [Fact]
    public void REQ_PROF_064_the_library_lists_every_profile_and_remembers_the_chosen_rom()
    {
        FakeProfileFileSystem files = new();
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(
                ("1", "Arkanoid", "profiles/ark.bytes"),
                ("47", "2D Mode", "profiles/empty.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        GameLibrary library = GameLibrary.Build(catalogue);

        Assert.Equal(2, library.Entries.Count);
        Assert.False(library.Entries[0].HasRom);
        Assert.Equal(0, library.Entries[0].LastPlayed);

        Assert.True(library.SetRomPath("1", @"D:\roms\test.nes"));
        Assert.True(library.MarkPlayed("1", 1760950000));
        Assert.False(library.SetRomPath("nope", @"D:\roms\test.nes"));

        GameLibraryEntry arkanoid = library.Find("1")!;
        Assert.Equal(@"D:\roms\test.nes", arkanoid.RomPath);
        Assert.Equal(1760950000, arkanoid.LastPlayed);
        Assert.Equal(GameProfileSource.BuiltIn, arkanoid.Source);
        Assert.False(arkanoid.CanDelete);
    }

    [Fact]
    public void REQ_PROF_061_the_2D_Mode_entry_keeps_its_name_whatever_the_custom_title()
    {
        FakeProfileFileSystem files = new();
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(("47", "2D Mode", "profiles/empty.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        GameLibrary library = GameLibrary.Build(catalogue);
        GameLibraryEntry twoD = library.Find("47")!;
        twoD.Title = "My 2D games";

        Assert.True(twoD.Is2DMode);
        Assert.Equal("2D Mode", twoD.DisplayTitle);
        Assert.False(twoD.CanDelete);
    }

    [Fact]
    public void REQ_PROF_065_only_a_game_from_a_user_profile_can_be_removed()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(("1", "Arkanoid", "profiles/ark.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        string userFile = Path.Combine(layout.ProfilesDirectory, "Mine.3dn");
        files.AddFile(userFile, MakeProfile("mine01", "Mine"));
        UserProfileFolder folder = new(files, layout.ProfilesDirectory);
        IReadOnlyList<UserProfile> userProfiles = folder.Scan(catalogue.Ids);

        GameLibrary library = GameLibrary.Build(catalogue, userProfiles);

        Assert.False(library.Remove("1"));
        Assert.True(library.Remove("mine01"));
        Assert.Single(library.Entries);

        folder.Delete(userProfiles[0]);
        Assert.False(files.FileExists(userFile));
    }

    [Fact]
    public void REQ_PERS_030_a_built_in_profile_wins_over_a_user_profile_of_the_same_identifier()
    {
        FakeProfileFileSystem files = new();
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(("1", "Arkanoid", "profiles/ark.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        RecordingProfileLog log = new();
        GameLibrary library = GameLibrary.Build(
            catalogue,
            [new UserProfile("1", "Mine", @"C:\x\mine.3dn", default)],
            persisted: null,
            log);

        GameLibraryEntry entry = Assert.Single(library.Entries);
        Assert.Equal("Arkanoid", entry.Title);
        Assert.Equal(GameProfileSource.BuiltIn, entry.Source);
        Assert.True(log.AnyContains("duplicated profile identifier"));
    }

    [Fact]
    public void REQ_PERS_035_the_library_file_is_reconciled_with_the_available_profiles()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(("1", "Arkanoid", "profiles/ark.bytes"), ("2", "Balloon", "profiles/bf.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        GameLibraryStore store = new(files);
        GameLibraryFile persisted = new()
        {
            Games =
            {
                new GameLibraryRecord { Id = "1", Title = "My Arkanoid", RomPath = @"D:\a.nes", LastPlayed = 7 },
                new GameLibraryRecord { Id = "ghost", Title = "Gone", RomPath = @"D:\g.nes" },
            },
        };
        store.Save(layout.LibraryFile, persisted);

        RecordingProfileLog log = new();
        GameLibraryFile? reloaded = store.Load(layout.LibraryFile, DateTimeOffset.UnixEpoch);
        GameLibrary library = GameLibrary.Build(catalogue, [], reloaded, log);

        Assert.Equal(2, library.Entries.Count);
        Assert.Equal("My Arkanoid", library.Find("1")!.Title);
        Assert.Equal(@"D:\a.nes", library.Find("1")!.RomPath);
        Assert.Equal(7, library.Find("1")!.LastPlayed);

        // A blank entry for the profile the file did not know, and the stale one is gone.
        Assert.Equal("Balloon", library.Find("2")!.Title);
        Assert.Equal(string.Empty, library.Find("2")!.RomPath);
        Assert.Null(library.Find("ghost"));
        Assert.True(log.AnyContains("matches no available profile"));
    }

    [Fact]
    public void REQ_PROF_064_the_rom_association_survives_a_save_and_a_reload()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(("1", "Arkanoid", "profiles/ark.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        GameLibraryStore store = new(files);
        GameLibrary library = GameLibrary.Build(catalogue);
        library.SetRomPath("1", @"D:\roms\nestest.nes");
        library.MarkPlayed("1", 1760950000);
        store.Save(layout.LibraryFile, library.ToFile());

        GameLibrary reloaded = GameLibrary.Build(
            catalogue, [], store.Load(layout.LibraryFile, DateTimeOffset.UnixEpoch));

        Assert.Equal(@"D:\roms\nestest.nes", reloaded.Find("1")!.RomPath);
        Assert.Equal(1760950000, reloaded.Find("1")!.LastPlayed);
        Assert.Equal(GameLibraryFile.CurrentSchemaVersion, store.Load(layout.LibraryFile, DateTimeOffset.UnixEpoch)!.SchemaVersion);
    }

    [Fact]
    public void REQ_PERS_037_an_unreadable_library_is_set_aside_and_the_library_is_rebuilt()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);
        files.AddText(layout.LibraryFile, "{{{ not json");

        RecordingProfileLog log = new();
        GameLibraryStore store = new(files, log);
        GameLibraryFile? loaded = store.Load(layout.LibraryFile, new DateTimeOffset(2026, 10, 20, 18, 30, 5, TimeSpan.Zero));

        Assert.Null(loaded);
        Assert.True(files.FileExists($"{layout.LibraryFile}.corrupt-20261020-183005"));
        Assert.True(log.AnyContains("unreadable library"));
    }

    [Fact]
    public void REQ_PERS_025_the_rom_paths_of_an_original_rom_json_can_be_imported()
    {
        FakeProfileFileSystem files = new();
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(("1", "Arkanoid", "profiles/ark.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        const string romJson = """
            {
              "Warning": "Don't touch id parameter,  you will regret it",
              "Items": [
                { "id": "1", "romPath": "D:\\roms\\arkanoid.nes", "title": "Arkanoid", "lastTime": 1760950000 },
                { "id": "999", "romPath": "D:\\roms\\gone.nes", "title": "Gone", "lastTime": 1 }
              ]
            }
            """;
        files.AddText(Path.Combine(Root, "rom.json"), romJson);

        GameLibraryStore store = new(files);
        IReadOnlyList<RomPathRecord> records = store.LoadRomPaths(Path.Combine(Root, "rom.json"));
        GameLibrary library = GameLibrary.Build(catalogue);

        Assert.Equal(2, records.Count);
        Assert.Equal(1, library.ApplyRomPaths(records));
        Assert.Equal(@"D:\roms\arkanoid.nes", library.Find("1")!.RomPath);
        Assert.Equal(1760950000, library.Find("1")!.LastPlayed);
    }

    [Fact]
    public void REQ_PERS_033_the_library_sorts_by_title_or_by_last_played()
    {
        FakeProfileFileSystem files = new();
        files.AddText(
            Path.Combine(Assets, "profiles.json"),
            MakeCatalogueJson(
                ("1", "Zelda", "profiles/z.bytes"),
                ("2", "Arkanoid", "profiles/a.bytes"),
                ("3", "Metroid", "profiles/m.bytes")));
        ProfileCatalogue catalogue = ProfileCatalogue.Load(files, Path.Combine(Assets, "profiles.json"));

        GameLibrary library = GameLibrary.Build(catalogue);
        library.MarkPlayed("1", 300);
        library.MarkPlayed("3", 500);

        Assert.Equal(
            ["Arkanoid", "Metroid", "Zelda"],
            library.Sorted(LibrarySortOption.Alphabetical).Select(entry => entry.DisplayTitle));
        Assert.Equal(
            ["Metroid", "Zelda", "Arkanoid"],
            library.Sorted(LibrarySortOption.Recent).Select(entry => entry.DisplayTitle));
    }

    [Fact]
    public void REQ_ARCH_033_the_user_data_layout_is_built_from_an_injected_root()
    {
        FakeProfileFileSystem files = new();
        UserDataLayout layout = new(Root);

        Assert.Equal(Path.Combine(Root, "profiles"), layout.ProfilesDirectory);
        Assert.Equal(Path.Combine(Root, "library.json"), layout.LibraryFile);
        Assert.Equal(Path.Combine(Root, "covers"), layout.CoversDirectory);

        layout.EnsureDirectories(files);
        Assert.True(files.DirectoryExists(layout.ProfilesDirectory));
        Assert.True(files.DirectoryExists(layout.CoversDirectory));

        // Q-008: the original's folder name is available for whoever wants it.
        UserDataLayout original = new(Root, "CustomProfiles");
        Assert.Equal(Path.Combine(Root, "CustomProfiles"), original.ProfilesDirectory);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_ACC_010_the_whole_library_can_be_built_from_the_real_catalogue()
    {
        ProfileCatalogue catalogue = ProfileCatalogue.Load(
            PhysicalProfileFileSystem.Instance, NeutralAssets.CatalogueFile!);

        RecordingProfileLog log = new();
        GameLibrary library = GameLibrary.Build(catalogue, [], null, log);

        Assert.Equal(101, library.Entries.Count);
        Assert.Empty(log.Warnings);
        Assert.Empty(log.Errors);
        Assert.Single(library.Entries, entry => entry.Is2DMode);
    }
}
