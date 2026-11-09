using Zax.Profiles.Hash;

namespace Zax.Profiles.Wire;

/// <summary>
/// Reads and writes the envelope of a « 3dh » profile (AMD-062 REQ-PROF-070, 071, 084).
/// </summary>
public static class HashEnvelopeCodec
{
    /// <summary>
    /// The five bytes a « 3dh » profile starts with: field 1, length 3, the ASCII of
    /// <c>"3dh"</c> (REQ-PROF-070).
    /// </summary>
    private static readonly byte[] SignatureBytes = [0x0A, 0x03, 0x33, 0x64, 0x68];

    /// <summary>
    /// True when these bytes are a « 3dh » profile (REQ-PROF-070). Anything else is read as a
    /// 3dn profile, which is what SPEC-03 already does.
    /// </summary>
    public static bool IsHashProfile(ReadOnlySpan<byte> bytes)
        => bytes.Length >= SignatureBytes.Length && bytes[..SignatureBytes.Length].SequenceEqual(SignatureBytes);

    /// <summary>
    /// Decodes the envelope. Fields may come in any order, an unknown field is skipped and an
    /// absent field is null (SPEC-03 §3.1).
    /// </summary>
    /// <exception cref="ProfileFormatException">The bytes are not a well-formed envelope.</exception>
    public static HashProfileEnvelope Decode(ReadOnlySpan<byte> bytes)
    {
        ProtoReader reader = new(bytes.ToArray());
        HashProfileEnvelope envelope = new();
        while (reader.TryReadTag(reader.End, out int field, out WireType type))
        {
            switch (field)
            {
                case ProfileSchema.HashEnvelope.Type:
                    envelope.Type = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.Version:
                    envelope.Version = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.Payload:
                    envelope.Payload = reader.ReadBytes();
                    break;
                case ProfileSchema.HashEnvelope.Hashes:
                    envelope.Hashes.Add(reader.ReadString());
                    break;
                case ProfileSchema.HashEnvelope.PayloadHash:
                    envelope.PayloadHash = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.Id:
                    envelope.Id = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.Title:
                    envelope.Title = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.Maker:
                    envelope.Author = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.GameDev:
                    envelope.Publisher = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.Des:
                    envelope.Description = reader.ReadString();
                    break;
                case ProfileSchema.HashEnvelope.Ids:
                    ReadIds(reader, envelope);
                    break;
                default:
                    reader.Skip(field, type);
                    break;
            }
        }

        return envelope;
    }

    /// <summary>
    /// Reads the field numbers of the envelope in the order they appear, which is what the
    /// acceptance criterion of REQ-PROF-071 looks at.
    /// </summary>
    /// <exception cref="ProfileFormatException">The bytes are not a well-formed envelope.</exception>
    public static IReadOnlyList<int> FieldOrder(ReadOnlySpan<byte> bytes)
    {
        ProtoReader reader = new(bytes.ToArray());
        List<int> order = [];
        while (reader.TryReadTag(reader.End, out int field, out WireType type))
        {
            order.Add(field);
            reader.Skip(field, type);
        }

        return order;
    }

    /// <summary>
    /// Writes the canonical envelope of REQ-PROF-084: fields 1, 2, 3, 4, 6 to 12 in that order,
    /// null strings and empty lists left out, <c>Ids</c> and <c>Hashes</c> sorted by their text
    /// form and <c>PayloadHash</c> recomputed.
    /// </summary>
    public static byte[] Encode(HashProfileEnvelope envelope)
    {
        ArgumentNullException.ThrowIfNull(envelope);

        byte[] payload = envelope.Payload ?? [];
        ProtoWriter writer = new(payload.Length + (envelope.Ids.Count * TileId.ByteCount) + 1024);

        writer.WriteString(ProfileSchema.HashEnvelope.Type, envelope.Type ?? HashProfileEnvelope.Signature);
        writer.WriteString(
            ProfileSchema.HashEnvelope.Version, envelope.Version ?? HashProfileEnvelope.CanonicalVersion);
        writer.WriteBytes(ProfileSchema.HashEnvelope.Payload, payload);

        List<string> hashes = [.. envelope.Hashes];
        hashes.Sort(StringComparer.Ordinal);
        foreach (string hash in hashes)
        {
            writer.WriteString(ProfileSchema.HashEnvelope.Hashes, hash);
        }

        writer.WriteString(ProfileSchema.HashEnvelope.PayloadHash, envelope.ComputePayloadHash());

        WriteOptionalString(writer, ProfileSchema.HashEnvelope.Id, envelope.Id);
        WriteOptionalString(writer, ProfileSchema.HashEnvelope.Title, envelope.Title);
        WriteOptionalString(writer, ProfileSchema.HashEnvelope.Maker, envelope.Author);
        WriteOptionalString(writer, ProfileSchema.HashEnvelope.GameDev, envelope.Publisher);
        WriteOptionalString(writer, ProfileSchema.HashEnvelope.Des, envelope.Description);

        if (envelope.Ids.Count > 0)
        {
            List<TileId> ids = [.. envelope.Ids];
            ids.Sort(static (left, right) => string.CompareOrdinal(left.ToText(), right.ToText()));
            byte[] table = new byte[ids.Count * TileId.ByteCount];
            for (int index = 0; index < ids.Count; index++)
            {
                ids[index].WriteBinary(table.AsSpan(index * TileId.ByteCount));
            }

            writer.WriteBytes(ProfileSchema.HashEnvelope.Ids, table);
        }

        return writer.ToArray();
    }

    private static void ReadIds(ProtoReader reader, HashProfileEnvelope envelope)
    {
        byte[] table = reader.ReadBytes();
        envelope.IdsByteLength = table.Length;
        for (int offset = 0; offset + TileId.ByteCount <= table.Length; offset += TileId.ByteCount)
        {
            envelope.Ids.Add(TileId.FromBinary(table.AsSpan(offset, TileId.ByteCount)));
        }
    }

    private static void WriteOptionalString(ProtoWriter writer, int fieldNumber, string? value)
    {
        if (value is not null)
        {
            writer.WriteString(fieldNumber, value);
        }
    }
}
