using System.Security.Cryptography;

namespace Zax.Profiles.Hash;

/// <summary>
/// The root message of a « 3dh » profile (AMD-062 REQ-PROF-071, <c>HashEnvelope</c>): the
/// profile itself as an opaque payload, the table of tile identifiers, the library header and
/// the integrity fields.
/// </summary>
/// <remarks>
/// <c>HashEnvelope</c> and the field names are the spec's own names of convenience; only the
/// field numbers and the wire types travel. Field 5 is never written.
/// </remarks>
public sealed class HashProfileEnvelope
{
    /// <summary>The value of field 1 of a « 3dh » profile.</summary>
    public const string Signature = "3dh";

    /// <summary>The envelope version the canonical writer produces (REQ-PROF-084).</summary>
    public const string CanonicalVersion = "5";

    /// <summary>Field 1; <c>"3dh"</c> for a recognised file.</summary>
    public string? Type { get; set; }

    /// <summary>Field 2: envelope version, <c>"1"</c>, <c>"2"</c> or <c>"5"</c>.</summary>
    public string? Version { get; set; }

    /// <summary>Field 3: the profile proper (REQ-PROF-074), or null when absent.</summary>
    public byte[]? Payload { get; set; }

    /// <summary>Field 4: inherited digests and text identifiers (REQ-PROF-076).</summary>
    public List<string> Hashes { get; } = [];

    /// <summary>Field 6: SHA-256 of <see cref="Payload"/>, 64 lowercase hexadecimal digits.</summary>
    public string? PayloadHash { get; set; }

    /// <summary>Field 7: profile identifier.</summary>
    public string? Id { get; set; }

    /// <summary>Field 8: title.</summary>
    public string? Title { get; set; }

    /// <summary>Field 9: author of the profile.</summary>
    public string? Author { get; set; }

    /// <summary>Field 10: publisher of the game.</summary>
    public string? Publisher { get; set; }

    /// <summary>Field 11: description.</summary>
    public string? Description { get; set; }

    /// <summary>Field 12: tile identifiers, sixteen bytes each, concatenated.</summary>
    public List<TileId> Ids { get; } = [];

    /// <summary>
    /// Length in bytes of field 12 as it was read, so that a length that is not a multiple of
    /// sixteen can be refused (REQ-PROF-072) rather than silently rounded.
    /// </summary>
    public int IdsByteLength { get; set; }

    /// <summary>
    /// Whether field 12 was present at all. It is a <b>bytes</b> field, not a repeated one, so
    /// an empty table is still a field the writer puts back: <c>empty.bytes</c>, the one profile
    /// with no tile, carries field 12 with a length of zero, and REQ-PROF-084 asks for its bytes
    /// unchanged.
    /// </summary>
    public bool HasIds { get; set; }

    /// <summary>
    /// True when the version lets a 32-digit entry of <see cref="Hashes"/> stand for a tile
    /// identifier in text form (versions <c>"2"</c> and <c>"5"</c>, REQ-PROF-072).
    /// </summary>
    public bool AcceptsTextIdentifiers => Version is "2" or "5";

    /// <summary>The library header of this envelope (REQ-PROF-073).</summary>
    public ProfileHeader ToHeader()
        => new(Type, Id, Title, Author, Publisher, Description, Version);

    /// <summary>The SHA-256 of the payload, 64 lowercase hexadecimal digits.</summary>
    public string ComputePayloadHash()
        => Convert.ToHexStringLower(SHA256.HashData(Payload ?? []));

    /// <summary>
    /// Builds the manifest, the set of strings this profile's tiles may refer to: the entries of
    /// <see cref="Hashes"/> and the text form of every identifier of <see cref="Ids"/> (§3).
    /// </summary>
    /// <returns>
    /// The manifest, or <see langword="null"/> with <paramref name="error"/> set when it is
    /// invalid (REQ-PROF-072).
    /// </returns>
    public HashProfileManifest? BuildManifest(out string? error)
    {
        if (IdsByteLength % TileId.ByteCount != 0)
        {
            error = HashProfileMessages.InvalidManifest;
            return null;
        }

        HashProfileManifest manifest = new();
        foreach (string hash in Hashes)
        {
            bool acceptable = HexText.IsLegacyDigest(hash)
                || (AcceptsTextIdentifiers && HexText.IsIdentifierText(hash));
            if (!acceptable || !manifest.AddHash(hash))
            {
                error = HashProfileMessages.InvalidManifest;
                return null;
            }
        }

        foreach (TileId id in Ids)
        {
            if (!manifest.AddIdentifier(id))
            {
                error = HashProfileMessages.InvalidManifest;
                return null;
            }
        }

        error = null;
        return manifest;
    }

    /// <summary>
    /// Validates the envelope in the order of REQ-PROF-072 and returns the manifest, or the
    /// refusal message.
    /// </summary>
    public bool Validate(out HashProfileManifest? manifest, out string? error)
    {
        manifest = null;

        if (Version is "4")
        {
            error = HashProfileMessages.Version4;
            return false;
        }

        if (Version is not ("1" or "2" or "5"))
        {
            error = HashProfileMessages.UnsupportedEnvelope;
            return false;
        }

        if (Payload is null)
        {
            error = HashProfileMessages.UnsupportedEnvelope;
            return false;
        }

        if (PayloadHash is not null
            && !string.Equals(PayloadHash, ComputePayloadHash(), StringComparison.Ordinal))
        {
            error = HashProfileMessages.DamagedPayload;
            return false;
        }

        manifest = BuildManifest(out error);
        return manifest is not null;
    }
}

/// <summary>
/// A « 3dh » profile was refused for one of the reasons of AMD-062 REQ-PROF-072 or
/// REQ-PROF-074. Its <see cref="Exception.Message"/> is the prescribed log text, verbatim.
/// </summary>
public sealed class HashProfileException : Exception
{
    /// <summary>Creates the exception.</summary>
    public HashProfileException(ProfileLoadError error, string message)
        : base(message)
    {
        Error = error;
    }

    /// <summary>Creates the exception for a malformed payload (REQ-PROF-074).</summary>
    public HashProfileException(string message)
        : this(ProfileLoadError.InvalidHashPayload, message)
    {
    }

    /// <summary>Why the profile was refused.</summary>
    public ProfileLoadError Error { get; }

    /// <summary>The error REQ-PROF-072 attaches to <paramref name="message"/>.</summary>
    public static ProfileLoadError ErrorOf(string message) => message switch
    {
        HashProfileMessages.DamagedPayload => ProfileLoadError.DamagedHashPayload,
        HashProfileMessages.InvalidManifest => ProfileLoadError.InvalidHashManifest,
        _ => ProfileLoadError.UnsupportedHashEnvelope,
    };
}

/// <summary>
/// The log messages REQ-PROF-072 and REQ-PROF-074 prescribe. They are log texts, in English,
/// never translated.
/// </summary>
public static class HashProfileMessages
{
    /// <summary>Envelope version, payload absent.</summary>
    public const string UnsupportedEnvelope = "Unsupported hash profile envelope";

    /// <summary>Envelope version <c>"4"</c>, whose identifiers can no longer be resolved.</summary>
    public const string Version4 =
        "Hash profile envelope version 4 is no longer supported: its tile identifiers cannot be "
        + "resolved. Import the original profile again.";

    /// <summary>The payload does not match its recorded digest.</summary>
    public const string DamagedPayload = "Damaged hash profile payload";

    /// <summary>The manifest is malformed or holds the same string twice.</summary>
    public const string InvalidManifest = "Invalid hash profile manifest";

    /// <summary>A tile refers to a string the manifest does not hold (REQ-PROF-074).</summary>
    public static string TileHashAbsent(string key)
        => $"Tile hash absent from profile manifest: {key}";

    /// <summary>A variant's <c>textureKind</c> or XOR texture is not usable (REQ-PROF-074).</summary>
    public const string InvalidXorTexture = "Invalid profile XOR texture";
}

/// <summary>
/// The manifest of a « 3dh » profile (AMD-062 §3): the set of strings its tiles refer to, each
/// either a 32-digit tile identifier or a 64-digit legacy digest.
/// </summary>
/// <remarks>
/// Every comparison of the format — duplicates, a tile's reference, the equality of two pending
/// tiles, resolution from a ROM or from the running game — is made on these strings, which is
/// what keeps an identifier and a legacy digest of the same tile distinct while they wait.
/// </remarks>
public sealed class HashProfileManifest
{
    private readonly List<string> _keys = [];
    private readonly HashSet<string> _keySet = new(StringComparer.Ordinal);
    private readonly List<TileId> _identifiers = [];
    private readonly Dictionary<uint, List<string>> _byPrefix = [];
    private readonly HashSet<string> _legacyDigests = new(StringComparer.Ordinal);

    /// <summary>The manifest strings, in the order they were added.</summary>
    public IReadOnlyList<string> Keys => _keys;

    /// <summary>The identifiers of field 12, in file order.</summary>
    public IReadOnlyList<TileId> Identifiers => _identifiers;

    /// <summary>Number of strings.</summary>
    public int Count => _keys.Count;

    /// <summary>True when at least one legacy digest is in the manifest.</summary>
    public bool HasLegacyDigests => _legacyDigests.Count > 0;

    /// <summary>True when <paramref name="key"/> is a manifest string.</summary>
    public bool Contains(string key) => _keySet.Contains(key);

    /// <summary>
    /// Adds an entry of field 4. Returns false when the string is already in the manifest, which
    /// REQ-PROF-072 refuses.
    /// </summary>
    public bool AddHash(string hash)
    {
        if (!_keySet.Add(hash))
        {
            return false;
        }

        _keys.Add(hash);
        if (hash.Length == TileId.TextLength && TileId.TryParseText(hash, out TileId id))
        {
            Index(id.Prefix, hash);
        }
        else
        {
            _legacyDigests.Add(hash);
        }

        return true;
    }

    /// <summary>
    /// Adds an identifier of field 12. Returns false when its text form is already in the
    /// manifest.
    /// </summary>
    public bool AddIdentifier(TileId id)
    {
        string key = id.ToText();
        if (!_keySet.Add(key))
        {
            return false;
        }

        _keys.Add(key);
        _identifiers.Add(id);
        Index(id.Prefix, key);
        return true;
    }

    /// <summary>
    /// The manifest strings whose identifier has this prefix, or an empty list. This is the fast
    /// filter of REQ-PROF-077: a scan hashes a window only when its prefix is here.
    /// </summary>
    public IReadOnlyList<string> KeysWithPrefix(uint prefix)
        => _byPrefix.TryGetValue(prefix, out List<string>? keys) ? keys : [];

    /// <summary>True when <paramref name="prefix"/> is the prefix of at least one identifier.</summary>
    public bool HasPrefix(uint prefix) => _byPrefix.ContainsKey(prefix);

    private void Index(uint prefix, string key)
    {
        if (!_byPrefix.TryGetValue(prefix, out List<string>? keys))
        {
            keys = [];
            _byPrefix[prefix] = keys;
        }

        keys.Add(key);
    }
}
