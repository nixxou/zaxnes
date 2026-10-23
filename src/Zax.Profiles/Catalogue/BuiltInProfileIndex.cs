using System.Security.Cryptography;

namespace Zax.Profiles.Catalogue;

/// <summary>
/// Tells whether the bytes of a user profile file are those of a built-in profile of the
/// catalogue in use (AMD-037 §2, SPEC-03 REQ-PROF-060).
/// </summary>
/// <remarks>
/// A user file identical to a built-in profile adds no game: the built-in one is shown once,
/// with its catalogue metadata. The rule holds whether the file carries an <c>Id</c> or not,
/// which is what keeps a 3dSen import (AMD-030) from doubling every game of the catalogue.
/// </remarks>
public interface IBuiltInProfileIndex
{
    /// <summary>
    /// The identifier of the built-in profile whose file holds exactly
    /// <paramref name="bytes"/>, or <see langword="null"/> when no built-in profile does.
    /// </summary>
    string? FindByContent(ReadOnlySpan<byte> bytes);
}

/// <summary>
/// The index over the profile files of a <see cref="ProfileCatalogue"/> (AMD-037 §2).
/// </summary>
/// <remarks>
/// <para>
/// Built lazily and only as far as it must: the file lengths of the catalogue are read on the
/// first question, and the bytes of a built-in profile are read — and its SHA-256 kept — only
/// when a user file is exactly as long. The 101 built-in profiles have 101 distinct lengths, so
/// a user profile that is not a copy of one costs no read at all.
/// </para>
/// <para>
/// The two generic entries of AMD-024 REQ-PUB-001 have no file and are not in the index; with
/// the catalogue of <see cref="BuiltInCatalogue"/> there is nothing to compare and
/// <see cref="For"/> returns <see langword="null"/>.
/// </para>
/// </remarks>
public sealed class BuiltInProfileIndex : IBuiltInProfileIndex
{
    private readonly IProfileFileSystem _files;
    private readonly List<(string Id, string Path)> _entries;
    private readonly Dictionary<string, byte[]> _hashes = new(StringComparer.OrdinalIgnoreCase);
    private Dictionary<long, List<int>>? _byLength;

    private BuiltInProfileIndex(IProfileFileSystem files, List<(string Id, string Path)> entries)
    {
        _files = files;
        _entries = entries;
    }

    /// <summary>
    /// An index over the entries of <paramref name="catalogue"/> that have a profile file, or
    /// <see langword="null"/> when none has — there is then nothing a user file could be a copy
    /// of.
    /// </summary>
    public static BuiltInProfileIndex? For(IProfileFileSystem files, ProfileCatalogue? catalogue)
    {
        ArgumentNullException.ThrowIfNull(files);
        if (catalogue is null)
        {
            return null;
        }

        List<(string Id, string Path)> entries = [];
        foreach (CatalogueEntry entry in catalogue.Entries)
        {
            string path = catalogue.ProfilePath(entry);
            if (!string.IsNullOrEmpty(path))
            {
                entries.Add((entry.Id, path));
            }
        }

        return entries.Count == 0 ? null : new BuiltInProfileIndex(files, entries);
    }

    /// <inheritdoc/>
    public string? FindByContent(ReadOnlySpan<byte> bytes)
    {
        _byLength ??= GroupByLength();
        if (!_byLength.TryGetValue(bytes.Length, out List<int>? candidates))
        {
            return null;
        }

        Span<byte> digest = stackalloc byte[SHA256.HashSizeInBytes];
        SHA256.HashData(bytes, digest);

        foreach (int index in candidates)
        {
            (string id, string path) = _entries[index];
            if (HashOf(path) is { } hash && digest.SequenceEqual(hash))
            {
                return id;
            }
        }

        return null;
    }

    private Dictionary<long, List<int>> GroupByLength()
    {
        Dictionary<long, List<int>> byLength = [];
        for (int index = 0; index < _entries.Count; index++)
        {
            long length = _files.FileLength(_entries[index].Path);
            if (length < 0)
            {
                // A missing or unreadable built-in file is simply not something to compare with;
                // REQ-PROF-060 reports it when the game is started, not here.
                continue;
            }

            if (!byLength.TryGetValue(length, out List<int>? indices))
            {
                indices = [];
                byLength.Add(length, indices);
            }

            indices.Add(index);
        }

        return byLength;
    }

    private byte[]? HashOf(string path)
    {
        if (_hashes.TryGetValue(path, out byte[]? hash))
        {
            return hash;
        }

        try
        {
            hash = SHA256.HashData(_files.ReadAllBytes(path));
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return null;
        }

        _hashes.Add(path, hash);
        return hash;
    }
}
