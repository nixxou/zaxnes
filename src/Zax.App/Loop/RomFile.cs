using System.IO.Compression;

namespace Zax.App.Loop;

/// <summary>What reading a ROM file produced (SPEC-07 REQ-LOOP-022, REQ-LOOP-023).</summary>
/// <param name="Image">The ROM image, or null on failure.</param>
/// <param name="Name">The ROM name the core records, or an empty string on failure.</param>
/// <param name="ErrorMessage">The exact message to show, or null on success.</param>
public readonly record struct RomReadResult(byte[]? Image, string Name, string? ErrorMessage)
{
    /// <summary>Whether the read succeeded.</summary>
    public bool Success => Image is not null;
}

/// <summary>
/// Reads the file a library entry points at (SPEC-07 §3.3.1): a <c>.zip</c> is decompressed and
/// its <b>first entry</b> used, anything else is read as it is.
/// </summary>
/// <remarks>
/// <para>
/// For an archive the ROM name is the archive's own name with the <c>.nes</c> extension, so
/// <c>Foo.zip</c> holding <c>bar.nes</c> loads under the name <c>Foo.nes</c> (REQ-LOOP-022).
/// </para>
/// <para>
/// <c>.7z</c> is in the spec's extension filter but is <b>not</b> supported here: no 7-Zip
/// decoder is available among the dependencies the plan allows, and writing an LZMA decoder is
/// out of this lot's scope. Such a file is refused with <see cref="UncompressionFailedMessage"/>
/// — see <c>spec/changes/Q-028.md</c>.
/// </para>
/// </remarks>
public static class RomFile
{
    /// <summary>Message for an unreadable archive (REQ-LOOP-023).</summary>
    public const string CorruptedZipMessage = "Corrupted zip file";

    /// <summary>Message for an entry that could not be extracted (REQ-LOOP-023).</summary>
    public const string UncompressionFailedMessage = "Uncompression is not successful";

    /// <summary>Message for content the core refuses (REQ-LOOP-023).</summary>
    public const string InvalidRomMessage = "Invalid rom file.";

    /// <summary>Reads <paramref name="path"/>, following REQ-LOOP-022.</summary>
    public static RomReadResult Read(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);

        string extension = Path.GetExtension(path);
        try
        {
            if (string.Equals(extension, ".zip", StringComparison.OrdinalIgnoreCase))
            {
                return ReadZip(path);
            }

            if (string.Equals(extension, ".7z", StringComparison.OrdinalIgnoreCase))
            {
                // Q-028: no 7-Zip decoder is available; the file is refused cleanly.
                return new RomReadResult(null, string.Empty, UncompressionFailedMessage);
            }

            byte[] image = File.ReadAllBytes(path);
            return new RomReadResult(image, Path.GetFileName(path), null);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // REQ-LOOP-023: a missing or unreadable file reports the file system's own message.
            return new RomReadResult(null, string.Empty, exception.Message);
        }
    }

    private static RomReadResult ReadZip(string path)
    {
        string romName = Path.GetFileNameWithoutExtension(path) + ".nes";

        ZipArchive archive;
        try
        {
            archive = ZipFile.OpenRead(path);
        }
        catch (InvalidDataException)
        {
            return new RomReadResult(null, string.Empty, CorruptedZipMessage);
        }

        using (archive)
        {
            if (archive.Entries.Count == 0)
            {
                return new RomReadResult(null, string.Empty, UncompressionFailedMessage);
            }

            try
            {
                ZipArchiveEntry entry = archive.Entries[0];
                using Stream source = entry.Open();
                using MemoryStream buffer = new();
                source.CopyTo(buffer);
                if (buffer.Length == 0)
                {
                    return new RomReadResult(null, string.Empty, UncompressionFailedMessage);
                }

                return new RomReadResult(buffer.ToArray(), romName, null);
            }
            catch (Exception exception) when (exception is InvalidDataException or IOException or NotSupportedException)
            {
                return new RomReadResult(null, string.Empty, UncompressionFailedMessage);
            }
        }
    }
}
