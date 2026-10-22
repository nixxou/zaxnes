using System.Text.Json;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Profiles.Library;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// SPEC-09 §3.2, §3.3, §3.4, §3.5, §3.6, §3.7 and §3.8: the settings file, the library, the user
/// profiles, the slots, the battery RAM, the bindings and the layout of the user root.
/// </summary>
public sealed class PersistenceStoreTests
{
    // ------------------------------------------------------------- §3.2 settings file

    /// <summary>
    /// REQ-PERS-020: the U settings and <c>ui.fullscreen</c> are persisted, indexed by stable
    /// key; a P or S setting present in the file is ignored, not an error.
    /// </summary>
    [Fact]
    public void REQ_PERS_020_only_the_user_preferences_are_persisted()
    {
        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, Now);

        settings.Fov = 85f;
        settings.Volume = 0.5f;
        settings.Skybox = 1;
        settings.TvMode = TvRegionSetting.Pal;
        settings.Fullscreen = true;
        store.Save(settings);

        string json = File.ReadAllText(root.Paths.SettingsFile);
        Assert.Contains("\"camera.fov\": 85", json, StringComparison.Ordinal);
        Assert.Contains("\"audio.volume\"", json, StringComparison.Ordinal);
        Assert.Contains("\"scene.skybox\"", json, StringComparison.Ordinal);
        Assert.Contains("\"ui.fullscreen\": true", json, StringComparison.Ordinal);

        // emu.tv_mode is profile-scoped (DEC-09-Q1) and must not be needed at start-up.
        Assert.DoesNotContain("\"emu.tv_mode\"", json, StringComparison.Ordinal);

        AppSettings second = new();
        new SettingsStore(root.Paths).Load(second, Now);
        Assert.Equal(85f, second.Fov);
        Assert.Equal(0.5f, second.Volume);
        Assert.Equal(1, second.Skybox);
        Assert.True(second.Fullscreen);

        // A P value in the file is ignored without an error.
        File.WriteAllText(
            root.Paths.SettingsFile,
            """{ "schemaVersion": 1, "values": { "conv.empty_tile": true, "camera.fov": 60 } }""");

        AppSettings third = new();
        SettingsStore ignoring = new(root.Paths);
        ignoring.Load(third, Now);
        Assert.Null(ignoring.Message);
        Assert.Equal(60f, third.Fov);
        Assert.False(third.Model.GetBool(SettingKeys.ConvEmptyTile));
    }

    /// <summary>
    /// REQ-PERS-021: the file is written at the exit and, at the latest two seconds after a
    /// change from the UI; every write is atomic — a temporary file then a replace.
    /// </summary>
    [Fact]
    public void REQ_PERS_021_a_change_reaches_the_disk_within_two_seconds()
    {
        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, Now);

        Assert.Equal(TimeSpan.FromSeconds(2.0), SettingsStore.WriteDelay);

        settings.Volume = 0.25f;
        Assert.True(store.HasPendingWrite);

        store.Tick(settings, TimeSpan.FromSeconds(1.0));
        Assert.False(File.Exists(root.Paths.SettingsFile));

        store.Tick(settings, TimeSpan.FromSeconds(3.1));
        Assert.True(File.Exists(root.Paths.SettingsFile));
        Assert.False(store.HasPendingWrite);

        // No temporary file is left behind, and the result is complete JSON.
        Assert.Empty(Directory.GetFiles(root.Paths.Root, "*.tmp"));
        using JsonDocument document = JsonDocument.Parse(File.ReadAllText(root.Paths.SettingsFile));
        Assert.Equal(1, document.RootElement.GetProperty("schemaVersion").GetInt32());
    }

    /// <summary>
    /// REQ-PERS-022: an unreadable settings file starts the program on the defaults, is renamed
    /// <c>settings.json.corrupt-&lt;stamp&gt;</c> and asks for the exact notification; a single
    /// invalid value is replaced on its own, silently.
    /// </summary>
    [Fact]
    public void REQ_PERS_022_a_corrupt_settings_file_is_set_aside_with_its_notification()
    {
        using TemporaryRoot root = new();
        File.WriteAllText(root.Paths.SettingsFile, "{{{");

        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, Now);

        Assert.Equal("Settings file was unreadable and has been reset.", store.Message);
        Assert.Equal(70f, settings.Fov);
        Assert.NotEmpty(Directory.GetFiles(root.Paths.Root, "settings.json.corrupt-*"));

        // A single value of the wrong type: that value alone goes back to its default, with no
        // notification and no loss of the others.
        File.WriteAllText(
            root.Paths.SettingsFile,
            """{ "schemaVersion": 1, "values": { "camera.fov": "abc", "audio.volume": 0.25 } }""");

        AppSettings second = new();
        SettingsStore silent = new(root.Paths);
        silent.Load(second, Now);

        Assert.Null(silent.Message);
        Assert.Equal(70f, second.Fov);
        Assert.Equal(0.25f, second.Volume);
    }

    /// <summary>REQ-PERS-023: an unknown key survives a rewrite untouched.</summary>
    [Fact]
    public void REQ_PERS_023_an_unknown_key_survives_a_rewrite()
    {
        using TemporaryRoot root = new();
        File.WriteAllText(
            root.Paths.SettingsFile,
            """{ "schemaVersion": 1, "values": { "future.x": 3, "vr.runtime": 2 } }""");

        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, Now);

        settings.Volume = 0.5f;
        store.Save(settings);

        string json = File.ReadAllText(root.Paths.SettingsFile);
        Assert.Contains("\"future.x\": 3", json, StringComparison.Ordinal);
    }

    /// <summary>
    /// REQ-PERS-024: every persistent file carries a <c>schemaVersion</c>; an older file is
    /// migrated after a <c>.v&lt;N&gt;.bak</c> copy, and a newer one is read at best and left
    /// alone for the session, with its notification.
    /// </summary>
    [Fact]
    public void REQ_PERS_024_the_schema_version_drives_the_migration()
    {
        using TemporaryRoot root = new();
        UserDataSettingsStorage storage = new(root.Paths);

        // An older file: a copy is kept and the file is rewritten at the current version.
        File.WriteAllText(
            root.Paths.SettingsFile,
            """{ "schemaVersion": 0, "values": { "camera.fov": 60 } }""");

        SettingsModel model = new();
        SettingsFile file = new(storage, SettingsFile.DefaultName, []);
        SettingsLoadResult older = file.Load(model, Now);

        Assert.Equal(SettingsLoadStatus.Migrated, older.Status);
        Assert.True(File.Exists(root.Paths.SettingsFile + ".v0.bak"));
        file.Save(model);
        Assert.Contains(
            "\"schemaVersion\": 1",
            File.ReadAllText(root.Paths.SettingsFile),
            StringComparison.Ordinal);

        // A newer file: the known keys are applied, nothing is written back.
        File.WriteAllText(
            root.Paths.SettingsFile,
            """{ "schemaVersion": 99, "values": { "camera.fov": 55 } }""");

        SettingsModel newer = new();
        SettingsFile reader = new(storage, SettingsFile.DefaultName, []);
        SettingsLoadResult result = reader.Load(newer, Now);

        Assert.Equal(SettingsLoadStatus.NewerVersion, result.Status);
        Assert.Equal("Settings were created by a newer version.", result.Message);
        Assert.Equal(55f, newer.GetFloat(SettingKeys.CameraFov));
        Assert.False(reader.Save(newer));
        Assert.Contains(
            "\"schemaVersion\": 99",
            File.ReadAllText(root.Paths.SettingsFile),
            StringComparison.Ordinal);

        // The other three persistent files carry one too.
        Assert.Equal(1, SaveSlotIndex.SchemaVersion);
        Assert.Equal(1, InputBindingStore.SchemaVersion);
        Assert.Equal(1, GameLibraryFile.CurrentSchemaVersion);
    }

    /// <summary>
    /// REQ-PERS-025: reading the original's files is not required; the optional <c>rom.json</c>
    /// import, when offered, associates the paths with the right games.
    /// </summary>
    [Fact]
    public void REQ_PERS_025_the_optional_rom_json_import_maps_the_right_games()
    {
        IReadOnlyList<RomPathRecord> records = GameLibraryStore.ParseRomPaths(
            """
            { "Warning": "Don't touch id parameter,  you will regret it",
              "Items": [ { "id": "10", "romPath": "D:\\roms\\digdug.nes",
                           "title": "Digdug", "lastTime": 1760950000 } ] }
            """);

        Assert.Single(records);
        Assert.Equal("10", records[0].Id);
        Assert.Equal("D:\\roms\\digdug.nes", records[0].RomPath);
        Assert.Equal(1760950000L, records[0].LastTime);

        // None of the original's binary files is required to start.
        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, Now);
        Assert.Null(store.Message);
    }

    // ------------------------------------------------------------- §3.3 library

    /// <summary>
    /// REQ-PERS-030: one entry per available profile, keyed by profile identifier, and a user
    /// profile whose identifier belongs to a built-in one is ignored.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_030_one_entry_per_profile_and_the_built_in_one_wins()
    {
        using HostDriver probe = HostDriver.Create();
        string builtIn = probe.Library.Entries[0].Id;
        int count = probe.Library.Entries.Count;

        GameProfile clash = GameProfile.CreateEmpty();
        clash.Title = "impostor";
        clash.Id = builtIn;

        using HostDriver host = HostDriver.Create(clash);
        Assert.Equal(count, host.Library.Entries.Count);

        GameLibraryEntry entry = host.Library.Find(builtIn)!;
        Assert.Equal(GameProfileSource.BuiltIn, entry.Source);
        Assert.NotEqual("impostor", entry.Title);

        // Every identifier appears once.
        Assert.Equal(
            host.Library.Entries.Count,
            host.Library.Entries.Select(static e => e.Id).Distinct(StringComparer.Ordinal).Count());
    }

    /// <summary>
    /// REQ-PERS-031: a game remembers its ROM path, its title, its last-played date and its
    /// cover — and nothing else. No camera, speed, region or Lua data is kept per game.
    /// </summary>
    [Fact]
    public void REQ_PERS_031_only_four_things_are_remembered_per_game()
    {
        string[] fields = [.. typeof(GameLibraryRecord)
            .GetProperties()
            .Select(static property => property.Name)];

        Assert.Equal<IEnumerable<string>>(
            ["Id", "Title", "RomPath", "LastPlayed", "Cover"],
            fields);
    }

    /// <summary>
    /// REQ-PERS-031 and REQ-PERS-032: the launch of a game does not remember its camera, and the
    /// chosen ROM path is persisted immediately.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_032_the_chosen_rom_is_persisted_at_once()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        string rom = host.WriteRom(LoopRom.Counter());

        host.SetRomPath(id, Path.GetFullPath(rom));

        Assert.True(File.Exists(host.Paths.LibraryFile));
        Assert.Contains(
            Path.GetFileName(rom),
            File.ReadAllText(host.Paths.LibraryFile),
            StringComparison.Ordinal);

        // Launching turns the camera; a relaunch starts from the default view again.
        host.Run(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        System.Numerics.Vector3 start = host.Scene.Camera.Current.Angles;
        host.Scene.Camera.Current.Angles = new System.Numerics.Vector3(30f, 30f, 0f);
        host.Run(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        Assert.Equal(start, host.Scene.Camera.Current.Angles);
    }

    /// <summary>
    /// REQ-PERS-033 and SPEC-10 V3: <c>lastPlayed</c> is stamped at every launch, and the two
    /// sort orders are the documented ones.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_033_the_launch_date_feeds_the_recent_sort()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        host.Now = DateTimeOffset.FromUnixTimeSeconds(1_761_000_123);
        host.Launch(id, host.WriteRom(LoopRom.Counter()));

        Assert.Equal(1_761_000_123L, host.Library.Find(id)!.LastPlayed);

        // The two orders, on the vector of SPEC-10 §5 V3.
        GameLibrary library = BuildSortLibrary();
        Assert.Equal<IEnumerable<string>>(
            ["Arkanoid", "Contra", "Mario", "Zelda"],
            [.. library.Sorted(LibrarySortOption.Alphabetical).Select(static e => e.DisplayTitle)]);

        Assert.Equal<IEnumerable<string>>(
            ["Mario", "Zelda", "Arkanoid", "Contra"],
            [.. library.Sorted(LibrarySortOption.Recent).Select(static e => e.DisplayTitle)]);
    }

    /// <summary>
    /// REQ-PERS-034: a title the user edited in the file is respected; an empty one falls back on
    /// the profile's title, and the identifier is not editable.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_034_an_edited_title_is_respected_and_an_empty_one_falls_back()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verify-title";
        profile.Scripts.Add(new ProfileScript("s", true, "-- none", "*"));

        using HostDriver first = HostDriver.Create(profile);
        string id = first.EntryIdByTitle("verify-title");
        first.Library.Find(id)!.Title = "Renamed";
        first.SaveLibrary();

        string library = File.ReadAllText(first.Paths.LibraryFile);
        Assert.Contains("\"title\": \"Renamed\"", library, StringComparison.Ordinal);
        Assert.Contains("Do not edit the id field.", library, StringComparison.Ordinal);

        // An empty title falls back on the profile's own.
        File.WriteAllText(
            first.Paths.LibraryFile,
            library.Replace("\"title\": \"Renamed\"", "\"title\": \"\"", StringComparison.Ordinal));

        GameLibraryStore store = new(Zax.Profiles.Catalogue.PhysicalProfileFileSystem.Instance);
        GameLibraryFile? read = store.Load(first.Paths.LibraryFile, Now);
        Assert.NotNull(read);
        Assert.Contains(read!.Games, record => record.Id == id && string.IsNullOrEmpty(record.Title));

        // GameLibraryEntry.Id has no setter: an identifier cannot be changed through the model.
        Assert.Null(typeof(GameLibraryEntry).GetProperty("Id")!.SetMethod);
    }

    /// <summary>
    /// REQ-PERS-035: an entry whose identifier names no available profile is dropped at start-up,
    /// and a blank entry is created for every new profile.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_035_the_library_is_reconciled_at_startup()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;

        File.WriteAllText(
            host.Paths.LibraryFile,
            $$"""
            { "schemaVersion": 1, "games": [
              { "id": "{{id}}", "title": "kept", "romPath": "x.nes", "lastPlayed": 7 },
              { "id": "no-such-profile", "title": "gone", "romPath": "y.nes", "lastPlayed": 9 } ] }
            """);

        using HostDriver reopened = HostDriver.Create();
        Directory.CreateDirectory(Path.GetDirectoryName(reopened.Paths.LibraryFile)!);
        File.Copy(host.Paths.LibraryFile, reopened.Paths.LibraryFile, overwrite: true);

        // The library of a fresh host, built over that file, keeps only the known identifier.
        GameLibraryStore store = new(Zax.Profiles.Catalogue.PhysicalProfileFileSystem.Instance);
        GameLibraryFile persisted = store.Load(reopened.Paths.LibraryFile, Now)!;
        Assert.Equal(2, persisted.Games.Count);

        using HostDriver after = HostDriver.Create();
        File.Copy(reopened.Paths.LibraryFile, after.Paths.LibraryFile, overwrite: true);
        after.SaveLibrary();

        GameLibraryFile written = store.Load(after.Paths.LibraryFile, Now)!;
        Assert.DoesNotContain(written.Games, record => record.Id == "no-such-profile");
        Assert.Contains(written.Games, record => record.Id == id);

        // Every available profile has an entry, blank when it was never played.
        Assert.All(after.Library.Entries, entry => Assert.NotNull(entry.Id));
    }

    /// <summary>
    /// REQ-PERS-036: the cover art of a game — an optional PNG per entry, shrunk to fit
    /// 256 × 192 keeping its proportions, without an alpha channel, under a name derived
    /// deterministically from the title and the identifier. The criterion: a 1024 × 1024 import
    /// gives a 192 × 192 PNG in the covers folder.
    /// </summary>
    /// <remarks>
    /// Round 1: the <c>cover</c> field made the round trip through <c>library.json</c> but nothing
    /// imported, reduced, named or captured an image, so this test only probed for the absence of
    /// any such route (<c>Q-050</c>). AMD-020 answers <c>Q-050</c> by bringing the requirement
    /// down to <b>P3</b> — without the tile library the import and the capture are not required in
    /// v1 — and the <c>R1-fixes</c> batch implements both anyway. The test is therefore rewritten
    /// against the criterion itself. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-PERS-036 — AMD-020 (Q-050)
    public void REQ_PERS_036_a_cover_can_be_imported_and_is_reduced_to_the_documented_size()
    {
        using HostDriver host = HostDriver.Create();

        // The data half: the field exists and survives a round trip.
        string id = host.TwoDimensionalModeId!;
        host.Library.Find(id)!.Cover = "cover.png";
        host.SaveLibrary();
        Assert.Contains(
            "\"cover\": \"cover.png\"",
            File.ReadAllText(host.Paths.LibraryFile),
            StringComparison.Ordinal);

        host.Library.Find(id)!.Cover = null;
        host.Launch2DMode();

        // The behaviour half, on the criterion's own numbers: 1024 × 1024 in, 192 × 192 out.
        string source = WriteSquarePng(host.Root, 1024);
        host.Run(ZaxCommands.ImportCover, CommandArgs.Of(source));

        string file = Assert.Single(Directory.GetFiles(host.Paths.CoversDirectory, "*.png"));
        Zax.Render.Assets.ImageData written = Zax.Render.Assets.PngImage.Load(file);
        Assert.Equal(192, written.Width);
        Assert.Equal(192, written.Height);

        // The name is derived from the title and the identifier (REQ-PERS-074's rule).
        string name = Path.GetFileName(file);
        Assert.Contains("_3dsen_", name, StringComparison.Ordinal);
        Assert.Contains(id, name, StringComparison.Ordinal);
        Assert.Equal(name, host.Library.Find(id)!.Cover);

        // Second source: the pause capture of « Create game art ».
        File.Delete(file);
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Run(ZaxCommands.CreateGameArt);
        Zax.Render.Assets.ImageData captured = Zax.Render.Assets.PngImage.Load(
            Assert.Single(Directory.GetFiles(host.Paths.CoversDirectory, "*.png")));
        Assert.Equal(256, captured.Width);
        Assert.Equal(192, captured.Height);
    }

    /// <summary>
    /// A square opaque PNG of <paramref name="side"/> pixels, written into
    /// <paramref name="directory"/>; the source image of the REQ-PERS-036 criterion.
    /// </summary>
    private static string WriteSquarePng(string directory, int side)
    {
        byte[] pixels = new byte[side * side * 3];
        for (int index = 0; index < pixels.Length; index++)
        {
            pixels[index] = (byte)(index % 251);
        }

        string path = Path.Combine(directory, $"source-{side}.png");
        Zax.Render.Assets.PngImage.WriteRgb(path, side, side, pixels);
        return path;
    }

    /// <summary>
    /// REQ-PERS-037: the library is written after a ROM choice and at the exit, and an unreadable
    /// file is rebuilt from the profiles with a notification.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_037_a_corrupt_library_is_rebuilt_from_the_profiles()
    {
        using HostDriver host = HostDriver.Create();
        File.WriteAllText(host.Paths.LibraryFile, "not json at all");

        GameLibraryStore store = new(Zax.Profiles.Catalogue.PhysicalProfileFileSystem.Instance);
        Assert.Null(store.Load(host.Paths.LibraryFile, Now));
        Assert.NotEmpty(Directory.GetFiles(host.Paths.Root, "library.json.corrupt-*"));

        using HostDriver rebuilt = HostDriver.Create();
        Assert.NotEmpty(rebuilt.Library.Entries);
        Assert.All(rebuilt.Library.Entries, entry => Assert.False(entry.HasRom));
    }

    // ------------------------------------------------------- §3.4 user profiles

    /// <summary>
    /// REQ-PERS-040: the user profile folder is created if missing and scanned for
    /// <c>*.3dn</c>; an unreadable or obsolete file is ignored without stopping the start-up.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_040_the_profile_folder_is_scanned_and_bad_files_are_ignored()
    {
        GameProfile valid = GameProfile.CreateEmpty();
        valid.Title = "verify-valid";
        valid.Scripts.Add(new ProfileScript("s", true, "-- none", "*"));

        using HostDriver host = HostDriver.Create(valid);
        Assert.True(Directory.Exists(host.Paths.ProfilesDirectory));

        // A truncated file beside the valid one.
        byte[] good = File.ReadAllBytes(Directory.GetFiles(host.Paths.ProfilesDirectory, "*.3dn")[0]);
        File.WriteAllBytes(
            Path.Combine(host.Paths.ProfilesDirectory, "truncated.3dn"),
            good[..(good.Length / 3)]);

        using HostDriver reopened = HostDriver.Create();
        foreach (string file in Directory.GetFiles(host.Paths.ProfilesDirectory, "*.3dn"))
        {
            File.Copy(file, Path.Combine(reopened.Paths.ProfilesDirectory, Path.GetFileName(file)), true);
        }

        using HostDriver scanned = HostDriver.Create();
        foreach (string file in Directory.GetFiles(host.Paths.ProfilesDirectory, "*.3dn"))
        {
            File.Copy(file, Path.Combine(scanned.Paths.ProfilesDirectory, Path.GetFileName(file)), true);
        }

        // The scan runs at construction, so a fresh driver over the same folder is the check.
        Zax.Profiles.Catalogue.UserProfileFolder folder = new(
            Zax.Profiles.Catalogue.PhysicalProfileFileSystem.Instance,
            host.Paths.ProfilesDirectory);

        IReadOnlyList<Zax.Profiles.Catalogue.UserProfile> found = folder.Scan();
        Assert.Single(found);
        Assert.Equal("verify-valid", found[0].Title);
    }

    /// <summary>
    /// REQ-PERS-041: the four cases of an import — a bad file with the exact text of
    /// REQ-UI-084, a new identifier, an existing user identifier that keeps its library entry,
    /// and a built-in identifier that changes nothing.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_041_the_four_import_cases()
    {
        using HostDriver host = HostDriver.Create();

        // 1. An unreadable file: the exact message of REQ-UI-084, path glued to "has".
        string bad = Path.Combine(host.Root, "not-a-profile.3dn");
        File.WriteAllText(bad, "this is plain text");
        (string outcome, string message) = host.ImportProfile(bad);
        Assert.Equal("Rejected", outcome);
        Assert.Equal(
            $"{bad}has an obsolate or wrong profile format. "
            + "This profile is not compatible with the current version of Zaxnes.",
            message);

        // 2. An unknown identifier: the file is copied and an entry appears.
        GameProfile fresh = GameProfile.CreateEmpty();
        fresh.Title = "verify-import";
        fresh.Scripts.Add(new ProfileScript("s", true, "-- none", "*"));
        ProfileExport.Normalize(fresh);
        string source = Path.Combine(host.Root, "verify-import.3dn");
        File.WriteAllBytes(source, ProfileFormat.Save(fresh));

        (outcome, message) = host.ImportProfile(source);
        Assert.Equal("Added", outcome);
        Assert.Equal(string.Empty, message);
        Assert.True(File.Exists(Path.Combine(host.Paths.ProfilesDirectory, "verify-import.3dn")));

        string id = host.EntryIdByTitle("verify-import");
        host.SetRomPath(id, Path.GetFullPath(host.WriteRom(LoopRom.Counter())));
        string romPath = host.Library.Find(id)!.RomPath;

        // 3. The same identifier again: one entry, the ROM path kept, the content replaced.
        fresh.Description = "second version";
        File.WriteAllBytes(source, ProfileFormat.Save(fresh));
        (outcome, _) = host.ImportProfile(source);
        Assert.Equal("Replaced", outcome);
        Assert.Single(host.Library.Entries, entry => entry.Id == id);
        Assert.Equal(romPath, host.Library.Find(id)!.RomPath);

        // 4. A built-in identifier: nothing changes.
        GameProfile clash = GameProfile.CreateEmpty();
        clash.Title = "impostor";
        clash.Id = host.Library.Entries.First(static e => e.Source == GameProfileSource.BuiltIn).Id;
        ProfileExport.Normalize(clash);
        string clashPath = Path.Combine(host.Root, "impostor.3dn");
        File.WriteAllBytes(clashPath, ProfileFormat.Save(clash));

        int before = host.Library.Entries.Count;
        (outcome, message) = host.ImportProfile(clashPath);
        Assert.Equal("AlreadyBuiltIn", outcome);
        Assert.Equal(string.Empty, message);
        Assert.Equal(before, host.Library.Entries.Count);
    }

    /// <summary>
    /// REQ-PERS-042: deleting a user game stops the emulation if it is the current one, empties
    /// every slot of that game — the quick slot included — and deletes the profile file, the
    /// cover and the library entry. A built-in profile is never deletable.
    /// </summary>
    /// <remarks>
    /// Round 1: nothing in the application performed any of it — no command, no menu entry, no
    /// call to <c>GameLibrary.Remove</c> — so this test only probed for the absence of a route
    /// (<c>verify/round1/R1-report-part-c.md</c>). The <c>R1-fixes</c> batch adds
    /// <c>file.delete_game</c> behind the confirmation box E8, so the test is rewritten against
    /// the five erasures the requirement lists. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-PERS-042
    public void REQ_PERS_042_a_user_game_can_be_deleted_with_its_save_states()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verify-delete";
        profile.Scripts.Add(new ProfileScript("s", true, "-- none", "*"));

        using HostDriver host = HostDriver.Create(profile);
        string id = host.EntryIdByTitle("verify-delete");
        string profileFile = host.Library.Find(id)!.ProfilePath;
        Assert.True(File.Exists(profileFile));

        host.Launch(id, host.WriteRom(LoopRom.Counter()));

        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));
        host.Run(ZaxCommands.QuickSave);
        Assert.True(host.Slots.Find("3")!.IsOccupied);
        Assert.True(host.Slots.Find("quick")!.IsOccupied);
        string slotFile = host.Paths.SlotStateFile("3");
        Assert.True(File.Exists(slotFile));

        // A cover, so that its erasure can be observed too (REQ-PERS-036's criterion).
        host.Run(ZaxCommands.ImportCover, CommandArgs.Of(WriteSquarePng(host.Root, 512)));
        string cover = Assert.Single(Directory.GetFiles(host.Paths.CoversDirectory, "*.png"));

        // The command model of REQ-UI-205 is the single route every interface uses.
        string[] names = [.. host.Commands.Commands.Select(static command => command.Name)];
        Assert.Contains(ZaxCommands.DeleteGame, names);

        // With no frontend to ask for the confirmation of REQ-UI-110, the loop acts at once.
        host.Run(ZaxCommands.DeleteGame, CommandArgs.Of(id));
        host.Settle();

        // 1. the emulation stopped, since the deleted game was the current one.
        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Null(host.CurrentGameId);

        // 2. every slot of that game, the quick slot included.
        Assert.False(host.Slots.Find("3")!.IsOccupied);
        Assert.False(host.Slots.Find("quick")!.IsOccupied);
        Assert.False(File.Exists(slotFile));

        // 3, 4 and 5. the profile file, the cover and the library entry.
        Assert.False(File.Exists(profileFile));
        Assert.False(File.Exists(cover));
        Assert.Null(host.Library.Find(id));
        Assert.DoesNotContain(
            $"\"{id}\"",
            File.ReadAllText(host.Paths.LibraryFile),
            StringComparison.Ordinal);
    }

    /// <summary>
    /// REQ-PERS-042 and SPEC-10 REQ-UI-037, last clause: a <b>built-in</b> profile and the
    /// "2D Mode" entry are never deletable.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-PERS-042, REQ-UI-037
    public void REQ_PERS_042_a_built_in_profile_is_not_deletable()
    {
        using HostDriver host = HostDriver.Create();
        string twoD = host.TwoDimensionalModeId!;

        int before = host.Library.Entries.Count;
        host.Run(ZaxCommands.DeleteGame, CommandArgs.Of(twoD));
        host.Settle();

        Assert.NotNull(host.Library.Find(twoD));
        Assert.Equal(before, host.Library.Entries.Count);

        GameLibraryEntry? builtIn = host.Library.Entries
            .FirstOrDefault(static entry => entry.Source == GameProfileSource.BuiltIn && !entry.Is2DMode);
        if (builtIn is not null)
        {
            host.Run(ZaxCommands.DeleteGame, CommandArgs.Of(builtIn.Id));
            host.Settle();
            Assert.NotNull(host.Library.Find(builtIn.Id));
            Assert.Equal(before, host.Library.Entries.Count);
        }
    }

    // -------------------------------------------------------------- §3.5 slots

    /// <summary>
    /// REQ-PERS-050: 36 numbered slots and one quick slot, common to every game, each carrying
    /// its identifier, the game it holds, a thumbnail and a date.
    /// </summary>
    [Fact]
    public void REQ_PERS_050_there_are_thirty_six_slots_and_a_quick_one()
    {
        SaveSlotIndex index = new();

        Assert.Equal(36, SaveSlotIndex.NumberedSlotCount);
        Assert.Equal(37, index.Slots.Count);
        Assert.Equal("quick", SaveSlotIndex.QuickSlotId);
        Assert.Equal("1", index.Slots[0].Id);
        Assert.Equal("36", index.Slots[35].Id);
        Assert.Equal("quick", index.Slots[36].Id);

        Assert.True(SaveSlotIndex.IsValidSlotId("1"));
        Assert.True(SaveSlotIndex.IsValidSlotId("36"));
        Assert.True(SaveSlotIndex.IsValidSlotId("quick"));
        Assert.False(SaveSlotIndex.IsValidSlotId("0"));
        Assert.False(SaveSlotIndex.IsValidSlotId("37"));

        index.Occupy("1", "10", 1_760_950_100L, hasThumbnail: true);
        SaveSlotRecord slot = index.Find("1")!;
        Assert.True(slot.IsOccupied);
        Assert.Equal("10", slot.GameId);
        Assert.Equal("1.state", slot.File);
        Assert.Equal("1.png", slot.Thumbnail);
        Assert.False(index.Find("2")!.IsOccupied);
    }

    /// <summary>
    /// REQ-PERS-051: the thumbnail of a numbered slot is a 256 × 192 RGB capture, an occupied
    /// slot is overwritten without confirmation and the metadata are written before the state.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_051_a_numbered_slot_carries_a_two_fifty_six_by_one_ninety_two_thumbnail()
    {
        using HostDriver host = HostDriver.Create();
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Launch2DMode();

        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));

        Assert.True(host.Slots.Find("1")!.IsOccupied);
        Assert.True(File.Exists(host.Paths.SaveIndexFile));
        Assert.True(File.Exists(host.Paths.SlotStateFile("1")));
        Assert.True(File.Exists(host.Paths.SlotThumbnailFile("1")));

        long first = host.Slots.Find("1")!.SavedAt;
        host.Now = host.Now.AddSeconds(60);
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));
        Assert.NotEqual(first, host.Slots.Find("1")!.SavedAt);

        // The metadata on disk name the game and the date.
        using JsonDocument index = JsonDocument.Parse(File.ReadAllText(host.Paths.SaveIndexFile));
        Assert.Equal(37, index.RootElement.GetProperty("slots").GetArrayLength());
    }

    /// <summary>
    /// REQ-PERS-052: the quick slot has a black thumbnail — none at all — and the quick load
    /// accepts a slot that belongs to another game; it appears in no Load or Save list.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_052_the_quick_slot_crosses_games_and_has_no_thumbnail()
    {
        using HostDriver host = HostDriver.Create();
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Launch2DMode();

        host.Run(ZaxCommands.QuickSave);
        Assert.True(host.Slots.Find("quick")!.IsOccupied);
        Assert.Null(host.Slots.Find("quick")!.Thumbnail);
        Assert.False(File.Exists(host.Paths.SlotThumbnailFile("quick")));

        // The quick slot is not part of the numbered range the lists walk.
        Assert.DoesNotContain(
            host.Slots.Slots.Take(SaveSlotIndex.NumberedSlotCount),
            slot => slot.Id == SaveSlotIndex.QuickSlotId);
    }

    /// <summary>
    /// REQ-PERS-053: loading a slot finds the game by identifier, fails with the exact message
    /// when it is gone, and never asks for the ROM.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_053_a_slot_load_never_asks_for_the_rom()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        host.Launch2DMode();
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(2));
        host.Run(ZaxCommands.Stop);

        // The ROM path is cleared: the state is self-contained.
        host.SetRomPath(id, string.Empty);
        host.Run(ZaxCommands.LoadSlot, CommandArgs.Of(2));

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(id, host.CurrentGameId);

        // A slot naming a game that no longer exists.
        host.Slots.Occupy("5", "no-such-game", 1L, hasThumbnail: false);
        File.Copy(host.Paths.SlotStateFile("2"), host.Paths.SlotStateFile("5"), overwrite: true);
        host.Run(ZaxCommands.LoadSlot, CommandArgs.Of(5));

        Assert.Equal("The game profile doesn't exist anymore.", host.Notifications.Current?.Text);
    }

    /// <summary>
    /// REQ-PERS-054: deleting a slot removes its file, empties the slot and persists the
    /// metadata.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_054_deleting_a_slot_removes_its_file()
    {
        using HostDriver host = HostDriver.Create();
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Launch2DMode();
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(4));
        Assert.True(File.Exists(host.Paths.SlotStateFile("4")));

        host.Run(ZaxCommands.DeleteSlot, CommandArgs.Of(4));

        Assert.False(host.Slots.Find("4")!.IsOccupied);
        Assert.False(File.Exists(host.Paths.SlotStateFile("4")));
        Assert.False(File.Exists(host.Paths.SlotThumbnailFile("4")));
        Assert.DoesNotContain(
            "\"gameId\": \"" + host.CurrentGameId + "\",\n      \"savedAt\": 0",
            File.ReadAllText(host.Paths.SaveIndexFile),
            StringComparison.Ordinal);
    }

    /// <summary>REQ-PERS-055 and §5.5: the four rows of the start-up consistency table.</summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_055_the_consistency_vectors_of_paragraph_5_5()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Launch2DMode();

        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(2));
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));

        // Row 2: a slot whose game is gone. Row 3: a slot whose file is gone.
        host.Slots.Occupy("2", "999", 1L, hasThumbnail: false);
        File.Delete(host.Paths.SlotStateFile("3"));
        host.Slots.Save(host.Paths);

        SaveSlotIndex reread = SaveSlotIndex.Load(host.Paths, Now);
        int cleared = reread.Reconcile(host.Paths, gameId => host.Library.Find(gameId) is not null);

        Assert.Equal(2, cleared);
        Assert.True(reread.Find("1")!.IsOccupied);
        Assert.False(reread.Find("2")!.IsOccupied);
        Assert.False(File.Exists(host.Paths.SlotStateFile("2")));
        Assert.False(reread.Find("3")!.IsOccupied);
        Assert.False(reread.Find("quick")!.IsOccupied);
        Assert.Equal(id, reread.Find("1")!.GameId);
    }

    /// <summary>
    /// REQ-PERS-055, recommended behaviour: an unreadable index is rebuilt from the headers of
    /// the state files that are present.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_055_an_unreadable_index_is_rebuilt_from_the_files()
    {
        using HostDriver host = HostDriver.Create();
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Launch2DMode();
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(7));

        File.WriteAllText(host.Paths.SaveIndexFile, "{ not json");
        SaveSlotIndex rebuilt = SaveSlotIndex.Load(host.Paths, Now);

        Assert.True(rebuilt.WasRecovered);
        Assert.True(rebuilt.Find("7")!.IsOccupied);
        Assert.Equal(host.CurrentGameId, rebuilt.Find("7")!.GameId);
        Assert.NotEmpty(Directory.GetFiles(host.Paths.SavesDirectory, "index.json.corrupt-*"));
    }

    /// <summary>
    /// REQ-PERS-056 and §4.5.4: the signature, the version, the metadata header and the four
    /// sections; an unknown signature or version is refused, two captures with no frame between
    /// them are identical, and the size is bounded.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_056_the_savestate_file_has_the_documented_structure()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));

        byte[] file = File.ReadAllBytes(host.Paths.SlotStateFile("1"));
        Assert.Equal("ZAXNESAV"u8.ToArray(), file[..8]);
        Assert.Equal(1, BitConverter.ToInt32(file, 8));

        Assert.True(SaveStateFile.TryRead(file, out Zax.App.Persistence.SaveStateContent? content));
        Assert.NotNull(content);
        Assert.Equal(host.CurrentGameId, content!.Header.GameId);
        Assert.Equal("1", content.Header.SlotId);
        Assert.Equal(40, content.Header.RomSha1.Length);
        Assert.NotEmpty(content.Core);
        Assert.Equal(256, content.Lua.Length);
        Assert.Equal(SaveStateFile.CameraSectionBytes, content.Camera.Length);
        Assert.NotEmpty(content.Conversion);

        // A wrong signature and an unknown version are both refused.
        byte[] wrongSignature = [.. file];
        wrongSignature[0] = 0x58;
        Assert.False(SaveStateFile.TryRead(wrongSignature, out _));

        byte[] wrongVersion = [.. file];
        wrongVersion[8] = 99;
        Assert.False(SaveStateFile.TryRead(wrongVersion, out _));

        // An inconsistent length makes the file invalid.
        Assert.False(SaveStateFile.TryRead(file[..(file.Length - 4)], out _));

        // Two captures with no emulated frame between them are identical, byte for byte.
        byte[] a = CaptureSections(host);
        byte[] b = CaptureSections(host);
        Assert.Equal(a, b);

        // The announced bound holds.
        Assert.True(file.Length <= host.MaxSaveStateSize);
        Assert.True(host.MaxSaveStateSize > 0);
    }

    /// <summary>
    /// REQ-PERS-057: the Load list holds the occupied slots only, and the Save list starts with
    /// the first free numbered slot (SPEC-10 REQ-UI-051, V4).
    /// </summary>
    [Fact]
    public void REQ_PERS_057_the_save_list_starts_with_the_first_free_slot()
    {
        SaveSlotIndex index = new();
        foreach (string slot in (string[])["1", "2", "5"])
        {
            index.Occupy(slot, "10", 100L, hasThumbnail: true);
        }

        List<SaveSlotRecord> occupied = [.. index.Slots
            .Where(static slot => slot.IsOccupied && slot.Id != SaveSlotIndex.QuickSlotId)];
        Assert.Equal(3, occupied.Count);

        string? firstFree = index.Slots
            .Where(static slot => !slot.IsOccupied && slot.Id != SaveSlotIndex.QuickSlotId)
            .Select(static slot => slot.Id)
            .FirstOrDefault();
        Assert.Equal("3", firstFree);

        // With all 36 occupied there is no empty tile at all.
        for (int slot = 1; slot <= SaveSlotIndex.NumberedSlotCount; slot++)
        {
            index.Occupy(SaveSlotIndex.NumberedSlotId(slot), "10", 100L, hasThumbnail: true);
        }

        Assert.DoesNotContain(
            index.Slots.Where(static slot => slot.Id != SaveSlotIndex.QuickSlotId),
            slot => !slot.IsOccupied);
    }

    /// <summary>
    /// REQ-PERS-058: the rollback history is not persisted — it is emptied at every launch, slot
    /// load and reset.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_058_the_rollback_history_is_never_persisted()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(180);
        Assert.True(host.Rollback.Count > 0);

        host.Run(ZaxCommands.Reset);
        Assert.Equal(0, host.Rollback.Count);

        host.RunFrames(180);
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(9));
        host.Run(ZaxCommands.LoadSlot, CommandArgs.Of(9));
        Assert.Equal(0, host.Rollback.Count);

        // Nothing of it reaches the disk.
        Assert.DoesNotContain(
            "rollback",
            File.ReadAllText(host.Paths.SaveIndexFile),
            StringComparison.OrdinalIgnoreCase);
    }

    // ----------------------------------------------------------- §3.6 battery RAM

    /// <summary>
    /// REQ-PERS-060: the battery-backed RAM is kept in <c>battery/&lt;sha1&gt;.srm</c>, reread at
    /// load, written at the stop and at most every 30 s when it changed.
    /// </summary>
    [Fact]
    public void REQ_PERS_060_the_battery_ram_is_kept_per_rom_fingerprint()
    {
        using TemporaryRoot root = new();
        BatteryRamStore store = new(root.Paths);

        Assert.Equal(30.0, BatteryRamStore.AutoSaveSeconds);

        const string fingerprint = "0123456789abcdef0123456789abcdef01234567";
        Assert.Null(store.Attach(fingerprint));
        Assert.Equal(fingerprint, store.Fingerprint);

        byte[] ram = new byte[8192];
        ram[0] = 0x42;
        Assert.True(store.Flush(ram));
        Assert.True(File.Exists(root.Paths.BatteryFile(fingerprint)));
        Assert.Equal(8192, new FileInfo(root.Paths.BatteryFile(fingerprint)).Length);

        // Unchanged content is not written again.
        int writes = store.Writes;
        Assert.False(store.Flush(ram));
        Assert.Equal(writes, store.Writes);

        // The periodic write only happens once the 30 s have elapsed and the content changed.
        ram[1] = 0x43;
        Assert.False(store.Advance(10.0, ram));
        Assert.True(store.Advance(25.0, ram));

        // A second ROM has its own file.
        store.Detach();
        const string other = "fedcba9876543210fedcba9876543210fedcba98";
        store.Attach(other);
        store.Flush(ram);
        Assert.True(File.Exists(root.Paths.BatteryFile(other)));

        // Reattaching the first one hands its content back.
        store.Detach();
        byte[]? reread = store.Attach(fingerprint);
        Assert.NotNull(reread);
        Assert.Equal(0x42, reread![0]);

        store.Delete(fingerprint);
        Assert.False(File.Exists(root.Paths.BatteryFile(fingerprint)));
    }

    // ---------------------------------------------- §3.7 bindings and small state

    /// <summary>
    /// REQ-PERS-065: the bindings live in <c>input.json</c> with a schema version, written at the
    /// exit and after each change; an unreadable file falls back on the defaults and is set
    /// aside. (The content itself is checked by the SPEC-08 tests.)
    /// </summary>
    [Fact]
    public void REQ_PERS_065_the_bindings_file_is_input_json()
    {
        using TemporaryRoot root = new();
        Assert.Equal(Path.Combine(root.Paths.Root, "input.json"), root.Paths.InputFile);

        Zax.App.Input.InputBindings bindings = new();
        InputBindingStore store = new(root.Paths);
        bindings.Keyboard(0).Set(Zax.App.Input.InputAction.A, Zax.Platform.Input.PhysicalControl.Key(75));
        store.Save(bindings);

        using JsonDocument document = JsonDocument.Parse(File.ReadAllText(root.Paths.InputFile));
        Assert.Equal(1, document.RootElement.GetProperty("schemaVersion").GetInt32());
        Assert.True(document.RootElement.TryGetProperty("keyboard", out _));
        Assert.True(document.RootElement.TryGetProperty("gamepads", out _));
        Assert.True(document.RootElement.TryGetProperty("lastGamepad", out _));
    }

    /// <summary>
    /// REQ-PERS-066: the last folder of the file browser is persisted under
    /// <c>ui.last_browse_dir</c> and offered again at the next opening.
    /// </summary>
    [Fact]
    public void REQ_PERS_066_the_last_browse_folder_is_persisted()
    {
        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, Now);

        settings.LastBrowseDirectory = @"D:\roms";
        store.Save(settings);

        Assert.Contains(
            "ui.last_browse_dir",
            File.ReadAllText(root.Paths.SettingsFile),
            StringComparison.Ordinal);

        AppSettings second = new();
        new SettingsStore(root.Paths).Load(second, Now);
        Assert.Equal(@"D:\roms", second.LastBrowseDirectory);
    }

    /// <summary>
    /// REQ-PERS-067: the full-screen state is persisted and restored, and the window rectangle is
    /// in the model with its "restart" effect.
    /// </summary>
    [Fact]
    public void REQ_PERS_067_the_full_screen_state_is_restored()
    {
        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, Now);

        settings.Fullscreen = true;
        store.Save(settings);

        AppSettings second = new();
        new SettingsStore(root.Paths).Load(second, Now);
        Assert.True(second.Fullscreen);

        SettingsModel model = new();
        model.SetRect(SettingKeys.UiWindowRect, new SettingRect(10, 20, 1280, 720));
        Assert.Equal(new SettingRect(10, 20, 1280, 720), model.GetRect(SettingKeys.UiWindowRect));
        Assert.Equal(SettingEffect.Restart, SettingsCatalogue.Get(SettingKeys.UiWindowRect).Effect);
    }

    // ---------------------------------------------------------- §3.8 file layout

    /// <summary>
    /// REQ-PERS-070: by default the user root is <c>%LOCALAPPDATA%\Zaxnes</c>, and nothing is
    /// written into the installation folder.
    /// </summary>
    [Fact]
    public void REQ_PERS_070_the_default_root_is_under_local_app_data()
    {
        Assert.Equal("Zaxnes", UserDataPaths.ProductName);

        string local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        string expected = Path.Combine(local, "Zaxnes");

        // Resolve() creates the folders, so the default root is only inspected, not built here.
        using TemporaryRoot explicitRoot = new();
        Assert.Equal(UserDataPaths.UserDataRootKind.Explicit, explicitRoot.Paths.Kind);
        Assert.Equal(Path.GetFullPath(explicitRoot.Root), explicitRoot.Paths.Root);
        Assert.NotEqual(expected, explicitRoot.Paths.Root);
    }

    /// <summary>
    /// REQ-PERS-071: a <c>portable.txt</c> beside the executable moves the root to
    /// <c>userdata\</c>, and <c>--userdata</c> wins over both.
    /// </summary>
    [Fact]
    public void REQ_PERS_071_portable_txt_moves_the_root_beside_the_executable()
    {
        Assert.Equal("portable.txt", UserDataPaths.PortableMarkerName);
        Assert.Equal("userdata", UserDataPaths.PortableFolderName);

        string folder = Path.Combine(Path.GetTempPath(), $"zax-portable-{Guid.NewGuid():N}");
        Directory.CreateDirectory(folder);
        try
        {
            File.WriteAllText(Path.Combine(folder, "portable.txt"), string.Empty);

            UserDataPaths portable = UserDataPaths.Resolve(null, folder);
            Assert.Equal(UserDataPaths.UserDataRootKind.Portable, portable.Kind);
            Assert.Equal(Path.Combine(folder, "userdata"), portable.Root);
            Assert.True(Directory.Exists(Path.Combine(folder, "userdata", "saves")));

            // --userdata has priority over portable.txt.
            string forced = Path.Combine(folder, "forced");
            UserDataPaths given = UserDataPaths.Resolve(forced, folder);
            Assert.Equal(UserDataPaths.UserDataRootKind.Explicit, given.Kind);
            Assert.Equal(Path.GetFullPath(forced), given.Root);
        }
        finally
        {
            try
            {
                Directory.Delete(folder, recursive: true);
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A locked temporary folder must not fail a verification run.
            }
        }
    }

    /// <summary>REQ-PERS-072: the layout of the user root, name by name.</summary>
    [Fact]
    public void REQ_PERS_072_the_layout_is_the_one_of_the_table()
    {
        using TemporaryRoot root = new();
        UserDataPaths paths = root.Paths;

        Assert.Equal(Path.Combine(paths.Root, "settings.json"), paths.SettingsFile);
        Assert.Equal(Path.Combine(paths.Root, "input.json"), paths.InputFile);
        Assert.Equal(Path.Combine(paths.Root, "library.json"), paths.LibraryFile);
        Assert.Equal(Path.Combine(paths.Root, "saves", "index.json"), paths.SaveIndexFile);
        Assert.Equal(Path.Combine(paths.Root, "saves", "12.state"), paths.SlotStateFile("12"));
        Assert.Equal(Path.Combine(paths.Root, "saves", "quick.png"), paths.SlotThumbnailFile("quick"));
        Assert.Equal(Path.Combine(paths.Root, "battery", "abc.srm"), paths.BatteryFile("abc"));
        Assert.Equal(Path.Combine(paths.Root, "profiles"), paths.ProfilesDirectory);
        Assert.Equal(Path.Combine(paths.Root, "covers"), paths.CoversDirectory);
        Assert.Equal(Path.Combine(paths.Root, "logs"), paths.LogsDirectory);

        foreach (string folder in (string[])
            [paths.SavesDirectory, paths.BatteryDirectory, paths.ProfilesDirectory,
             paths.CoversDirectory, paths.LogsDirectory])
        {
            Assert.True(Directory.Exists(folder), folder);
        }
    }

    /// <summary>
    /// REQ-PERS-073: a root that cannot be written to still lets the program start on in-memory
    /// defaults, carries the exact notification and refuses the write actions with it.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_073_an_unwritable_root_degrades_instead_of_failing()
    {
        // A path no directory can be created at: a file stands where the folder would go.
        string file = Path.Combine(Path.GetTempPath(), $"zax-not-a-folder-{Guid.NewGuid():N}");
        File.WriteAllText(file, "x");
        try
        {
            UserDataPaths paths = UserDataPaths.Resolve(file);
            Assert.False(paths.IsWritable);
            Assert.Equal($"User data folder is not writable: {Path.GetFullPath(file)}", paths.UnwritableMessage);

            AppSettings settings = new();
            SettingsStore store = new(paths);
            store.Load(settings, Now);
            Assert.Equal(70f, settings.Fov);

            // The settings write simply fails; nothing throws.
            settings.Fov = 80f;
            store.Save(settings);
            Assert.Equal(80f, settings.Fov);
        }
        finally
        {
            File.Delete(file);
        }
    }

    /// <summary>
    /// REQ-PERS-074: a file name derived from user data has every character the file system
    /// forbids replaced by <c>_</c> and carries the profile identifier, so that it is unique. The
    /// criterion: a game titled <c>A/B:C?</c> of identifier <c>42</c> gives
    /// <c>A_B_C__3dsen_42.png</c>.
    /// </summary>
    /// <remarks>
    /// Round 1: no derivation existed at all, its only user being the cover art of REQ-PERS-036,
    /// and this test only probed for a method of that shape (<c>Q-050</c>). The <c>R1-fixes</c>
    /// batch adds <c>GameArt.SanitiseFileName</c> and <c>GameArt.CoverFileName</c>, which are
    /// internal to <c>Zax.App</c> and reached here by reflection, as the harness does elsewhere.
    /// Re-verified 2026-10-21, <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Fact] // REQ-PERS-074
    public void REQ_PERS_074_a_derived_file_name_is_sanitised()
    {
        Type art = typeof(Zax.App.AppOptions).Assembly.GetType("Zax.App.Persistence.GameArt", true)!;
        System.Reflection.MethodInfo name = art.GetMethod(
            "CoverFileName",
            System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)!;
        System.Reflection.MethodInfo sanitise = art.GetMethod(
            "SanitiseFileName",
            System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)!;

        // The criterion of the requirement, character for character.
        Assert.Equal("A_B_C__3dsen_42.png", name.Invoke(null, ["A/B:C?", "42"]));

        // Every character Windows and POSIX forbid, plus the control characters.
        string derived = (string)sanitise.Invoke(null, ["a<b>c:d\"e/f\\g|h?i*j\nk"])!;
        Assert.Equal("a_b_c_d_e_f_g_h_i_j_k", derived);
        Assert.All(Path.GetInvalidFileNameChars(), invalid => Assert.DoesNotContain(invalid, derived));

        // And the identifier is what makes two games of the same title distinguishable.
        Assert.NotEqual(
            name.Invoke(null, ["Same Title", "7"]),
            name.Invoke(null, ["Same Title", "8"]));
    }

    /// <summary>
    /// REQ-PERS-075: a first launch on an empty root gives the whole library with no ROM
    /// associated, no slot occupied, the default settings and no dialogue.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_075_the_first_launch_needs_no_dialogue()
    {
        using HostDriver host = HostDriver.Create();

        Assert.NotEmpty(host.Library.Entries);
        Assert.All(host.Library.Entries, entry =>
        {
            Assert.False(entry.HasRom);
            Assert.Equal(0L, entry.LastPlayed);
        });

        Assert.All(host.Slots.Slots, slot => Assert.False(slot.IsOccupied));
        Assert.Equal(70f, host.Settings.Fov);
        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Null(host.Notifications.Current);
    }

    private static DateTimeOffset Now => DateTimeOffset.FromUnixTimeSeconds(1_761_000_000);

    private static byte[] CaptureSections(HostDriver host)
    {
        Zax.App.Persistence.SaveStateContent content = host.CaptureSaveState("1");
        return [.. content.Core, .. content.Lua, .. content.Camera, .. content.Conversion];
    }

    private static GameLibrary BuildSortLibrary()
    {
        GameLibrary library = GameLibrary.Build(
            Zax.Profiles.Catalogue.ProfileCatalogue.Parse(
                """
                { "editions": { "full": [
                  { "id": "1", "title": "Zelda", "file": "a.bytes" },
                  { "id": "2", "title": "Contra", "file": "b.bytes" },
                  { "id": "3", "title": "Mario", "file": "c.bytes" },
                  { "id": "4", "title": "Arkanoid", "file": "d.bytes" } ] } }
                """,
                ".",
                Zax.Profiles.Catalogue.ProfileCatalogue.FullEdition));

        library.MarkPlayed("1", 100L);
        library.MarkPlayed("3", 200L);
        return library;
    }
}
