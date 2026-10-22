namespace Zax.Render.Assets;

/// <summary>
/// A decoder for baseline JPEG files, written against the public standards: ITU-T T.81
/// (ISO/IEC 10918-1) for the entropy coding and the inverse DCT, and the JFIF note for the
/// colour conversion.
/// </summary>
/// <remarks>
/// <para>
/// AMD-025 asks for a JPEG decoder "written from the public standards, no new dependency": the
/// glTF model of the television carries one JPEG among its six textures, and
/// <see cref="GltfBinary"/> hands its bytes here.
/// </para>
/// <para>
/// What is supported is baseline sequential DCT, Huffman coded (SOF0 and, since the two differ
/// only in the entropy coder's optimisation, SOF1): one to four components, 8-bit samples, any
/// sampling factor up to 4 × 4, restart markers. Progressive (SOF2) and arithmetic coding are
/// not: a file using them is rejected rather than half-decoded. Output is always 8-bit RGBA,
/// rows top-down, like <see cref="PngImage"/>.
/// </para>
/// </remarks>
public static class JpegImage
{
    /// <summary>Decodes the JPEG file <paramref name="path"/>.</summary>
    /// <exception cref="InvalidDataException">The file is not a JPEG this decoder supports.</exception>
    public static ImageData Load(string path) => Decode(File.ReadAllBytes(path));

    /// <summary>Whether <paramref name="file"/> starts with the SOI marker of a JPEG stream.</summary>
    public static bool IsJpeg(ReadOnlySpan<byte> file)
        => file.Length >= 2 && file[0] == 0xFF && file[1] == 0xD8;

    /// <summary>Decodes a JPEG file held in memory.</summary>
    /// <exception cref="InvalidDataException">The data is not a JPEG this decoder supports.</exception>
    public static ImageData Decode(ReadOnlySpan<byte> file)
    {
        Decoder decoder = new(file);
        return decoder.Run();
    }

    /// <summary>The zig-zag order of T.81 figure A.6, from coefficient index to raster index.</summary>
    private static readonly int[] ZigZag =
    [
        0, 1, 8, 16, 9, 2, 3, 10,
        17, 24, 32, 25, 18, 11, 4, 5,
        12, 19, 26, 33, 40, 48, 41, 34,
        27, 20, 13, 6, 7, 14, 21, 28,
        35, 42, 49, 56, 57, 50, 43, 36,
        29, 22, 15, 23, 30, 37, 44, 51,
        58, 59, 52, 45, 38, 31, 39, 46,
        53, 60, 61, 54, 47, 55, 62, 63,
    ];

    /// <summary>One Huffman table, decoded into the canonical form of T.81 §F.2.2.3.</summary>
    private sealed class HuffmanTable
    {
        private readonly int[] _minCode = new int[17];
        private readonly int[] _maxCode = new int[17];
        private readonly int[] _firstIndex = new int[17];
        private readonly byte[] _values;

        public HuffmanTable(ReadOnlySpan<byte> counts, byte[] values)
        {
            _values = values;

            int code = 0;
            int index = 0;
            for (int length = 1; length <= 16; length++)
            {
                _firstIndex[length] = index;
                _minCode[length] = code;
                code += counts[length - 1];
                index += counts[length - 1];
                _maxCode[length] = counts[length - 1] == 0 ? -1 : code - 1;
                code <<= 1;
            }
        }

        /// <summary>Reads one symbol, bit by bit, as T.81 §F.2.2.3 decodes it.</summary>
        public byte Decode(ref BitReader bits)
        {
            int code = 0;
            for (int length = 1; length <= 16; length++)
            {
                code = (code << 1) | bits.ReadBit();
                if (_maxCode[length] >= 0 && code <= _maxCode[length])
                {
                    int at = _firstIndex[length] + (code - _minCode[length]);
                    if (at < 0 || at >= _values.Length)
                    {
                        throw new InvalidDataException("Corrupt JPEG Huffman table.");
                    }

                    return _values[at];
                }
            }

            throw new InvalidDataException("Corrupt JPEG Huffman code.");
        }
    }

    /// <summary>
    /// The entropy-coded segment read bit by bit, with the byte stuffing of T.81 §F.1.2.3:
    /// a <c>0xFF</c> followed by <c>0x00</c> is a data byte, any other marker ends the segment.
    /// </summary>
    private ref struct BitReader(ReadOnlySpan<byte> data, int at)
    {
        private readonly ReadOnlySpan<byte> _data = data;
        private int _at = at;
        private int _bits;
        private int _count;

        /// <summary>Where the reader stands, markers included.</summary>
        public readonly int Position => _at;

        /// <summary>Drops the partial byte, as a restart marker requires (§F.2.1.3.1).</summary>
        public void Align()
        {
            _bits = 0;
            _count = 0;
        }

        /// <summary>Skips a two-byte marker at the current position.</summary>
        public void SkipMarker() => _at += 2;

        /// <summary>The marker at the current position, or 0 when there is none.</summary>
        public readonly byte PeekMarker()
            => _at + 1 < _data.Length && _data[_at] == 0xFF && _data[_at + 1] != 0x00
                ? _data[_at + 1]
                : (byte)0;

        /// <summary>One bit, most significant first; past the end the stream reads as zero.</summary>
        public int ReadBit()
        {
            if (_count == 0)
            {
                if (_at >= _data.Length)
                {
                    return 0;
                }

                byte value = _data[_at];
                if (value == 0xFF)
                {
                    byte next = _at + 1 < _data.Length ? _data[_at + 1] : (byte)0xD9;
                    if (next != 0x00)
                    {
                        // A marker: the segment is over, the rest reads as zero.
                        return 0;
                    }

                    _at += 2;
                }
                else
                {
                    _at++;
                }

                _bits = value;
                _count = 8;
            }

            _count--;
            return (_bits >> _count) & 1;
        }

        /// <summary><paramref name="length"/> bits as an unsigned integer.</summary>
        public int ReadBits(int length)
        {
            int value = 0;
            for (int i = 0; i < length; i++)
            {
                value = (value << 1) | ReadBit();
            }

            return value;
        }

        /// <summary>The EXTEND procedure of T.81 §F.2.2.1: a signed coefficient difference.</summary>
        public int ReadSigned(int length)
        {
            if (length == 0)
            {
                return 0;
            }

            int value = ReadBits(length);
            return value < (1 << (length - 1)) ? value - (1 << length) + 1 : value;
        }
    }

    private sealed class Component
    {
        public int Id;
        public int HorizontalFactor = 1;
        public int VerticalFactor = 1;
        public int QuantTable;
        public int DcTable;
        public int AcTable;
        public int Predictor;
        public int BlocksPerLine;
        public int BlocksPerColumn;
        public byte[] Samples = [];
        public int Stride;
    }

    private ref struct Decoder(ReadOnlySpan<byte> file)
    {
        private readonly ReadOnlySpan<byte> _file = file;
        private readonly ushort[][] _quant = new ushort[4][];
        private readonly HuffmanTable?[] _dc = new HuffmanTable?[4];
        private readonly HuffmanTable?[] _ac = new HuffmanTable?[4];
        private Component[] _components = [];
        private int _width;
        private int _height;
        private int _maxH = 1;
        private int _maxV = 1;
        private int _mcusPerLine;
        private int _mcusPerColumn;
        private int _restartInterval;

        public ImageData Run()
        {
            if (!IsJpeg(_file))
            {
                throw new InvalidDataException("Not a JPEG file: the SOI marker is missing.");
            }

            int at = 2;
            while (at + 1 < _file.Length)
            {
                if (_file[at] != 0xFF)
                {
                    at++;
                    continue;
                }

                byte marker = _file[at + 1];
                at += 2;
                switch (marker)
                {
                    case 0xD8: // SOI
                    case 0x01: // TEM
                        continue;

                    case >= 0xD0 and <= 0xD7: // RSTn
                        continue;

                    case 0xD9: // EOI
                        at = _file.Length;
                        continue;
                }

                if (at + 1 >= _file.Length)
                {
                    break;
                }

                int length = (_file[at] << 8) | _file[at + 1];
                if (length < 2 || at + length > _file.Length)
                {
                    throw new InvalidDataException("Truncated JPEG segment.");
                }

                ReadOnlySpan<byte> payload = _file.Slice(at + 2, length - 2);
                switch (marker)
                {
                    case 0xC0: // SOF0, baseline
                    case 0xC1: // SOF1, extended sequential: same decoding path
                        ReadFrame(payload);
                        break;

                    case 0xC2:
                        throw new InvalidDataException("Progressive JPEG files are not supported.");

                    case 0xC3 or (>= 0xC5 and <= 0xC7) or (>= 0xC9 and <= 0xCF):
                        throw new InvalidDataException(
                            $"JPEG frames of type 0x{marker:X2} are not supported.");

                    case 0xC4: // DHT
                        ReadHuffmanTables(payload);
                        break;

                    case 0xDB: // DQT
                        ReadQuantTables(payload);
                        break;

                    case 0xDD: // DRI
                        _restartInterval = payload.Length >= 2 ? (payload[0] << 8) | payload[1] : 0;
                        break;

                    case 0xDA: // SOS
                        at = ReadScan(payload, at + length);
                        continue;

                    default:
                        break;
                }

                at += length;
            }

            if (_width <= 0 || _height <= 0 || _components.Length == 0)
            {
                throw new InvalidDataException("The JPEG file has no usable frame header.");
            }

            return ToRgba();
        }

        private void ReadFrame(ReadOnlySpan<byte> payload)
        {
            if (payload.Length < 6)
            {
                throw new InvalidDataException("Truncated JPEG frame header.");
            }

            if (payload[0] != 8)
            {
                throw new InvalidDataException($"JPEG sample precision {payload[0]} is not supported.");
            }

            _height = (payload[1] << 8) | payload[2];
            _width = (payload[3] << 8) | payload[4];
            int count = payload[5];
            if (count is < 1 or > 4 || payload.Length < 6 + (count * 3))
            {
                throw new InvalidDataException($"JPEG component count {count} is not supported.");
            }

            _components = new Component[count];
            for (int i = 0; i < count; i++)
            {
                int at = 6 + (i * 3);
                Component component = new()
                {
                    Id = payload[at],
                    HorizontalFactor = payload[at + 1] >> 4,
                    VerticalFactor = payload[at + 1] & 0x0F,
                    QuantTable = payload[at + 2] & 3,
                };

                if (component.HorizontalFactor is < 1 or > 4 || component.VerticalFactor is < 1 or > 4)
                {
                    throw new InvalidDataException("JPEG sampling factors must be 1 to 4.");
                }

                _components[i] = component;
                _maxH = Math.Max(_maxH, component.HorizontalFactor);
                _maxV = Math.Max(_maxV, component.VerticalFactor);
            }

            _mcusPerLine = (_width + (8 * _maxH) - 1) / (8 * _maxH);
            _mcusPerColumn = (_height + (8 * _maxV) - 1) / (8 * _maxV);

            foreach (Component component in _components)
            {
                component.BlocksPerLine = _mcusPerLine * component.HorizontalFactor;
                component.BlocksPerColumn = _mcusPerColumn * component.VerticalFactor;
                component.Stride = component.BlocksPerLine * 8;
                component.Samples = new byte[component.Stride * component.BlocksPerColumn * 8];
            }
        }

        private readonly void ReadQuantTables(ReadOnlySpan<byte> payload)
        {
            int at = 0;
            while (at < payload.Length)
            {
                int precision = payload[at] >> 4;
                int slot = payload[at] & 0x0F;
                at++;
                if (slot > 3)
                {
                    throw new InvalidDataException($"JPEG quantisation table {slot} is out of range.");
                }

                ushort[] table = new ushort[64];
                for (int i = 0; i < 64; i++)
                {
                    if (precision == 0)
                    {
                        table[ZigZag[i]] = payload[at++];
                    }
                    else
                    {
                        table[ZigZag[i]] = (ushort)((payload[at] << 8) | payload[at + 1]);
                        at += 2;
                    }
                }

                _quant[slot] = table;
            }
        }

        private readonly void ReadHuffmanTables(ReadOnlySpan<byte> payload)
        {
            int at = 0;
            while (at + 17 <= payload.Length)
            {
                int kind = payload[at] >> 4;
                int slot = payload[at] & 0x0F;
                at++;
                if (slot > 3)
                {
                    throw new InvalidDataException($"JPEG Huffman table {slot} is out of range.");
                }

                ReadOnlySpan<byte> counts = payload.Slice(at, 16);
                at += 16;

                int total = 0;
                foreach (byte count in counts)
                {
                    total += count;
                }

                if (at + total > payload.Length)
                {
                    throw new InvalidDataException("Truncated JPEG Huffman table.");
                }

                byte[] values = payload.Slice(at, total).ToArray();
                at += total;

                HuffmanTable table = new(counts, values);
                if (kind == 0)
                {
                    _dc[slot] = table;
                }
                else
                {
                    _ac[slot] = table;
                }
            }
        }

        /// <summary>Decodes one baseline scan and returns where the stream continues.</summary>
        private int ReadScan(ReadOnlySpan<byte> header, int dataStart)
        {
            if (_components.Length == 0)
            {
                throw new InvalidDataException("A JPEG scan came before its frame header.");
            }

            int count = header.Length > 0 ? header[0] : 0;
            if (count < 1 || header.Length < 1 + (count * 2))
            {
                throw new InvalidDataException("Truncated JPEG scan header.");
            }

            Component[] scan = new Component[count];
            for (int i = 0; i < count; i++)
            {
                int id = header[1 + (i * 2)];
                Component component = Array.Find(_components, c => c.Id == id)
                    ?? throw new InvalidDataException($"The JPEG scan names an unknown component {id}.");
                component.DcTable = header[2 + (i * 2)] >> 4;
                component.AcTable = header[2 + (i * 2)] & 0x0F;
                component.Predictor = 0;
                scan[i] = component;
            }

            BitReader bits = new(_file, dataStart);
            int[] block = new int[64];
            byte[] pixels = new byte[64];

            int restart = _restartInterval;
            int untilRestart = restart;
            int mcus = _mcusPerLine * _mcusPerColumn;

            // A single-component scan walks that component's own blocks, not the MCU grid
            // (T.81 §A.2.2); with one component in the frame the two coincide anyway.
            bool interleaved = scan.Length > 1;
            int units = interleaved ? mcus : BlocksOf(scan[0]);

            for (int unit = 0; unit < units; unit++)
            {
                if (restart > 0 && untilRestart == 0)
                {
                    bits.Align();
                    byte marker = bits.PeekMarker();
                    if (marker is >= 0xD0 and <= 0xD7)
                    {
                        bits.SkipMarker();
                    }

                    foreach (Component component in scan)
                    {
                        component.Predictor = 0;
                    }

                    untilRestart = restart;
                }

                if (interleaved)
                {
                    int mcuX = unit % _mcusPerLine;
                    int mcuY = unit / _mcusPerLine;
                    foreach (Component component in scan)
                    {
                        for (int v = 0; v < component.VerticalFactor; v++)
                        {
                            for (int h = 0; h < component.HorizontalFactor; h++)
                            {
                                DecodeBlock(
                                    ref bits,
                                    component,
                                    (mcuX * component.HorizontalFactor) + h,
                                    (mcuY * component.VerticalFactor) + v,
                                    block,
                                    pixels);
                            }
                        }
                    }
                }
                else
                {
                    Component component = scan[0];
                    int perLine = BlocksAcross(component);
                    DecodeBlock(ref bits, component, unit % perLine, unit / perLine, block, pixels);
                }

                untilRestart--;
            }

            // The scan ends at the next marker that is not a restart.
            int at = bits.Position;
            while (at + 1 < _file.Length)
            {
                if (_file[at] == 0xFF && _file[at + 1] != 0x00
                    && !(_file[at + 1] >= 0xD0 && _file[at + 1] <= 0xD7))
                {
                    return at;
                }

                at++;
            }

            return _file.Length;
        }

        /// <summary>Blocks across one component in a non-interleaved scan (§A.2.2).</summary>
        private readonly int BlocksAcross(Component component)
        {
            int across = ((_width * component.HorizontalFactor / _maxH) + 7) / 8;
            return Math.Max(across, 1);
        }

        /// <summary>Blocks of one component a non-interleaved scan covers (§A.2.2).</summary>
        private readonly int BlocksOf(Component component)
        {
            int down = ((_height * component.VerticalFactor / _maxV) + 7) / 8;
            return BlocksAcross(component) * Math.Max(down, 1);
        }

        private readonly void DecodeBlock(
            ref BitReader bits,
            Component component,
            int blockX,
            int blockY,
            int[] block,
            byte[] pixels)
        {
            Array.Clear(block);

            HuffmanTable dc = _dc[component.DcTable]
                ?? throw new InvalidDataException("The JPEG scan uses an undefined DC table.");
            HuffmanTable ac = _ac[component.AcTable]
                ?? throw new InvalidDataException("The JPEG scan uses an undefined AC table.");
            ushort[] quant = _quant[component.QuantTable]
                ?? throw new InvalidDataException("The JPEG scan uses an undefined quantisation table.");

            int t = dc.Decode(ref bits);
            component.Predictor += bits.ReadSigned(t);
            block[0] = component.Predictor * quant[0];

            for (int k = 1; k < 64;)
            {
                int rs = ac.Decode(ref bits);
                int run = rs >> 4;
                int size = rs & 0x0F;
                if (size == 0)
                {
                    if (run != 15)
                    {
                        break; // EOB
                    }

                    k += 16;
                    continue;
                }

                k += run;
                if (k > 63)
                {
                    break;
                }

                int raster = ZigZag[k];
                block[raster] = bits.ReadSigned(size) * quant[raster];
                k++;
            }

            InverseDct(block, pixels);

            // Blocks past the right or bottom edge of the component still exist in the MCU
            // grid; their samples are simply never read back.
            int originX = blockX * 8;
            int originY = blockY * 8;
            if (originX >= component.Stride || originY >= component.BlocksPerColumn * 8)
            {
                return;
            }

            for (int y = 0; y < 8; y++)
            {
                int target = ((originY + y) * component.Stride) + originX;
                if (target + 8 > component.Samples.Length)
                {
                    break;
                }

                pixels.AsSpan(y * 8, 8).CopyTo(component.Samples.AsSpan(target, 8));
            }
        }

        /// <summary>
        /// The separable inverse DCT of T.81 §A.3.3, written straight from the definition:
        /// eight one-dimensional transforms on the rows, then eight on the columns, with the
        /// level shift of +128 and the clamp to [0, 255] of §A.3.1.
        /// </summary>
        private static void InverseDct(int[] block, byte[] pixels)
        {
            Span<float> rows = stackalloc float[64];
            for (int y = 0; y < 8; y++)
            {
                for (int x = 0; x < 8; x++)
                {
                    float sum = 0f;
                    for (int u = 0; u < 8; u++)
                    {
                        float coefficient = block[(y * 8) + u];
                        if (coefficient != 0f)
                        {
                            sum += Alpha(u) * coefficient * Cosine[(x * 8) + u];
                        }
                    }

                    rows[(y * 8) + x] = sum;
                }
            }

            for (int x = 0; x < 8; x++)
            {
                for (int y = 0; y < 8; y++)
                {
                    float sum = 0f;
                    for (int v = 0; v < 8; v++)
                    {
                        sum += Alpha(v) * rows[(v * 8) + x] * Cosine[(y * 8) + v];
                    }

                    // Each of the two passes carries a factor 1/2, so the separable transform
                    // of §A.3.3 ends on 1/4 of the sum.
                    int value = (int)MathF.Round((sum * 0.25f) + 128f);
                    pixels[(y * 8) + x] = (byte)Math.Clamp(value, 0, 255);
                }
            }
        }

        private static float Alpha(int u) => u == 0 ? 0.70710678f : 1f;

        /// <summary><c>cos((2x + 1) u π / 16)</c>, the only transcendental the IDCT needs.</summary>
        private static readonly float[] Cosine = BuildCosine();

        private static float[] BuildCosine()
        {
            float[] table = new float[64];
            for (int x = 0; x < 8; x++)
            {
                for (int u = 0; u < 8; u++)
                {
                    table[(x * 8) + u] = MathF.Cos(((2 * x) + 1) * u * MathF.PI / 16f);
                }
            }

            return table;
        }

        /// <summary>
        /// Upsamples every component to the full image grid and converts to RGBA: the JFIF
        /// YCbCr formulas for three components, grey for one, and YCCK/CMYK is rejected.
        /// </summary>
        private readonly ImageData ToRgba()
        {
            byte[] pixels = new byte[_width * _height * 4];
            Component[] parts = _components;

            for (int y = 0; y < _height; y++)
            {
                for (int x = 0; x < _width; x++)
                {
                    int at = ((y * _width) + x) * 4;
                    byte r;
                    byte g;
                    byte b;

                    if (parts.Length >= 3)
                    {
                        float luma = Sample(parts[0], x, y);
                        float cb = Sample(parts[1], x, y) - 128f;
                        float cr = Sample(parts[2], x, y) - 128f;
                        r = Clamp(luma + (1.402f * cr));
                        g = Clamp(luma - (0.344136f * cb) - (0.714136f * cr));
                        b = Clamp(luma + (1.772f * cb));
                    }
                    else
                    {
                        r = g = b = Clamp(Sample(parts[0], x, y));
                    }

                    pixels[at] = r;
                    pixels[at + 1] = g;
                    pixels[at + 2] = b;
                    pixels[at + 3] = 255;
                }
            }

            return new ImageData(_width, _height, pixels);
        }

        /// <summary>
        /// The sample of <paramref name="component"/> under the full-resolution pixel
        /// (<paramref name="x"/>, <paramref name="y"/>), by nearest neighbour — what §A.2.1
        /// calls replication, which is what the sampling factors of a baseline file describe.
        /// </summary>
        private readonly float Sample(Component component, int x, int y)
        {
            int sx = x * component.HorizontalFactor / _maxH;
            int sy = y * component.VerticalFactor / _maxV;
            sx = Math.Min(sx, component.Stride - 1);
            int rows = component.Samples.Length / Math.Max(component.Stride, 1);
            sy = Math.Min(sy, rows - 1);
            return component.Samples[(sy * component.Stride) + sx];
        }

        private static byte Clamp(float value)
            => (byte)Math.Clamp((int)MathF.Round(value), 0, 255);
    }
}
