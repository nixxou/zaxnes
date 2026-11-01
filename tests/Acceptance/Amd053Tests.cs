using System.Text.Json;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-053 — the favourites of REQ-UI-210, the search of REQ-UI-211, the import inspection of
/// REQ-UI-212 and the documentary changes of §3: the 38 slots of REQ-PERS-050, the two schema
/// migrations, and the third value of <c>ui.sort_option</c>. The resume point of REQ-LOOP-076
/// and the command line of REQ-UI-213 are driven through the real loop in
/// <c>Zax.App.Tests.Amd053Tests</c>.
/// </summary>
public sealed class Amd053Tests
{
    // ----------------------------------------------------------- REQ-UI-210

    /// <summary>
    /// REQ-UI-210, first criterion: marking a game, quitting and relaunching leaves it
    /// favourite, and <c>library.json</c> carries <c>"favorite": true</c> for it alone.
    /// </summary>
    [Fact] // REQ-UI-210
    public void REQ_UI_210_the_favourite_mark_survives_an_exit()
    {
        using TempFolder folder = new();
        string file = Path.Combine(folder.Path, "library.json");
        GameLibraryStore store = new(PhysicalProfileFileSystem.Instance);

        ProfileCatalogue catalogue = Catalogue(("1", "Arkanoid"), ("2", "Digdug"), ("3", "Tennis"));
        GameLibrary library = GameLibrary.Build(catalogue);

        Assert.True(library.ToggleFavorite("2"));
        store.Save(file, library.ToFile());

        // « Quitter, relancer » : the file is read again and the library rebuilt from it.
        string json = File.ReadAllText(file);
        Assert.Equal(1, Occurrences(json, "\"favorite\": true"));
        Assert.Equal(2, Occurrences(json, "\"favorite\": false"));

        GameLibrary reopened = GameLibrary.Build(
            catalogue,
            null,
            store.Load(file, DateTimeOffset.UnixEpoch));

        Assert.True(reopened.Find("2")!.Favorite);
        Assert.False(reopened.Find("1")!.Favorite);
        Assert.False(reopened.Find("3")!.Favorite);
    }

    /// <summary>
    /// REQ-UI-210, second criterion: two favourites out of ten entries, the filter ticked gives
    /// exactly the two; unticked, the ten.
    /// </summary>
    [Fact] // REQ-UI-210
    public void REQ_UI_210_the_filter_keeps_the_favourites_only()
    {
        GameLibrary library = GameLibrary.Build(Catalogue(
            [.. Enumerable.Range(1, 10).Select(n => (n.ToString(System.Globalization.CultureInfo.InvariantCulture), $"Game {n:00}"))]));

        library.SetFavorite("3", true);
        library.SetFavorite("7", true);

        Assert.Equal(
            ["Game 03", "Game 07"],
            library.Enumerate(LibrarySortOption.Alphabetical, favoritesOnly: true)
                .Select(static entry => entry.DisplayTitle));

        Assert.Equal(10, library.Enumerate(LibrarySortOption.Alphabetical).Count);
    }

    /// <summary>
    /// REQ-UI-210, third criterion: <c>ui.sort_option</c> = 2 gives the favourites first, then
    /// the others, each group by ascending title (REQ-PERS-033 amended); the slot lists of
    /// REQ-PERS-057 are unchanged and follow "A-Z".
    /// </summary>
    [Fact] // REQ-UI-210, REQ-PERS-033, REQ-PERS-057
    public void REQ_UI_210_favourites_first_orders_each_group_by_title()
    {
        GameLibrary library = GameLibrary.Build(Catalogue(
            ("1", "Arkanoid"), ("2", "Digdug"), ("3", "Excitebike"), ("4", "Balloon Fight")));

        library.SetFavorite("3", true);
        library.SetFavorite("4", true);

        Assert.Equal(
            ["Balloon Fight", "Excitebike", "Arkanoid", "Digdug"],
            library.Sorted(LibrarySortOption.FavoritesFirst).Select(static entry => entry.DisplayTitle));

        // The third value exists in the model and in the setting's domain.
        Assert.Equal(2, (int)LibrarySortOption.FavoritesFirst);

        AppSettings settings = new() { SortOption = 2 };
        Assert.Equal(LibrarySortOption.FavoritesFirst, settings.LibrarySortOrder);

        // REQ-PERS-057: the slot lists are without the third order; at 2 they follow "A-Z".
        Assert.Equal(LibrarySortOption.Alphabetical, settings.SlotSortOrder);
        Assert.Equal(LibrarySortOption.Recent, new AppSettings { SortOption = 1 }.SlotSortOrder);
    }

    // ----------------------------------------------------------- REQ-UI-211

    /// <summary>
    /// REQ-UI-211, first criterion: « mario » keeps the titles holding « Mario » and « MARIO »,
    /// « pokemon » keeps « Pokémon », « bros » keeps « Mario Bros. ».
    /// </summary>
    [Fact] // REQ-UI-211
    public void REQ_UI_211_the_search_ignores_case_and_diacritics()
    {
        Assert.True(LibrarySearch.Matches("Super Mario Bros.", "mario"));
        Assert.True(LibrarySearch.Matches("SUPER MARIO BROS.", "mario"));
        Assert.True(LibrarySearch.Matches("Pokémon", "pokemon"));
        Assert.True(LibrarySearch.Matches("Mario Bros.", "bros"));
        Assert.False(LibrarySearch.Matches("Arkanoid", "mario"));

        // The normalisation is the one the requirement writes: NFD, marks of category Mn
        // dropped, lower-cased with the invariant culture. Q-090: the decomposition of the
        // Latin letters comes from a table, because String.Normalize is a no-op in the
        // invariant-globalization mode this program is built in.
        Assert.True(LibrarySearch.TablesAreAligned);
        Assert.Equal("pokemon", LibrarySearch.Normalize("Pokémon"));
        Assert.Equal("ecole", LibrarySearch.Normalize("École"));
        Assert.Equal("ganbare goemon", LibrarySearch.Normalize("Ganbare Goemon"));

        // A character NFD does not decompose is left alone, exactly as NFD leaves it.
        Assert.Equal("straße", LibrarySearch.Normalize("Straße"));
        Assert.Equal("cœur", LibrarySearch.Normalize("Cœur"));
    }

    /// <summary>
    /// REQ-UI-211, second criterion: a blank field excludes nothing, and combined with the
    /// favourites filter the text keeps only the favourites whose title holds it.
    /// </summary>
    [Fact] // REQ-UI-210, REQ-UI-211
    public void REQ_UI_211_the_search_combines_with_the_favourites_filter()
    {
        GameLibrary library = GameLibrary.Build(Catalogue(
            ("1", "Super Mario Bros."), ("2", "Mario Bros."), ("3", "Arkanoid")));

        library.SetFavorite("2", true);

        Assert.Equal(3, library.Enumerate(LibrarySortOption.Alphabetical, "   ").Count);
        Assert.Equal(3, library.Enumerate(LibrarySortOption.Alphabetical, null).Count);

        Assert.Equal(
            ["Mario Bros.", "Super Mario Bros."],
            library.Enumerate(LibrarySortOption.Alphabetical, "mario")
                .Select(static entry => entry.DisplayTitle));

        Assert.Equal(
            ["Mario Bros."],
            library.Enumerate(LibrarySortOption.Alphabetical, "mario", favoritesOnly: true)
                .Select(static entry => entry.DisplayTitle));
    }

    // ----------------------------------------------------------- REQ-UI-212

    /// <summary>
    /// REQ-UI-212, first criterion: importing any text file gives the
    /// <c>&lt;path&gt; has an obsolete…</c> message — the full path, a space, then the text
    /// (AMD-056 §2) — and nothing is added to the library.
    /// </summary>
    [Fact] // REQ-UI-212, REQ-UI-084, REQ-PERS-041
    public void REQ_UI_212_a_text_file_names_the_obsolete_format()
    {
        using TempFolder folder = new();
        string path = Path.Combine(folder.Path, "notes.txt");
        File.WriteAllText(path, "This file is not a profile at all, it is a note to myself.");

        ProfileImportResult result = Folder(folder).Import(path);

        Assert.Equal(ProfileImportOutcome.Rejected, result.Outcome);
        Assert.Equal(ProfileImportMessages.ObsoleteFormat(path), result.Message);
        Assert.StartsWith($"{path} has an obsolete", result.Message, StringComparison.Ordinal);
        Assert.Null(result.Profile);
    }

    /// <summary>
    /// REQ-UI-212, second criterion: a well-formed protobuf whose <c>Type</c> is <c>"nes"</c>
    /// gives <c>This file is not a 3dn file</c>; a profile whose <c>Ver</c> is <c>"1"</c> gives
    /// <c>3dn format version 1  is not supported</c>, with its two spaces; a profile with no
    /// <c>Id</c> gives the <c>has an obsolete…</c> text.
    /// </summary>
    [Fact] // REQ-UI-212, REQ-PROF-021, REQ-PROF-033, REQ-PROF-063
    public void REQ_UI_212_the_message_follows_the_cause()
    {
        using TempFolder folder = new();
        UserProfileFolder target = Folder(folder);

        string wrongType = Write(folder, "type.3dn", profile =>
        {
            profile.Signature = "nes";
            profile.Id = "abc123def456";
        });

        Assert.Equal(ProfileImportMessages.NotA3dnFile, target.Import(wrongType).Message);
        Assert.Equal("This file is not a 3dn file", target.Import(wrongType).Message);

        string oldVersion = Write(folder, "old.3dn", profile =>
        {
            profile.VersionText = "1";
            profile.Id = "abc123def456";
        });

        Assert.Equal(
            "3dn format version 1  is not supported",
            target.Import(oldVersion).Message);

        string noId = Write(folder, "noid.3dn", profile => profile.Id = null);

        Assert.Equal(ProfileImportMessages.ObsoleteFormat(noId), target.Import(noId).Message);
    }

    /// <summary>
    /// REQ-UI-212, third criterion: a valid profile of an unknown identifier is accepted and its
    /// title and author are the ones the notification shows — <c>Title</c> and <c>Maker</c>, the
    /// title falling back to the file name without its extension (REQ-PERS-034) and an empty
    /// author being left out.
    /// </summary>
    [Fact] // REQ-UI-212, REQ-PROF-022, REQ-PERS-034
    public void REQ_UI_212_an_accepted_profile_carries_its_title_and_author()
    {
        using TempFolder folder = new();
        UserProfileFolder target = Folder(folder);

        string both = Write(folder, "both.3dn", profile =>
        {
            profile.Id = "aaaaaaaaaaaa";
            profile.Title = "Foo";
            profile.Author = "Bar";
        });

        ProfileImportResult accepted = target.Import(both);
        Assert.Equal(ProfileImportOutcome.Added, accepted.Outcome);
        Assert.Equal("Foo", accepted.Profile!.Title);
        Assert.Equal("Bar", accepted.Profile.Header.Author);

        string titleOnly = Write(folder, "title.3dn", profile =>
        {
            profile.Id = "bbbbbbbbbbbb";
            profile.Title = "Foo";
        });

        ProfileImportResult noAuthor = target.Import(titleOnly);
        Assert.Equal("Foo", noAuthor.Profile!.Title);
        Assert.True(string.IsNullOrEmpty(noAuthor.Profile.Header.Author));

        string noTitle = Write(folder, "Baz.3dn", profile => profile.Id = "cccccccccccc");

        Assert.Equal("Baz", target.Import(noTitle).Profile!.Title);
    }

    // ------------------------------------------- §3 points 4, 5 and 12: the slots

    /// <summary>
    /// REQ-PERS-050 as §3 point 4 amends it: 36 numbered slots, one quick slot and one resume
    /// slot — 38 in all — and §3 point 12: the metadata file holds exactly 38 entries at schema
    /// version 2.
    /// </summary>
    [Fact] // REQ-PERS-050, REQ-PERS-052
    public void REQ_PERS_050_there_are_thirty_eight_slots()
    {
        SaveSlotIndex index = new();

        Assert.Equal(38, index.Slots.Count);
        Assert.Equal("quick", index.Slots[36].Id);
        Assert.Equal("resume", index.Slots[37].Id);
        Assert.Equal("resume", SaveSlotIndex.ResumeSlotId);
        Assert.Equal(2, SaveSlotIndex.SchemaVersion);

        // The model holds it; the Load and Save lists and their commands do not.
        Assert.True(SaveSlotIndex.IsValidSlotId("resume"));
        Assert.False(SaveSlotIndex.IsUserSlotId("resume"));
        Assert.True(SaveSlotIndex.IsUserSlotId("quick"));
        Assert.True(SaveSlotIndex.IsUserSlotId("36"));
        Assert.False(SaveSlotIndex.IsUserSlotId("37"));
    }

    /// <summary>
    /// §3 point 12: a <c>saves/index.json</c> at schema version 1 is migrated at read time by
    /// the addition of an empty <c>resume</c> entry, a <c>.v1.bak</c> copy being kept
    /// (REQ-PERS-024).
    /// </summary>
    [Fact] // REQ-PERS-024
    public void The_slot_index_is_migrated_from_schema_one()
    {
        using TempFolder folder = new();
        UserDataPaths paths = UserDataPaths.Resolve(folder.Path);

        File.WriteAllText(
            paths.SaveIndexFile,
            """
            {
              "schemaVersion": 1,
              "slots": [
                { "id": "1", "gameId": "10", "savedAt": 1760950100, "file": "1.state" },
                { "id": "quick", "gameId": "", "savedAt": 0 }
              ]
            }
            """);

        SaveSlotIndex index = SaveSlotIndex.Load(paths, DateTimeOffset.UnixEpoch);

        Assert.Equal(1, index.MigratedFromVersion);
        Assert.True(File.Exists(paths.SaveIndexFile + ".v1.bak"));
        Assert.Equal("10", index.Find("1")!.GameId);
        Assert.False(index.Find("resume")!.IsOccupied);

        index.Save(paths);

        using JsonDocument written = JsonDocument.Parse(File.ReadAllText(paths.SaveIndexFile));
        Assert.Equal(2, written.RootElement.GetProperty("schemaVersion").GetInt32());
        Assert.Equal(38, written.RootElement.GetProperty("slots").GetArrayLength());
    }

    /// <summary>
    /// §3 point 11: <c>library.json</c> goes to schema version 2 with the <c>favorite</c> field;
    /// a version-1 file is migrated at read time with <c>favorite</c> false everywhere and the
    /// copy <c>library.json.v1.bak</c> is kept (REQ-PERS-024, REQ-PERS-088).
    /// </summary>
    [Fact] // REQ-PERS-024, REQ-PERS-088
    public void The_library_file_is_migrated_from_schema_one()
    {
        using TempFolder folder = new();
        string file = Path.Combine(folder.Path, "library.json");

        File.WriteAllText(
            file,
            """
            {
              "schemaVersion": 1,
              "notice": "Do not edit the id field.",
              "games": [
                { "id": "1", "title": "Arkanoid", "romPath": "", "lastPlayed": 0 }
              ]
            }
            """);

        GameLibraryStore store = new(PhysicalProfileFileSystem.Instance);
        GameLibraryFile? read = store.Load(file, DateTimeOffset.UnixEpoch);

        Assert.Equal(2, GameLibraryFile.CurrentSchemaVersion);
        Assert.Equal(1, store.MigratedFromVersion);
        Assert.True(File.Exists(file + ".v1.bak"));
        Assert.False(read!.Games[0].Favorite);

        store.Save(file, read);

        using JsonDocument written = JsonDocument.Parse(File.ReadAllText(file));
        Assert.Equal(2, written.RootElement.GetProperty("schemaVersion").GetInt32());
        Assert.False(written.RootElement
            .GetProperty("games")[0]
            .GetProperty("favorite")
            .GetBoolean());
    }

    // ------------------------------------------- §3 point 10: the settings row

    /// <summary>
    /// §3 point 10: the row of <c>ui.sort_option</c> gains the value 2 « Favorites first », the
    /// rest of the row being unchanged; the value 2 is never written into profile field 36 — 0
    /// goes in its place — and a 2 read from a profile is ignored like every value of user
    /// scope (REQ-PERS-005 point 2).
    /// </summary>
    [Fact] // REQ-PERS-033, REQ-UI-014
    public void The_sort_option_row_gains_the_third_value()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(SettingKeys.UiSortOption);

        Assert.Equal(SettingKind.Enum, descriptor.Kind);
        Assert.Equal(SettingScope.User, descriptor.Scope);
        Assert.Equal("36", descriptor.ProfileField);
        Assert.Equal("SortOption", descriptor.LuaPath);
        Assert.Equal(0, descriptor.DefaultValue.AsInt);

        Assert.Equal(
            ["A-Z", "Recent", "Favorites first"],
            descriptor.Domain.Options!.Select(static option => option.Label));

        // The value 2 never reaches profile field 36.
        Assert.Equal(0, new AppSettings { SortOption = 2 }.ProfileSortOption);
        Assert.Equal(1, new AppSettings { SortOption = 1 }.ProfileSortOption);

        // And a value read from a profile is ignored, the key being one a profile leaves alone.
        Assert.Contains(SettingKeys.UiSortOption, ProfileSettingApplication.PreservedKeys);
        Assert.DoesNotContain(SettingKeys.UiSortOption, ProfileSettingApplication.AppliedKeys);
    }

    // ------------------------------------------------------------- helpers

    private static int Occurrences(string text, string needle)
    {
        int count = 0;
        for (int at = text.IndexOf(needle, StringComparison.Ordinal);
            at >= 0;
            at = text.IndexOf(needle, at + needle.Length, StringComparison.Ordinal))
        {
            count++;
        }

        return count;
    }

    private static UserProfileFolder Folder(TempFolder folder)
        => new(PhysicalProfileFileSystem.Instance, Path.Combine(folder.Path, "profiles"));

    private static string Write(TempFolder folder, string name, Action<GameProfile> edit)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        edit(profile);
        string path = Path.Combine(folder.Path, name);
        File.WriteAllBytes(path, ProfileFormat.Save(profile));
        return path;
    }

    private static ProfileCatalogue Catalogue(params (string Id, string Title)[] entries)
    {
        string json = "{\"editions\":{\"full\":["
            + string.Join(
                ',',
                entries.Select(static entry =>
                    $"{{\"id\":\"{entry.Id}\",\"title\":\"{entry.Title}\","
                    + $"\"file\":\"profiles/{entry.Id}.bytes\"}}"))
            + "]}}";

        return ProfileCatalogue.Parse(json, @"X:\fake\assets-neutral", ProfileCatalogue.FullEdition);
    }

    /// <summary>A throw-away folder, so no file of these tests outlives the run.</summary>
    private sealed class TempFolder : IDisposable
    {
        public TempFolder()
        {
            Path = System.IO.Path.Combine(
                System.IO.Path.GetTempPath(),
                "zax-amd053-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path);
            Directory.CreateDirectory(System.IO.Path.Combine(Path, "profiles"));
        }

        public string Path { get; }

        public void Dispose()
        {
            try
            {
                Directory.Delete(Path, recursive: true);
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A locked temporary folder must not fail a test run.
            }
        }
    }
}
