using System.Runtime.InteropServices;

namespace Zax.App.Ui;

/// <summary>
/// Finds a text font <b>installed on the machine</b> (AMD-024 REQ-PUB-006): the interface loads
/// it at run time and never redistributes it.
/// </summary>
/// <remarks>
/// <para>
/// REQ-PUB-002 removes the two TTF files the earlier lots read from <c>assets-neutral/fonts</c>:
/// their copies came from the original build and were not sourced
/// (<c>legal/P7-ASSET-INVENTORY.md</c>). Nothing is shipped in their place — the file is opened
/// from the system font folder, which is a licence the user already has. When no candidate is
/// found, Dear ImGui's built-in font stays in place (MIT, part of the library) and the French
/// labels of AMD-010 lose their accents, which REQ-PUB-006 explicitly allows.
/// </para>
/// <para>
/// Windows is the only platform of this phase, so the candidates are the Windows faces
/// REQ-PUB-006 names, Segoe UI first and Arial as the fallback. A few Linux and macOS paths are
/// listed as well so that a future port has somewhere to start; the lookup is a plain file
/// probe, so an absent path simply does not match.
/// </para>
/// </remarks>
internal static class SystemFonts
{
    /// <summary>
    /// The faces to try for the body text, in order. The regular weight is what the interface
    /// uses; the bold file of the same family serves as the heading face when it exists.
    /// </summary>
    private static readonly (string Regular, string? Bold)[] WindowsCandidates =
    [
        ("segoeui.ttf", "segoeuib.ttf"),
        ("arial.ttf", "arialbd.ttf"),
        ("tahoma.ttf", "tahomabd.ttf"),
        ("verdana.ttf", "verdanab.ttf"),
    ];

    private static readonly (string Regular, string? Bold)[] UnixCandidates =
    [
        ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
        ("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"),
        ("/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf"),
    ];

    /// <summary>
    /// The regular and the bold file of the first installed candidate, or <c>(null, null)</c>
    /// when the machine has none of them. <paramref name="hint"/> — the value of
    /// <c>--font</c> — is tried first, as a file path or as a file name inside the system font
    /// folder.
    /// </summary>
    public static (string? Regular, string? Bold) Locate(string? hint = null)
    {
        if (!string.IsNullOrWhiteSpace(hint))
        {
            if (File.Exists(hint))
            {
                return (Path.GetFullPath(hint), null);
            }

            if (Resolve(hint) is { } named)
            {
                return (named, null);
            }
        }

        (string Regular, string? Bold)[] candidates = OperatingSystem.IsWindows()
            ? WindowsCandidates
            : UnixCandidates;

        foreach ((string regular, string? bold) in candidates)
        {
            if (Resolve(regular) is not { } path)
            {
                continue;
            }

            return (path, bold is not null ? Resolve(bold) : null);
        }

        return (null, null);
    }

    /// <summary>The folder the system fonts live in, or null when it cannot be determined.</summary>
    public static string? Folder
    {
        get
        {
            if (!OperatingSystem.IsWindows())
            {
                return null;
            }

            string folder = Environment.GetFolderPath(Environment.SpecialFolder.Fonts);
            if (!string.IsNullOrEmpty(folder) && Directory.Exists(folder))
            {
                return folder;
            }

            // GetFolderPath comes back empty on some configurations; %WINDIR%\Fonts is the
            // documented location.
            string? windows = Environment.GetEnvironmentVariable("WINDIR");
            if (string.IsNullOrEmpty(windows))
            {
                return null;
            }

            string fallback = Path.Combine(windows, "Fonts");
            return Directory.Exists(fallback) ? fallback : null;
        }
    }

    /// <summary>
    /// The full path of <paramref name="name"/>: an absolute path as given, or a file of the
    /// system font folder. Null when there is no such file.
    /// </summary>
    private static string? Resolve(string name)
    {
        if (Path.IsPathRooted(name))
        {
            return File.Exists(name) ? name : null;
        }

        if (Folder is not { } folder)
        {
            return null;
        }

        string path = Path.Combine(folder, name);
        return File.Exists(path) ? path : null;
    }

    /// <summary>
    /// Whether the process runs on a platform this lot has a candidate list for. Used by the
    /// tests, which must not fail on a machine with no font at all.
    /// </summary>
    public static bool Supported => OperatingSystem.IsWindows()
        || RuntimeInformation.IsOSPlatform(OSPlatform.Linux)
        || RuntimeInformation.IsOSPlatform(OSPlatform.OSX);
}
