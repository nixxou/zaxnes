using System.Diagnostics;
using System.Globalization;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Nes.Rom;
using Zax.Profiles;
using Zax.Render;
using Zax.Render.Assets;

namespace Zax.Harness;

/// <summary>
/// The <c>stress</c> command: replays the shape of the REQ-ACC-030 bench in a loop, so that an
/// intermittent fault can be caught and described (Q-062).
/// </summary>
/// <remarks>
/// <para>
/// Q-062 reports one <c>NullReferenceException</c> attributed to <c>Ppu2C02.ProcessDot</c> in
/// one isolated run out of five of
/// <c>FinalRomCaptureTests.The_reference_games_hold_sixty_frames_per_second_in_1080p</c>, with a
/// stack holding a single managed frame. The two discriminants the ticket asks for are options
/// here: <c>--no-gl</c> drops the render and keeps the emulation, and <c>--threaded</c> moves
/// the emulation off the thread that owns the GL context, which is the arrangement AMD-023 asks
/// to try.
/// </para>
/// <para>
/// Every exception is reported twice: once from <see cref="AppDomain.FirstChanceException"/>,
/// which fires at the throw site before any unwinding, and once where it is caught. That is the
/// only way to tell a truncated stack walk from a real call chain.
/// </para>
/// </remarks>
internal static class StressRun
{
    /// <summary>Usage text of the command.</summary>
    public const string Usage = """
        Zax.Harness stress — repeat the REQ-ACC-030 bench to catch an intermittent fault (Q-062)

        Usage:
          Zax.Harness stress [options]

        Options:
          --roms <dir>          ROM folder (default: the folder ZAXNES_ROMS names).
          --assets <dir>        Folder holding profiles.json; found automatically otherwise.
          --rounds <n>          Times the whole game list is replayed (default 1).
          --games <n>           Only the first n games of the list (default all).
          --warmup <n>          Unmeasured frames per game (default 600).
          --frames <n>          Measured frames per game (default 1800).
          --size <w>x<h>        Render size (default 1920x1080).
          --no-shadows          Render with the shadows off.
          --no-gl               Emulate and convert, but never render: Q-062 discriminant (a).
          --threaded            Emulate on a worker thread, render on the context thread.
          --seed <n>            Seed of the scripted input (default 9, as the bench uses).
          --quiet               One line per game instead of per round progress.
        """;

    /// <summary>Environment variable naming the folder the user keeps their own dumps in.</summary>
    public const string RomsVariable = "ZAXNES_ROMS";

    /// <summary>One of the user's dumps and the catalogue profile it is played with.</summary>
    private sealed record Game(string Title, string Rom, string Profile);

    /// <summary>
    /// The same thirteen (ROM, profile) pairs the REQ-ACC-020 and REQ-ACC-030 benches use.
    /// </summary>
    private static readonly Game[] Games =
    [
        new("Blaster Master", "Blaster Master (USA).nes", "Blaster Master (U).bytes"),
        new("Contra", "Contra (USA).nes", "Contra.bytes"),
        new("Duck Hunt", "Duck Hunt (World).nes", "Duck Hunt (World).bytes"),
        new("Elevator Action", "Elevator Action (USA).nes", "Elevator Action.bytes"),
        new("Excitebike", "Excitebike (Japan, USA) (En).nes", "Excitebike.bytes"),
        new("Final Fantasy", "Final Fantasy (USA).nes", "Final_Fantasy_USA.bytes"),
        new("Legend Of Zelda", "Legend of Zelda, The (USA).nes", "Legend Of Zelda.bytes"),
        new("Mega Man 2", "Mega Man 2 (USA).nes", "Mega Man 2.bytes"),
        new("Metroid", "Metroid (USA).nes", "Metroid.bytes"),
        new("Super Mario Bros.", "Super Mario Bros. (World).nes", "Super Mario Bros.bytes"),
        new("Super Mario Bros. 3", "Super Mario Bros. 3 (USA).nes", "Super Mario Bros 3 (U) (PRG1).bytes"),
        new("Tetris - Nintendo", "Tetris (USA).nes", "Tetris-Nintendo.bytes"),
        new(
            "Zelda II: The Adventure of Link",
            "Zelda II - The Adventure of Link (Europe).nes",
            "Zelda II - The Adventure of Link (U).bytes"),
    ];

    /// <summary>Runs the command.</summary>
    /// <param name="args">Arguments after <c>stress</c>.</param>
    /// <returns>0 when every round went through, 1 when a fault was caught.</returns>
    public static int Run(string[] args)
    {
        ArgumentNullException.ThrowIfNull(args);

        // No machine path is written in this repository (lot PUB-1): the ROM folder comes from
        // --roms or from the environment variable ZAXNES_ROMS, and the command stops with a
        // message when it has neither.
        string? roms = Environment.GetEnvironmentVariable(RomsVariable)?.Trim();
        string? assets = null;
        int rounds = 1;
        int gameLimit = int.MaxValue;
        int warmup = 600;
        int frames = 1_800;
        int width = 1920;
        int height = 1080;
        bool shadows = true;
        bool withGl = true;
        bool threaded = false;
        int seed = 9;
        bool quiet = false;

        for (int i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--roms":
                    roms = Next(args, ref i);
                    break;

                case "--assets":
                    assets = Next(args, ref i);
                    break;

                case "--rounds":
                    rounds = Number(args, ref i);
                    break;

                case "--games":
                    gameLimit = Number(args, ref i);
                    break;

                case "--warmup":
                    warmup = Number(args, ref i);
                    break;

                case "--frames":
                    frames = Number(args, ref i);
                    break;

                case "--size":
                    (width, height) = ParseSize(Next(args, ref i));
                    break;

                case "--seed":
                    seed = Number(args, ref i);
                    break;

                case "--no-shadows":
                    shadows = false;
                    break;

                case "--no-gl":
                    withGl = false;
                    break;

                case "--threaded":
                    threaded = true;
                    break;

                case "--quiet":
                    quiet = true;
                    break;

                case "--help":
                case "-h":
                    Console.Out.WriteLine(Usage);
                    return 0;

                default:
                    Console.Error.WriteLine($"stress: unknown option {args[i]}");
                    Console.Error.WriteLine();
                    Console.Error.WriteLine(Usage);
                    return 2;
            }
        }

        if (string.IsNullOrEmpty(roms))
        {
            Console.Error.WriteLine(
                $"stress: no ROM folder; pass --roms <dir> or set {RomsVariable} to the folder "
                + "holding your own dumps.");
            return 2;
        }

        Game[] list = [.. Games.Where(game => File.Exists(Path.Combine(roms, game.Rom))).Take(gameLimit)];
        if (list.Length == 0)
        {
            Console.Error.WriteLine($"stress: no ROM of the list was found under {roms}");
            return 2;
        }

        string? assetRoot = LocateAssets(assets);
        if (assetRoot is null)
        {
            Console.Error.WriteLine("stress: assets-neutral was not found; pass --assets <dir>.");
            return 2;
        }

        AppDomain.CurrentDomain.FirstChanceException += (_, e) => ReportFirstChance(e.Exception);

        Console.Out.WriteLine(string.Create(
            CultureInfo.InvariantCulture,
            $"stress: {list.Length} game(s), {rounds} round(s), {warmup} + {frames} frames, "
                + $"{(withGl ? $"{width}x{height}{(shadows ? " with shadows" : " no shadows")}" : "no GL")}"
                + $"{(threaded ? ", emulation on a worker thread" : string.Empty)}, seed {seed}"));
        Console.Out.WriteLine(string.Create(
            CultureInfo.InvariantCulture,
            $"stress: runtime {Environment.Version}, tiered compilation "
                + $"{Environment.GetEnvironmentVariable("DOTNET_TieredCompilation") ?? "default"}, "
                + $"server GC {System.Runtime.GCSettings.IsServerGC}"));

        OffscreenContext? context = null;
        SceneRenderer? renderer = null;
        try
        {
            if (withGl)
            {
                context = OffscreenContext.Create(width, height);
                renderer = new SceneRenderer(context.Gl, NeutralSceneAssets.TryOpen(assetRoot));
            }

            Stopwatch wall = Stopwatch.StartNew();
            for (int round = 0; round < rounds; round++)
            {
                foreach (Game game in list)
                {
                    double worst = Replay(
                        game, roms, assetRoot, context, renderer,
                        warmup, frames, shadows, threaded, seed);

                    if (!quiet)
                    {
                        Console.Out.WriteLine(string.Create(
                            CultureInfo.InvariantCulture,
                            $"  round {round + 1}/{rounds}  {game.Title,-32} worst frame {worst:F2} ms"
                                + $"  ({wall.Elapsed.TotalSeconds:F0} s elapsed)"));
                    }
                }
            }

            Console.Out.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"stress: {rounds} round(s) of {list.Length} game(s) went through in "
                    + $"{wall.Elapsed.TotalSeconds:F0} s, no fault."));
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine();
            Console.Error.WriteLine("stress: FAULT");
            Console.Error.WriteLine(exception.ToString());
            Console.Error.WriteLine();
            Console.Error.WriteLine("caught on thread "
                + Environment.CurrentManagedThreadId.ToString(CultureInfo.InvariantCulture)
                + ", stack of the catch site:");
            Console.Error.WriteLine(Environment.StackTrace);
            return 1;
        }
        finally
        {
            renderer?.Dispose();
            context?.Dispose();
        }
    }

    /// <summary>
    /// Plays one game for <paramref name="warmup"/> + <paramref name="frames"/> frames and
    /// returns the worst frame time in milliseconds.
    /// </summary>
    private static double Replay(
        Game game,
        string roms,
        string assetRoot,
        OffscreenContext? context,
        SceneRenderer? renderer,
        int warmup,
        int frames,
        bool shadows,
        bool threaded,
        int seed)
    {
        GameProfile profile = LoadProfile(assetRoot, game.Profile);

        NesCore core = new();
        RomLoadResult load = core.LoadRom(File.ReadAllBytes(Path.Combine(roms, game.Rom)), game.Rom);
        if (!load.Success)
        {
            throw new InvalidOperationException($"{game.Rom}: {load.Error}");
        }

        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(profile);
        core.VideoCallback = (in FrameBatch batch) =>
        {
            engine.Convert(batch, (int)core.FrameCount);
            engine.Publish();
        };

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Settings.Shadows = shadows;

        Random random = new(seed);
        ushort held = 0;
        int frame = 0;
        double worst = 0;

        void Advance()
        {
            core.RunFrame(ScriptedInput(frame++, random, ref held));
            geometry.Commit();
            scene.Clipping = engine.Settings.Clipping;
            scene.DrawList = engine.BuildDrawList();
            scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
            scene.EmulatedTime = engine.PublishedFrame.Time;
        }

        void Draw()
        {
            if (context is not null && renderer is not null)
            {
                renderer.Render(context.Target, scene);
                context.Gl.Finish();
            }
        }

        try
        {
            if (threaded)
            {
                // The arrangement AMD-023 asks to try: the emulation on a worker thread, the
                // render on the thread the GL context is current on. The two alternate and never
                // overlap — the draw list of REQ-E3D-641 is a single buffer the engine rewrites
                // on every frame, so rendering frame n while frame n + 1 is being built would be
                // reading it as it is rewritten (see Q-062 §6). What this does exercise is every
                // object of the core and of the engine being touched from two threads in turn.
                for (int step = 0; step < warmup + frames; step++)
                {
                    long start = Stopwatch.GetTimestamp();
                    Task.Run(Advance).GetAwaiter().GetResult();
                    if (step >= warmup)
                    {
                        Draw();
                    }

                    worst = Math.Max(worst, Stopwatch.GetElapsedTime(start).TotalMilliseconds);
                }
            }
            else
            {
                for (int step = 0; step < warmup; step++)
                {
                    Advance();
                }

                Draw();

                for (int step = 0; step < frames; step++)
                {
                    long start = Stopwatch.GetTimestamp();
                    Advance();
                    Draw();
                    worst = Math.Max(worst, Stopwatch.GetElapsedTime(start).TotalMilliseconds);
                }
            }
        }
        catch (Exception exception)
        {
            throw new InvalidOperationException(
                $"{game.Title}: frame {frame} of {warmup + frames} failed.", exception);
        }
        finally
        {
            geometry.Clear();
        }

        return worst;
    }

    /// <summary>
    /// The scripted input of REQ-ACC-020, the same sequence the bench drives the games with.
    /// </summary>
    private static ushort ScriptedInput(int frame, Random random, ref ushort held)
    {
        if (frame < 120)
        {
            return 0;
        }

        if (frame < 600)
        {
            return (ushort)((frame % 60) < 30 ? 0x08 : 0x00);
        }

        if (frame % 600 is >= 0 and < 20)
        {
            return 0x08;
        }

        if (frame % 8 == 0)
        {
            int direction = random.Next(4) switch
            {
                0 => 0x10,
                1 => 0x20,
                2 => 0x40,
                _ => 0x80,
            };

            held = (ushort)(direction | (random.Next(2) == 0 ? 0x01 : 0x02));
        }

        return held;
    }

    private static GameProfile LoadProfile(string assetRoot, string file)
    {
        ProfileLoadResult result = ProfileFormat.Load(
            File.ReadAllBytes(Path.Combine(assetRoot, "profiles", file)), sourceName: file);
        return result.IsSuccess
            ? result.Profile!
            : throw new InvalidOperationException($"{file}: {result.Message}");
    }

    /// <summary>
    /// Prints every exception at its throw site. A stack that is complete here and truncated in
    /// the report of the test runner is a stack-walk artefact; one that is truncated here as
    /// well points at the code it names (Q-062 §3).
    /// </summary>
    private static void ReportFirstChance(Exception exception)
    {
        // The GL loader probes entry points with exceptions and the profile reader uses them
        // for its own control flow; only the ones the ticket is about are interesting.
        if (exception is not (NullReferenceException or AccessViolationException
            or IndexOutOfRangeException or InvalidCastException or OutOfMemoryException))
        {
            return;
        }

        Console.Error.WriteLine();
        Console.Error.WriteLine(string.Create(
            CultureInfo.InvariantCulture,
            $"stress: first chance {exception.GetType().FullName} on thread "
                + $"{Environment.CurrentManagedThreadId}: {exception.Message}"));
        Console.Error.WriteLine("  stack recorded in the exception:");
        Console.Error.WriteLine(exception.StackTrace ?? "    (none yet)");
        Console.Error.WriteLine("  stack walked from the throw site:");
        Console.Error.WriteLine(Environment.StackTrace);
        Console.Error.Flush();
    }

    private static string? LocateAssets(string? hint)
    {
        if (!string.IsNullOrWhiteSpace(hint))
        {
            return Directory.Exists(hint) ? hint : null;
        }

        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName, "assets-neutral");
            if (File.Exists(Path.Combine(candidate, "profiles.json")))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }

    private static (int Width, int Height) ParseSize(string text)
    {
        int cross = text.IndexOf('x', StringComparison.OrdinalIgnoreCase);
        if (cross <= 0)
        {
            throw new ArgumentException($"--size wants <w>x<h>, got '{text}'.", nameof(text));
        }

        return (
            int.Parse(text[..cross], CultureInfo.InvariantCulture),
            int.Parse(text[(cross + 1)..], CultureInfo.InvariantCulture));
    }

    private static int Number(string[] args, ref int index)
        => int.Parse(Next(args, ref index), CultureInfo.InvariantCulture);

    private static string Next(string[] args, ref int index)
    {
        if (index + 1 >= args.Length)
        {
            throw new ArgumentException($"{args[index]} wants a value.", nameof(args));
        }

        return args[++index];
    }
}
