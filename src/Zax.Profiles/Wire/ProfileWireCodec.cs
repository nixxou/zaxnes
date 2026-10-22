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

        return new ProfileHeader(signature, id, title, author, publisher, description);
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
