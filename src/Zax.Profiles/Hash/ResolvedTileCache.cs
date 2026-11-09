using System.Buffers.Binary;
using System.Text;

namespace Zax.Profiles.Hash;

/// <summary>
/// Keeps the tiles resolved for a « 3dh » profile between two sessions (AMD-062 REQ-PROF-082),
/// one file per profile identifier in the user's data folder.
/// </summary>
/// <remarks>
/// <para>
/// A cache file holds nothing but pairs « manifest string → sixteen bytes », and every pair is
/// verifiable: the string recomputes from the bytes. A file that cannot be read, or an entry
/// whose string does not recompute, is ignored without a word — the profile then simply starts
/// with more pending tiles, which the ROM and the game resolve again. Nothing of the profile
/// file is touched, and the format of the cache has no bearing on the format of a profile.
/// </para>
/// <para>
/// The determinism of REQ-ARCH-012 is stated at equal initial resolved set (REQ-PROF-080), so a
/// determinism test runs with an empty cache: <see cref="Disabled"/> is the cache that keeps
/// nothing, which is what the harness option passes.
/// </para>
/// </remarks>
public sealed class ResolvedTileCache
{
    private const int Magic = 0x5A58_5443; // "ZXTC"
    private const int Version = 1;

    private readonly string? _directory;

    /// <summary>Creates a cache under <paramref name="directory"/>, which is created on demand.</summary>
    public ResolvedTileCache(string directory)
    {
        ArgumentException.ThrowIfNullOrEmpty(directory);
        _directory = directory;
    }

    private ResolvedTileCache()
    {
    }

    /// <summary>A cache that reads nothing and writes nothing (REQ-PROF-082, determinism tests).</summary>
    public static ResolvedTileCache Disabled { get; } = new();

    /// <summary>True when this cache keeps nothing.</summary>
    public bool IsDisabled => _directory is null;

    /// <summary>The file a profile identifier maps to, or null when the cache is disabled.</summary>
    public string? FileOf(string profileId)
        => _directory is null || string.IsNullOrEmpty(profileId)
            ? null
            : Path.Combine(_directory, SafeName(profileId) + ".tiles");

    /// <summary>
    /// Applies what was kept for <paramref name="profileId"/>, before the ROM is looked at
    /// (REQ-PROF-082). Returns how many manifest strings this resolved.
    /// </summary>
    public int Apply(string? profileId, HashProfileTiles tiles)
    {
        ArgumentNullException.ThrowIfNull(tiles);
        if (profileId is null || FileOf(profileId) is not { } file || !File.Exists(file))
        {
            return 0;
        }

        byte[] bytes;
        try
        {
            bytes = File.ReadAllBytes(file);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return 0;
        }

        int resolved = 0;
        foreach ((string key, byte[] raw) in Read(bytes))
        {
            // Every entry is checked against its own string, so a truncated or arbitrary file
            // resolves nothing rather than poisoning the profile.
            if (tiles.Apply(key, raw))
            {
                resolved++;
            }
        }

        return resolved;
    }

    /// <summary>
    /// Keeps what has been resolved for <paramref name="profileId"/>. A write that fails is
    /// ignored: the cache is an optimisation, never a condition of play.
    /// </summary>
    public void Save(string? profileId, HashProfileTiles tiles)
    {
        ArgumentNullException.ThrowIfNull(tiles);
        if (profileId is null || FileOf(profileId) is not { } file || tiles.ResolvedCount == 0)
        {
            return;
        }

        try
        {
            Directory.CreateDirectory(_directory!);
            File.WriteAllBytes(file, Write(tiles));
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
        }
    }

    private static byte[] Write(HashProfileTiles tiles)
    {
        List<string> keys = [.. tiles.Resolved.Keys];
        keys.Sort(StringComparer.Ordinal);

        using MemoryStream stream = new();
        Span<byte> header = stackalloc byte[12];
        BinaryPrimitives.WriteInt32LittleEndian(header, Magic);
        BinaryPrimitives.WriteInt32LittleEndian(header[4..], Version);
        BinaryPrimitives.WriteInt32LittleEndian(header[8..], keys.Count);
        stream.Write(header);

        foreach (string key in keys)
        {
            byte[] text = Encoding.ASCII.GetBytes(key);
            stream.WriteByte((byte)text.Length);
            stream.Write(text);
            stream.Write(tiles.Resolved[key]);
        }

        return stream.ToArray();
    }

    private static IEnumerable<(string Key, byte[] Raw)> Read(byte[] bytes)
    {
        if (bytes.Length < 12
            || BinaryPrimitives.ReadInt32LittleEndian(bytes) != Magic
            || BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(4)) != Version)
        {
            yield break;
        }

        int count = BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(8));
        int at = 12;
        for (int entry = 0; entry < count; entry++)
        {
            if (at >= bytes.Length)
            {
                yield break;
            }

            int length = bytes[at++];
            if (at + length + RawTile.ByteCount > bytes.Length)
            {
                yield break;
            }

            string key = Encoding.ASCII.GetString(bytes, at, length);
            at += length;
            byte[] raw = bytes.AsSpan(at, RawTile.ByteCount).ToArray();
            at += RawTile.ByteCount;
            yield return (key, raw);
        }
    }

    /// <summary>
    /// A file name derived from the identifier: the identifiers of AMD-037 are short and
    /// alphanumeric, but a hand-written profile may carry anything.
    /// </summary>
    private static string SafeName(string profileId)
    {
        StringBuilder name = new(profileId.Length);
        foreach (char character in profileId)
        {
            bool plain = char.IsAsciiLetterOrDigit(character) || character is '-' or '_';
            name.Append(plain ? character : '_');
        }

        return name.Length == 0 ? "profile" : name.ToString();
    }
}
