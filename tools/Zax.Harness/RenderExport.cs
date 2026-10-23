using System.Diagnostics;
using System.Globalization;
using System.Text.Json;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Nes.Rom;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Render;
using Zax.Render.Assets;
using Zax.Scripting;

namespace Zax.Harness;

/// <summary>
/// The <c>render</c> command: renders frames offscreen through a hidden GL context and writes PNG
/// captures plus a statistics file (SPEC-01 REQ-ARCH-031, SPEC-06 REQ-SCN-057).
/// </summary>
internal static class RenderExport
{
    /// <summary>Usage text of the command.</summary>
    public const string Usage = """
        Zax.Harness render — offscreen captures of SPEC-06

        Usage:
          Zax.Harness render --out <dir> [--frames synthetic[:<scene>]] [options]
          Zax.Harness render --out <dir> --frames <file.json> --profile <title> [options]
          Zax.Harness render --out <dir> --rom <file.nes> [--profile <title>] [--run <n>]

        Options:
          --out <dir>              Where the PNG captures and render.json go. Required.
          --frames <source>        'synthetic' (default) renders the built-in scenes;
                                   'synthetic:<scene>' renders one of them;
                                   a path renders the frames a JSON file describes.
          --rom <file.nes>         Run a real ROM headless and capture it (REQ-ARCH-031).
          --run <n>                Emulated frames to run before capturing (default 120).
          --sav <file>             Battery-backed PRG-RAM to import before the run, so that a
                                   capture can start from a saved game (REQ-CORE-095).
          --dump-frame <file.json> Write the captured frame as a '--frames' JSON file, so the
                                   same scene can be replayed without the ROM.
          --press <spec>           Port 1 buttons to hold while replaying, as a comma
                                   separated list of <first>[-<last>]:<buttons>, the frame
                                   numbers counting from the reset and the buttons being
                                   a|b|select|start|up|down|left|right joined by '+'
                                   (for example 200:start,260:start,400-700:right).
          --profile <title>        Built-in profile, or a profile file path.
          --assets <dir>           Folder holding profiles.json (profile lookup) and the
                                   cubemaps, meshes and textures the scene uses. Without it
                                   the scene assets are the procedural ones (REQ-PUB-003/004).
          --size <w>x<h>           Capture size in pixels (default 640x480).
          --fov <degrees>          Field of view, 45 to 95 (default 70).
          --angles <rx>,<ry>[,<rz>]  Camera setpoint angles in degrees (default 0.81,0.81,0).
          --distance <d>           Camera setpoint distance (default 104 / atan(FOV/2)).
          --mesh <algorithm>       greedy (default), marching or multicolor (REQ-E3D-340).
          --no-shadows             Render with the shadows off.
          --shadow-map <texels>    Side of the shadow map (default 4096, AMD-034). A size the
                                   hardware refuses is halved until it accepts one, and the
                                   granted size is reported.
          --anti-alias <n>         video.anti_alias: 0 off (default), 1 MSAA 4x, 2 MSAA 8x
                                   (AMD-032). The granted sample count is reported.
          --skybox <name>          default, color, gradient or fantasy (REQ-SCN-049).
          --bg-color <r,g,b>       The custom background colour, 0-255 (REQ-SCN-046).
          --real-time <seconds>    Real time the star twinkle uses (REQ-SCN-050).
          --sky-bodies             Light the procedural sun, moon and clouds of the skybox,
                                   which REQ-SCN-050 leaves off by default (REQ-PUB-003, Q-064).
          --bench <frames>         Also time that many frames per scene and report them.
          --explain                Print the draw list grouped by 3D object, with the NES
                                   colours each group's texels resolve to (colour fidelity).
          --list-scenes            Print the built-in synthetic scene names and exit.

        Scenes: framing, sky, shadow, no-shadow, layer-shadow-off, nudge, alpha, hud, clip,
                backdrop, heavy.
        """;

    /// <summary>Runs the command.</summary>
    /// <param name="args">Arguments after <c>render</c>.</param>
    /// <returns>A process exit code.</returns>
    public static int Run(string[] args)
    {
        ArgumentNullException.ThrowIfNull(args);

        string? output = null;
        string frames = "synthetic";
        string? profile = null;
        string? assets = null;
        string? rom = null;
        string? press = null;
        string? battery = null;
        string? dumpFrame = null;
        int runFrames = 120;
        int width = 640;
        int height = 480;
        float fov = 70f;
        bool shadows = true;
        int shadowMapSize = SceneConstants.ShadowMapSize;
        int antiAlias = 0;
        bool explain = false;
        int bench = 0;
        SkyboxKind skybox = SkyboxKind.Default;
        Rgb24 customBackground = default;
        float realTime = 0f;
        bool skyBodies = false;
        System.Numerics.Vector3? angles = null;
        float? distance = null;
        RenderAlgorithm mesh = RenderAlgorithm.Greedy;

        for (int i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--out":
                    output = Next(args, ref i);
                    break;

                case "--frames":
                    frames = Next(args, ref i);
                    break;

                case "--profile":
                    profile = Next(args, ref i);
                    break;

                case "--assets":
                    assets = Next(args, ref i);
                    break;

                case "--rom":
                    rom = Next(args, ref i);
                    break;

                case "--run":
                    runFrames = int.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--press":
                    press = Next(args, ref i);
                    break;

                case "--sav":
                    battery = Next(args, ref i);
                    break;

                case "--dump-frame":
                    dumpFrame = Next(args, ref i);
                    break;

                case "--size":
                    (width, height) = ParseSize(Next(args, ref i));
                    break;

                case "--fov":
                    fov = float.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--angles":
                    angles = ParseAngles(Next(args, ref i));
                    break;

                case "--distance":
                    distance = float.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--mesh":
                    mesh = Next(args, ref i).ToLowerInvariant() switch
                    {
                        "greedy" => RenderAlgorithm.Greedy,
                        "marching" => RenderAlgorithm.Marching,
                        "multicolor" or "multicolour" => RenderAlgorithm.GreedyMulticolor,
                        _ => throw new ArgumentException($"--mesh expects greedy, marching or multicolor."),
                    };
                    break;

                case "--no-shadows":
                    shadows = false;
                    break;

                case "--shadow-map":
                    shadowMapSize = int.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--anti-alias":
                    antiAlias = int.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    if (antiAlias is < 0 or > 2)
                    {
                        throw new ArgumentException("--anti-alias expects 0, 1 or 2.");
                    }

                    break;

                case "--explain":
                    explain = true;
                    break;

                case "--skybox":
                    skybox = Next(args, ref i).ToLowerInvariant() switch
                    {
                        "default" => SkyboxKind.Default,
                        "color" or "colour" => SkyboxKind.Color,
                        "gradient" => SkyboxKind.Gradient,
                        "fantasy" => SkyboxKind.Fantasy,
                        _ => throw new ArgumentException(
                            "--skybox expects default, color, gradient or fantasy."),
                    };
                    break;

                case "--bg-color":
                    customBackground = ParseColor(Next(args, ref i));
                    break;

                case "--real-time":
                    realTime = float.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--sky-bodies":
                    skyBodies = true;
                    break;

                case "--bench":
                    bench = int.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--list-scenes":
                    foreach (string name in SyntheticScenes.Names)
                    {
                        Console.Out.WriteLine(name);
                    }

                    return 0;

                case "-h":
                case "--help":
                    Console.Out.WriteLine(Usage);
                    return 0;

                default:
                    Console.Error.WriteLine($"render: unknown argument '{args[i]}'");
                    Console.Error.WriteLine();
                    Console.Error.WriteLine(Usage);
                    return 2;
            }
        }

        if (output is null)
        {
            Console.Error.WriteLine("render: --out <dir> is required.");
            return 2;
        }

        Directory.CreateDirectory(output);

        List<SyntheticScene> scenes;
        try
        {
            scenes = rom is null
                ? BuildScenes(frames, profile, assets, mesh)
                : [FromRom(rom, runFrames, profile, assets, mesh, InputScript.Parse(press), battery, dumpFrame)];
        }
        catch (Exception ex) when (ex is ArgumentException or IOException or JsonException
            or InvalidOperationException or ProfileCatalogueException)
        {
            Console.Error.WriteLine($"render: {ex.Message}");
            return 1;
        }

        foreach (SyntheticScene scene in scenes)
        {
            scene.Scene.Settings.Fov = fov;
            scene.Scene.Camera.Fov = fov;
            if (angles is { } r)
            {
                scene.Scene.Camera.Current.Angles = r;
            }

            if (distance is { } d)
            {
                scene.Scene.Camera.Current.Distance = d;
            }

            scene.Scene.Camera.SnapToSetpoints();
            scene.Scene.Settings.Skybox = skybox;
            scene.Scene.Settings.CustomBackgroundColor = customBackground;
            scene.Scene.RealTime = realTime;

            // Q-064: the three bodies exist and are parametric, but REQ-SCN-050 leaves them
            // off; this is what a capture turns them on with.
            if (skyBodies)
            {
                scene.Scene.Settings.SkyBodies.SunGain = 1f;
                scene.Scene.Settings.SkyBodies.MoonGain = 1f;
                scene.Scene.Settings.SkyBodies.CloudGain = 0.6f;
                scene.Scene.Settings.SkyBodies.CloudCoverage = 0.3f;
            }

            if (!shadows)
            {
                scene.Scene.Settings.Shadows = false;
                scene.Engine.Settings.Shadow = false;
            }

            // AMD-032 §2: video.anti_alias as the application applies it.
            scene.Scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(antiAlias);
            scene.Scene.Settings.ShadowMapSize = shadowMapSize;
        }

        using OffscreenContext context = OffscreenContext.Create(width, height);

        // AMD-024 REQ-PUB-001: the scene assets are only the folder --assets names, as in the
        // application — never one found by walking up the tree. Without the option the renderer
        // produces the procedural skyboxes of REQ-PUB-003, which is what a capture of the
        // publishable build has to show. The profile lookup below keeps its own search: the
        // harness is a development tool and the 101 embedded profiles are still what it replays.
        using SceneRenderer renderer = new(context.Gl, NeutralSceneAssets.TryOpen(assets));

        List<string> report = [];
        foreach (SyntheticScene scene in scenes)
        {
            renderer.Render(context.Target, scene.Scene);
            byte[] pixels = context.ReadPixels();

            string file = Path.Combine(output, $"{scene.Name}.png");
            PngWriter.WriteRgba(file, width, height, pixels);

            if (scene.NesPicture is { Length: >= 256 * 240 } picture)
            {
                WriteNesPicture(Path.Combine(output, $"{scene.Name}-2d.png"), picture);
            }

            if (explain)
            {
                SceneState state = scene.Scene;
                CameraView camera = renderer.LastView;
                VisibleRect r = state.VisibleRectangle;
                Console.Out.WriteLine(string.Create(
                    CultureInfo.InvariantCulture,
                    $"camera: fov {camera.Fov:F1}  d {camera.Distance:F2}  angles "
                        + $"({state.Camera.Angles.X:F2}, {state.Camera.Angles.Y:F2}, {state.Camera.Angles.Z:F2})\n"
                        + $"        eye ({camera.Eye.X:F2}, {camera.Eye.Y:F2}, {camera.Eye.Z:F2})"
                        + $"  T' ({camera.Target.X:F2}, {camera.Target.Y:F2})"
                        + $"  P* ({camera.AimPoint.X:F2}, {camera.AimPoint.Y:F2})\n"
                        + $"clipping: ({state.Clipping.X}, {state.Clipping.Y}, {state.Clipping.Z}, {state.Clipping.W})"
                        + $"  R = [{r.MinX}, {r.MaxX}] x [{r.MinY}, {r.MaxY}]"));
                if (scene.Scene.DrawList is { } drawList)
                {
                    Console.Out.WriteLine(DrawListReport.Describe(drawList));
                }
            }

            CaptureStatistics statistics = CaptureStatistics.Measure(pixels, width, height);
            (double Median, double P99)? measured = bench > 0
                ? Bench(renderer, context, scene, bench)
                : null;

            report.Add(Describe(scene, renderer.Statistics, statistics, measured, width, height));

            string timing = measured is { } ms
                ? string.Create(CultureInfo.InvariantCulture, $"  {ms.Median:F2} ms median, {ms.P99:F2} ms p99")
                : string.Empty;
            RenderStatistics counts = renderer.Statistics;
            string msaa = counts.AntiAliasSamples > 1
                ? string.Create(CultureInfo.InvariantCulture, $"  MSAA {counts.AntiAliasSamples}x")
                : string.Empty;

            // AMD-034 §2: the granted size is only worth printing when it is not the one asked
            // for, which is how a hardware fallback gets reported.
            string shadow = counts.ShadowMapSize != counts.RequestedShadowMapSize
                ? string.Create(
                    CultureInfo.InvariantCulture,
                    $"  shadow map {counts.ShadowMapSize} (asked {counts.RequestedShadowMapSize})")
                : string.Empty;
            Console.Out.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"{scene.Name,-18} {statistics.DistinctColors,6} colours  {counts.OpaqueTiles,5} opaque  {counts.TransparentTiles,4} alpha  {counts.HudTiles,4} HUD  {counts.ShadowCasters,5} casters{msaa}{shadow}{timing}"));
        }

        File.WriteAllText(
            Path.Combine(output, "render.json"),
            "[\n  " + string.Join(",\n  ", report) + "\n]\n");

        Console.Out.WriteLine($"render: {scenes.Count} capture(s) of {width}x{height} written to {output}");
        return 0;
    }

    private static List<SyntheticScene> BuildScenes(
        string frames,
        string? profile,
        string? assets,
        RenderAlgorithm mesh)
    {
        if (frames.StartsWith("synthetic", StringComparison.OrdinalIgnoreCase))
        {
            int colon = frames.IndexOf(':', StringComparison.Ordinal);
            IEnumerable<string> names = colon < 0
                ? SyntheticScenes.Names
                : [frames[(colon + 1)..]];
            return [.. names.Select(name => SyntheticScenes.Build(name, mesh))];
        }

        return [FromJson(frames, profile, assets, mesh)];
    }

    /// <summary>
    /// Builds one scene from a JSON frame description and a built-in profile. The format is the
    /// one <c>render --help</c> documents: a background colour index, up to 32 palette bytes and
    /// a list of tile records.
    /// </summary>
    private static SyntheticScene FromJson(
        string path,
        string? profileName,
        string? assets,
        RenderAlgorithm mesh)
    {
        using JsonDocument document = JsonDocument.Parse(File.ReadAllBytes(path));
        JsonElement root = document.RootElement;

        byte[] palette = new byte[32];
        if (root.TryGetProperty("palette", out JsonElement paletteElement))
        {
            int index = 0;
            foreach (JsonElement entry in paletteElement.EnumerateArray())
            {
                if (index < palette.Length)
                {
                    palette[index++] = (byte)entry.GetInt32();
                }
            }
        }

        if (root.TryGetProperty("background", out JsonElement background))
        {
            palette[0] = (byte)background.GetInt32();
        }

        List<TileRecord> records = [];
        int spriteOrder = 0;
        if (root.TryGetProperty("tiles", out JsonElement tiles))
        {
            foreach (JsonElement tile in tiles.EnumerateArray())
            {
                bool sprite = tile.TryGetProperty("sprite", out JsonElement spriteFlag)
                    && spriteFlag.GetBoolean();
                records.Add(new TileRecord(
                    tile.GetProperty("low").GetUInt64(),
                    tile.GetProperty("high").GetUInt64(),
                    (short)tile.GetProperty("x").GetInt32(),
                    (short)tile.GetProperty("y").GetInt32(),
                    sprite ? spriteOrder++ : records.Count,
                    tile.TryGetProperty("mirrorH", out JsonElement mh) && mh.GetBoolean(),
                    tile.TryGetProperty("mirrorV", out JsonElement mv) && mv.GetBoolean(),
                    tile.TryGetProperty("hidden", out JsonElement hidden) && hidden.GetBoolean(),
                    (byte)(tile.TryGetProperty("palette", out JsonElement p) ? p.GetInt32() : 0),
                    !sprite));
            }
        }

        GameProfile profile = LoadProfile(profileName, assets);
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.Settings.RenderAlgorithm = mesh;
        engine.TwoDimensionalMode = profile.PermanentPatterns.Count == 0
            && profile.TemporaryPatterns.Count == 0;
        engine.LoadProfile(profile);
        engine.Convert(new FrameBatch(records.ToArray(), palette, 0, 0), frameCounter: 0);
        engine.Publish();

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Clipping = engine.Settings.Clipping;
        scene.DrawList = engine.BuildDrawList();
        scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);

        return new SyntheticScene(
            Path.GetFileNameWithoutExtension(path),
            scene,
            engine,
            $"{records.Count} tile record(s) from {path}");
    }

    /// <summary>
    /// Runs a real ROM headless for <paramref name="runFrames"/> emulated frames and captures the
    /// last one: the "load a ROM and a profile, replay, export an offscreen capture" of
    /// SPEC-01 REQ-ARCH-031. The controllers stay idle; replaying an input sequence comes with
    /// the input lot.
    /// </summary>
    private static SyntheticScene FromRom(
        string romPath,
        int runFrames,
        string? profileName,
        string? assets,
        RenderAlgorithm mesh,
        InputScript input,
        string? batteryPath,
        string? dumpFrame)
    {
        GameProfile profile = LoadProfile(profileName, assets);
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.Settings.RenderAlgorithm = mesh;
        engine.TwoDimensionalMode = profile.PermanentPatterns.Count == 0
            && profile.TemporaryPatterns.Count == 0;

        NesCore core = new();
        RomLoadResult load = core.LoadRom(File.ReadAllBytes(romPath), Path.GetFileName(romPath));
        if (!load.Success)
        {
            throw new InvalidOperationException($"Invalid rom file. ({load.Error})");
        }

        core.Region = load.Region;
        if (batteryPath is not null)
        {
            // REQ-CORE-095: the battery RAM is imported before the reset, as the application
            // does, so the run starts from the player's saved game.
            core.ImportBatteryRam(File.ReadAllBytes(batteryPath));
        }

        SceneState scene = new();

        // SPEC-05 REQ-LUA-015: the chunks run first, then the profile's settings and the
        // patterns with their BuildPattern hooks, then Start before the first emulated frame.
        using ProfileScriptRuntime runtime = ScriptBridge.Create(profile, engine, core, scene);

        // The frame hooks are timed on their own, which is what the budget of REQ-LUA-110 is
        // about: steps 1 to 5 of REQ-LUA-025, not the conversion around them.
        TimedScriptHost timed = new(runtime);
        engine.ScriptHost = timed;
        engine.LoadProfile(profile);
        ScriptBridge.Apply(runtime, scene);
        runtime.InvokeStart();

        int counter = 0;
        TileRecord[] lastRecords = [];
        byte[] lastPalette = new byte[32];
        core.VideoCallback = (in FrameBatch batch) =>
        {
            if (dumpFrame is not null)
            {
                lastRecords = batch.Records.ToArray();
                batch.Palette.CopyTo(lastPalette);
            }

            engine.Convert(batch, counter);
        };
        core.Reset();
        for (; counter < Math.Max(1, runFrames); counter++)
        {
            core.RunFrame(input.At(counter));
            engine.Publish();
        }

        runtime.InvokeEnd();
        ScriptBridge.Report(runtime, timed);

        if (dumpFrame is not null)
        {
            File.WriteAllText(dumpFrame, FrameJson.Write(lastRecords, lastPalette));
            Console.Out.WriteLine($"render: frame written to {dumpFrame}");
        }

        byte[] picture = core.PixelFrame.ToArray();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Clipping = engine.Settings.Clipping;
        scene.DrawList = engine.BuildDrawList();
        scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
        scene.EmulatedTime = engine.PublishedFrame.Time;

        // AMD-009 (Q-022): without a profile the scene shows the core's pixel frame.
        scene.TwoDimensionalMode = engine.TwoDimensionalMode;
        scene.NesPicture = picture;

        return new SyntheticScene(
            Path.GetFileNameWithoutExtension(romPath),
            scene,
            engine,
            $"{Path.GetFileName(romPath)} after {counter} emulated frames, profile '{profile.Title ?? "2D Mode"}'")
        {
            NesPicture = picture,
        };
    }

    /// <summary>Writes the flat NES picture, one colour index per pixel (REQ-CORE-048).</summary>
    private static void WriteNesPicture(string path, byte[] indices)
    {
        byte[] pixels = new byte[256 * 240 * 4];
        for (int i = 0; i < 256 * 240; i++)
        {
            Rgb24 color = NesPalette.Color(indices[i]);
            pixels[(i * 4) + 0] = color.R;
            pixels[(i * 4) + 1] = color.G;
            pixels[(i * 4) + 2] = color.B;
            pixels[(i * 4) + 3] = 255;
        }

        PngWriter.WriteRgba(path, 256, 240, pixels);
    }

    private static GameProfile LoadProfile(string? name, string? assets)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return GameProfile.CreateEmpty();
        }

        if (File.Exists(name))
        {
            ProfileLoadResult direct = ProfileFormat.Load(File.ReadAllBytes(name), sourceName: name);
            return direct.IsSuccess ? direct.Profile! : throw new InvalidOperationException(direct.Message!);
        }

        string root = LocateAssets(assets)
            ?? throw new InvalidOperationException(
                "assets-neutral was not found; pass --assets <dir> or a profile file path.");
        ProfileCatalogue catalogue = ProfileCatalogue.Load(
            PhysicalProfileFileSystem.Instance,
            Path.Combine(root, "profiles.json"));

        foreach (CatalogueEntry entry in catalogue.Entries)
        {
            if (!string.Equals(entry.Title, name, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            ProfileLoadResult result = catalogue.LoadProfile(PhysicalProfileFileSystem.Instance, entry);
            return result.IsSuccess ? result.Profile! : throw new InvalidOperationException(result.Message!);
        }

        throw new InvalidOperationException($"no built-in profile is titled '{name}'.");
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

    /// <summary>
    /// Renders <paramref name="count"/> frames and returns the median and the 99th percentile
    /// frame time in milliseconds — the measurement REQ-SCN-064 asks for.
    /// </summary>
    private static (double Median, double P99) Bench(
        SceneRenderer renderer,
        OffscreenContext context,
        SyntheticScene scene,
        int count)
    {
        // A few warm-up frames so that the mesh and texture uploads do not count.
        for (int frame = 0; frame < 8; frame++)
        {
            renderer.Render(context.Target, scene.Scene);
        }

        context.Gl.Finish();

        double[] samples = new double[count];
        Stopwatch watch = new();
        for (int frame = 0; frame < count; frame++)
        {
            watch.Restart();
            renderer.Render(context.Target, scene.Scene);
            context.Gl.Finish();
            samples[frame] = watch.Elapsed.TotalMilliseconds;
        }

        Array.Sort(samples);
        return (samples[count / 2], samples[Math.Min(count - 1, (int)(count * 0.99))]);
    }

    private static string Describe(
        SyntheticScene scene,
        RenderStatistics render,
        CaptureStatistics capture,
        (double Median, double P99)? timing,
        int width,
        int height)
    {
        string bench = timing is { } ms
            ? string.Create(
                CultureInfo.InvariantCulture,
                $", \"medianFrameMs\": {ms.Median:F3}, \"p99FrameMs\": {ms.P99:F3}")
            : string.Empty;

        return string.Create(
            CultureInfo.InvariantCulture,
            $"{{ \"scene\": \"{scene.Name}\", \"description\": \"{scene.Description}\", "
                + $"\"width\": {width}, \"height\": {height}, "
                + $"\"opaqueTiles\": {render.OpaqueTiles}, \"transparentTiles\": {render.TransparentTiles}, "
                + $"\"hudTiles\": {render.HudTiles}, \"shadowCasters\": {render.ShadowCasters}, "
                + $"\"drawCalls\": {render.DrawCalls}, \"triangles\": {render.Triangles}, "
                + $"\"residentMeshes\": {render.ResidentMeshes}, "
                + $"\"antiAliasSamples\": {render.AntiAliasSamples}, "
                + $"\"shadowMapSize\": {render.ShadowMapSize}, "
                + $"\"requestedShadowMapSize\": {render.RequestedShadowMapSize}, "
                + $"\"distinctColors\": {capture.DistinctColors}, "
                + $"\"backgroundPixels\": {capture.BackgroundPixels}, "
                + $"\"meanLuma\": {capture.MeanLuma:F4}{bench} }}");
    }

    private static string Next(string[] args, ref int index)
    {
        if (index + 1 >= args.Length)
        {
            throw new ArgumentException($"{args[index]} requires a value.");
        }

        return args[++index];
    }

    private static Rgb24 ParseColor(string value)
    {
        string[] parts = value.Split(',');
        if (parts.Length != 3)
        {
            throw new ArgumentException($"--bg-color expects <r>,<g>,<b>, got '{value}'.");
        }

        return new Rgb24(
            byte.Parse(parts[0], CultureInfo.InvariantCulture),
            byte.Parse(parts[1], CultureInfo.InvariantCulture),
            byte.Parse(parts[2], CultureInfo.InvariantCulture));
    }

    private static System.Numerics.Vector3 ParseAngles(string value)
    {
        string[] parts = value.Split(',');
        if (parts.Length is < 2 or > 3)
        {
            throw new ArgumentException($"--angles expects <rx>,<ry>[,<rz>], got '{value}'.");
        }

        return new System.Numerics.Vector3(
            float.Parse(parts[0], CultureInfo.InvariantCulture),
            float.Parse(parts[1], CultureInfo.InvariantCulture),
            parts.Length > 2 ? float.Parse(parts[2], CultureInfo.InvariantCulture) : 0f);
    }

    private static (int Width, int Height) ParseSize(string value)
    {
        string[] parts = value.Split('x', 'X');
        if (parts.Length != 2
            || !int.TryParse(parts[0], CultureInfo.InvariantCulture, out int width)
            || !int.TryParse(parts[1], CultureInfo.InvariantCulture, out int height)
            || width <= 0
            || height <= 0)
        {
            throw new ArgumentException($"--size expects <width>x<height>, got '{value}'.");
        }

        return (width, height);
    }
}

/// <summary>
/// Pixel statistics of a capture, the kind of measurement the golden-image tests of the lot rely
/// on instead of a byte-for-byte reference image.
/// </summary>
/// <param name="DistinctColors">Number of distinct RGB values.</param>
/// <param name="BackgroundPixels">Pixels equal to the colour of the top-left corner.</param>
/// <param name="MeanLuma">Mean of <c>0.299 R + 0.587 G + 0.114 B</c> over the capture, 0 to 1.</param>
internal readonly record struct CaptureStatistics(int DistinctColors, int BackgroundPixels, double MeanLuma)
{
    /// <summary>Measures a capture given as rows of RGBA bytes.</summary>
    public static CaptureStatistics Measure(ReadOnlySpan<byte> pixels, int width, int height)
    {
        HashSet<int> colors = [];
        int background = 0;
        double luma = 0;
        int corner = Packed(pixels, 0);

        for (int index = 0; index < width * height; index++)
        {
            int packed = Packed(pixels, index);
            colors.Add(packed);
            if (packed == corner)
            {
                background++;
            }

            luma += ((0.299 * ((packed >> 16) & 0xFF))
                + (0.587 * ((packed >> 8) & 0xFF))
                + (0.114 * (packed & 0xFF))) / 255.0;
        }

        return new CaptureStatistics(colors.Count, background, luma / (width * height));
    }

    private static int Packed(ReadOnlySpan<byte> pixels, int index)
    {
        int at = index * 4;
        return (pixels[at] << 16) | (pixels[at + 1] << 8) | pixels[at + 2];
    }
}
