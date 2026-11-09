using System.Security.Cryptography;
using System.Text;
using Zax.Profiles.Hash;
using Zax.Profiles.Wire;

namespace Zax.Profiles.Tests;

/// <summary>
/// A minimal writer of the protobuf binary encoding, for the tests that need to build a profile
/// byte by byte. The production writer is internal to <c>Zax.Profiles</c> on purpose, and the
/// synthetic envelopes AMD-062 REQ-PROF-072 asks for have to be malformed in ways no production
/// writer would produce.
/// </summary>
internal sealed class ProtoBuilder
{
    private readonly List<byte> _bytes = [];

    internal void Tag(int field, int wireType) => Varint(((ulong)field << 3) | (uint)wireType);

    internal void Varint(ulong value)
    {
        while (value >= 0x80)
        {
            _bytes.Add((byte)(value | 0x80));
            value >>= 7;
        }

        _bytes.Add((byte)value);
    }

    internal void Int32(int field, int value)
    {
        Tag(field, 0);
        Varint((ulong)(long)value);
    }

    internal void Int64(int field, long value)
    {
        Tag(field, 0);
        Varint((ulong)value);
    }

    internal void Bool(int field, bool value) => Int32(field, value ? 1 : 0);

    internal void Single(int field, float value)
    {
        Tag(field, 5);
        _bytes.AddRange(BitConverter.GetBytes(value));
    }

    internal void Str(int field, string value) => Bytes(field, Encoding.UTF8.GetBytes(value));

    internal void Bytes(int field, ReadOnlySpan<byte> value)
    {
        Tag(field, 2);
        Varint((ulong)value.Length);
        _bytes.AddRange(value);
    }

    internal void Message(int field, Action<ProtoBuilder> body)
    {
        ProtoBuilder nested = new();
        body(nested);
        Bytes(field, nested.ToArray());
    }

    internal void Group(int field, Action<ProtoBuilder> body)
    {
        Tag(field, 3);
        ProtoBuilder nested = new();
        body(nested);
        _bytes.AddRange(nested.ToArray());
        Tag(field, 4);
    }

    internal byte[] ToArray() => [.. _bytes];
}

/// <summary>One cell of a synthetic pattern: a manifest string, or an empty cell.</summary>
/// <param name="Key">The manifest string, or null for an empty cell.</param>
/// <param name="Flip">The flip stored with the tile.</param>
/// <param name="AsIdentifier">True to store field 5 (sixteen bytes), false for field 4.</param>
internal readonly record struct SyntheticCell(string? Key, TileFlip Flip = TileFlip.None, bool AsIdentifier = true)
{
    /// <summary>A cell that carries its planes, which makes it an ordinary tile.</summary>
    internal static SyntheticCell Empty => new(null);
}

/// <summary>
/// Builds synthetic « 3dh » profiles and envelopes, for the branches AMD-062 cannot test on the
/// 101 built-in files: envelope versions 1 and 2, text entries of field 4, legacy digests,
/// malformed manifests and a pattern with a chosen pending tile.
/// </summary>
internal static class SyntheticHashProfile
{
    /// <summary>
    /// Builds a « 3dh » profile holding one permanent pattern of <c>cells.Count × 1</c> cells.
    /// </summary>
    internal static byte[] Build(
        IEnumerable<TileId>? manifest = null,
        IEnumerable<string>? hashes = null,
        IEnumerable<string?>? tileKeys = null,
        IEnumerable<SyntheticCell>? cells = null,
        int? textureKind = 1,
        VariantTexture? xorTexture = null,
        VariantGeometry geometry = VariantGeometry.Default,
        bool fullWarp = false,
        bool hashRootOrder = true,
        string version = "5",
        string? payloadHashOverride = null)
    {
        List<SyntheticCell> grid = cells is not null
            ? [.. cells]
            : [.. (tileKeys ?? []).Select(key => new SyntheticCell(key))];

        byte[] payload = BuildPayload(grid, textureKind, xorTexture, geometry, fullWarp, hashRootOrder);
        return EncodeEnvelope(
            payload,
            version: version,
            hashes: hashes,
            ids: manifest,
            payloadHashOverride: payloadHashOverride);
    }

    /// <summary>
    /// Writes an envelope field by field (REQ-PROF-071), with every value under the test's
    /// control so that a refusal of REQ-PROF-072 can be provoked.
    /// </summary>
    internal static byte[] EncodeEnvelope(
        byte[]? payload,
        string? type = "3dh",
        string? version = "5",
        IEnumerable<string>? hashes = null,
        IEnumerable<TileId>? ids = null,
        byte[]? rawIds = null,
        string? payloadHashOverride = null,
        bool writePayloadHash = true,
        string? id = "SyntheticId",
        string? title = "Synthetic",
        string? author = null,
        string? publisher = null,
        string? description = null)
    {
        ProtoBuilder writer = new();
        if (type is not null)
        {
            writer.Str(1, type);
        }

        if (version is not null)
        {
            writer.Str(2, version);
        }

        if (payload is not null)
        {
            writer.Bytes(3, payload);
        }

        foreach (string hash in hashes ?? [])
        {
            writer.Str(4, hash);
        }

        if (writePayloadHash)
        {
            writer.Str(6, payloadHashOverride ?? Convert.ToHexStringLower(SHA256.HashData(payload ?? [])));
        }

        if (id is not null)
        {
            writer.Str(7, id);
        }

        if (title is not null)
        {
            writer.Str(8, title);
        }

        if (author is not null)
        {
            writer.Str(9, author);
        }

        if (publisher is not null)
        {
            writer.Str(10, publisher);
        }

        if (description is not null)
        {
            writer.Str(11, description);
        }

        byte[]? table = rawIds;
        if (table is null && ids is not null)
        {
            List<TileId> list = [.. ids];
            table = new byte[list.Count * TileId.ByteCount];
            for (int index = 0; index < list.Count; index++)
            {
                list[index].WriteBinary(table.AsSpan(index * TileId.ByteCount));
            }
        }

        if (table is { Length: > 0 })
        {
            writer.Bytes(12, table);
        }

        return writer.ToArray();
    }

    private static byte[] BuildPayload(
        List<SyntheticCell> cells,
        int? textureKind,
        VariantTexture? xorTexture,
        VariantGeometry geometry,
        bool fullWarp,
        bool hashRootOrder)
    {
        ProtoBuilder writer = new();

        void WriteHeader(ProtoBuilder target)
        {
            target.Str(1, "3dn");
            target.Str(2, "2");
        }

        if (!hashRootOrder)
        {
            WriteHeader(writer);
        }

        writer.Message(4, pattern =>
        {
            pattern.Int32(2, 1);
            pattern.Message(10, message => WritePattern(message, cells, textureKind, xorTexture, geometry, fullWarp));
        });
        writer.Message(6, WriteSettings);

        if (hashRootOrder)
        {
            WriteHeader(writer);
        }

        return writer.ToArray();
    }

    private static void WritePattern(
        ProtoBuilder writer,
        List<SyntheticCell> cells,
        int? textureKind,
        VariantTexture? xorTexture,
        VariantGeometry geometry,
        bool fullWarp)
    {
        int nextKey = 2;
        foreach (SyntheticCell cell in cells)
        {
            writer.Group(1, group =>
            {
                if (cell.Key is null)
                {
                    return;
                }

                int key = nextKey++;
                group.Message(1, envelope =>
                {
                    envelope.Int32(2, key);
                    envelope.Message(10, body => body.Message(1, planes =>
                    {
                        planes.Int32(3, (int)cell.Flip);
                        if (cell.AsIdentifier)
                        {
                            Assert.True(
                                TileId.TryParseText(cell.Key, out TileId id),
                                $"\"{cell.Key}\" is not a tile identifier.");
                            planes.Bytes(5, id.ToBinary());
                        }
                        else
                        {
                            planes.Str(4, cell.Key);
                        }
                    }));
                });
            });
        }

        writer.Message(2, size =>
        {
            size.Int32(1, 99);
            size.Int32(2, 99);
        });
        writer.Message(3, size =>
        {
            size.Int32(1, cells.Count);
            size.Int32(2, 1);
        });
        writer.Bool(4, true);
        writer.Int32(5, 0);
        writer.Int32(6, 0);
        writer.Int32(7, 0);
        writer.Int32(8, 0);
        writer.Message(10, envelope =>
        {
            envelope.Int32(2, nextKey);
            envelope.Message(10, variant =>
            {
                variant.Int32(1, 3);
                variant.Int32(2, (int)geometry);
                variant.Bool(6, fullWarp);
                variant.Single(12, 1f);
                if (xorTexture is not null)
                {
                    variant.Message(24, texture => WriteTexture(texture, xorTexture));
                }

                if (textureKind is { } kind)
                {
                    variant.Int32(25, kind);
                }
            });
        });
        writer.Bool(11, true);
        writer.Bool(12, false);
    }

    private static void WriteTexture(ProtoBuilder writer, VariantTexture texture)
    {
        if (texture.Pixels is { } pixels)
        {
            writer.Message(10, payload =>
            {
                foreach (int word in pixels.Data)
                {
                    payload.Int32(1, word);
                }

                payload.Int32(2, (int)pixels.Format);
                foreach (int word in pixels.Extra)
                {
                    payload.Int32(3, word);
                }
            });
        }

        writer.Int32(1, texture.Width);
        writer.Int32(2, texture.Height);
    }

    private static void WriteSettings(ProtoBuilder writer)
    {
        writer.Message(1, light =>
        {
            light.Message(1, direction =>
            {
                direction.Single(1, 1f);
                direction.Single(2, -1f);
                direction.Single(3, 1f);
            });
            light.Message(2, color => color.Int32(1, -1));
            light.Single(3, 1f);
        });
        writer.Bool(2, true);
        writer.Message(4, clipping =>
        {
            clipping.Single(1, 0.1f);
            clipping.Single(2, 8.1f);
            clipping.Single(3, 8.1f);
            clipping.Single(4, 0.1f);
        });
        writer.Message(6, layers =>
        {
            for (int layer = 0; layer < 6; layer++)
            {
                layers.Message(1, parameters =>
                {
                    parameters.Single(1, 0f);
                    parameters.Int32(2, 0);
                    parameters.Bool(3, true);
                    parameters.Bool(4, true);
                });
            }

            layers.Int32(2, 1);
            layers.Bool(3, true);
        });
    }

    /// <summary>The stored form of a plane, which the reader undoes (SPEC-03 REQ-PROF-030).</summary>
    internal static long EncodePlane(ulong plane)
        => (long)System.Numerics.BitOperations.RotateRight(plane ^ TilePlaneCipher.Key, TilePlaneCipher.Rotation);
}
