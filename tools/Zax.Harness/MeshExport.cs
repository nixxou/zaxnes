using System.Globalization;
using System.Text;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;

namespace Zax.Harness;

/// <summary>
/// The <c>meshes</c> command: loads a profile, feeds it frames and exports the geometry the
/// 3D engine produces as Wavefront OBJ plus a JSON instance list (SPEC-04B REQ-E3D-640,
/// REQ-E3D-641; REQ-ARCH-031).
/// </summary>
/// <remarks>
/// <para>
/// No ROM and no window are involved: the frames are either synthetic — the tiles of the
/// profile's own patterns laid out over the background grid, so that the recognition of
/// SPEC-04 matches them — or read from a frame dump, a small text format documented in
/// <see cref="Usage"/>.
/// </para>
/// <para>
/// The scene frame of REQ-E3D-400 is left handed. By default the OBJ is written right handed
/// (<c>z' = −z</c>, winding reversed) so that any viewer shows it the right way round;
/// <c>--handedness left</c> writes the raw scene coordinates instead.
/// </para>
/// </remarks>
internal static class MeshExport
{
    /// <summary>UTF-8 without a byte order mark, so the exports stay plain text.</summary>
    private static readonly UTF8Encoding Utf8 = new(encoderShouldEmitUTF8Identifier: false);

    /// <summary>The command line of the command.</summary>
    public static string Usage => """
        Zax.Harness meshes --profile <file|name> [options]

          --profile <p>     a built-in profile of assets-neutral/profiles (name or path),
                            or any .3dn / .bytes file
          --out <dir>       output directory (default: mesh-export next to the profile name)
          --algorithm <a>   global mesh algorithm: greedy (default), marching, multicolor
          --frames <file>   a frame dump to replay instead of the synthetic frames
          --frame-count <n> number of synthetic frames (default 1)
          --max-patterns <n> patterns used by the synthetic frames (default: as many as fit)
          --handedness <h>  right (default) or left; see the remarks
          --local           also export each 3D object's tile meshes in their local frame

        Output, in <dir>:
          summary.json          profile, per-geometry counts, mesh totals, build time
          frame<NNNN>.obj       the drawn tiles of that frame, placed in the scene
          frame<NNNN>.json      the tiles to draw of REQ-E3D-641, one entry per tile
          objects.obj           with --local, every 3D object's tile meshes, one group each

        Frame dump format (one directive per line, # starts a comment):
          palette <64 hex chars>                      the 32 PPU palette bytes
          bg <x> <y> <pal> <low16> <high16> <fx> <fy>  a background tile
          sp <x> <y> <pal> <low16> <high16> <fx> <fy> <hidden>  a sprite tile
          end                                         closes the frame and starts the next
        """;

    /// <summary>Runs the command.</summary>
    /// <param name="arguments">The arguments after the command name.</param>
    /// <returns>The process exit code.</returns>
    public static int Run(string[] arguments)
    {
        Options options;
        try
        {
            options = Options.Parse(arguments);
        }
        catch (ArgumentException error)
        {
            Console.Error.WriteLine(error.Message);
            Console.Error.WriteLine();
            Console.Error.WriteLine(Usage);
            return 2;
        }

        string? path = ResolveProfile(options.Profile);
        if (path is null)
        {
            Console.Error.WriteLine($"profile not found: {options.Profile}");
            return 2;
        }

        ProfileLoadResult loaded = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
        if (!loaded.IsSuccess || loaded.Profile is null)
        {
            Console.Error.WriteLine($"{path}: {loaded.Message}");
            return 1;
        }

        Directory.CreateDirectory(options.Output);

        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Settings.RenderAlgorithm = options.Algorithm;

        System.Diagnostics.Stopwatch watch = System.Diagnostics.Stopwatch.StartNew();
        engine.LoadProfile(loaded.Profile);
        watch.Stop();

        List<DumpFrame> frames = options.Frames is { } dump
            ? FrameDump.Read(dump)
            : SyntheticFrames.Build(engine, options.FrameCount, options.MaxPatterns);

        int exported = 0;
        int tiles = 0;
        for (int n = 0; n < frames.Count; n++)
        {
            DumpFrame frame = frames[n];
            ConversionFrame converted = frame.Convert(engine, n);
            engine.Publish();
            FrameDrawList list = engine.BuildDrawList(converted);

            string stem = Path.Combine(options.Output, $"frame{n:0000}");
            File.WriteAllText($"{stem}.obj", ObjWriter.Scene(list, options.RightHanded), Utf8);
            File.WriteAllText($"{stem}.json", JsonWriter.Frame(list, n), Utf8);
            exported++;
            tiles += list.Tiles.Count;
        }

        if (options.Local)
        {
            File.WriteAllText(
                Path.Combine(options.Output, "objects.obj"),
                ObjWriter.Objects(engine, geometry, options.RightHanded),
                Utf8);
        }

        string summary = JsonWriter.Summary(
            Path.GetFileName(path),
            loaded.Profile,
            engine,
            geometry,
            options,
            watch.Elapsed.TotalMilliseconds,
            exported,
            tiles);
        File.WriteAllText(Path.Combine(options.Output, "summary.json"), summary, Utf8);

        Console.Out.WriteLine(summary);
        return 0;
    }

    /// <summary>
    /// Finds a profile: a path as given, or a file of <c>assets-neutral/profiles</c> by name,
    /// with or without its extension.
    /// </summary>
    private static string? ResolveProfile(string profile)
    {
        if (File.Exists(profile))
        {
            return profile;
        }

        string? directory = NeutralProfiles.Directory;
        if (directory is null)
        {
            return null;
        }

        foreach (string candidate in new[] { profile, profile + ".bytes", profile + ".3dn" })
        {
            string full = Path.Combine(directory, candidate);
            if (File.Exists(full))
            {
                return full;
            }
        }

        return null;
    }

    /// <summary>The parsed command line.</summary>
    internal sealed class Options
    {
        private Options(string profile)
        {
            Profile = profile;
            Output = Path.Combine("mesh-export", Path.GetFileNameWithoutExtension(profile));
        }

        /// <summary>The profile to load.</summary>
        public string Profile { get; }

        /// <summary>The output directory.</summary>
        public string Output { get; private set; }

        /// <summary>The global mesh algorithm (REQ-E3D-500).</summary>
        public RenderAlgorithm Algorithm { get; private set; } = RenderAlgorithm.Greedy;

        /// <summary>A frame dump to replay, or null for synthetic frames.</summary>
        public string? Frames { get; private set; }

        /// <summary>Number of synthetic frames.</summary>
        public int FrameCount { get; private set; } = 1;

        /// <summary>Patterns the synthetic frames may use; 0 means "as many as fit".</summary>
        public int MaxPatterns { get; private set; }

        /// <summary>True to write the OBJ in a right-handed frame.</summary>
        public bool RightHanded { get; private set; } = true;

        /// <summary>True to also export the local tile meshes of every 3D object.</summary>
        public bool Local { get; private set; }

        /// <summary>Parses the arguments.</summary>
        /// <exception cref="ArgumentException">An argument is missing or unknown.</exception>
        public static Options Parse(string[] arguments)
        {
            ArgumentNullException.ThrowIfNull(arguments);

            string? profile = null;
            Dictionary<string, string> values = [];
            List<string> flags = [];

            for (int n = 0; n < arguments.Length; n++)
            {
                string argument = arguments[n];
                if (!argument.StartsWith("--", StringComparison.Ordinal))
                {
                    throw new ArgumentException($"unexpected argument: {argument}");
                }

                string key = argument[2..];
                if (string.Equals(key, "local", StringComparison.Ordinal))
                {
                    flags.Add(key);
                    continue;
                }

                if (n + 1 >= arguments.Length)
                {
                    throw new ArgumentException($"--{key} needs a value");
                }

                values[key] = arguments[++n];
            }

            if (!values.TryGetValue("profile", out profile))
            {
                throw new ArgumentException("--profile is required");
            }

            Options options = new(profile);
            if (values.TryGetValue("out", out string? output))
            {
                options.Output = output;
            }

            if (values.TryGetValue("algorithm", out string? algorithm))
            {
                options.Algorithm = algorithm.ToLowerInvariant() switch
                {
                    "greedy" => RenderAlgorithm.Greedy,
                    "marching" => RenderAlgorithm.Marching,
                    "multicolor" or "multicolour" => RenderAlgorithm.GreedyMulticolor,
                    _ => throw new ArgumentException($"unknown algorithm: {algorithm}"),
                };
            }

            if (values.TryGetValue("frames", out string? frames))
            {
                options.Frames = frames;
            }

            if (values.TryGetValue("frame-count", out string? count))
            {
                options.FrameCount = Math.Max(1, int.Parse(count, CultureInfo.InvariantCulture));
            }

            if (values.TryGetValue("max-patterns", out string? patterns))
            {
                options.MaxPatterns = Math.Max(0, int.Parse(patterns, CultureInfo.InvariantCulture));
            }

            if (values.TryGetValue("handedness", out string? handedness))
            {
                options.RightHanded = handedness.ToLowerInvariant() switch
                {
                    "right" => true,
                    "left" => false,
                    _ => throw new ArgumentException($"unknown handedness: {handedness}"),
                };
            }

            options.Local = flags.Contains("local");
            return options;
        }
    }
}

/// <summary>Locates <c>assets-neutral/profiles</c> by walking up from the binary.</summary>
internal static class NeutralProfiles
{
    /// <summary>The folder, or null when the extracted assets are not next to the solution.</summary>
    public static string? Directory { get; } = Locate();

    private static string? Locate()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName, "assets-neutral", "profiles");
            if (System.IO.Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}

/// <summary>One frame to feed the engine: tile records and the 32 palette bytes.</summary>
internal sealed class DumpFrame
{
    private readonly List<TileRecord> _records = [];
    private readonly byte[] _palette = new byte[32];

    /// <summary>The records, in emission order.</summary>
    public IReadOnlyList<TileRecord> Records => _records;

    /// <summary>Sets one of the 32 PPU palette bytes.</summary>
    public void SetPalette(int index, byte value)
    {
        if ((uint)index < 32u)
        {
            _palette[index] = value;
        }
    }

    /// <summary>Adds a background tile.</summary>
    public void AddBackground(TileImageKey image, int x, int y, int palette)
        => Add(image, x, y, palette, background: true, hidden: false);

    /// <summary>Adds a sprite tile.</summary>
    public void AddSprite(TileImageKey image, int x, int y, int palette, bool hidden)
        => Add(image, x, y, palette, background: false, hidden);

    /// <summary>Converts the frame with <paramref name="engine"/>.</summary>
    public ConversionFrame Convert(ConversionEngine engine, int counter)
    {
        ArgumentNullException.ThrowIfNull(engine);
        FrameBatch batch = new(System.Runtime.InteropServices.CollectionsMarshal.AsSpan(_records), _palette, 0, 0);
        return engine.Convert(batch, counter);
    }

    private void Add(TileImageKey image, int x, int y, int palette, bool background, bool hidden)
        => _records.Add(new TileRecord(
            image.Low,
            image.High,
            (short)x,
            (short)y,
            _records.Count,
            image.FlipX,
            image.FlipY,
            hidden,
            (byte)palette,
            background));
}

/// <summary>
/// Lays the tiles of a profile's own patterns over the background grid, so that the
/// recognition of SPEC-04 binds them and the geometry of every pattern used shows up.
/// </summary>
internal static class SyntheticFrames
{
    /// <summary>Builds <paramref name="count"/> frames from the patterns of the registry.</summary>
    public static List<DumpFrame> Build(ConversionEngine engine, int count, int maxPatterns)
    {
        ArgumentNullException.ThrowIfNull(engine);

        List<DumpFrame> frames = [];
        int pattern = 0;
        int available = engine.Patterns.Permanent.Count;
        int limit = maxPatterns > 0 ? Math.Min(maxPatterns, available) : available;

        for (int n = 0; n < count; n++)
        {
            DumpFrame frame = new();
            for (int entry = 0; entry < 32; entry++)
            {
                // A readable palette: the universal background plus a spread of NES colours.
                frame.SetPalette(entry, (byte)(entry == 0 ? 0x0F : 0x01 + (entry % 0x3F)));
            }

            // The grid is 32 columns by 30 rows of tiles; a cursor advances row by row.
            int column = 0;
            int row = 0;
            bool placed = false;

            while (pattern < limit && row < 30)
            {
                EnginePattern current = engine.Patterns.Permanent[pattern];
                int width = Math.Max(1, current.Width);
                int height = Math.Max(1, current.Height);
                if (width > 32 || height > 30)
                {
                    pattern++;
                    continue;
                }

                if (column + width > 32)
                {
                    column = 0;
                    row += height;
                    continue;
                }

                if (row + height > 30)
                {
                    break;
                }

                for (int j = 0; j < height; j++)
                {
                    for (int i = 0; i < width; i++)
                    {
                        TileImage? image = current.ImageAt(i, j);
                        if (image is not null)
                        {
                            frame.AddBackground(image.Key, 8 * (column + i), 8 * (row + j), palette: 0);
                        }
                    }
                }

                column += width;
                pattern++;
                placed = true;
            }

            frames.Add(frame);
            if (!placed && pattern >= limit)
            {
                break;
            }
        }

        return frames;
    }
}

/// <summary>Reads the frame dump format documented in <see cref="MeshExport.Usage"/>.</summary>
internal static class FrameDump
{
    /// <summary>Reads the frames of <paramref name="path"/>.</summary>
    /// <exception cref="FormatException">A line does not parse.</exception>
    public static List<DumpFrame> Read(string path)
    {
        List<DumpFrame> frames = [];
        DumpFrame current = new();
        bool dirty = false;
        int number = 0;

        foreach (string raw in File.ReadLines(path))
        {
            number++;
            string line = raw.Trim();
            int comment = line.IndexOf('#', StringComparison.Ordinal);
            if (comment >= 0)
            {
                line = line[..comment].Trim();
            }

            if (line.Length == 0)
            {
                continue;
            }

            string[] parts = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            switch (parts[0].ToLowerInvariant())
            {
                case "end":
                    frames.Add(current);
                    current = new DumpFrame();
                    dirty = false;
                    break;

                case "palette":
                    Expect(parts, 2, number);
                    for (int entry = 0; entry < 32 && (2 * entry) + 1 < parts[1].Length; entry++)
                    {
                        current.SetPalette(entry, Convert.ToByte(parts[1].Substring(2 * entry, 2), 16));
                    }

                    dirty = true;
                    break;

                case "bg":
                    Expect(parts, 8, number);
                    current.AddBackground(
                        Image(parts[4], parts[5], parts[6], parts[7]),
                        Number(parts[1]),
                        Number(parts[2]),
                        Number(parts[3]));
                    dirty = true;
                    break;

                case "sp":
                    Expect(parts, 9, number);
                    current.AddSprite(
                        Image(parts[4], parts[5], parts[6], parts[7]),
                        Number(parts[1]),
                        Number(parts[2]),
                        Number(parts[3]),
                        Number(parts[8]) != 0);
                    dirty = true;
                    break;

                default:
                    throw new FormatException($"line {number}: unknown directive {parts[0]}");
            }
        }

        if (dirty)
        {
            frames.Add(current);
        }

        return frames;
    }

    private static void Expect(string[] parts, int count, int line)
    {
        if (parts.Length < count)
        {
            throw new FormatException($"line {line}: {parts[0]} needs {count - 1} values");
        }
    }

    private static TileImageKey Image(string low, string high, string flipX, string flipY)
        => new(
            Convert.ToUInt64(low, 16),
            Convert.ToUInt64(high, 16),
            Number(flipX) != 0,
            Number(flipY) != 0);

    private static int Number(string text) => int.Parse(text, CultureInfo.InvariantCulture);
}
