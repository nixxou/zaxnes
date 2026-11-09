using Zax.Profiles.Hash;

namespace Zax.Profiles.Wire;

/// <summary>
/// Reads and writes the bytes of a profile (<c>.bytes</c>, <c>.3dn</c>) — SPEC-03.
/// </summary>
/// <remarks>
/// This is the only place that knows the wire format. Callers that want the robust, all-or-
/// nothing behaviour of REQ-PROF-033 should use <see cref="ProfileFormat"/> instead, which
/// turns every failure here into a refusal.
/// </remarks>
public static class ProfileWireCodec
{
    /// <summary>
    /// Decodes a profile, keeping the shared identities and the field sets of the file.
    /// </summary>
    /// <exception cref="ProfileFormatException">The bytes are not a well-formed profile.</exception>
    public static ProfileDocument Decode(ReadOnlySpan<byte> bytes)
        => new ProfileWireDecoder(bytes.ToArray()).ReadDocument();

    /// <summary>
    /// Decodes a « 3dh » profile: the envelope, its validation, then the payload with the three
    /// differences of REQ-PROF-074 (AMD-062). The tile table is built and the textures of the
    /// variants are reconstructed, with every tile still pending.
    /// </summary>
    /// <exception cref="HashProfileException">
    /// The envelope or the payload is refused; the message is the log text REQ-PROF-072 and
    /// REQ-PROF-074 prescribe.
    /// </exception>
    /// <exception cref="ProfileFormatException">The bytes are not a well-formed profile.</exception>
    public static ProfileDocument DecodeHashProfile(ReadOnlySpan<byte> bytes)
    {
        HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(bytes);
        if (!envelope.Validate(out HashProfileManifest? manifest, out string? error))
        {
            throw new HashProfileException(HashProfileException.ErrorOf(error!), error!);
        }

        ProfileDocument document =
            new ProfileWireDecoder(envelope.Payload!, manifest, envelope).ReadDocument();
        HashProfileTiles tiles = new(document.Profile, manifest!);
        tiles.RebuildAllTextures();
        document.Tiles = tiles;
        return document;
    }

    /// <summary>
    /// Writes a decoded « 3dh » profile back as a « 3dh » profile (REQ-PROF-084): the canonical
    /// envelope around the canonical payload.
    /// </summary>
    /// <exception cref="ArgumentException">The document did not come from a « 3dh » profile.</exception>
    public static byte[] EncodeHashProfile(ProfileDocument document)
    {
        ArgumentNullException.ThrowIfNull(document);
        if (document.HashEnvelope is not { } source)
        {
            throw new ArgumentException("This profile did not come from a « 3dh » file.", nameof(document));
        }

        HashProfileEnvelope envelope = new()
        {
            Type = source.Type ?? HashProfileEnvelope.Signature,
            Version = HashProfileEnvelope.CanonicalVersion,
            Payload = new ProfileWireEncoder(
                document.Layout, EstimateCapacity(document.Profile), hashMode: true).Write(document.Profile),
            Id = source.Id,
            Title = source.Title,
            Author = source.Author,
            Publisher = source.Publisher,
            Description = source.Description,
        };

        // REQ-PROF-084: the inherited digests and text identifiers still referenced by a tile,
        // and the identifier table of field 12, which stays the profile's tile table.
        HashSet<string> referenced = new(StringComparer.Ordinal);
        foreach (ProfileTile tile in document.HashTiles)
        {
            if (tile.Hash is { StoredAsIdentifier: false } hash)
            {
                referenced.Add(hash.Text);
            }
        }

        foreach (string hash in source.Hashes)
        {
            if (referenced.Contains(hash))
            {
                envelope.Hashes.Add(hash);
            }
        }

        envelope.Ids.AddRange(source.Ids);
        return HashEnvelopeCodec.Encode(envelope);
    }

    /// <summary>
    /// Encodes a profile with the canonical writer (REQ-PROF-051). Pass the
    /// <paramref name="layout"/> that came with a decoded profile to rewrite it byte for byte;
    /// pass none for a profile built in memory, and every scalar is written.
    /// </summary>
    public static byte[] Encode(GameProfile profile, ProfileWireLayout? layout = null)
    {
        ArgumentNullException.ThrowIfNull(profile);
        int capacity = EstimateCapacity(profile);
        return new ProfileWireEncoder(layout, capacity).Write(profile);
    }

    /// <summary>
    /// Reads only the header fields a library entry needs — signature, id, title, author,
    /// publisher, description (REQ-PROF-062) — skipping patterns, scripts and settings.
    /// </summary>
    /// <exception cref="ProfileFormatException">The bytes are not a well-formed profile.</exception>
    public static ProfileHeader DecodeHeader(ReadOnlySpan<byte> bytes)
    {
        ProtoReader reader = new(bytes.ToArray());
        string? signature = null;
        string? version = null;
        string? id = null;
        string? title = null;
        string? author = null;
        string? publisher = null;
        string? description = null;
        while (reader.TryReadTag(reader.End, out int field, out WireType type))
        {
            switch (field)
            {
                case ProfileSchema.Root.Type:
                    signature = reader.ReadString();
                    break;
                case ProfileSchema.Root.Ver:
                    // One field beyond the header of REQ-PROF-062: the import of AMD-053
                    // REQ-UI-212 names the cause of a refusal, and one of its causes is the
                    // version (R2 point 53-N1).
                    version = reader.ReadString();
                    break;
                case ProfileSchema.Root.Id:
                    id = reader.ReadString();
                    break;
                case ProfileSchema.Root.Title:
                    title = reader.ReadString();
                    break;
                case ProfileSchema.Root.Maker:
                    author = reader.ReadString();
                    break;
                case ProfileSchema.Root.GameDev:
                    publisher = reader.ReadString();
                    break;
                case ProfileSchema.Root.Des:
                    description = reader.ReadString();
                    break;
                default:
                    reader.Skip(field, type);
                    break;
            }
        }

        return new ProfileHeader(signature, id, title, author, publisher, description, version);
    }

    private static int EstimateCapacity(GameProfile profile)
    {
        int patterns = profile.PermanentPatterns.Count + profile.TemporaryPatterns.Count;
        return Math.Clamp(4096 + (patterns * 512), 4096, 32 * 1024 * 1024);
    }
}

/// <summary>
/// The bytes of a profile were not a well-formed profile: malformed protobuf, a reference to an
/// unknown key, a key out of sequence, a truncated file (SPEC-03 REQ-PROF-033).
/// </summary>
public sealed class ProfileFormatException : Exception
{
    /// <summary>Creates the exception.</summary>
    public ProfileFormatException(string message)
        : base(message)
    {
    }

    /// <summary>Creates the exception with an inner cause.</summary>
    public ProfileFormatException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
