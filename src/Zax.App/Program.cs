namespace Zax.App;

/// <summary>
/// Entry point of the desktop host.
/// </summary>
internal static class Program
{
    private const int ExitSuccess = 0;
    private const int ExitFailure = 1;
    private const int ExitUsage = 2;

    private static int Main(string[] args)
    {
        AppOptions options;
        try
        {
            options = AppOptions.Parse(args);
        }
        catch (ArgumentException ex)
        {
            Console.Error.WriteLine(ex.Message);
            Console.Error.Write(AppOptions.Usage);
            return ExitUsage;
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
