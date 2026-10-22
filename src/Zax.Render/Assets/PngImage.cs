using System.Buffers.Binary;
using System.IO.Compression;

namespace Zax.Render.Assets;

/// <summary>
/// A decoder for the PNG files of <c>assets-neutral/</c>, written against the public PNG
/// specification (ISO/IEC 15948, W3C PNG 1.2).
/// </summary>
/// <remarks>
/// Only what the extracted assets use is supported: non-interlaced images, bit depths 8 and 16
/// (16 keeps the high byte) and colour types 0, 2, 3, 4 and 6. Everything is returned as 8-bit
/// RGBA. <c>.NET</c> supplies the zlib stream, so only the chunk layer and the five filters of
/// §9.2 are implemented here.
/// </remarks>
public static class PngImage
{
    private static readonly byte[] Signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    /// <summary>Decodes the PNG file <paramref name="path"/>.</summary>
    /// <exception cref="InvalidDataException">The file is not a PNG this decoder supports.</exception>
    public static ImageData Load(string path) => Decode(File.ReadAllBytes(path));

    /// <summary>
    /// Writes an 8-bit RGB PNG: the savestate thumbnails of SPEC-09 REQ-PERS-051 and the cover
    /// art of REQ-PERS-036, which are both stored without an alpha channel.
    /// </summary>
    /// <param name="path">Destination file.</param>
    /// <param name="width">Width in pixels.</param>
    /// <param name="height">Height in pixels.</param>
    /// <param name="pixels">Rows top to bottom, three bytes per pixel.</param>
    public static void WriteRgb(string path, int width, int height, ReadOnlySpan<byte> pixels)
        => File.WriteAllBytes(path, EncodeRgb(width, height, pixels));

    /// <summary>Encodes an 8-bit RGB PNG into memory.</summary>
    /// <param name="width">Width in pixels.</param>
    /// <param name="height">Height in pixels.</param>
    /// <param name="pixels">Rows top to bottom, three bytes per pixel.</param>
    public static byte[] EncodeRgb(int width, int height, ReadOnlySpan<byte> pixels)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(width, 1);
        ArgumentOutOfRangeException.ThrowIfLessThan(height, 1);

        int stride = width * 3;
        if (pixels.Length < stride * height)
        {
            throw new ArgumentException(
                "The pixel buffer is shorter than the declared image.", nameof(pixels));
        }

        using MemoryStream file = new();
        file.Write(Signature);

        byte[] header = new byte[13];
        BinaryPrimitives.WriteInt32BigEndian(header.AsSpan(0, 4), width);
        BinaryPrimitives.WriteInt32BigEndian(header.AsSpan(4, 4), height);
        header[8] = 8;  // bit depth
        header[9] = 2;  // colour type 2: truecolour without alpha
        WriteChunk(file, "IHDR"u8, header);

        // Filter 0 on every row, which §9.2 of the specification always allows.
        byte[] raw = new byte[(stride + 1) * height];
        for (int y = 0; y < height; y++)
        {
            pixels.Slice(y * stride, stride).CopyTo(raw.AsSpan((y * (stride + 1)) + 1, stride));
        }

        using MemoryStream deflated = new();
        using (ZLibStream stream = new(deflated, CompressionLevel.Optimal, leaveOpen: true))
        {
            stream.Write(raw);
        }

        WriteChunk(file, "IDAT"u8, deflated.ToArray());
        WriteChunk(file, "IEND"u8, []);
        return file.ToArray();
    }

    private static void WriteChunk(Stream stream, ReadOnlySpan<byte> type, ReadOnlySpan<byte> data)
    {
        Span<byte> length = stackalloc byte[4];
        BinaryPrimitives.WriteInt32BigEndian(length, data.Length);
        stream.Write(length);
        stream.Write(type);
        stream.Write(data);

        Span<byte> checksum = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(checksum, Crc32(type, data));
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

    /// <summary>Decodes a PNG file held in memory.</summary>
    /// <exception cref="InvalidDataException">The data is not a PNG this decoder supports.</exception>
    public static ImageData Decode(ReadOnlySpan<byte> file)
    {
        if (file.Length < Signature.Length || !file[..Signature.Length].SequenceEqual(Signature))
        {
            throw new InvalidDataException("Not a PNG file: the signature does not match.");
        }

        int width = 0;
        int height = 0;
        int bitDepth = 0;
        int colorType = 0;
        byte[]? palette = null;
        byte[]? paletteAlpha = null;
        using MemoryStream compressed = new();

        int at = Signature.Length;
        while (at + 8 <= file.Length)
        {
            int length = BinaryPrimitives.ReadInt32BigEndian(file[at..]);
            if (length < 0 || at + 12 + length > file.Length)
            {
                throw new InvalidDataException("Truncated PNG chunk.");
            }

            ReadOnlySpan<byte> type = file.Slice(at + 4, 4);
            ReadOnlySpan<byte> data = file.Slice(at + 8, length);
            at += 12 + length;

            if (type.SequenceEqual("IHDR"u8))
            {
                width = BinaryPrimitives.ReadInt32BigEndian(data);
                height = BinaryPrimitives.ReadInt32BigEndian(data[4..]);
                bitDepth = data[8];
                colorType = data[9];
                if (data[12] != 0)
                {
                    throw new InvalidDataException("Interlaced PNG files are not supported.");
                }
            }
            else if (type.SequenceEqual("PLTE"u8))
            {
                palette = data.ToArray();
            }
            else if (type.SequenceEqual("tRNS"u8))
            {
                paletteAlpha = data.ToArray();
            }
            else if (type.SequenceEqual("IDAT"u8))
            {
                compressed.Write(data);
            }
            else if (type.SequenceEqual("IEND"u8))
            {
                break;
            }
        }

        if (width <= 0 || height <= 0)
        {
            throw new InvalidDataException("The PNG file has no usable IHDR chunk.");
        }

        if (bitDepth is not (8 or 16))
        {
            throw new InvalidDataException($"PNG bit depth {bitDepth} is not supported.");
        }

        int channels = colorType switch
        {
            0 => 1,
            2 => 3,
            3 => 1,
            4 => 2,
            6 => 4,
            _ => throw new InvalidDataException($"PNG colour type {colorType} is not supported."),
        };

        int sampleBytes = bitDepth / 8;
        int pixelBytes = channels * sampleBytes;
        int stride = width * pixelBytes;

        compressed.Position = 0;
        byte[] raw = new byte[(stride + 1) * height];
        using (ZLibStream inflate = new(compressed, CompressionMode.Decompress, leaveOpen: true))
        {
            inflate.ReadExactly(raw);
        }

        byte[] lines = Unfilter(raw, width, height, stride, pixelBytes);
        return new ImageData(width, height, ToRgba(lines, width, height, stride, colorType, sampleBytes, palette, paletteAlpha));
    }

    /// <summary>Applies the five filters of PNG §9.2, in place, and drops the filter bytes.</summary>
    private static byte[] Unfilter(byte[] raw, int width, int height, int stride, int pixelBytes)
    {
        byte[] lines = new byte[stride * height];
        for (int y = 0; y < height; y++)
        {
            int source = (y * (stride + 1)) + 1;
            int target = y * stride;
            int filter = raw[source - 1];
            int previous = target - stride;

            for (int x = 0; x < stride; x++)
            {
                int a = x >= pixelBytes ? lines[target + x - pixelBytes] : 0;
                int b = y > 0 ? lines[previous + x] : 0;
                int c = y > 0 && x >= pixelBytes ? lines[previous + x - pixelBytes] : 0;
                int value = raw[source + x];

                lines[target + x] = filter switch
                {
                    0 => (byte)value,
                    1 => (byte)(value + a),
                    2 => (byte)(value + b),
                    3 => (byte)(value + ((a + b) >> 1)),
                    4 => (byte)(value + Paeth(a, b, c)),
                    _ => throw new InvalidDataException($"Unknown PNG filter {filter}."),
                };
            }
        }

        _ = width;
        return lines;
    }

    private static int Paeth(int a, int b, int c)
    {
        int p = a + b - c;
        int pa = Math.Abs(p - a);
        int pb = Math.Abs(p - b);
        int pc = Math.Abs(p - c);
        return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
    }

    private static byte[] ToRgba(
        byte[] lines,
        int width,
        int height,
        int stride,
        int colorType,
        int sampleBytes,
        byte[]? palette,
        byte[]? paletteAlpha)
    {
        byte[] pixels = new byte[width * height * 4];
        for (int y = 0; y < height; y++)
        {
            int source = y * stride;
            int target = y * width * 4;
            for (int x = 0; x < width; x++)
            {
                byte r;
                byte g;
                byte b;
                byte a = 255;

                switch (colorType)
                {
                    case 0:
                        r = g = b = lines[source];
                        source += sampleBytes;
                        break;

                    case 2:
                        r = lines[source];
                        g = lines[source + sampleBytes];
                        b = lines[source + (2 * sampleBytes)];
                        source += 3 * sampleBytes;
                        break;

                    case 3:
                        int index = lines[source];
                        source += sampleBytes;
                        if (palette is null || (index * 3) + 2 >= palette.Length)
                        {
                            r = g = b = 0;
                        }
                        else
                        {
                            r = palette[index * 3];
                            g = palette[(index * 3) + 1];
                            b = palette[(index * 3) + 2];
                        }

                        if (paletteAlpha is not null && index < paletteAlpha.Length)
                        {
                            a = paletteAlpha[index];
                        }

                        break;

                    case 4:
                        r = g = b = lines[source];
                        a = lines[source + sampleBytes];
                        source += 2 * sampleBytes;
                        break;

                    default:
                        r = lines[source];
                        g = lines[source + sampleBytes];
                        b = lines[source + (2 * sampleBytes)];
                        a = lines[source + (3 * sampleBytes)];
                        source += 4 * sampleBytes;
                        break;
                }

                pixels[target++] = r;
                pixels[target++] = g;
                pixels[target++] = b;
                pixels[target++] = a;
            }
        }

        return pixels;
    }
}
