using System.Globalization;
using Zax.Platform;

namespace Zax.App;

/// <summary>
/// Command line options of the desktop host.
/// </summary>
public sealed record AppOptions
{
    /// <summary>Number of frames <c>--smoke-test</c> renders when no count is given.</summary>
    public const int DefaultSmokeTestFrames = 60;

    /// <summary>Whether <c>--help</c> was requested.</summary>
    public bool ShowHelp { get; init; }

    /// <summary>Whether to list the built-in profiles and exit.</summary>
    public bool ListProfiles { get; init; }

    /// <summary>Path of the user's <c>.nes</c> ROM, or null.</summary>
    public string? RomPath { get; init; }

    /// <summary>
    /// The installation folder of 3dSen — or its profile bundle — to import from, then exit
    /// (AMD-030 §2, <c>--import-3dsen</c>); null when the option was not given.
    /// </summary>
    public string? ImportThreeDSenPath { get; init; }

    /// <summary>
    /// Title of a built-in profile, or the path of a profile file. Null falls back to the
    /// "2D Mode" entry (REQ-SCN-055).
    /// </summary>
    public string? Profile { get; init; }

    /// <summary>
    /// A folder of user-supplied asset files (<c>--assets</c>), or null. AMD-024 REQ-PUB-001:
    /// the folder is never discovered, only named.
    /// </summary>
    public string? AssetRoot { get; init; }

    /// <summary>
    /// A font file, or the name of a file of the system font folder, to use for the interface
    /// instead of the first installed candidate (AMD-024 REQ-PUB-006).
    /// </summary>
    public string? FontFile { get; init; }

    /// <summary>
    /// Whether to force Dear ImGui's built-in font, the degraded mode of REQ-PUB-006. Used by
    /// the capture that documents the fallback.
    /// </summary>
    public bool BuiltInFont { get; init; }

    /// <summary>
    /// Override for the user root (SPEC-09 REQ-PERS-071): it wins over <c>portable.txt</c> and
    /// over <c>%LOCALAPPDATA%\Zaxnes</c>.
    /// </summary>
    public string? UserDataRoot { get; init; }

    /// <summary>Window width in pixels.</summary>
    public int Width { get; init; } = 1280;

    /// <summary>Window height in pixels.</summary>
    public int Height { get; init; } = 720;

    /// <summary>Whether the audio output is opened (SPEC-11).</summary>
    public bool Audio { get; init; } = true;

    // The options below override a persisted setting of SPEC-09. They are nullable on purpose:
    // null means "not given on the command line", in which case the value of settings.json —
    // or the model's default on a first run — is the one that is used. Before FIX-1 they
    // carried the model's default and were copied into the settings at every launch, which
    // silently reset whatever the user had chosen in the options window (Q-066).

    /// <summary>Whether the shadows are enabled (REQ-SCN-030), or null for the setting.</summary>
    public bool? Shadows { get; init; }

    /// <summary>Field of view in degrees, 45 to 95 (REQ-SCN-005), or null for the setting.</summary>
    public float? Fov { get; init; }

    /// <summary>
    /// Buffer-swap synchronisation (REQ-SCN-061, SPEC-09 <c>video.vsync</c>): 0 off, 1 every
    /// refresh, 2 every other refresh, or null for the setting.
    /// </summary>
    public int? VSync { get; init; }

    /// <summary>Target frame rate: 60, 90 or 120 (REQ-SCN-061), or null for the setting.</summary>
    public int? FrameRate { get; init; }

    /// <summary>Whether render skip is on (REQ-SCN-060), or null for the setting.</summary>
    public bool? SkipRender { get; init; }

    /// <summary>
    /// Skybox setting: −1 Default, 0 Color, 1 Gradient, 2 Fantasy (REQ-SCN-049), or null for the
    /// setting.
    /// </summary>
    public int? Skybox { get; init; }

    /// <summary>Whether to run non-interactively: render a fixed number of frames, then exit.</summary>
    public bool SmokeTest { get; init; }

    /// <summary>Number of frames to render in smoke-test mode.</summary>
    public int SmokeTestFrames { get; init; } = DefaultSmokeTestFrames;

    /// <summary>
    /// Emulated frames the smoke run must produce before it stops, or 0 to stop on the rendered
    /// count alone. It is what makes « Elevator Action plays 60 s » of AMD-024 REQ-PUB-001 an
    /// exact criterion rather than a guess at the render rate: 3600 frames is a minute of NTSC.
    /// </summary>
    public int SmokeEmulatedFrames { get; init; }

    /// <summary>
    /// A PNG the smoke run writes its last frame to, or null. It captures the window itself,
    /// so what it shows is what the application draws, interface included.
    /// </summary>
    public string? SmokeShotPath { get; init; }

    /// <summary>
    /// Folder the interface screenshots of the round-1 verification go into, or null. The
    /// program puts the interface into each documented state, captures the window and exits.
    /// </summary>
    public string? UiShotDirectory { get; init; }

    /// <summary>Usage text.</summary>
    public static string Usage =>
        """
        Zaxnes - NES emulator with 3D rendering

        Usage:
          Zax.App --rom <path> [--profile <title-or-file>] [options]
          Zax.App --list-profiles
          Zax.App --import-3dsen <folder-or-bundle>

        Options:
          --rom <path>            The .nes ROM to play. Without it only the empty scene shows.
          --profile <name>        Built-in profile title, or the path of a profile file.
                                  Defaults to the "2D Mode" entry (SPEC-06 REQ-SCN-055).
          --assets <dir>          Optional folder of your own asset files (profiles.json,
                                  cubemaps, meshes, textures, audio, fonts). Nothing is
                                  bundled; without it the app uses what the code generates.
          --font <file>           Interface font: a file, or a name in the system font folder.
          --builtin-font          Use the built-in ImGui font (no accents) instead of a
                                  system font.
          --userdata <dir>        Where settings, save states and battery RAM go.
                                  Default: %LOCALAPPDATA%\Zaxnes, or userdata\ in portable mode.
          --list-profiles         Print the built-in catalogue and exit.
          --import-3dsen <path>   Import the profiles of your own 3dSen installation into your
                                  profile folder, print the summary and exit. Give the folder
                                  holding 3dSen.exe, or the profile bundle itself. Nothing is
                                  downloaded and the installation is only read (AMD-030).
          --size <w>x<h>          Window size in pixels (default 1280x720).
          --no-audio              Do not open the audio device.

        Settings overrides (this run only; without them the saved settings are used):
          --fov <degrees>         Field of view, 45 to 95 (setting default 70).
          --no-shadows            Start with the shadows off.
          --vsync [0|1|2]         Synchronise buffer swaps: 0 off, 1 every refresh,
                                  2 every other refresh. Bare, it means 1.
          --fps <60|90|120>       Target frame rate when vsync is off (setting default 90).
          --no-render-skip        Redraw the scene even without a new emulated frame.
          --skybox <name>         default, color, gradient or fantasy (REQ-SCN-049).
          --smoke-test            Render frames without waiting for input, then exit with 0.
          --smoke-frames <count>  Frames to render in smoke-test mode (default 60).
          --smoke-emulated <n>    Stop the smoke run after n emulated frames (60 = 1 s NTSC).
          --smoke-shot <file.png> Write the last frame of the smoke run to that PNG.
          --ui-shots <dir>        Capture one PNG per interface screen, then exit.
          -h, --help              Show this help and exit.

        Keyboard (player 1, SPEC-08 section 4.2):
          Arrows    directions         X / Z       A / B
          Space     Select             Enter       Start
          F5 / F6   rotate left/right  F7 / F8     rotate up/down
          F9 / F10  zoom in/out        Mouse drag  orbit, wheel zooms
          Esc       pause / resume (hold it 0.3 s to reset the view)
          , / .     rollback / fast forward
          Select + stick down / up     quick save / quick load

        Menu shortcuts (AMD-010 REQ-UI-204; see spec/changes/Q-030.md for Ctrl+Fn):
          P         pause / resume     Ctrl+R      reset
          Ctrl+Fn   load state n       Shift+Fn    save state n
          F11       2D / 3D mode       Alt+Enter   full screen
          Ctrl+Q    quit               Ctrl+,      options

        """;

    /// <summary>Parses the command line.</summary>
    /// <param name="args">Raw arguments.</param>
    /// <exception cref="ArgumentException">An argument is unknown or malformed.</exception>
    public static AppOptions Parse(string[] args)
    {
        ArgumentNullException.ThrowIfNull(args);

        AppOptions options = new();

        for (int i = 0; i < args.Length; i++)
        {
            string arg = args[i];
            switch (arg)
            {
                case "-h":
                case "--help":
                case "-?":
                case "/?":
                    options = options with { ShowHelp = true };
                    break;

                case "--list-profiles":
                    options = options with { ListProfiles = true };
                    break;

                case "--import-3dsen":
                    options = options with
                    {
                        ImportThreeDSenPath = Next(args, ref i, "--import-3dsen"),
                    };
                    break;

                case "--rom":
                    options = options with { RomPath = Next(args, ref i, "--rom") };
                    break;

                case "--profile":
                    options = options with { Profile = Next(args, ref i, "--profile") };
                    break;

                case "--assets":
                    options = options with { AssetRoot = Next(args, ref i, "--assets") };
                    break;

                case "--font":
                    options = options with { FontFile = Next(args, ref i, "--font") };
                    break;

                case "--builtin-font":
                    options = options with { BuiltInFont = true };
                    break;

                case "--userdata":
                    options = options with { UserDataRoot = Next(args, ref i, "--userdata") };
                    break;

                case "--size":
                    options = ParseSize(options, Next(args, ref i, "--size"));
                    break;

                case "--fov":
                    options = options with { Fov = ParseFloat(Next(args, ref i, "--fov"), "--fov") };
                    break;

                case "--no-shadows":
                    options = options with { Shadows = false };
                    break;

                case "--no-audio":
                    options = options with { Audio = false };
                    break;

                case "--vsync":
                    // SPEC-09 gives video.vsync three values, so the flag takes an optional one;
                    // bare, it means "every refresh" as it always did.
                    options = options with { VSync = NextVSync(args, ref i) };
                    break;

                case "--fps":
                    options = options with { FrameRate = ParseIndex(Next(args, ref i, "--fps"), "--fps") };
                    break;

                case "--no-render-skip":
                    options = options with { SkipRender = false };
                    break;

                case "--skybox":
                    options = options with
                    {
                        Skybox = Next(args, ref i, "--skybox").ToLowerInvariant() switch
                        {
                            "default" => -1,
                            "color" or "colour" => 0,
                            "gradient" => 1,
                            "fantasy" => 2,
                            _ => throw new ArgumentException(
                                "--skybox expects default, color, gradient or fantasy."),
                        },
                    };
                    break;

                case "--ui-shots":
                    options = options with
                    {
                        UiShotDirectory = Next(args, ref i, "--ui-shots"),
                    };
                    break;

                case "--smoke-test":
                    options = options with { SmokeTest = true };
                    break;

                case "--smoke-emulated":
                    options = options with
                    {
                        SmokeTest = true,
                        SmokeTestFrames = int.MaxValue,
                        SmokeEmulatedFrames = ParseCount(Next(args, ref i, "--smoke-emulated")),
                    };
                    break;

                case "--smoke-shot":
                    options = options with
                    {
                        SmokeTest = true,
                        SmokeShotPath = Next(args, ref i, "--smoke-shot"),
                    };
                    break;

                case "--smoke-frames":
                    options = options with
                    {
                        SmokeTest = true,
                        SmokeTestFrames = ParseCount(Next(args, ref i, "--smoke-frames")),
                    };
                    break;

                default:
                    throw new ArgumentException($"Unknown argument '{arg}'.");
            }
        }

        return options;
    }

    /// <summary>
    /// The value of <c>--vsync</c>: the 0, 1 or 2 that follows it, or 1 when it is used bare.
    /// </summary>
    private static int NextVSync(string[] args, ref int index)
    {
        if (index + 1 >= args.Length
            || !int.TryParse(
                args[index + 1],
                NumberStyles.Integer,
                CultureInfo.InvariantCulture,
                out int value))
        {
            return 1;
        }

        if (value is < 0 or > DisplayPacing.MaximumVSync)
        {
            throw new ArgumentException($"--vsync expects 0, 1 or 2, got '{args[index + 1]}'.");
        }

        index++;
        return value;
    }

    private static string Next(string[] args, ref int index, string option)
    {
        if (index + 1 >= args.Length)
        {
            throw new ArgumentException($"{option} requires a value.");
        }

        return args[++index];
    }

    private static AppOptions ParseSize(AppOptions options, string value)
    {
        string[] parts = value.Split('x', 'X');
        if (parts.Length != 2
            || !int.TryParse(parts[0], NumberStyles.Integer, CultureInfo.InvariantCulture, out int width)
            || !int.TryParse(parts[1], NumberStyles.Integer, CultureInfo.InvariantCulture, out int height)
            || width <= 0
            || height <= 0)
        {
            throw new ArgumentException($"--size expects <width>x<height>, got '{value}'.");
        }

        return options with { Width = width, Height = height };
    }

    private static float ParseFloat(string value, string option)
    {
        if (!float.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out float result))
        {
            throw new ArgumentException($"{option} expects a number, got '{value}'.");
        }

        return result;
    }

    private static int ParseCount(string value)
    {
        if (!int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int frames)
            || frames <= 0)
        {
            throw new ArgumentException($"a frame count must be a positive integer, got '{value}'.");
        }

        return frames;
    }

    private static int ParseIndex(string value, string option)
    {
        if (!int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int index)
            || index < 0)
        {
            throw new ArgumentException($"{option} expects a whole, non-negative number, got '{value}'.");
        }

        return index;
    }
}
