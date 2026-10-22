namespace Zax.Nes.Tests;

/// <summary>
/// Locates the public test ROMs under <c>impl/tests/roms/</c>.
/// </summary>
/// <remarks>
/// The ROMs are not redistributed by this repository; <c>tests/roms/README.md</c> says
/// where to fetch them and under what licence. A test that needs one is skipped, with a
/// message naming the missing file, when it is absent.
/// </remarks>
internal static class TestRomLibrary
{
    /// <summary>
    /// Absolute path of <c>impl/tests/roms</c>, or <see langword="null"/> when the folder
    /// cannot be found.
    /// </summary>
    public static string? Directory { get; } = Locate();

    /// <summary>
    /// Returns the full path of <paramref name="relativePath"/> under the ROM folder, or
    /// <see langword="null"/> when it is not there.
    /// </summary>
    public static string? Find(string relativePath)
    {
        if (Directory is null)
        {
            return null;
        }

        string candidate = Path.Combine(Directory, relativePath);
        return File.Exists(candidate) ? candidate : null;
    }

    /// <summary>
    /// The message a skipped test reports, telling the reader exactly what to download.
    /// </summary>
    public static string MissingMessage(params string[] relativePaths)
        => $"Public test ROM not available: {string.Join(", ", relativePaths)}. "
        + $"See {Path.Combine(Directory ?? "impl/tests/roms", "README.md")} for its source and licence.";

    private static string? Locate()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);

        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName, "tests", "roms");
            if (System.IO.Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}
