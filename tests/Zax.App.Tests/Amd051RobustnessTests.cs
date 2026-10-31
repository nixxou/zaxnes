using System.Text.Json.Nodes;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.App.Tests;

/// <summary>
/// AMD-051 §2, host half: the blocked load of REQ-LOOP-073 and the scope of the atomic write of
/// REQ-PERS-088.
/// </summary>
public sealed class Amd051RobustnessTests
{
    // ------------------------------------------------- REQ-LOOP-073, blocked load

    /// <summary>
    /// REQ-LOOP-073, first acceptance criterion: the harness blocks step 6 of REQ-LOOP-021 —
    /// the wait for the meshes — and the notification appears after 10 s ± 1 s. Its text is the
    /// one the requirement fixes, with the severity and duration of the launch failures of
    /// REQ-LOOP-023 (SPEC-07 §4).
    /// </summary>
    [Fact] // REQ-LOOP-073
    public void REQ_LOOP_073_a_load_that_stops_advancing_times_out_after_ten_seconds()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);

        harness.Host.StallMeshWait = true;
        harness.Host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        harness.Host.ApplyCommands();

        // A handful of rendered frames reaches step 6, nowhere near ten seconds.
        double elapsed = 0.0;
        for (int tick = 0; tick < 20; tick++)
        {
            elapsed += 1.0 / 60.0;
            harness.Tick(1.0 / 60.0);
        }

        Assert.True(harness.Host.IsLoading);
        Assert.Null(harness.Host.Notifications.Current);

        // From here on the loading indicator is up and nothing advances any more.
        while (harness.Host.IsLoading && elapsed < 20.0)
        {
            elapsed += 0.25;
            harness.Tick(0.25);
        }

        Assert.False(harness.Host.IsLoading);
        Assert.InRange(
            elapsed,
            EmulationHost.LoadTimeoutSeconds - 1.0,
            EmulationHost.LoadTimeoutSeconds + 1.0);

        Notification shown = Assert.Single(
            harness.Host.Notifications.Log,
            entry => entry.Text == EmulationHost.LoadingTimedOutMessage);
        Assert.Equal("Loading timed out.", shown.Text);
        Assert.Equal(NotificationSeverity.Error, shown.Severity);
        Assert.Equal(NotificationCenter.DefaultSeconds, shown.Seconds);
    }

    /// <summary>
    /// REQ-LOOP-073, second acceptance criterion: after the notification the state is Stopped,
    /// the loading indicator is false, and a command of the command model (AMD-010 REQ-UI-205)
    /// runs within the second that follows.
    /// </summary>
    [Fact] // REQ-LOOP-073, REQ-LOOP-023, REQ-UI-205
    public void REQ_LOOP_073_after_the_timeout_the_program_is_stopped_and_answers_commands()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);

        harness.Host.StallMeshWait = true;
        harness.Host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        harness.Host.ApplyCommands();
        for (int tick = 0; tick < 64 && harness.Host.IsLoading; tick++)
        {
            harness.Tick(0.25);
        }

        Assert.False(harness.Host.IsLoading);
        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.Null(harness.Host.CurrentGameId);
        Assert.Equal(0f, harness.Host.LoadProgress);

        // A command runs inside the second that follows: at 1/60 s per rendered frame, one
        // frame is enough for it to be taken up.
        harness.Host.StallMeshWait = false;
        harness.Host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        harness.Tick(1.0 / 60.0);
        Assert.True(harness.Host.IsLoading);

        harness.Settle();
        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal("g1", harness.Host.CurrentGameId);
    }

    /// <summary>
    /// REQ-LOOP-073 read with REQ-ACC-031 (point 51-N1 of the R2 review): the total duration of
    /// a mass build is not bounded. As long as the progress of REQ-E3D-600 moves, the load is
    /// never interrupted — even well past ten seconds.
    /// </summary>
    [Fact] // REQ-LOOP-073, REQ-E3D-600, REQ-ACC-031
    public void REQ_LOOP_073_a_build_that_keeps_advancing_is_never_interrupted()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom, Amd051Profiles.ManyPatterns(40));
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // A mesh regeneration (REQ-LOOP-024) whose transfers are handed over four objects at a
        // time: step 6 lasts far longer than ten seconds and the progress moves all along.
        GeometryBuilder geometry =
            Assert.IsType<GeometryBuilder>(harness.Host.Session!.Engine.Geometry);
        geometry.DeferTransfer = true;
        harness.Host.ChangeMeshAlgorithm(RenderAlgorithm.Marching);

        double elapsed = 0.0;
        int steps = 0;
        while (harness.Host.IsLoading && elapsed < 60.0)
        {
            elapsed += 4.0;
            harness.Tick(4.0);
            geometry.Commit();
            steps++;
        }

        // More than ten seconds of rendered frames went by in more than one step, the progress
        // always moving, and the load finished of its own accord.
        Assert.True(elapsed > EmulationHost.LoadTimeoutSeconds, $"only {elapsed} s elapsed");
        Assert.True(steps >= 3, $"only {steps} steps");
        Assert.False(harness.Host.IsLoading);
        Assert.DoesNotContain(
            harness.Host.Notifications.Log,
            entry => entry.Text == EmulationHost.LoadingTimedOutMessage);
    }

    /// <summary>
    /// AMD-051 §3 point 6, read with AMD-035 §3 and AMD-036: the message is an entry of the
    /// English table with its French translation, and it is translated where it is shown.
    /// </summary>
    [Fact] // REQ-LOOP-073, REQ-UI-082
    public void REQ_LOOP_073_the_message_is_in_both_text_tables()
    {
        Assert.Equal(
            EmulationHost.LoadingTimedOutMessage,
            UiStrings.EnglishTable[UiKeys.NotifyLoadingTimedOut]);

        using (UiLanguageScope english = new(UiLanguage.English))
        {
            Assert.Equal(
                "Loading timed out.",
                UiNotifications.Translate(EmulationHost.LoadingTimedOutMessage));
        }

        using UiLanguageScope french = new(UiLanguage.French);
        Assert.Equal(
            "Délai de chargement dépassé.",
            UiNotifications.Translate(EmulationHost.LoadingTimedOutMessage));
    }

    /// <summary>
    /// REQ-E3D-644 and REQ-E3D-646 in the application: the degradations of the geometry reach
    /// the log file of the session, the host attaching its own sink when it creates one.
    /// </summary>
    [Fact] // REQ-E3D-644, REQ-E3D-646
    public void REQ_E3D_644_the_geometry_degradations_reach_the_log_of_the_host()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom, Amd051Profiles.ManyPatterns(4));
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));

        Assert.NotNull(harness.Host.Session);
        harness.Log.Clear();

        // Nothing fits any more: the rebuild of REQ-LOOP-024 refuses every object.
        harness.Host.Session!.Geometry.TextureTexelBudget = 0;
        harness.Host.ChangeMeshAlgorithm(RenderAlgorithm.Marching);
        harness.Settle();

        Assert.Equal(4, harness.Host.Session.Geometry.UnhousedObjects);
        Assert.Equal(4, harness.Log.Count(line => line.Contains("REQ-E3D-644", StringComparison.Ordinal)));
    }

    // ------------------------------------- REQ-PERS-088, scope of the atomic write

    /// <summary>
    /// REQ-PERS-088, acceptance criterion, for the slot index: a write interrupted after half
    /// the bytes leaves the old file readable and unchanged, and no temporary file is taken for
    /// the final one at the next start-up.
    /// </summary>
    [Fact] // REQ-PERS-088, REQ-PERS-021, REQ-PERS-055
    public void REQ_PERS_088_an_interrupted_write_of_the_slot_index_keeps_the_old_file()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Paths.EnsureDirectories();

        SaveSlotIndex index = SaveSlotIndex.Load(harness.Paths, harness.Now);
        index.Occupy("1", "g1", harness.Now.ToUnixTimeSeconds(), hasThumbnail: false);
        index.Save(harness.Paths);

        string path = harness.Paths.SaveIndexFile;
        byte[] before = File.ReadAllBytes(path);

        index.Occupy("2", "g1", harness.Now.ToUnixTimeSeconds(), hasThumbnail: false);
        AssertInterruptedWriteIsHarmless(path, before, () => index.Save(harness.Paths));

        // Next start-up: the index read back is the old one, never the leftover temporary file.
        SaveSlotIndex reread = SaveSlotIndex.Load(harness.Paths, harness.Now);
        Assert.True(reread.Find("1")!.IsOccupied);
        Assert.False(reread.Find("2")!.IsOccupied);
    }

    /// <summary>REQ-PERS-088 for the bindings file (REQ-PERS-024, REQ-INP-052).</summary>
    [Fact] // REQ-PERS-088, REQ-PERS-021
    public void REQ_PERS_088_an_interrupted_write_of_the_bindings_keeps_the_old_file()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Paths.EnsureDirectories();

        InputBindings bindings = new();
        InputBindingStore store = new(harness.Paths, () => harness.Now);
        store.Save(bindings);

        string path = harness.Paths.InputFile;
        byte[] before = File.ReadAllBytes(path);

        bindings.Keyboard(0).ClearAll();
        AssertInterruptedWriteIsHarmless(path, before, () => store.Save(bindings));

        // Next start-up: what is read is the old, complete file.
        InputBindings reread = new();
        new InputBindingStore(harness.Paths, () => harness.Now).Load(reread);
        Assert.NotEmpty(reread.Keyboard(0).Bindings);
    }

    /// <summary>REQ-PERS-088 for a savestate file (REQ-PERS-056).</summary>
    [Fact] // REQ-PERS-088, REQ-PERS-056
    public void REQ_PERS_088_an_interrupted_write_of_a_savestate_keeps_the_old_file()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        harness.RunFrames(4);
        Assert.True(harness.Host.SaveToSlot("1"));

        string path = harness.Paths.SlotStateFile("1");
        byte[] before = File.ReadAllBytes(path);
        Assert.True(before.Length > 0);

        harness.RunFrames(4);
        AssertInterruptedWriteIsHarmless(path, before, () => harness.Host.SaveToSlot("1"));
        Assert.True(SaveStateFile.TryRead(File.ReadAllBytes(path), out _));

        // A write that is not interrupted replaces the temporary file and the final one.
        harness.RunFrames(4);
        Assert.True(harness.Host.SaveToSlot("1"));
        Assert.NotEqual(before, File.ReadAllBytes(path));
        Assert.True(SaveStateFile.TryRead(File.ReadAllBytes(path), out _));
    }

    /// <summary>
    /// REQ-PERS-088, ordering clause: the <c>.v&lt;N&gt;.bak</c> copy of REQ-PERS-024 is written
    /// before the first migrated rewrite, and the <c>*.corrupt-&lt;timestamp&gt;</c> rename of
    /// REQ-PERS-022 happens before any write of the replacement file. Neither is ever the target
    /// of the atomic replace.
    /// </summary>
    [Fact] // REQ-PERS-088, REQ-PERS-022, REQ-PERS-024
    public void REQ_PERS_088_the_backup_and_the_set_aside_copy_come_before_the_rewrite()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Paths.EnsureDirectories();

        // A settings file from an earlier schema version: the copy is kept, the file is
        // migrated and rewritten, and the copy holds the *old* contents.
        string settings = harness.Paths.SettingsFile;
        string original = """{"schemaVersion":0,"values":{"ui.language":"fr"}}""";
        File.WriteAllText(settings, original);

        SettingsStore store = new(harness.Paths);
        store.Load(new AppSettings(), harness.Now);

        string backup = settings + ".v0.bak";
        Assert.True(File.Exists(backup));
        Assert.Equal(original, File.ReadAllText(backup));
        Assert.Equal(SettingsStore.SchemaVersion, ReadSchemaVersion(settings));

        // An unreadable file is set aside before anything is written in its place.
        File.WriteAllText(settings, "{ not json");
        SettingsStore second = new(harness.Paths);
        second.Load(new AppSettings(), harness.Now);

        string aside = settings + $".corrupt-{harness.Now:yyyyMMdd-HHmmss}";
        Assert.True(File.Exists(aside));
        Assert.Equal("{ not json", File.ReadAllText(aside));
        Assert.Contains(SettingsStore.UnreadableMessage, second.Messages);
    }

    private static int ReadSchemaVersion(string path)
        => JsonNode.Parse(File.ReadAllText(path))!["schemaVersion"]!.GetValue<int>();

    /// <summary>
    /// Runs <paramref name="write"/> with the write of <paramref name="path"/> interrupted once,
    /// after half its bytes, and checks the two guarantees of REQ-PERS-088: the old file is byte
    /// for byte what it was, and the half-written temporary file sits beside it under its own
    /// name, never taken for the final one.
    /// </summary>
    /// <remarks>
    /// The interruption is fired once and only for this path, which is what a process killed in
    /// the middle of one write does. Other writes the same call makes go through, so what the
    /// assertions see is the file the requirement is about.
    /// </remarks>
    private static void AssertInterruptedWriteIsHarmless(string path, byte[] before, Action write)
    {
        string temporary = path + ".tmp";
        bool fired = false;
        UserDataPaths.InterruptAfterHalf = candidate =>
        {
            if (!fired && string.Equals(candidate, temporary, StringComparison.OrdinalIgnoreCase))
            {
                fired = true;
                throw new IOException("interrupted");
            }
        };

        try
        {
            write();
        }
        catch (IOException)
        {
            // A store that lets the failure out is fine: what matters is the file on disk.
        }
        finally
        {
            UserDataPaths.InterruptAfterHalf = null;
        }

        Assert.True(fired, "the write never reached the interruption point");
        Assert.True(File.Exists(path));
        Assert.Equal(before, File.ReadAllBytes(path));
        Assert.True(File.Exists(temporary));
        Assert.NotEqual(before.Length, new FileInfo(temporary).Length);
    }
}

/// <summary>Profiles the AMD-051 tests need.</summary>
internal static class Amd051Profiles
{
    /// <summary>
    /// A profile of <paramref name="count"/> single-tile permanent patterns, one 3D object each.
    /// The high plane carries <c>n + 1</c>, so every tile image is distinct (SPEC-04 §2).
    /// </summary>
    public static GameProfile ManyPatterns(int count)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "amd-051";
        for (int n = 0; n < count; n++)
        {
            ProfilePattern pattern = new()
            {
                TileSize = new Vec2I(1, 1),
                Permanent = true,
                Grid = new PatternGrid(
                    1,
                    1,
                    [new ProfileTile(ulong.MaxValue, (ulong)(n + 1), TileFlip.None)]),
            };
            pattern.Variants.Add(new PatternVariant
            {
                Depth = 4,
                Alpha = 1f,
                RenderStyle = VariantRenderStyle.Normal,
            });
            profile.PermanentPatterns.Add(pattern);
        }

        return profile;
    }
}
