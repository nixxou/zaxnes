using Zax.Render.Assets;

namespace Zax.App.Persistence;

/// <summary>
/// The cover art of a library entry (SPEC-09 REQ-PERS-036) and the deterministic file names
/// derived from user data (REQ-PERS-074).
/// </summary>
/// <remarks>
/// <para>
/// A cover is an optional PNG per entry, stored in <c>&lt;root&gt;/covers</c>, shrunk to fit
/// 256 × 192 px with its proportions kept — the factor is
/// <c>max(width / 256, height / 192)</c> — and written without an alpha channel. Its name is
/// <c>&lt;Title&gt;_3dsen_&lt;Id&gt;.png</c>, every character a file system forbids replaced by
/// <c>_</c> (REQ-PERS-074).
/// </para>
/// <para>
/// AMD-020 (Q-050) brings REQ-PERS-036 and REQ-PERS-074 down to <b>P3</b>: without the tile
/// library of AMD-010 the import and the capture are no longer required in v1, and the profile
/// chooser only <i>may</i> show a cover it finds. They are implemented all the same, because the
/// deterministic naming rule is the only thing REQ-PERS-074 is about and both halves are a few
/// lines on top of the PNG codec SPEC-06 already needs. Of the three sources REQ-PERS-036 lists,
/// the file import and the pause capture are here; the automatic refresh of the "2D Mode" cover
/// is the loop's (<c>EmulationHost</c>).
/// </para>
/// <para>
/// Only PNG sources are decoded. REQ-PERS-036 also names <c>jpg</c>, which would mean a JPEG
/// decoder the rest of the program has no use for; a JPEG import is refused with a message
/// instead, which the P3 status of the requirement allows. Q-050.
/// </para>
/// </remarks>
// Q-050
internal static class GameArt
{
    /// <summary>Widest a cover may be (REQ-PERS-036).</summary>
    public const int MaximumWidth = 256;

    /// <summary>Tallest a cover may be (REQ-PERS-036).</summary>
    public const int MaximumHeight = 192;

    /// <summary>The infix the original's cover names carry.</summary>
    public const string NameInfix = "_3dsen_";

    /// <summary>Message shown when the chosen file is not a PNG this build can read.</summary>
    public const string UnsupportedSourceMessage = "Only PNG images can be imported as cover art.";

    /// <summary>
    /// Replaces every character forbidden in a file name by <c>_</c> (REQ-PERS-074).
    /// </summary>
    /// <remarks>
    /// The set is the union of what <see cref="Path.GetInvalidFileNameChars"/> reports on the
    /// running system and the characters Windows reserves, so a name derived on one platform is
    /// usable on the other — a profile and its covers travel with the user root.
    /// </remarks>
    public static string SanitiseFileName(string name)
    {
        ArgumentNullException.ThrowIfNull(name);

        char[] result = name.ToCharArray();
        for (int index = 0; index < result.Length; index++)
        {
            char value = result[index];
            if (value < ' ' || ReservedEverywhere.Contains(value) || Path.GetInvalidFileNameChars().Contains(value))
            {
                result[index] = '_';
            }
        }

        return new string(result);
    }

    /// <summary>
    /// The cover file name of a game: <c>&lt;Title&gt;_3dsen_&lt;Id&gt;.png</c>, sanitised
    /// (REQ-PERS-036, REQ-PERS-074). The identifier is what makes it unique.
    /// </summary>
    public static string CoverFileName(string title, string id)
    {
        ArgumentNullException.ThrowIfNull(id);
        return SanitiseFileName((title ?? string.Empty) + NameInfix + id) + ".png";
    }

    /// <summary>The absolute path a game's cover would have under <paramref name="paths"/>.</summary>
    public static string CoverFile(UserDataPaths paths, string title, string id)
    {
        ArgumentNullException.ThrowIfNull(paths);
        return Path.Combine(paths.CoversDirectory, CoverFileName(title, id));
    }

    /// <summary>
    /// Imports <paramref name="sourcePath"/> as the cover of a game: decoded, shrunk to fit
    /// 256 × 192 and written as RGB without alpha (REQ-PERS-036).
    /// </summary>
    /// <returns>
    /// The file name to store in the library entry, or null with <paramref name="message"/> set
    /// when the file could not be used.
    /// </returns>
    public static string? ImportCover(
        UserDataPaths paths,
        string title,
        string id,
        string sourcePath,
        out string? message)
    {
        ArgumentNullException.ThrowIfNull(paths);
        ArgumentException.ThrowIfNullOrEmpty(sourcePath);

        message = null;
        ImageData image;
        try
        {
            image = PngImage.Load(sourcePath);
        }
        catch (Exception exception) when (exception is InvalidDataException
            or IOException
            or UnauthorizedAccessException)
        {
            message = UnsupportedSourceMessage;
            return null;
        }

        (int width, int height) = FittedSize(image.Width, image.Height);
        byte[] pixels = Resample(image, width, height);
        return Write(paths, title, id, width, height, pixels, out message);
    }

    /// <summary>
    /// Stores a pause capture as the cover of a game: the « Create game art » source of
    /// REQ-PERS-036, and what the "2D Mode" entry refreshes automatically.
    /// </summary>
    /// <param name="paths">The user root, whose <c>covers/</c> folder receives the file.</param>
    /// <param name="title">Displayed title of the game, which names the file.</param>
    /// <param name="id">Profile identifier, which makes the name unique (REQ-PERS-074).</param>
    /// <param name="capture">
    /// A 256 × 192 RGB capture, the format REQ-PERS-051 already produces for the slots.
    /// </param>
    /// <param name="message">Set when the file could not be written.</param>
    public static string? SetCoverFromCapture(
        UserDataPaths paths,
        string title,
        string id,
        ReadOnlySpan<byte> capture,
        out string? message)
    {
        ArgumentNullException.ThrowIfNull(paths);

        message = null;
        if (capture.Length < MaximumWidth * MaximumHeight * 3)
        {
            return null;
        }

        return Write(
            paths,
            title,
            id,
            MaximumWidth,
            MaximumHeight,
            capture[..(MaximumWidth * MaximumHeight * 3)],
            out message);
    }

    /// <summary>Deletes the cover of a game, which deleting the game does (REQ-PERS-042).</summary>
    public static void DeleteCover(UserDataPaths paths, string title, string id, string? storedName)
    {
        ArgumentNullException.ThrowIfNull(paths);

        // Both the derived name and whatever the entry recorded, so a cover imported by an
        // earlier version — or renamed by hand — goes too.
        foreach (string? name in (string?[])[storedName, CoverFileName(title, id)])
        {
            if (string.IsNullOrEmpty(name))
            {
                continue;
            }

            try
            {
                string file = Path.Combine(paths.CoversDirectory, Path.GetFileName(name));
                if (File.Exists(file))
                {
                    File.Delete(file);
                }
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A cover that cannot be removed must not stop the game from being deleted.
            }
        }
    }

    /// <summary>
    /// The size a source image is shrunk to: the factor of REQ-PERS-036,
    /// <c>max(width / 256, height / 192)</c>, applied only when it is above 1 — a smaller image
    /// is kept as it is rather than blown up.
    /// </summary>
    public static (int Width, int Height) FittedSize(int width, int height)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(width, 1);
        ArgumentOutOfRangeException.ThrowIfLessThan(height, 1);

        float factor = MathF.Max((float)width / MaximumWidth, (float)height / MaximumHeight);
        if (factor <= 1f)
        {
            return (width, height);
        }

        return (
            Math.Clamp((int)MathF.Round(width / factor), 1, MaximumWidth),
            Math.Clamp((int)MathF.Round(height / factor), 1, MaximumHeight));
    }

    private static readonly char[] ReservedEverywhere = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    private static string? Write(
        UserDataPaths paths,
        string title,
        string id,
        int width,
        int height,
        ReadOnlySpan<byte> rgb,
        out string? message)
    {
        message = null;
        string name = CoverFileName(title, id);
        try
        {
            Directory.CreateDirectory(paths.CoversDirectory);
            PngImage.WriteRgb(Path.Combine(paths.CoversDirectory, name), width, height, rgb);
            return name;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            message = exception.Message;
            return null;
        }
    }

    /// <summary>
    /// Box-average down-sampling of an RGBA image into RGB, which is enough for a 256 × 192
    /// thumbnail and keeps the code free of a filtering library.
    /// </summary>
    private static byte[] Resample(ImageData image, int width, int height)
    {
        byte[] target = new byte[width * height * 3];
        ReadOnlySpan<byte> source = image.Pixels;

        for (int y = 0; y < height; y++)
        {
            int firstRow = y * image.Height / height;
            int lastRow = Math.Max(firstRow + 1, (y + 1) * image.Height / height);

            for (int x = 0; x < width; x++)
            {
                int firstColumn = x * image.Width / width;
                int lastColumn = Math.Max(firstColumn + 1, (x + 1) * image.Width / width);

                int red = 0;
                int green = 0;
                int blue = 0;
                int count = 0;

                for (int row = firstRow; row < lastRow; row++)
                {
                    int line = row * image.Width * 4;
                    for (int column = firstColumn; column < lastColumn; column++)
                    {
                        int at = line + (column * 4);
                        red += source[at];
                        green += source[at + 1];
                        blue += source[at + 2];
                        count++;
                    }
                }

                int offset = ((y * width) + x) * 3;
                target[offset] = (byte)(red / count);
                target[offset + 1] = (byte)(green / count);
                target[offset + 2] = (byte)(blue / count);
            }
        }

        return target;
    }
}
