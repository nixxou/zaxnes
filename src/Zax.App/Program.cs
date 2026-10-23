using System.Runtime.InteropServices;

namespace Zax.App;

/// <summary>
/// Entry point of the desktop host.
/// </summary>
internal static partial class Program
{
    private const int ExitSuccess = 0;
    private const int ExitFailure = 1;
    private const int ExitUsage = 2;

    /// <summary>What <c>AttachConsole</c> is passed to borrow the parent's console.</summary>
    private const uint AttachParentProcess = 0xFFFFFFFF;

    private static int Main(string[] args)
    {
        AppOptions options;
        try
        {
            options = AppOptions.Parse(args);
        }
        catch (ArgumentException ex)
        {
            // A usage error prints and exits like the three options below, so it borrows the
            // terminal too; without that the message would go nowhere.
            AttachToParentConsole();
            Console.Error.WriteLine(ex.Message);
            Console.Error.Write(AppOptions.Usage);
            return ExitUsage;
        }

        if (PrintsAndExits(options))
        {
            AttachToParentConsole();
        }

        if (options.ShowHelp)
        {
            Console.Out.Write(AppOptions.Usage);
            return ExitSuccess;
        }

        if (options.ListProfiles)
        {
            return ListProfiles(options);
        }

        // AMD-030 §2: the import runs without a window and exits with 0 when it all worked. Like
        // --list-profiles it is checked before the single-instance guard, so it stays usable
        // while the interface is open.
        if (options.ImportThreeDSenPath is not null)
        {
            return ThreeDSenImportCommand.Run(options, Console.Out, Console.Error);
        }

        // REQ-LOOP-054: a second launch on the same user root closes rather than opening a
        // second window over the same settings, library and slot files. The guard is released
        // when the process exits, abnormal exits included.
        using SingleInstance instance = SingleInstance.Acquire(options.UserDataRoot);
        if (!instance.IsOwner)
        {
            Console.Error.WriteLine(SingleInstance.AlreadyRunningMessage);
            return ExitSuccess;
        }

        try
        {
            using ZaxApplication application = new(options);
            return application.Run();
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine(ex);
            return ExitFailure;
        }
    }

    /// <summary>
    /// Whether this run only prints something and exits, which is the single case where the
    /// terminal's console is borrowed (AMD-032 §5).
    /// </summary>
    /// <param name="options">The parsed command line.</param>
    /// <remarks>
    /// A normal launch must <b>not</b> attach: the window then shares the terminal it was
    /// started from, so the log lines land over the shell prompt and a Ctrl+C meant for the
    /// shell reaches the emulator's process group and kills it. Only <c>--help</c>,
    /// <c>--list-profiles</c> and the import of AMD-030 §2 write to the console and exit
    /// without ever opening a window.
    /// </remarks>
    internal static bool PrintsAndExits(AppOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        return options.ShowHelp
            || options.ListProfiles
            || options.ImportThreeDSenPath is not null;
    }

    /// <summary>
    /// Borrows the console of the process that launched this one, when there is one and the
    /// standard output has not been given a handle already, and writes UTF-8 either way.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The handle is checked before attaching so that a redirected run is left exactly as it
    /// is: <c>AttachConsole</c> replaces the standard handles, which would send the output to
    /// the terminal instead of the pipe the caller set up — the publish script and the
    /// architecture tests both read this program through a pipe. A launch from Explorer has no
    /// parent console and the call simply fails, which is not an error.
    /// </para>
    /// <para>
    /// A process with no console of its own encodes its output with the system's single-byte
    /// code page, which turns the accents of the French messages into replacement characters.
    /// UTF-8 is set in both cases, so what is written is what the program means, whether it
    /// goes to a terminal or into a pipe.
    /// </para>
    /// </remarks>
    private static void AttachToParentConsole()
    {
        if (!OperatingSystem.IsWindows())
        {
            return;
        }

        try
        {
            if (GetStdHandle(StandardOutputHandle) == IntPtr.Zero)
            {
                AttachConsole(AttachParentProcess);
            }

            Console.OutputEncoding = System.Text.Encoding.UTF8;
        }
        catch (IOException)
        {
            // No console and no pipe to write to; the program runs on without one.
        }
        catch (DllNotFoundException)
        {
            // Nothing to borrow: the program runs on without a console.
        }
        catch (EntryPointNotFoundException)
        {
        }
    }

    /// <summary>Identifier of the standard output handle for <c>GetStdHandle</c>.</summary>
    private const int StandardOutputHandle = -11;

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool AttachConsole(uint processId);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    private static partial IntPtr GetStdHandle(int handle);

    /// <summary>
    /// Prints the built-in catalogue, marking the profiles with no Lua script: those render
    /// fully today, since script execution arrives with lot L5.
    /// </summary>
    private static int ListProfiles(AppOptions options)
    {
        string? root = NeutralAssets.Locate(options.AssetRoot);

        // AMD-024 REQ-PUB-001: with no asset folder the catalogue is the built-in one, the two
        // generic entries written in code (Q-065). Nothing is an error here any more.
        Console.Out.WriteLine(root is null
            ? "Zaxnes: built-in entries (no asset folder; pass --assets <dir> for your own files)"
            : $"Zaxnes: built-in profiles of {root}");
        Console.Out.WriteLine("  '*' marks a profile with no Lua script, which renders fully today.");
        Console.Out.WriteLine();

        foreach (Zax.Profiles.Catalogue.CatalogueEntry entry in ProfileResolver.List(root))
        {
            int scripts = root is null || string.IsNullOrEmpty(entry.File)
                ? 0
                : CountScripts(root, entry);
            string mark = scripts == 0 ? "*" : " ";
            Console.Out.WriteLine($"  {mark} {entry.Title}  [{scripts} script(s)]");
        }

        return ExitSuccess;
    }

    private static int CountScripts(string root, Zax.Profiles.Catalogue.CatalogueEntry entry)
    {
        try
        {
            string path = Path.Combine(root, entry.File.Replace('/', Path.DirectorySeparatorChar));
            Zax.Profiles.ProfileLoadResult result = Zax.Profiles.ProfileFormat.Load(
                File.ReadAllBytes(path),
                sourceName: path);
            return result.Profile?.Scripts.Count ?? -1;
        }
        catch (IOException)
        {
            return -1;
        }
    }
}
