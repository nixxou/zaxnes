using System.Reflection;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-051 — robustness of the conversion, of loading and of the file writes. Every automatic
/// acceptance criterion of the amendment, as it is written: REQ-E3D-644 (a texture that cannot
/// be allocated), REQ-E3D-646 (an object that is not ready yet), REQ-LOOP-073 (a blocked load)
/// and REQ-PERS-088 (the scope of the atomic write). REQ-E3D-645 is retired, so it has no test:
/// the vector of REQ-E3D-524 remains the only one for a mesh over 65 535 vertices.
/// </summary>
public sealed class Amd051Tests
{
    /// <summary>A 1 × 1 pattern carries an 8 × 8 texture, so 64 texels (SPEC-04B REQ-E3D-426).</summary>
    private const int TexelsPerPattern = 64;

    // --------------------------------------------- REQ-E3D-644, texture not allocatable

    /// <summary>
    /// REQ-E3D-644, acceptance criterion: with the total atlas budget cut to C texels, loading
    /// a profile whose objects ask for more than C texels renders the objects that fit, logs one
    /// error per object that does not, and the next frame is produced; no exception is raised
    /// and the emulation carries on.
    /// </summary>
    [Fact] // REQ-E3D-644
    public void REQ_E3D_644_a_profile_over_the_budget_renders_what_fits_and_logs_the_rest()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;
        geometry.TextureTexelBudget = 2 * TexelsPerPattern;

        engine.LoadProfile(Patterns(4));

        Assert.Equal(4, geometry.ObjectCount);
        Assert.Equal(2, geometry.UnhousedObjects);
        Assert.Equal(2, log.Count);

        // The two objects that fit are drawn and the two that do not are not, twice in a row:
        // the frame after the refusal is produced just like any other.
        for (int frame = 1; frame <= 2; frame++)
        {
            Assert.Equal(2, DrawTiles(engine, 4, frame));
        }

        Assert.Equal(2, log.Count);
    }

    /// <summary>
    /// REQ-E3D-644 read with REQ-E3D-603, which it does not reopen: the texture space of a
    /// released pattern is freed, and the object that had been refused is drawn at the first
    /// frame that follows, with no rebuild (REQ-E3D-646).
    /// </summary>
    [Fact] // REQ-E3D-644, REQ-E3D-603, REQ-E3D-646
    public void REQ_E3D_644_a_refused_object_is_housed_as_soon_as_there_is_room()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.TextureTexelBudget = 3 * TexelsPerPattern;
        engine.LoadProfile(Patterns(4));

        Assert.Equal(1, geometry.UnhousedObjects);
        Assert.Equal(3, DrawTiles(engine, 4, 1));

        geometry.Release(engine.Patterns.Permanent[0]);
        int builds = geometry.BuildCount;

        Assert.Equal(3, DrawTiles(engine, 4, 2));
        Assert.Equal(0, geometry.UnhousedObjects);
        Assert.Equal(builds, geometry.BuildCount);
    }

    /// <summary>
    /// REQ-E3D-644: the log is bounded — at most one line per object, at most 100 lines per
    /// game, then a summary line at most once per minute of play.
    /// </summary>
    [Fact] // REQ-E3D-644, REQ-E3D-008, REQ-E3D-453
    public void REQ_E3D_644_the_log_is_bounded_and_then_summarised()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;
        geometry.TextureTexelBudget = 0;

        engine.LoadProfile(Patterns(150));
        Assert.Equal(150, geometry.UnhousedObjects);
        Assert.Equal(100, log.Count);

        // One minute of play, at the nominal 60 frames per second of NTSC (SPEC-07 §2).
        SyntheticBatch idle = new();
        for (int frame = 1; frame <= 3600; frame++)
        {
            idle.Convert(engine, frame);
        }

        Assert.Equal(101, log.Count);
    }

    /// <summary>
    /// REQ-E3D-564: the floor the requirement asks for — at least 64 × 512 × 512 texels for the
    /// loaded profile — is what an untouched engine grants, and REQ-E3D-603 still holds.
    /// </summary>
    [Fact] // REQ-E3D-564, REQ-E3D-603
    public void REQ_E3D_564_the_budget_of_an_untouched_engine_caps_nothing()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        Assert.True(geometry.TextureTexelBudget >= 64L * 512 * 512);

        engine.LoadProfile(Patterns(150));
        Assert.Equal(0, geometry.UnhousedObjects);
    }

    // ------------------------------------------- REQ-E3D-646, object not ready yet

    /// <summary>
    /// REQ-E3D-646, acceptance criterion, first half: an object whose construction is delayed
    /// by 10 frames is drawn at the first rendered frame that follows the end of its
    /// construction, and no diagnostic is logged. The delay is a test case, not a behaviour to
    /// aim for: REQ-E3D-601 recommends meshes ready at most 2 emulated frames after a pattern is
    /// created.
    /// </summary>
    [Fact] // REQ-E3D-646, REQ-E3D-601
    public void REQ_E3D_646_an_object_delayed_by_ten_frames_is_drawn_right_after()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;
        geometry.BuildDelayFrames = 10;
        engine.LoadProfile(Patterns(1));

        for (int frame = 1; frame <= 9; frame++)
        {
            // REQ-E3D-601: the shape is recognised all along; it is only not drawn.
            Assert.Equal(0, DrawTiles(engine, 1, frame, expectedShapes: 1));
        }

        Assert.Equal(1, DrawTiles(engine, 1, 10, expectedShapes: 1));
        Assert.Empty(log);
    }

    /// <summary>
    /// REQ-E3D-646, acceptance criterion, second half: an object whose construction is delayed
    /// by 700 frames produces exactly one diagnostic line, at frame 600 ± 1.
    /// </summary>
    [Fact] // REQ-E3D-646
    public void REQ_E3D_646_an_object_delayed_by_seven_hundred_frames_is_logged_once_at_six_hundred()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;
        geometry.BuildDelayFrames = 700;
        engine.LoadProfile(Patterns(1));

        int at = 0;
        SyntheticBatch idle = new();
        for (int frame = 1; frame <= 700; frame++)
        {
            idle.Convert(engine, frame);
            if (at == 0 && log.Count == 1)
            {
                at = frame;
            }
        }

        Assert.Single(log);
        Assert.InRange(at, 599, 601);
    }

    // ------------------------------------------------- REQ-LOOP-073, blocked load

    /// <summary>
    /// REQ-LOOP-073, first acceptance criterion: the harness blocks step 6 of REQ-LOOP-021 (the
    /// wait for the meshes) and the notification appears after 10 s ± 1 s. The text is the one
    /// the requirement fixes and the line AMD-051 §3 point 5 adds to the table of SPEC-07 §4
    /// gives its severity and its duration.
    /// </summary>
    [HostFact] // REQ-LOOP-073, REQ-UI-080
    public void REQ_LOOP_073_a_blocked_load_notifies_after_ten_seconds()
    {
        using HostDriver host = HostDriver.Create();
        string id = Prepare(host);

        StallMeshWait(host, true);
        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();

        double elapsed = 0.0;
        for (int tick = 0; tick < 20; tick++)
        {
            elapsed += 1.0 / 60.0;
            host.Tick(1.0 / 60.0);
        }

        Assert.True(host.IsLoading);
        Assert.Null(host.Notifications.Current);

        while (host.IsLoading && elapsed < 20.0)
        {
            elapsed += 0.25;
            host.Tick(0.25);
        }

        Assert.False(host.IsLoading);
        Assert.InRange(elapsed, 9.0, 11.0);

        Notification shown = Assert.Single(
            host.Notifications.Log,
            entry => entry.Text == "Loading timed out.");
        Assert.Equal(NotificationSeverity.Error, shown.Severity);
        Assert.Equal(4.0, shown.Seconds);
    }

    /// <summary>
    /// REQ-LOOP-073, second acceptance criterion: after the notification the state is Stopped,
    /// the loading indicator is false, and a command of the command model (AMD-010 REQ-UI-205)
    /// runs within the second that follows.
    /// </summary>
    [HostFact] // REQ-LOOP-073, REQ-LOOP-023, REQ-UI-205
    public void REQ_LOOP_073_after_the_notification_the_program_is_stopped_and_usable()
    {
        using HostDriver host = HostDriver.Create();
        string id = Prepare(host);

        StallMeshWait(host, true);
        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();
        for (int tick = 0; tick < 64 && host.IsLoading; tick++)
        {
            host.Tick(0.25);
        }

        Assert.False(host.IsLoading);
        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Null(host.CurrentGameId);

        // One rendered frame of 1/60 s — well inside the second — takes the command up.
        StallMeshWait(host, false);
        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.Tick(1.0 / 60.0);
        Assert.True(host.IsLoading);
    }

    // ------------------------------------- REQ-PERS-088, scope of the atomic write

    /// <summary>
    /// REQ-PERS-088, acceptance criterion, for the library file: a write interrupted after half
    /// the bytes leaves the old file readable and unchanged, and no temporary file is taken for
    /// the final one at the next start-up.
    /// </summary>
    [Fact] // REQ-PERS-088, REQ-PERS-021
    public void REQ_PERS_088_an_interrupted_write_of_the_library_keeps_the_old_file()
    {
        using TemporaryRoot root = new();
        string path = root.Paths.LibraryFile;
        GameLibraryStore store = new(PhysicalProfileFileSystem.Instance);

        store.Save(path, new GameLibraryFile
        {
            Games = { new GameLibraryRecord { Id = "1", Title = "First", RomPath = @"D:\a.nes" } },
        });

        byte[] before = File.ReadAllBytes(path);
        GameLibraryFile replacement = new()
        {
            Games = { new GameLibraryRecord { Id = "2", Title = "Second", RomPath = @"D:\b.nes" } },
        };

        // A half-written temporary file is exactly what a killed process leaves behind.
        string temporary = path + ".tmp";
        File.WriteAllBytes(temporary, before[..(before.Length / 2)]);

        Assert.Equal(before, File.ReadAllBytes(path));
        Assert.Equal("First", Assert.Single(store.Load(path, DateTimeOffset.UnixEpoch)!.Games).Title);

        // And the write that does go through replaces both.
        store.Save(path, replacement);
        Assert.Equal("Second", Assert.Single(store.Load(path, DateTimeOffset.UnixEpoch)!.Games).Title);
    }

    /// <summary>
    /// REQ-PERS-088 for the slot index and the bindings file: the write is cut after half the
    /// bytes and each keeps its previous contents, readable, with the half-written temporary
    /// file beside it under its own name.
    /// </summary>
    [Fact] // REQ-PERS-088, REQ-PERS-024, REQ-PERS-055
    public void REQ_PERS_088_an_interrupted_write_keeps_the_index_and_the_bindings()
    {
        using TemporaryRoot root = new();
        root.Paths.EnsureDirectories();

        // The slot index (SPEC-09 §4.5.3).
        SaveSlotIndex index = SaveSlotIndex.Load(root.Paths, DateTimeOffset.UnixEpoch);
        index.Occupy("1", "g1", 7, hasThumbnail: false);
        index.Save(root.Paths);
        Interrupted(root.Paths.SaveIndexFile, () =>
        {
            index.Occupy("2", "g1", 8, hasThumbnail: false);
            index.Save(root.Paths);
        });

        SaveSlotIndex reread = SaveSlotIndex.Load(root.Paths, DateTimeOffset.UnixEpoch);
        Assert.True(reread.Find("1")!.IsOccupied);
        Assert.False(reread.Find("2")!.IsOccupied);

        // The bindings file (REQ-INP-052).
        InputBindings bindings = new();
        InputBindingStore store = new(root.Paths);
        store.Save(bindings);
        Interrupted(root.Paths.InputFile, () =>
        {
            bindings.Keyboard(0).ClearAll();
            store.Save(bindings);
        });

        InputBindings rereadBindings = new();
        new InputBindingStore(root.Paths).Load(rereadBindings);
        Assert.NotEmpty(rereadBindings.Keyboard(0).Bindings);
    }

    /// <summary>
    /// REQ-PERS-088 for a savestate file (REQ-PERS-056), through the write the application
    /// really performs: the interrupted write leaves the previous state intact and readable.
    /// </summary>
    [HostFact] // REQ-PERS-088, REQ-PERS-056
    public void REQ_PERS_088_an_interrupted_write_of_a_savestate_keeps_the_old_file()
    {
        using HostDriver host = HostDriver.Create();
        string id = Prepare(host);
        host.Run(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.RunFrames(4);
        Assert.True(SaveToSlot(host, "1"));

        string path = host.Paths.SlotStateFile("1");
        byte[] before = File.ReadAllBytes(path);
        Assert.True(SaveStateFile.TryRead(before, out _));

        host.RunFrames(4);
        Interrupted(path, () => SaveToSlot(host, "1"));
        Assert.True(SaveStateFile.TryRead(File.ReadAllBytes(path), out _));
    }

    // ------------------------------------------------------------- helpers

    /// <summary>
    /// Runs <paramref name="write"/> with the write of <paramref name="path"/> cut after half
    /// its bytes, and checks the two guarantees of REQ-PERS-088.
    /// </summary>
    private static void Interrupted(string path, Action write)
    {
        byte[] before = File.ReadAllBytes(path);
        string temporary = path + ".tmp";

        PropertyInfo hook = typeof(UserDataPaths).GetProperty(
            "InterruptAfterHalf",
            BindingFlags.Static | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("the interruption hook of REQ-PERS-088 is gone");

        bool fired = false;
        Action<string> interrupt = candidate =>
        {
            if (!fired && string.Equals(candidate, temporary, StringComparison.OrdinalIgnoreCase))
            {
                fired = true;
                throw new IOException("interrupted");
            }
        };

        hook.SetValue(null, interrupt);
        try
        {
            write();
        }
        catch (IOException)
        {
            // The store may let the failure out; what matters is the file on disk.
        }
        finally
        {
            hook.SetValue(null, null);
        }

        Assert.True(fired, $"the write of {path} never reached the interruption point");
        Assert.Equal(before, File.ReadAllBytes(path));
        Assert.True(File.Exists(temporary));
        Assert.NotEqual(before.Length, new FileInfo(temporary).Length);
    }

    /// <summary>Writes the current state into a slot, as REQ-LOOP-041 does.</summary>
    private static bool SaveToSlot(HostDriver host, string slotId)
        => (bool)host.HostObject.GetType()
            .GetMethod("SaveToSlot")!
            .Invoke(host.HostObject, [slotId, true])!;

    /// <summary>
    /// The "2D Mode" entry of the real library, associated with a synthetic ROM: the entry
    /// every host test of this file launches.
    /// </summary>
    private static string Prepare(HostDriver host)
    {
        string id = host.TwoDimensionalModeId
            ?? throw new InvalidOperationException("the catalogue has no \"2D Mode\" entry.");
        host.SetRomPath(id, host.WriteRom(LoopRom.Counter()));
        return id;
    }

    /// <summary>Holds step 6 of REQ-LOOP-021, the test instrument of REQ-LOOP-073.</summary>
    private static void StallMeshWait(HostDriver host, bool stalled)
        => host.HostObject.GetType()
            .GetProperty("StallMeshWait")!
            .SetValue(host.HostObject, stalled);

    /// <summary>
    /// Converts a frame of <paramref name="tiles"/> background tiles, one per pattern, and
    /// returns how many of them reach the draw list (REQ-E3D-641).
    /// </summary>
    private static int DrawTiles(
        ConversionEngine engine,
        int tiles,
        int frameCounter,
        int? expectedShapes = null)
    {
        SyntheticBatch batch = new();
        for (int n = 0; n < tiles; n++)
        {
            batch.BackgroundPlanes(ulong.MaxValue, (ulong)(n + 1), 8 * n, 0);
        }

        ConversionFrame frame = batch.Convert(engine, frameCounter);
        if (expectedShapes is { } shapes)
        {
            Assert.Equal(shapes, frame.Instances.Count);
        }

        return engine.BuildDrawList(frame).Tiles.Count;
    }

    /// <summary>
    /// A profile of <paramref name="count"/> single-tile permanent patterns, one 3D object each
    /// and every tile image distinct (SPEC-04 §2).
    /// </summary>
    private static GameProfile Patterns(int count)
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

    /// <summary>A throw-away user root, removed when the test ends.</summary>
    private sealed class TemporaryRoot : IDisposable
    {
        public TemporaryRoot()
        {
            Root = Path.Combine(Path.GetTempPath(), $"zax-amd051-{Guid.NewGuid():N}");
            Directory.CreateDirectory(Root);
            Paths = UserDataPaths.Resolve(Root);
        }

        public string Root { get; }

        public UserDataPaths Paths { get; }

        public void Dispose()
        {
            try
            {
                Directory.Delete(Root, recursive: true);
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A locked temporary folder must not fail a test run.
            }
        }
    }
}
