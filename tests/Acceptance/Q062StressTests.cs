using System.Security.Cryptography;
using Xunit.Sdk;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Nes.Ppu;
using Zax.Render;
using Xunit.Abstractions;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Records every exception the process throws, at the throw site, for the whole lifetime of a
/// block. Q-062 is about a fault whose reported stack held a single managed frame; a first-chance
/// handler is the only place where the throw site can be seen before any unwinding.
/// </summary>
internal sealed class FaultWatch : IDisposable
{
    private readonly List<string> _faults = [];
    private readonly object _gate = new();
    private bool _disposed;

    /// <summary>Starts watching.</summary>
    public FaultWatch() => AppDomain.CurrentDomain.FirstChanceException += OnFirstChance;

    /// <summary>What was caught, one entry per hard fault, in order.</summary>
    public IReadOnlyList<string> Faults
    {
        get
        {
            lock (_gate)
            {
                return [.. _faults];
            }
        }
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        AppDomain.CurrentDomain.FirstChanceException -= OnFirstChance;
    }

    private void OnFirstChance(object? sender, System.Runtime.ExceptionServices.FirstChanceExceptionEventArgs e)
    {
        // The exceptions the program uses for control flow — a missing profile file, a Lua
        // error, a GL entry point that is not there — are not what this watches. Only the
        // families a memory or lifetime fault shows up as are recorded.
        if (e.Exception is not (NullReferenceException or AccessViolationException
            or IndexOutOfRangeException or InvalidCastException or OutOfMemoryException
            or DataMisalignedException))
        {
            return;
        }

        string entry = $"{e.Exception.GetType().FullName} on thread "
            + $"{Environment.CurrentManagedThreadId}: {e.Exception.Message}\n"
            + $"  stack in the exception:\n{e.Exception.StackTrace ?? "    (none)"}\n"
            + $"  stack walked from the throw site:\n{Environment.StackTrace}";

        lock (_gate)
        {
            _faults.Add(entry);
        }
    }
}

/// <summary>
/// A fact that is reported as skipped, rather than failed, on a machine that carries none of the
/// user's dumps. Same intent as <see cref="RomOfTheUserFactAttribute"/>, for a test that walks
/// whatever is present instead of naming one file.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class AnyRomOfTheUserFactAttribute : FactAttribute
{
    /// <summary>Requires at least one dump of <see cref="UserRoms.All"/>.</summary>
    public AnyRomOfTheUserFactAttribute()
    {
        if (UserRoms.MissingReason is { } missing)
        {
            Skip = missing;
        }
    }
}

/// <summary>
/// The same, for a fact that also needs a GL 3.3 core context.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class AnyRomOfTheUserGlFactAttribute : FactAttribute
{
    /// <summary>Requires at least one dump and a usable GL context.</summary>
    public AnyRomOfTheUserGlFactAttribute()
    {
        if (GlAvailability.Reason is { } reason)
        {
            Skip = reason;
        }
        else if (UserRoms.MissingReason is { } missing)
        {
            Skip = missing;
        }
    }
}

/// <summary>
/// The regression and stress cover of <c>Q-062</c> (answer R0 in <c>AMD-023</c>): an intermittent
/// <see cref="NullReferenceException"/> attributed to <c>Ppu2C02.ProcessDot</c>, seen once in five
/// isolated runs of the REQ-ACC-030 bench and never since.
/// </summary>
/// <remarks>
/// <para>
/// The four tests cover the three causes AMD-023 asks to rule out, plus the ground the ticket's
/// own reasoning stands on:
/// </para>
/// <list type="bullet">
/// <item><description>
/// <see cref="The_measurement_bench_replays_without_a_fault"/> replays the shape of the bench —
/// several games in a row, one GL context, 1920 × 1080 with shadows — and fails on the first
/// hard fault thrown anywhere in the process, not only on one that escapes.
/// </description></item>
/// <item><description>
/// <see cref="The_bench_sequence_is_identical_from_one_run_to_the_next"/> is the executable form
/// of the argument of Q-062 §2: the emulated sequence of the bench is bit-identical from run to
/// run, so a fault that depended on the emulated state would land on the same frame every time.
/// It landed once in five.
/// </description></item>
/// <item><description>
/// <see cref="The_dot_loop_holds_through_a_detach_a_reset_and_a_region_change"/> covers the
/// object-lifetime hypothesis: the dot loop is walked with no cartridge attached, across a
/// reset and across a region change in mid-frame.
/// </description></item>
/// <item><description>
/// <see cref="The_emulation_and_the_render_hold_on_two_threads"/> is the arrangement AMD-023
/// asks to try: the emulation off the thread that owns the GL context.
/// </description></item>
/// </list>
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class Q062StressTests(ITestOutputHelper output)
{
    /// <summary>Games the fault was seen around: the first three of the bench's list.</summary>
    private const int GamesUnderStress = 3;

    /// <summary>
    /// REQ-ACC-030, REQ-ACC-041 and the "sans plantage" clause of REQ-CORE-115: the measurement
    /// bench of REQ-ACC-030 runs to the end without any hard fault being thrown — not merely
    /// without one escaping to the test.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [CorpusAnyRomGlFact] // REQ-ACC-030, REQ-ACC-041, REQ-CORE-115, Q-062
    public void The_measurement_bench_replays_without_a_fault()
    {
        UserGame[] games = [.. UserRoms.Present.Take(GamesUnderStress)];
        Assert.NotEmpty(games);

        using FaultWatch watch = new();
        using GlTarget target = GlTarget.Create(1920, 1080);
        using SceneRenderer renderer = new(target.Gl);

        // Two passes over the same games, so the second one runs with the mesh and texture
        // caches of the first still resident and expiring underneath it — the state the bench
        // is in by its second and third game, which is where the fault was seen.
        for (int pass = 0; pass < 2; pass++)
        {
            foreach (UserGame game in games)
            {
                int frames = Replay(game, target, renderer, warmup: 200, measured: 400);
                output.WriteLine($"pass {pass + 1}: {game.Title}, {frames} frames.");
            }
        }

        AssertNoFault(watch);
    }

    /// <summary>
    /// REQ-ARCH-012 applied to the bench: two replays of the same games with the same seed emulate
    /// exactly the same frames, down to the PPU state and the tile records of every frame. This is
    /// what makes "one run in five" incompatible with a fault in the emulation itself (Q-062 §2).
    /// </summary>
    [Trait("Requires", "UserRoms")]
    [AnyRomOfTheUserFact] // REQ-ARCH-012, Q-062
    public void The_bench_sequence_is_identical_from_one_run_to_the_next()
    {
        UserGame[] games = [.. UserRoms.Present.Take(GamesUnderStress)];
        Assert.NotEmpty(games);

        foreach (UserGame game in games)
        {
            string first = Fingerprint(game, 1_200);
            string second = Fingerprint(game, 1_200);
            output.WriteLine($"{game.Title}: {first}");
            Assert.Equal(first, second);
        }
    }

    /// <summary>
    /// The object-lifetime hypothesis of AMD-023: the dot loop is walked with no cartridge
    /// attached, then across a reset and a region change made in mid-frame. Nothing on the path
    /// of <c>ProcessDot</c> may dereference without checking (Q-062).
    /// </summary>
    [Trait("Requires", "UserRoms")]
    [AnyRomOfTheUserFact] // REQ-CORE-004, REQ-CORE-005, REQ-CORE-015, Q-062
    public void The_dot_loop_holds_through_a_detach_a_reset_and_a_region_change()
    {
        using FaultWatch watch = new();

        // A bare PPU, no cartridge: every dot of two whole frames, rendering enabled, so the
        // fetch pipeline, the sprite evaluation, the pixel composition and both capture points
        // all run with _cartridge null. One CPU cycle is three NTSC dots.
        Ppu2C02 bare = new();
        bare.Reset();
        bare.Write(1, 0x1E); // PPUMASK: background and sprites on, no left-column masking.
        bare.Write(0, 0x08); // PPUCTRL: sprites from $1000, so both pattern tables are read.
        int cycles = 2 * RegionTiming.DotsPerScanline * bare.Timing.ScanlinesPerFrame / 3;
        for (int cycle = 0; cycle < cycles; cycle++)
        {
            bare.StepCpuCycle();
        }

        Assert.NotEqual(0, bare.Dot + bare.Scanline);

        // Four-screen mode asks the cartridge for its own nametable pages, which is the branch
        // the null-forgiving dereference used to sit on.
        bare.WriteVramThroughBus(0x2000, 0x42);
        Assert.Equal(0x42, bare.ReadNametableByte(0x2000));

        // Now a loaded core: a reset and a region change between two frames, and a close while
        // frames keep being asked for (REQ-CORE-005: they do nothing).
        UserGame present = UserRoms.Present.First();
        NesCore core = UserRoms.Load(present);
        Random random = new(9);
        ushort held = 0;
        for (int frame = 0; frame < 400; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));

            if (frame % 97 == 0)
            {
                core.Reset();
            }

            if (frame % 53 == 0)
            {
                core.Region = frame % 106 == 0 ? NesRegion.Pal : NesRegion.Ntsc;
            }
        }

        core.CloseRom();
        for (int frame = 0; frame < 10; frame++)
        {
            Assert.Equal(0, core.RunFrame(0));
        }

        AssertNoFault(watch);
    }

    /// <summary>
    /// What AMD-023 asks to try: the emulation on a worker thread and the render on the thread
    /// that owns the GL context, meeting at a barrier once per frame. Nothing in <c>src/</c> is
    /// shared across threads except the APU's sample queue, which this exercises too by reading
    /// audio from the render side (REQ-CORE-071 allows that read from another thread).
    /// </summary>
    /// <remarks>
    /// The two steps <strong>alternate</strong>, they do not overlap. That is not a convenience:
    /// the draw list of REQ-E3D-641 is a single buffer the engine rewrites on every frame
    /// (<c>DrawListBuilder</c>), unlike the converted frames, which are triple-buffered
    /// (REQ-E3D-280). A host that renders frame <em>n</em> while the engine builds frame
    /// <em>n + 1</em> would be reading that buffer as it is rewritten; nothing in the spec asks
    /// for it and the desktop host never does it (it drives both from the render callback). This
    /// test therefore checks what a threaded host may legitimately do — emulate off the context
    /// thread — and the finding about overlapping is recorded in <c>spec/changes/Q-062.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [CorpusAnyRomGlFact] // REQ-ARCH-010, REQ-CORE-071, Q-062, AMD-023
    public void The_emulation_and_the_render_hold_on_two_threads()
    {
        UserGame game = UserRoms.Present.First();

        using FaultWatch watch = new();
        using GlTarget target = GlTarget.Create(1280, 720);
        using SceneRenderer renderer = new(target.Gl);

        NesCore core = UserRoms.Load(game);
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(UserRoms.Profile(game));
        core.VideoCallback = (in FrameBatch batch) =>
        {
            engine.Convert(batch, (int)core.FrameCount);
            engine.Publish();
        };

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Settings.Shadows = true;

        Random random = new(9);
        ushort held = 0;
        int frame = 0;
        float[] audio = new float[800];

        void Advance()
        {
            core.RunFrame(UserRoms.ScriptedInput(frame++, random, ref held));
            geometry.Commit();
            scene.Clipping = engine.Settings.Clipping;
            scene.DrawList = engine.BuildDrawList();
            scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
            scene.EmulatedTime = engine.PublishedFrame.Time;
        }

        try
        {
            for (int step = 0; step < 900; step++)
            {
                // The frame is emulated, converted and committed on a thread-pool thread, which
                // is never the thread the GL context is current on; the render then runs on the
                // context thread once that one is done. Every object the core and the engine own
                // is therefore touched from two different threads in turn.
                Task.Run(Advance).GetAwaiter().GetResult();

                target.Draw(renderer, scene);
                core.ReadAudio(audio);
            }
        }
        finally
        {
            geometry.Clear();
        }

        output.WriteLine($"{game.Title}: {frame} frames emulated off the context thread.");
        Assert.Equal(900, frame);
        AssertNoFault(watch);
    }

    /// <summary>Plays one game through the bench's own per-frame work and returns the frame count.</summary>
    private static int Replay(UserGame game, GlTarget target, SceneRenderer renderer, int warmup, int measured)
    {
        NesCore core = UserRoms.Load(game);
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(UserRoms.Profile(game));
        core.VideoCallback = (in FrameBatch batch) =>
        {
            engine.Convert(batch, (int)core.FrameCount);
            engine.Publish();
        };

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Settings.Shadows = true;

        Random random = new(9);
        ushort held = 0;
        int frame = 0;

        try
        {
            for (int step = 0; step < warmup + measured; step++)
            {
                core.RunFrame(UserRoms.ScriptedInput(frame++, random, ref held));
                geometry.Commit();
                scene.Clipping = engine.Settings.Clipping;
                scene.DrawList = engine.BuildDrawList();
                scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
                scene.EmulatedTime = engine.PublishedFrame.Time;

                if (step >= warmup)
                {
                    target.Draw(renderer, scene);
                }
            }
        }
        finally
        {
            geometry.Clear();
        }

        return frame;
    }

    /// <summary>
    /// A hash of what the emulation produced over <paramref name="frames"/> frames: the frame
    /// counter, the CPU cycle count, the scan position, the pixel image and every tile record.
    /// </summary>
    private static string Fingerprint(UserGame game, int frames)
    {
        NesCore core = UserRoms.Load(game);
        Random random = new(9);
        ushort held = 0;

        using IncrementalHash hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        Span<byte> scratch = stackalloc byte[32];

        for (int frame = 0; frame < frames; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));

            BitConverter.TryWriteBytes(scratch[..8], core.CycleCount);
            BitConverter.TryWriteBytes(scratch[8..12], core.Ppu.Dot);
            BitConverter.TryWriteBytes(scratch[12..16], core.Ppu.Scanline);
            BitConverter.TryWriteBytes(scratch[16..20], core.MaskX);
            BitConverter.TryWriteBytes(scratch[20..24], core.MaskY);
            BitConverter.TryWriteBytes(scratch[24..28], core.TileRecords.Length);
            hash.AppendData(scratch[..28]);
            hash.AppendData(core.PixelFrame);
            hash.AppendData(core.Ppu.Oam);
            hash.AppendData(core.Ppu.Vram);

            foreach (TileRecord record in core.TileRecords)
            {
                BitConverter.TryWriteBytes(scratch[..8], record.Plane0);
                BitConverter.TryWriteBytes(scratch[8..16], record.Plane1);
                BitConverter.TryWriteBytes(scratch[16..18], record.X);
                BitConverter.TryWriteBytes(scratch[18..20], record.Y);
                BitConverter.TryWriteBytes(scratch[20..24], record.Index);
                scratch[24] = record.Palette;
                scratch[25] = (byte)((record.MirrorH ? 1 : 0) | (record.MirrorV ? 2 : 0)
                    | (record.Hidden ? 4 : 0) | (record.Background ? 8 : 0));
                hash.AppendData(scratch[..26]);
            }
        }

        return Convert.ToHexString(hash.GetHashAndReset());
    }

    private void AssertNoFault(FaultWatch watch)
    {
        IReadOnlyList<string> faults = watch.Faults;
        foreach (string fault in faults)
        {
            output.WriteLine(fault);
        }

        Assert.True(
            faults.Count == 0,
            $"{faults.Count} hard fault(s) were thrown; the first one is:\n"
                + (faults.Count > 0 ? faults[0] : string.Empty));
    }
}
