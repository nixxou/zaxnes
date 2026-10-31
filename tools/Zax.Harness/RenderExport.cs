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
          --run <n>                Real frames to run before capturing (default 120).
          --run-ahead <n>          Run-ahead frames N, 0 to 3 (default 0, AMD-041). Every real
                                   frame then emulates N + 1 frames and the capture is the
                                   displayed one. N is written to render.json and to the
                                   '--dump-frame' file: a frame, tile or conversion sequence is
                                   only comparable with another one at equal N (REQ-ARCH-012).
          --sav <file>             Battery-backed PRG-RAM to import before the run, so that a
                                   capture can start from a saved game (REQ-CORE-095).
          --dump-frame <file.json> Write the captured frame as a '--frames' JSON file, so the
                                   same scene can be replayed without the ROM.
          --dump-meshes <file.json> Write one entry per drawn tile instance of the captured
                                   frame — every tile of every active shape the renderer draws,
                                   the HUD included and marked: the tile's screen position as
                                   the core emitted it (the '--dump-frame' convention), its
                                   sprite and HUD flags, its mesh sizes, the bounds of its mesh
                                   in the tile's own centred frame (REQ-E3D-504: a full 8 x 8
                                   tile spans [-4, 4] on x and y and [-D/2, D/2] on z) and after
                                   the object transform of REQ-E3D-406 (bending excluded), the
                                   extrusion depth sizeZ the conversion used, the mesh algorithm
                                   and the shape's tags. Capturing several scenes writes one
                                   file per scene, the name inserted before the extension.
          --press <spec>           Port 1 buttons to hold while replaying, as a comma
                                   separated list of <first>[-<last>]:<buttons>, the frame
                                   numbers counting real frames from the power-on and the
                                   buttons being a|b|select|start|up|down|left|right joined
                                   by '+' (for example 200:start,260:start,400-700:right).
                                   A soft reset or a rollback does not renumber them: the
                                   count is the harness's own, not the core's counter.
          --soft-reset-at <n>      After real frame n, press the console's RESET button:
                                   the soft reset of AMD-044 (REQ-CORE-122, REQ-LOOP-072) as
                                   the application's command applies it — RAM kept, core
                                   frame counter back to 0, rollback history emptied, the
                                   profile not relaunched (no End, no Start, PData and the
                                   learned conversion state untouched).
          --rollback-at <n>        After real frame n, roll back as the application's command
                                   does (AMD-042 REQ-LOOP-069 to 071, REQ-LOOP-030 to 032):
                                   a snapshot at the start of every frame whose core counter
                                   is a multiple of 60, 30 of them at most, and the
                                   second-to-last one restored — core, the 64 PData values
                                   and the camera.
          --frame-log <file.jsonl> One JSON object per real frame, one per line: the real
                                   frame number, the port 1 input word, the core's frame
                                   counter once the frame has run, the non-empty tile count
                                   and the FNV-1a 64 of the frame's tiles as '--dump-frame'
                                   writes them (16 hexadecimal digits).
          --profile <title>        Built-in profile, or a profile file path.
          --assets <dir>           Folder holding profiles.json (profile lookup) and the
                                   cubemaps, meshes and textures the scene uses. Without it
                                   the scene assets are the procedural ones (REQ-PUB-003/004).
          --size <w>x<h>           Capture size in pixels (default 640x480).
          --fov <degrees>          Field of view, 45 to 95. Left out, the scene keeps what the
                                   profile and its scripts set, or 70.
          --angles <rx>,<ry>[,<rz>]  Camera setpoint angles in degrees (default 0.81,0.81,0).
          --distance <d>           Camera setpoint distance (default 132.7964 / tan(FOV/2),
                                   AMD-045 REQ-SCN-069).
          --mesh <algorithm>       greedy (default), marching or multicolor (REQ-E3D-340).
          --no-shadows             Render with the shadows off.
          --shadow-map <texels>    Side of the shadow map (default 4096, AMD-034). A size the
                                   hardware refuses is halved until it accepts one, and the
                                   granted size is reported.
          --anti-alias <n>         video.anti_alias: 0 off (default), 1 MSAA 4x, 2 MSAA 8x
                                   (AMD-032). The granted sample count is reported.
          --bend <lr,tb,arc[,c]>   Space bending in degrees, −180 to 180 by steps of 30, and
                                   the arc centre 0 (screen) or 1 (bottom edge) (AMD-043
                                   REQ-SCN-066).
          --flat-2d                scene.bend_2d_mode off: the 2D-mode rectangle stays at its
                                   flat position under the bending (AMD-046 REQ-SCN-074).
          --skybox <name>          default, color, gradient or fantasy (REQ-SCN-049). Left out,
                                   the scene keeps what the profile and its scripts set.
          --bg-color <r,g,b>       The custom background colour, 0-255 (REQ-SCN-046). Left out,
                                   the scene keeps what the profile and its scripts set.
          --no-nes-bg-color        scene.use_nes_bg_color off: the background of the scene is
                                   the custom colour instead of the universal NES one
                                   (REQ-SCN-046). This is what makes the transparency of the
                                   2D-mode rectangle visible (AMD-050 REQ-SCN-076), which the
                                   default configuration hides.
          --real-time <seconds>    Real time the star twinkle uses (REQ-SCN-050).
          --sky-bodies             Light the procedural sun, moon and clouds of the skybox,
                                   which REQ-SCN-050 leaves off by default (REQ-PUB-003, Q-064).
          --bench <frames>         Also time that many frames per scene and report them.
          --explain                Print the draw list grouped by 3D object, with the NES
                                   colours each group's texels resolve to (colour fidelity).
          --list-scenes            Print the built-in synthetic scene names and exit.

        Scenes: framing, sky, shadow, no-shadow, layer-shadow-off, nudge, alpha, hud, clip,
                backdrop, heavy, picture.
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
        string? dumpMeshes = null;
        int? softResetAt = null;
        int? rollbackAt = null;
        string? frameLog = null;
        int runFrames = 120;
        int runAhead = 0;
        int width = 640;
        int height = 480;
        float? fov = null;
        bool shadows = true;
        int shadowMapSize = SceneConstants.ShadowMapSize;
        int antiAlias = 0;
        bool explain = false;
        int bench = 0;
        SkyboxKind? skybox = null;
        (int Lr, int Tb, int Arc, int Centre) bend = default;
        bool bend2dMode = true;
        Rgb24? customBackground = null;
        bool? nesBackground = null;
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

                case "--run-ahead":
                    runAhead = int.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    if (runAhead is < 0 or > 3)
                    {
                        throw new ArgumentException("--run-ahead expects 0, 1, 2 or 3 (AMD-041 REQ-LOOP-064).");
                    }

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

                case "--dump-meshes":
                    dumpMeshes = Next(args, ref i);
                    break;

                // AMD-044 REQ-LOOP-072 and AMD-042 REQ-LOOP-070, applied after the real frame
                // they name (see RomRunControl).
                case "--soft-reset-at":
                    softResetAt = int.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--rollback-at":
                    rollbackAt = int.Parse(Next(args, ref i), CultureInfo.InvariantCulture);
                    break;

                case "--frame-log":
                    frameLog = Next(args, ref i);
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

                case "--bend":
                    bend = ParseBend(Next(args, ref i));
                    break;

                // AMD-046 REQ-SCN-074: « Bend 2D Mode » off, which keeps the 2D-mode rectangle
                // at its flat position.
                case "--flat-2d":
                    bend2dMode = false;
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

                case "--no-nes-bg-color":
                    nesBackground = false;
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

        RomRunControl control = new()
        {
            SoftResetAt = softResetAt,
            RollbackAt = rollbackAt,
            FrameLogPath = frameLog,
        };

        if (rom is null && !control.IsEmpty)
        {
            Console.Error.WriteLine(
                "render: --soft-reset-at, --rollback-at and --frame-log replay real frames and "
                    + "therefore need --rom.");
            return 2;
        }

        Directory.CreateDirectory(output);

        List<SyntheticScene> scenes;
        try
        {
            scenes = rom is null
                ? BuildScenes(frames, profile, assets, mesh)
                : [FromRom(
                    rom,
                    runFrames,
                    profile,
                    assets,
                    mesh,
                    InputScript.Parse(press),
                    battery,
                    dumpFrame,
                    runAhead,
                    control)];
        }
        catch (Exception ex) when (ex is ArgumentException or IOException or JsonException
            or InvalidOperationException or ProfileCatalogueException)
        {
            Console.Error.WriteLine($"render: {ex.Message}");
            return 1;
        }

        // The options the user really passed, applied after the scene is built and therefore last
        // of the four layers (see SceneOverrides).
        SceneOverrides overrides = new()
        {
            Fov = fov,
            Angles = angles,
            Distance = distance,
            Skybox = skybox,
            CustomBackground = customBackground,
            NesBackground = nesBackground,
            Shadows = shadows,
            Bend = bend,
            Bend2dMode = bend2dMode,
            AntiAlias = antiAlias,
            ShadowMapSize = shadowMapSize,
            RealTime = realTime,
            SkyBodies = skyBodies,
        };

        foreach (SyntheticScene scene in scenes)
        {
            overrides.ApplyTo(scene);
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

            // The per-tile geometry of the very frame that was just captured: the draw list the
            // renderer consumed, written out tile by tile (SPEC-04B REQ-E3D-641).
            if (dumpMeshes is not null && scene.Scene.DrawList is { } meshes)
            {
                string dump = MeshDump.PathFor(dumpMeshes, scene.Name, scenes.Count == 1);
                MeshDump.WriteFile(dump, meshes, scene.Engine.Settings.RenderAlgorithm, scene.Name);
                Console.Out.WriteLine(
                    $"render: {meshes.Tiles.Count} tile mesh(es) written to {dump}");
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

            report.Add(Describe(scene, renderer.Statistics, statistics, measured, width, height, runAhead));

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
    /// Runs a real ROM headless for <paramref name="runFrames"/> real frames and captures the
    /// last displayed one: the "load a ROM and a profile, replay, export an offscreen capture" of
    /// SPEC-01 REQ-ARCH-031. The controllers stay idle; replaying an input sequence comes with
    /// the input lot.
    /// </summary>
    /// <remarks>
    /// <c>runAhead</c> is the N of AMD-041 REQ-LOOP-065. 0 runs one frame per real frame, which
    /// is what the harness did before the amendment and what a libretro frontend does (§4).
    /// </remarks>
    private static SyntheticScene FromRom(
        string romPath,
        int runFrames,
        string? profileName,
        string? assets,
        RenderAlgorithm mesh,
        InputScript input,
        string? batteryPath,
        string? dumpFrame,
        int runAhead,
        RomRunControl? control)
    {
        GameProfile profile = LoadProfile(profileName, assets, out string label);
        return FromRom(
            romPath,
            runFrames,
            profile,
            label,
            mesh,
            input,
            batteryPath,
            dumpFrame,
            runAhead,
            control);
    }

    /// <summary>
    /// The same run on a profile the caller already holds, <c>profileLabel</c> being how the
    /// capture names it: its title, or « 2D Mode » when there is none.
    /// </summary>
    internal static SyntheticScene FromRom(
        string romPath,
        int runFrames,
        GameProfile profile,
        string profileLabel,
        RenderAlgorithm mesh,
        InputScript input,
        string? batteryPath = null,
        string? dumpFrame = null,
        int runAhead = 0,
        RomRunControl? control = null)
    {
        ArgumentNullException.ThrowIfNull(profile);
        control?.Validate(runFrames);
        ConversionEngine engine = new();
        engine.UseGeometry();
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
        using ProfileScriptRuntime runtime = ScriptBridge.Create(profile, engine, core, scene, mesh);

        // The frame hooks are timed on their own, which is what the budget of REQ-LUA-110 is
        // about: steps 1 to 5 of REQ-LUA-025, not the conversion around them.
        TimedScriptHost timed = new(runtime);
        engine.ScriptHost = timed;

        // REQ-LUA-015 step 2, REQ-PROF-041 and REQ-PERS-005: the profile's own P settings land in
        // the settings model, which mirrors them into the engine slice and pushes them into the
        // scene through the bridge connected above — the light direction and colour, the top-down
        // camera and the clip margins included. This is what GameSession.Create does, and it runs
        // *after* the chunks so that an assignment made at the main level of a chunk is
        // overwritten, as REQ-LUA-015 demands. A capture always loads the profile's data, so this
        // is always the "different game" case of REQ-PERS-005.
        runtime.Settings.Model.ApplyProfile(
            profile.Settings is { } embedded
                ? ProfileSettingApplication.ToSettingValues(embedded)
                : []);

        // Step 3. The settings were applied above, through the model, so the engine must not copy
        // them a second time.
        engine.LoadProfile(profile, applySettings: false);

        // REQ-LUA-015 step 4. ScriptBridge.Create connected the settings to the scene, so every
        // write Start or a later hook makes already reaches it. The push before the hook is the
        // application's (GameSession.ApplyProfileScene) and carries the P settings applied above;
        // the one after it costs nothing and covers a write that landed on an already equal value
        // and therefore raised no change.
        ScriptBridge.Push(runtime, scene);
        runtime.InvokeStart();
        ScriptBridge.Push(runtime, scene);

        // The session's frame counter, the one GameSession keeps: it feeds the conversion and
        // the Lua globals « frame » and « time », goes back to 0 on a soft reset (AMD-044
        // REQ-CORE-122) and is left alone by a rollback, exactly as in the application. The real
        // frame count below is the harness's own and never restarts.
        int sessionFrame = 0;
        TileRecord[] lastRecords = [];
        byte[] lastPalette = new byte[32];
        FrameLog? log = control?.FrameLogPath is not null ? new FrameLog() : null;
        (int Tiles, ulong Hash) measured = FrameLog.Empty;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            if (dumpFrame is not null)
            {
                lastRecords = batch.Records.ToArray();
                batch.Palette.CopyTo(lastPalette);
            }

            if (log is not null)
            {
                measured = FrameLog.Measure(batch.Records);
            }

            engine.Convert(batch, sessionFrame);
        };
        core.Reset();

        // The five steps of AMD-041 REQ-LOOP-065, the same ones the desktop loop runs: the real
        // frame silent and invisible, its state kept, N − 1 more hidden frames, the displayed
        // frame with its image and its sound, then the real state back. With N = 0 this is one
        // ordinary frame, exactly as before.
        Zax.Nes.State.CoreState real = new();
        HarnessRollbackHistory history = new();
        int realFrames = Math.Max(1, runFrames);
        int counter = 0;
        for (; counter < realFrames; counter++)
        {
            // REQ-LOOP-030 and AMD-042 REQ-LOOP-069: the snapshot is taken at the start of a
            // frame whose core frame counter is a multiple of 60 — so before that frame runs,
            // the core being in its real state — and carries the core, the 64 PData values and
            // the camera state, read as they stand at that instant and with no script hook
            // involved (AMD-042 §3).
            if (control is { KeepsHistory: true }
                && HarnessRollbackHistory.IsSnapshotFrame(core.FrameCount))
            {
                history.Capture(new HarnessRollbackSnapshot(
                    core.SaveState(includeRomImage: false),
                    runtime.CapturePersistentData(),
                    scene.Camera.Capture()));
            }

            ushort word = input.At(counter);
            if (runAhead <= 0)
            {
                core.RunFrame(word, FrameOutput.All);
            }
            else
            {
                core.RunFrame(word, FrameOutput.None);
                core.CaptureState(real);
                for (int ahead = 1; ahead < runAhead; ahead++)
                {
                    core.RunFrame(word, FrameOutput.None);
                }

                core.RunFrame(word, FrameOutput.All);
                core.RestoreState(real);
            }

            engine.Publish();
            sessionFrame++;
            log?.Add(counter, word, core.FrameCount, measured.Tiles, measured.Hash);

            // The two commands of the game menu, applied between two real frames — the moment
            // REQ-LOOP-007 reserves for them — so the --press script keeps counting real frames
            // across them.
            if (control?.SoftResetAt == counter)
            {
                SoftReset(core, engine, history, ref sessionFrame, counter);
            }

            if (control?.RollbackAt == counter)
            {
                Rollback(core, engine, runtime, scene, history, counter);
            }
        }

        // AMD-042 REQ-LOOP-069 reads them the way GetPData(i) does. They are taken before the
        // End hooks run, so what the capture reports is the state of the frame it shows.
        float[] persistentData = PersistentData.Read(runtime.CapturePersistentData());

        runtime.InvokeEnd();
        ScriptBridge.Report(runtime, timed);

        if (dumpFrame is not null)
        {
            File.WriteAllText(dumpFrame, FrameJson.Write(lastRecords, lastPalette, runAhead));
            Console.Out.WriteLine($"render: frame written to {dumpFrame}");
        }

        if (log is not null && control?.FrameLogPath is { } logPath)
        {
            log.Write(logPath);
            Console.Out.WriteLine($"render: {log.Lines.Count} frame(s) logged to {logPath}");
        }

        byte[] picture = core.PixelFrame.ToArray();
        byte[] opacity = core.OpacityMask.ToArray();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Clipping = engine.Settings.Clipping;
        scene.DrawList = engine.BuildDrawList();
        scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
        scene.EmulatedTime = engine.PublishedFrame.Time;

        // AMD-009 (Q-022): without a profile the scene shows the core's pixel frame. AMD-050
        // REQ-SCN-076: the opacity mask of REQ-CORE-049 goes with it, so the pixels that carry
        // no opaque pixel are not drawn.
        scene.TwoDimensionalMode = engine.TwoDimensionalMode;
        scene.NesPicture = picture;
        scene.NesOpacityMask = opacity;

        return new SyntheticScene(
            Path.GetFileNameWithoutExtension(romPath),
            scene,
            engine,
            $"{Path.GetFileName(romPath)} after {counter} real frames at run-ahead {runAhead}, "
                + $"profile '{profileLabel}'")
        {
            NesPicture = picture,
            NesOpacityMask = opacity,
            PersistentData = persistentData,
            Core = core,
            FrameLog = log,
        };
    }

    /// <summary>
    /// The soft reset of AMD-044 REQ-LOOP-072, as <c>EmulationHost.SoftResetGame</c> applies it:
    /// the core imitates a press on the RESET button (REQ-CORE-122), the session's frame counter
    /// goes back to 0, the converted frames in flight are dropped (REQ-E3D-281) and the rollback
    /// history is emptied (REQ-LOOP-031).
    /// </summary>
    /// <remarks>
    /// What it does <b>not</b> do is relaunch the profile: the ROM is not read again, no
    /// <c>End</c> and no <c>Start</c> hook runs, <c>PData</c> is untouched and the learned
    /// conversion state stays. The pause, the pacing clock and the fast forward the application
    /// also resets have no counterpart in a headless replay.
    /// </remarks>
    private static void SoftReset(
        NesCore core,
        ConversionEngine engine,
        HarnessRollbackHistory history,
        ref int sessionFrame,
        int realFrame)
    {
        core.SoftReset();
        sessionFrame = 0;
        engine.ClearFrames();
        history.Clear();
        Console.Out.WriteLine(
            $"render: soft reset after real frame {realFrame} (AMD-044 REQ-LOOP-072); "
                + "core frame counter back to 0, RAM kept");
    }

    /// <summary>
    /// The rollback of REQ-LOOP-032 and AMD-042 REQ-LOOP-070, as
    /// <c>EmulationHost.PerformRollback</c> applies it: the second-to-last snapshot is restored
    /// — the core, then the 64 <c>PData</c> values, then the camera — and both it and the last
    /// snapshot leave the history.
    /// </summary>
    /// <remarks>
    /// The <c>Save</c> and <c>Load</c> hooks are never called: the entries go back the way
    /// <c>SetPData</c> writes them, and the camera goes through <c>SceneCamera.Restore</c>, the
    /// one path a savestate load also uses, so that the "shown straight at its setpoints" of
    /// AMD-045 REQ-SCN-073 applies here too.
    /// </remarks>
    private static void Rollback(
        NesCore core,
        ConversionEngine engine,
        ProfileScriptRuntime runtime,
        SceneState scene,
        HarnessRollbackHistory history,
        int realFrame)
    {
        if (history.TakeRollbackTarget() is not { } snapshot)
        {
            // REQ-LOOP-032, notes: with an empty history nothing is restored, and the rest of
            // step 3 has no headless counterpart.
            Console.Out.WriteLine(
                $"render: rollback after real frame {realFrame}: the history is empty, "
                    + "nothing restored (REQ-LOOP-032, notes)");
            return;
        }

        if (!core.TryLoadState(snapshot.Core, 0, out _, out _, out string? error))
        {
            Console.Error.WriteLine($"render: rollback snapshot refused: {error}");
            return;
        }

        engine.ClearFrames();
        runtime.RestorePersistentData(snapshot.PersistentData);
        scene.Camera.Restore(snapshot.Camera);
        Console.Out.WriteLine(
            $"render: rollback after real frame {realFrame} (AMD-042 REQ-LOOP-070) to core "
                + $"frame {core.FrameCount}; {history.Count} snapshot(s) left");
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
        => LoadProfile(name, assets, out _);

    /// <summary>
    /// Loads the profile <paramref name="name"/> designates and, in
    /// <paramref name="label"/>, how a capture names it. « 2D Mode » is the label of the
    /// <b>absent</b> profile only: a built-in one takes its title from the catalogue, not from
    /// its metadata fields, which are usually empty (REQ-PROF-022, REQ-PROF-060).
    /// </summary>
    internal static GameProfile LoadProfile(string? name, string? assets, out string label)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            label = ProfileCatalogue.TwoDimensionalModeTitle;
            return GameProfile.CreateEmpty();
        }

        if (File.Exists(name))
        {
            ProfileLoadResult direct = ProfileFormat.Load(File.ReadAllBytes(name), sourceName: name);
            GameProfile loaded = direct.IsSuccess
                ? direct.Profile!
                : throw new InvalidOperationException(direct.Message!);

            // A profile given by path carries its own title; without one the file name is what
            // the user asked for.
            label = loaded.Title is { Length: > 0 } title
                ? title
                : Path.GetFileNameWithoutExtension(name);
            return loaded;
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
            label = entry.Title;
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
        int height,
        int runAhead)
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

                // AMD-041: the run-ahead the frames were produced with is part of the export, so
                // two sequences are never compared at different N (REQ-ARCH-012, REQ-ARCH-013).
                + $"\"runAhead\": {runAhead}, "
                + $"\"opaqueTiles\": {render.OpaqueTiles}, \"transparentTiles\": {render.TransparentTiles}, "
                + $"\"hudTiles\": {render.HudTiles}, \"shadowCasters\": {render.ShadowCasters}, "
                + $"\"drawCalls\": {render.DrawCalls}, \"triangles\": {render.Triangles}, "
                + $"\"residentMeshes\": {render.ResidentMeshes}, "
                + $"\"antiAliasSamples\": {render.AntiAliasSamples}, "
                + $"\"shadowMapSize\": {render.ShadowMapSize}, "
                + $"\"requestedShadowMapSize\": {render.RequestedShadowMapSize}, "
                + $"\"distinctColors\": {capture.DistinctColors}, "
                + $"\"backgroundPixels\": {capture.BackgroundPixels}, "
                + $"\"meanLuma\": {capture.MeanLuma:F4}{bench}, "
                + $"{CameraJson(scene.Scene.Camera)}{PdataJson(scene.PersistentData)} }}");
    }

    /// <summary>
    /// The camera of the captured frame: its mode, its field of view, the setpoint distance and
    /// angles of that mode, and the framed distance d₀ of AMD-045 REQ-SCN-069 — the value a view
    /// reset would write at this FOV, before the [0, 512] bound of REQ-SCN-011, which is also
    /// what <c>Camera.ResetDistance</c> reads (REQ-LUA-113).
    /// </summary>
    internal static string CameraJson(SceneCamera camera)
    {
        ArgumentNullException.ThrowIfNull(camera);
        return string.Create(
            CultureInfo.InvariantCulture,
            $"\"camera\": {{ \"mode\": \"{camera.Mode}\", \"fov\": {camera.Fov:0.######}, "
                + $"\"setpointDistance\": {camera.Current.Distance:0.######}, "
                + $"\"framedDistance\": {camera.ResetDistance:0.######}, "
                + $"\"angles\": [{camera.Current.Angles.X:0.######}, "
                + $"{camera.Current.Angles.Y:0.######}, {camera.Current.Angles.Z:0.######}] }}");
    }

    /// <summary>
    /// The 64 persistent values of the captured frame in the order Lua reads them, so that
    /// <c>pdata[0]</c> of the export is <c>PData[1]</c> of the script (SPEC-05 §4.6). A value
    /// Lua left non-finite is written as null, which keeps the file readable as JSON.
    /// </summary>
    internal static string PdataJson(float[]? values)
    {
        if (values is null)
        {
            return string.Empty;
        }

        IEnumerable<string> entries = values.Select(static value => float.IsFinite(value)
            ? string.Create(CultureInfo.InvariantCulture, $"{value:0.######}")
            : "null");
        return ", \"pdata\": [" + string.Join(", ", entries) + "]";
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

    /// <summary>
    /// The four curvature settings of AMD-043 REQ-SCN-066, as
    /// <c>&lt;lr&gt;,&lt;tb&gt;,&lt;arc&gt;[,&lt;centre&gt;]</c>.
    /// </summary>
    private static (int Lr, int Tb, int Arc, int Centre) ParseBend(string value)
    {
        string[] parts = value.Split(',', StringSplitOptions.TrimEntries);
        if (parts.Length is < 3 or > 4)
        {
            throw new ArgumentException($"--bend expects <lr>,<tb>,<arc>[,<centre>], got '{value}'.");
        }

        int Degrees(int index) => int.Parse(parts[index], CultureInfo.InvariantCulture);
        return (
            Degrees(0),
            Degrees(1),
            Degrees(2),
            parts.Length == 4 ? Degrees(3) : 0);
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
/// The scene values a <c>render</c> command line overrides, applied once the scene is built and
/// therefore last of the four layers that decide what a capture shows: the scene's own defaults,
/// the profile's embedded P settings (REQ-PROF-041), the writes its scripts make, then these.
/// </summary>
/// <remarks>
/// A nullable field left null means the option was not given, which keeps what the profile and
/// its scripts decided (REQ-SCN-049, REQ-SCN-046, REQ-SCN-005); the non-nullable ones default to
/// the scene's own values, so a capture without a profile is unchanged.
/// </remarks>
internal sealed class SceneOverrides
{
    /// <summary><c>--fov</c>, in degrees.</summary>
    public float? Fov { get; init; }

    /// <summary><c>--angles</c>, the camera setpoint angles in degrees.</summary>
    public System.Numerics.Vector3? Angles { get; init; }

    /// <summary><c>--distance</c>, the camera setpoint distance.</summary>
    public float? Distance { get; init; }

    /// <summary><c>--skybox</c> (REQ-SCN-049).</summary>
    public SkyboxKind? Skybox { get; init; }

    /// <summary><c>--bg-color</c>, the custom background colour (REQ-SCN-046).</summary>
    public Rgb24? CustomBackground { get; init; }

    /// <summary>
    /// False for <c>--no-nes-bg-color</c>: the setting <c>scene.use_nes_bg_color</c>
    /// (REQ-SCN-046), which AMD-050 REQ-SCN-076 needs off to show the transparency.
    /// </summary>
    public bool? NesBackground { get; init; }

    /// <summary>False for <c>--no-shadows</c>.</summary>
    public bool Shadows { get; init; } = true;

    /// <summary><c>--bend</c>, the four curvature settings (AMD-043 REQ-SCN-066).</summary>
    public (int Lr, int Tb, int Arc, int Centre) Bend { get; init; }

    /// <summary>False for <c>--flat-2d</c> (AMD-046 REQ-SCN-074).</summary>
    public bool Bend2dMode { get; init; } = true;

    /// <summary><c>--anti-alias</c>: 0 off, 1 MSAA 4x, 2 MSAA 8x (AMD-032).</summary>
    public int AntiAlias { get; init; }

    /// <summary><c>--shadow-map</c>, the side of the shadow map (AMD-034).</summary>
    public int ShadowMapSize { get; init; } = SceneConstants.ShadowMapSize;

    /// <summary><c>--real-time</c>, the real time the star twinkle uses (REQ-SCN-050).</summary>
    public float RealTime { get; init; }

    /// <summary><c>--sky-bodies</c> (Q-064).</summary>
    public bool SkyBodies { get; init; }

    /// <summary>Applies the overrides to one built scene.</summary>
    public void ApplyTo(SyntheticScene scene)
    {
        ArgumentNullException.ThrowIfNull(scene);

        if (Fov is { } degrees)
        {
            scene.Scene.Settings.Fov = degrees;
            scene.Scene.Camera.Fov = degrees;
        }

        if (Angles is { } r)
        {
            scene.Scene.Camera.Current.Angles = r;
        }

        if (Distance is { } d)
        {
            scene.Scene.Camera.Current.Distance = d;
        }

        scene.Scene.Camera.SnapToSetpoints();
        if (Skybox is { } sky)
        {
            scene.Scene.Settings.Skybox = sky;
        }

        // AMD-043 REQ-SCN-066: the four curvature settings, as the options window writes them.
        scene.Scene.Settings.BendLeftRight = Bend.Lr;
        scene.Scene.Settings.BendTopBottom = Bend.Tb;
        scene.Scene.Settings.ArcHorizontal = Bend.Arc;
        scene.Scene.Settings.ArcAtEdge = Bend.Centre != 0;
        scene.Scene.Settings.Bend2dMode = Bend2dMode;
        if (CustomBackground is { } colour)
        {
            scene.Scene.Settings.CustomBackgroundColor = colour;
        }

        if (NesBackground is { } nesBackground)
        {
            scene.Scene.UseNesBackgroundColor = nesBackground;
        }

        scene.Scene.RealTime = RealTime;

        // Q-064: the three bodies exist and are parametric, but REQ-SCN-050 leaves them off;
        // this is what a capture turns them on with.
        if (SkyBodies)
        {
            scene.Scene.Settings.SkyBodies.SunGain = 1f;
            scene.Scene.Settings.SkyBodies.MoonGain = 1f;
            scene.Scene.Settings.SkyBodies.CloudGain = 0.6f;
            scene.Scene.Settings.SkyBodies.CloudCoverage = 0.3f;
        }

        if (!Shadows)
        {
            scene.Scene.Settings.Shadows = false;
            scene.Engine.Settings.Shadow = false;
        }

        // AMD-032 §2: video.anti_alias as the application applies it.
        scene.Scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(AntiAlias);
        scene.Scene.Settings.ShadowMapSize = ShadowMapSize;
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
