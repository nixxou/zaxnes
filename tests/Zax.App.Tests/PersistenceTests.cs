using System.IO.Compression;
using Zax.App.Loop;
using Zax.App.Persistence;

namespace Zax.App.Tests;

/// <summary>
/// The persistence this lot owns: the user root of SPEC-09 §3.8, the settings file, the slot
/// index, the battery RAM and the ROM reading of SPEC-07 §3.3.1.
/// </summary>
public sealed class PersistenceTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), $"zax-pers-{Guid.NewGuid():N}");

    public PersistenceTests() => Directory.CreateDirectory(_root);

    /// <inheritdoc/>
    public void Dispose()
    {
        try
        {
            Directory.Delete(_root, recursive: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A locked temporary folder must not fail the run.
        }
    }

    /// <summary>
    /// REQ-PERS-072: the first start creates <c>saves</c>, <c>battery</c>, <c>profiles</c>,
    /// <c>covers</c> and <c>logs</c> under the user root.
    /// </summary>
    [Fact]
    public void REQ_PERS_072_the_layout_is_created_on_the_first_start()
    {
        UserDataPaths paths = UserDataPaths.Resolve(Path.Combine(_root, "userdata"));

        Assert.True(paths.IsWritable);
        Assert.True(Directory.Exists(paths.SavesDirectory));
        Assert.True(Directory.Exists(paths.BatteryDirectory));
        Assert.True(Directory.Exists(paths.ProfilesDirectory));
        Assert.True(Directory.Exists(paths.CoversDirectory));
        Assert.True(Directory.Exists(paths.LogsDirectory));
        Assert.EndsWith("index.json", paths.SaveIndexFile);
    }

    /// <summary>
    /// REQ-PERS-071 and DEC-09-Q7: a <c>portable.txt</c> next to the executable moves the root
    /// to <c>userdata\</c>, and <c>--userdata</c> wins over both rules.
    /// </summary>
    [Fact]
    public void REQ_PERS_071_portable_mode_uses_the_userdata_folder()
    {
        string executable = Path.Combine(_root, "bin");
        Directory.CreateDirectory(executable);
        File.WriteAllText(Path.Combine(executable, UserDataPaths.PortableMarkerName), string.Empty);

        UserDataPaths portable = UserDataPaths.Resolve(null, executable);
        Assert.Equal(UserDataPaths.UserDataRootKind.Portable, portable.Kind);
        Assert.Equal(Path.Combine(executable, UserDataPaths.PortableFolderName), portable.Root);

        string forced = Path.Combine(_root, "forced");
        UserDataPaths overridden = UserDataPaths.Resolve(forced, executable);
        Assert.Equal(UserDataPaths.UserDataRootKind.Explicit, overridden.Kind);
        Assert.Equal(Path.GetFullPath(forced), overridden.Root);
    }

    /// <summary>
    /// AMD-029 §2: the first run after the rename copies <c>&lt;local&gt;\Sen</c> into
    /// <c>&lt;local&gt;\Zaxnes</c>, sub-folders included, says where it came from, and leaves the
    /// old root exactly as it was. <c>%LOCALAPPDATA%</c> is stood in for by a temporary folder:
    /// the test never touches the real one.
    /// </summary>
    [Fact]
    public void AMD_029_the_first_run_copies_the_root_of_the_old_name()
    {
        string local = Path.Combine(_root, "local");
        string legacy = Path.Combine(local, UserDataPaths.LegacyProductName);
        Directory.CreateDirectory(Path.Combine(legacy, "saves"));
        Directory.CreateDirectory(Path.Combine(legacy, "profiles"));
        File.WriteAllText(Path.Combine(legacy, "settings.json"), """{ "schemaVersion": 1 }""");
        File.WriteAllBytes(Path.Combine(legacy, "saves", "quick.state"), [1, 2, 3, 4]);
        File.WriteAllBytes(Path.Combine(legacy, "profiles", "a.3dn"), [9]);

        UserDataPaths paths = UserDataPaths.Resolve(localAppDataDirectory: local);

        Assert.Equal(UserDataPaths.UserDataRootKind.LocalAppData, paths.Kind);
        Assert.Equal(Path.Combine(local, UserDataPaths.ProductName), paths.Root);
        Assert.Equal(legacy, paths.MigratedFrom);

        Assert.Equal("""{ "schemaVersion": 1 }""", File.ReadAllText(paths.SettingsFile));
        Assert.Equal(new byte[] { 1, 2, 3, 4 }, File.ReadAllBytes(paths.SlotStateFile("quick")));
        Assert.Equal(new byte[] { 9 }, File.ReadAllBytes(Path.Combine(paths.ProfilesDirectory, "a.3dn")));

        // The old root is read, never written to, moved or deleted.
        Assert.True(Directory.Exists(legacy));
        Assert.Equal(
            new byte[] { 1, 2, 3, 4 },
            File.ReadAllBytes(Path.Combine(legacy, "saves", "quick.state")));
        Assert.Equal(3, Directory.GetFileSystemEntries(legacy).Length);
    }

    /// <summary>
    /// AMD-029 §2: the copy happens once. A root that is already there is left alone — even an
    /// empty one — and with no old root there is nothing to report.
    /// </summary>
    [Fact]
    public void AMD_029_an_existing_root_is_never_overwritten()
    {
        string local = Path.Combine(_root, "second-run");
        string legacy = Path.Combine(local, UserDataPaths.LegacyProductName);
        Directory.CreateDirectory(legacy);
        File.WriteAllText(Path.Combine(legacy, "settings.json"), "old");

        string current = Path.Combine(local, UserDataPaths.ProductName);
        Directory.CreateDirectory(current);
        File.WriteAllText(Path.Combine(current, "settings.json"), "mine");

        UserDataPaths paths = UserDataPaths.Resolve(localAppDataDirectory: local);

        Assert.Null(paths.MigratedFrom);
        Assert.Equal("mine", File.ReadAllText(paths.SettingsFile));

        // No old root at all: a plain first start, with nothing to say.
        string fresh = Path.Combine(_root, "fresh");
        Directory.CreateDirectory(fresh);
        UserDataPaths none = UserDataPaths.Resolve(localAppDataDirectory: fresh);
        Assert.Null(none.MigratedFrom);
        Assert.True(none.IsWritable);

        // And the explicit and portable roots are not migrated: §2 only moves the default one.
        UserDataPaths explicitRoot = UserDataPaths.Resolve(Path.Combine(_root, "explicit"), null, local);
        Assert.Equal(UserDataPaths.UserDataRootKind.Explicit, explicitRoot.Kind);
        Assert.Null(explicitRoot.MigratedFrom);
    }

    /// <summary>
    /// AMD-029 §2: <see cref="UserDataPaths.TryCopyLegacyRoot"/> answers false rather than
    /// throwing when there is nothing to copy, and refuses to copy a folder onto itself.
    /// </summary>
    [Fact]
    public void AMD_029_the_copy_refuses_the_cases_that_are_not_a_migration()
    {
        string legacy = Path.Combine(_root, "edge", "Sen");
        string target = Path.Combine(_root, "edge", "Zaxnes");

        // No old root.
        Assert.False(UserDataPaths.TryCopyLegacyRoot(legacy, target));
        Assert.False(Directory.Exists(target));

        Directory.CreateDirectory(legacy);
        File.WriteAllText(Path.Combine(legacy, "library.json"), "[]");

        // The same folder under two spellings.
        Assert.False(UserDataPaths.TryCopyLegacyRoot(legacy, legacy + Path.DirectorySeparatorChar));

        Assert.True(UserDataPaths.TryCopyLegacyRoot(legacy, target));
        Assert.Equal("[]", File.ReadAllText(Path.Combine(target, "library.json")));

        // Second call: the target now exists, so nothing is copied over it.
        File.WriteAllText(Path.Combine(legacy, "library.json"), "changed");
        Assert.False(UserDataPaths.TryCopyLegacyRoot(legacy, target));
        Assert.Equal("[]", File.ReadAllText(Path.Combine(target, "library.json")));
    }

    /// <summary>
    /// REQ-PERS-021 and REQ-PERS-023: the settings file round-trips, is written atomically, and
    /// keeps the keys it does not know.
    /// </summary>
    [Fact]
    public void REQ_PERS_023_unknown_keys_survive_a_rewrite()
    {
        UserDataPaths paths = UserDataPaths.Resolve(Path.Combine(_root, "settings"));
        File.WriteAllText(
            paths.SettingsFile,
            """
            { "schemaVersion": 1, "values": { "audio.volume": 0.5, "future.x": 3 } }
            """);

        AppSettings settings = new();
        SettingsStore store = new(paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);

        Assert.Equal(0.5f, settings.Volume, 1e-6f);
        Assert.Null(store.Message);

        settings.Volume = 0.75f;
        store.Save(settings);

        string json = File.ReadAllText(paths.SettingsFile);
        Assert.Contains("\"future.x\": 3", json, StringComparison.Ordinal);
        Assert.Contains("0.75", json, StringComparison.Ordinal);
    }

    /// <summary>
    /// REQ-PERS-022: an unreadable settings file falls back to the defaults, is renamed
    /// <c>.corrupt-&lt;timestamp&gt;</c> and reports its notification.
    /// </summary>
    [Fact]
    public void REQ_PERS_022_an_unreadable_settings_file_is_set_aside()
    {
        UserDataPaths paths = UserDataPaths.Resolve(Path.Combine(_root, "corrupt"));
        File.WriteAllText(paths.SettingsFile, "{{{");

        AppSettings settings = new();
        SettingsStore store = new(paths);
        store.Load(settings, DateTimeOffset.FromUnixTimeSeconds(1_700_000_000));

        Assert.Equal(SettingsStore.UnreadableMessage, store.Message);
        Assert.Equal(1f, settings.Volume, 1e-6f);
        Assert.Equal(70f, settings.Fov, 1e-6f);
        Assert.False(File.Exists(paths.SettingsFile));
        Assert.Single(Directory.GetFiles(paths.Root, "settings.json.corrupt-*"));
    }

    /// <summary>
    /// REQ-PERS-003: a single invalid value is replaced on its own, with no notification.
    /// </summary>
    [Fact]
    public void REQ_PERS_003_an_invalid_value_falls_back_alone()
    {
        UserDataPaths paths = UserDataPaths.Resolve(Path.Combine(_root, "domain"));
        File.WriteAllText(
            paths.SettingsFile,
            """
            { "schemaVersion": 1, "values": { "camera.fov": "abc", "video.frame_rate": 75,
              "emu.game_speed": 12, "audio.volume": 0.5 } }
            """);

        AppSettings settings = new();
        SettingsStore store = new(paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);

        Assert.Null(store.Message);
        Assert.Equal(70f, settings.Fov, 1e-6f);
        Assert.Equal(90, settings.FrameRate);
        Assert.Equal(8, settings.GameSpeed);
        Assert.Equal(0.5f, settings.Volume, 1e-6f);
    }

    /// <summary>
    /// REQ-PERS-024: a file from a newer schema version is read for what is known and is not
    /// written again during the session.
    /// </summary>
    [Fact]
    public void REQ_PERS_024_a_newer_file_is_left_alone()
    {
        UserDataPaths paths = UserDataPaths.Resolve(Path.Combine(_root, "newer"));
        const string content = """
            { "schemaVersion": 99, "values": { "audio.volume": 0.25 } }
            """;
        File.WriteAllText(paths.SettingsFile, content);

        AppSettings settings = new();
        SettingsStore store = new(paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);

        Assert.Equal(SettingsStore.NewerVersionMessage, store.Message);
        Assert.Equal(0.25f, settings.Volume, 1e-6f);

        settings.Volume = 1f;
        store.Save(settings);
        Assert.Equal(content, File.ReadAllText(paths.SettingsFile));
    }

    /// <summary>
    /// REQ-PERS-009: a setting that really changes raises one event; rewriting the same value
    /// raises none.
    /// </summary>
    [Fact]
    public void REQ_PERS_009_only_a_real_change_raises_an_event()
    {
        AppSettings settings = new();
        List<string> changes = [];
        settings.Changed += changes.Add;

        settings.Fov = 80f;
        settings.Fov = 80f;
        settings.Volume = 1f;

        Assert.Equal(["camera.fov"], changes);
    }

    /// <summary>
    /// REQ-PERS-055: a slot whose state file is gone is emptied at start-up, and a slot whose
    /// game no longer exists is emptied and its file deleted.
    /// </summary>
    [Fact]
    public void REQ_PERS_055_the_slots_are_reconciled_at_start_up()
    {
        UserDataPaths paths = UserDataPaths.Resolve(Path.Combine(_root, "slots"));
        SaveSlotIndex index = new();

        index.Occupy("1", "alive", 10, hasThumbnail: false);
        index.Occupy("2", "alive", 20, hasThumbnail: false);
        index.Occupy("3", "gone", 30, hasThumbnail: false);
        File.WriteAllBytes(paths.SlotStateFile("1"), [1]);
        File.WriteAllBytes(paths.SlotStateFile("3"), [3]);
        index.Save(paths);

        SaveSlotIndex reread = SaveSlotIndex.Load(paths, DateTimeOffset.UnixEpoch);
        int cleared = reread.Reconcile(paths, id => id == "alive");

        Assert.Equal(2, cleared);
        Assert.True(reread.Find("1")!.IsOccupied);
        Assert.False(reread.Find("2")!.IsOccupied);
        Assert.False(reread.Find("3")!.IsOccupied);
        Assert.False(File.Exists(paths.SlotStateFile("3")));
    }

    /// <summary>
    /// REQ-PERS-055, SHOULD behaviour: an unreadable index is rebuilt from the headers of the
    /// state files present.
    /// </summary>
    [Fact]
    public void REQ_PERS_055_a_corrupt_index_is_rebuilt_from_the_files()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(9));
        harness.Host.Shutdown();

        File.WriteAllText(harness.Paths.SaveIndexFile, "not json at all");

        SaveSlotIndex rebuilt = SaveSlotIndex.Load(harness.Paths, harness.Now);

        Assert.True(rebuilt.WasRecovered);
        Assert.True(rebuilt.Find("9")!.IsOccupied);
        Assert.Equal("g1", rebuilt.Find("9")!.GameId);
        Assert.Single(Directory.GetFiles(harness.Paths.SavesDirectory, "index.json.corrupt-*"));
    }

    /// <summary>
    /// REQ-PERS-073: a root that cannot be written to still lets the program start, reports the
    /// exact notification and refuses the write actions.
    /// </summary>
    [Fact]
    public void REQ_PERS_073_an_unwritable_root_is_reported()
    {
        // A path whose parent is a file can never be created.
        string file = Path.Combine(_root, "blocker");
        File.WriteAllText(file, "x");

        UserDataPaths paths = UserDataPaths.Resolve(Path.Combine(file, "root"));

        Assert.False(paths.IsWritable);
        Assert.Equal($"User data folder is not writable: {paths.Root}", paths.UnwritableMessage);
    }

    /// <summary>
    /// REQ-LOOP-022: a <c>.zip</c> is decompressed and its first entry used, under the name of
    /// the archive with the <c>.nes</c> extension.
    /// </summary>
    [Fact]
    public void REQ_LOOP_022_a_zip_uses_its_first_entry()
    {
        string archive = Path.Combine(_root, "Foo.zip");
        using (FileStream stream = File.Create(archive))
        using (ZipArchive zip = new(stream, ZipArchiveMode.Create))
        {
            using Stream entry = zip.CreateEntry("bar.nes").Open();
            entry.Write(SyntheticRom.Build());
        }

        RomReadResult result = RomFile.Read(archive);

        Assert.True(result.Success);
        Assert.Equal("Foo.nes", result.Name);
        Assert.Equal(16 + (32 * 1024) + (8 * 1024), result.Image!.Length);
    }

    /// <summary>
    /// REQ-LOOP-023 and vector T10: an unreadable archive reports <c>Corrupted zip file</c>,
    /// and an empty one <c>Uncompression is not successful</c>.
    /// </summary>
    [Fact]
    public void REQ_LOOP_023_archive_failures_report_the_exact_messages()
    {
        string broken = Path.Combine(_root, "broken.zip");
        File.WriteAllBytes(broken, [0x50, 0x4B, 0x03, 0x04, 0xFF, 0xFF]);
        Assert.Equal(RomFile.CorruptedZipMessage, RomFile.Read(broken).ErrorMessage);

        string empty = Path.Combine(_root, "empty.zip");
        using (FileStream stream = File.Create(empty))
        {
            using ZipArchive zip = new(stream, ZipArchiveMode.Create);
        }

        Assert.Equal(RomFile.UncompressionFailedMessage, RomFile.Read(empty).ErrorMessage);

        // Q-028: no 7-Zip decoder is available, so such a file is refused cleanly.
        string seven = Path.Combine(_root, "game.7z");
        File.WriteAllBytes(seven, [0x37, 0x7A]);
        Assert.Equal(RomFile.UncompressionFailedMessage, RomFile.Read(seven).ErrorMessage);
    }

    /// <summary>
    /// REQ-LOOP-023: a missing file reports the file system's own message.
    /// </summary>
    [Fact]
    public void REQ_LOOP_023_a_missing_file_reports_the_system_message()
    {
        RomReadResult result = RomFile.Read(Path.Combine(_root, "absent.nes"));

        Assert.False(result.Success);
        Assert.NotNull(result.ErrorMessage);
        Assert.NotEqual(RomFile.InvalidRomMessage, result.ErrorMessage);
    }

    /// <summary>
    /// REQ-PERS-060: the cartridge RAM of a ROM with a battery is written when the game stops
    /// and read back when the same ROM is loaded again.
    /// </summary>
    [Fact]
    public void REQ_PERS_060_the_battery_ram_survives_a_restart()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.BatteryWriter(0x5A), "battery.nes");
        harness.Catalog.Add("bat", rom);

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("bat"));
        harness.RunFrames(10);

        Assert.Equal(0x5A, harness.Host.Session!.Core.ExportBatteryRam()[0]);

        harness.Run(ZaxCommands.Stop);

        string[] files = Directory.GetFiles(harness.Paths.BatteryDirectory, "*.srm");
        Assert.Single(files);
        Assert.Equal(0x5A, File.ReadAllBytes(files[0])[0]);

        // A second launch reads the file back before the first frame runs.
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("bat"));
        Assert.Equal(0x5A, harness.Host.Session!.Core.ExportBatteryRam()[0]);
    }

    /// <summary>
    /// REQ-PERS-060: a ROM without a battery writes nothing.
    /// </summary>
    [Fact]
    public void REQ_PERS_060_a_rom_without_a_battery_writes_nothing()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "plain");
        harness.RunFrames(10);
        harness.Run(ZaxCommands.Stop);

        Assert.Empty(Directory.GetFiles(harness.Paths.BatteryDirectory, "*.srm"));
    }
}
