using System.Buffers.Binary;
using System.IO.Compression;

namespace Zax.Harness;

/// <summary>
/// Writes a minimal 8-bit RGB or RGBA PNG, so that the offscreen captures of
/// SPEC-01 REQ-ARCH-031 need no image library.
/// </summary>
/// <remarks>
/// One IHDR, one IDAT with the zlib stream of the scanlines (filter 0 on every row) and one
/// IEND, which is what the PNG specification calls a minimal valid file.
/// </remarks>
internal static class PngWriter
{
    /// <summary>Writes <paramref name="pixels"/> as an RGBA PNG.</summary>
    /// <param name="path">Destination file.</param>
    /// <param name="width">Width in pixels.</param>
    /// <param name="height">Height in pixels.</param>
    /// <param name="pixels">Rows top to bottom, 4 bytes per pixel.</param>
    public static void WriteRgba(string path, int width, int height, ReadOnlySpan<byte> pixels)
        => Write(path, width, height, 4, pixels);

    private static void Write(string path, int width, int height, int channels, ReadOnlySpan<byte> pixels)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(width, 1);
        ArgumentOutOfRangeException.ThrowIfLessThan(height, 1);
        int stride = width * channels;
        if (pixels.Length < stride * height)
        {
            throw new ArgumentException("The pixel buffer is shorter than the declared image.", nameof(pixels));
        }

        using FileStream file = File.Create(path);
        file.Write([0x89, (byte)'P', (byte)'N', (byte)'G', 0x0D, 0x0A, 0x1A, 0x0A]);

        byte[] header = new byte[13];
        BinaryPrimitives.WriteInt32BigEndian(header.AsSpan(0, 4), width);
        BinaryPrimitives.WriteInt32BigEndian(header.AsSpan(4, 4), height);
        header[8] = 8;
        header[9] = (byte)(channels == 4 ? 6 : 2);
        WriteChunk(file, "IHDR", header);

        byte[] raw = new byte[(stride + 1) * height];
        for (int y = 0; y < height; y++)
        {
            raw[y * (stride + 1)] = 0;
            pixels.Slice(y * stride, stride).CopyTo(raw.AsSpan((y * (stride + 1)) + 1, stride));
        }

        using MemoryStream compressed = new();
        using (ZLibStream deflate = new(compressed, CompressionLevel.Optimal, leaveOpen: true))
        {
            deflate.Write(raw);
        }

        WriteChunk(file, "IDAT", compressed.ToArray());
        WriteChunk(file, "IEND", []);
    }

    private static void WriteChunk(Stream stream, string type, ReadOnlySpan<byte> data)
    {
        Span<byte> length = stackalloc byte[4];
        BinaryPrimitives.WriteInt32BigEndian(length, data.Length);
        stream.Write(length);

        Span<byte> name = stackalloc byte[4];
        for (int i = 0; i < 4; i++)
        {
            name[i] = (byte)type[i];
        }

        stream.Write(name);
        stream.Write(data);

        uint crc = Crc32(name, data);
        Span<byte> checksum = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(checksum, crc);
        stream.Write(checksum);
    }

    private static readonly uint[] CrcTable = BuildCrcTable();

    private static uint[] BuildCrcTable()
    {
        uint[] table = new uint[256];
        for (uint n = 0; n < 256; n++)
        {
            uint c = n;
            for (int k = 0; k < 8; k++)
            {
                c = (c & 1) != 0 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
            }

            table[n] = c;
        }

        return table;
    }

    private static uint Crc32(ReadOnlySpan<byte> first, ReadOnlySpan<byte> second)
    {
        uint crc = 0xFFFFFFFFu;
        foreach (byte value in first)
        {
            crc = CrcTable[(crc ^ value) & 0xFF] ^ (crc >> 8);
        }

        foreach (byte value in second)
        {
            crc = CrcTable[(crc ^ value) & 0xFF] ^ (crc >> 8);
        }

        return crc ^ 0xFFFFFFFFu;
    }
}
