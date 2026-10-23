namespace Zax.Profiles.Import;

/// <summary>
/// The compression of a bundle's information block or of one data block (AMD-030 §4.1, §4.2):
/// bits 0 to 5 of the flags.
/// </summary>
public enum UnityCompression
{
    /// <summary>Stored as is.</summary>
    None = 0,

    /// <summary>LZMA, which this reader refuses on purpose (AMD-030 §4.1).</summary>
    Lzma = 1,

    /// <summary>LZ4 block format.</summary>
    Lz4 = 2,

    /// <summary>LZ4HC: the same block format, a different encoder.</summary>
    Lz4Hc = 3,
}

/// <summary>One file node of a bundle (AMD-030 §4.2).</summary>
/// <param name="Path">The node's name, <c>CAB-…</c> for the serialized file.</param>
/// <param name="Flags">The node's flags, 4 on the measured bundle.</param>
/// <param name="Data">The node's bytes, cut out of the decompressed stream.</param>
public sealed record UnityBundleFile(string Path, uint Flags, byte[] Data);

/// <summary>
/// Reader of a Unity « UnityFS » asset bundle, the public format AMD-030 §4 describes: header,
/// 16-byte alignment, information block at the start or at the end, data blocks, file nodes.
/// </summary>
/// <remarks>
/// <para>
/// Nothing of Unity is linked in and no third-party package is added: the format is read from
/// its public description, and the only compressions supported are « none », LZ4 and LZ4HC
/// (<see cref="Lz4Block"/>). LZMA is refused with a message, as AMD-030 §4.1 asks.
/// </para>
/// <para>
/// Every count, size and offset is checked against what the file actually holds before it is
/// used, and the sizes a bundle may announce are capped: the reader never allocates a buffer a
/// corrupt header asks for (AMD-030 §5, robustness).
/// </para>
/// </remarks>
public sealed class UnityBundle
{
    /// <summary>The only signature this reader accepts.</summary>
    public const string Signature = "UnityFS";

    /// <summary>Lowest accepted format version (AMD-030 §4.1).</summary>
    public const uint MinimumFormatVersion = 6;

    /// <summary>Highest accepted format version.</summary>
    public const uint MaximumFormatVersion = 8;

    /// <summary>Bits 0 to 5 of the flags: the compression.</summary>
    private const uint CompressionMask = 0x3F;

    /// <summary>Bit 0x40: the information block also carries the file list.</summary>
    private const uint FileListFlag = 0x40;

    /// <summary>Bit 0x80: the information block sits at the end of the file.</summary>
    private const uint InfoAtEndFlag = 0x80;

    /// <summary>Largest accepted information block, decompressed.</summary>
    private const int MaximumInfoSize = 64 * 1024 * 1024;

    /// <summary>
    /// Largest accepted decompressed stream, the sum of the data blocks. It stays below
    /// <see cref="int.MaxValue"/> so that an absurd block table comes out as a refusal rather
    /// than an overflow (the measured bundle decompresses to about 43 MB).
    /// </summary>
    private const long MaximumStreamSize = 1024L * 1024 * 1024;

    /// <summary>Largest accepted number of data blocks or of file nodes.</summary>
    private const int MaximumTableLength = 1 << 20;

    private UnityBundle(
        uint formatVersion,
        string playerVersion,
        string engineVersion,
        IReadOnlyList<UnityBundleFile> files)
    {
        FormatVersion = formatVersion;
        PlayerVersion = playerVersion;
        EngineVersion = engineVersion;
        Files = files;
    }

    /// <summary>The bundle's format version, 6 to 8 (7 on the measured bundle).</summary>
    public uint FormatVersion { get; }

    /// <summary>The player version string, <c>5.x.x</c> on the measured bundle.</summary>
    public string PlayerVersion { get; }

    /// <summary>The engine version string, <c>2019.4.41f2</c> on the measured bundle.</summary>
    public string EngineVersion { get; }

    /// <summary>The file nodes, in the order the bundle lists them.</summary>
    public IReadOnlyList<UnityBundleFile> Files { get; }

    /// <summary>
    /// Reads a whole bundle and extracts its file nodes.
    /// </summary>
    /// <param name="bytes">The bundle file, whole.</param>
    /// <exception cref="UnityAssetFormatException">
    /// The signature, the format version, the compression or one of the tables is not something
    /// this reader can use, or the file is shorter than it says it is.
    /// </exception>
    public static UnityBundle Read(ReadOnlySpan<byte> bytes)
    {
        // The header and the information block are big-endian whatever the serialized file
        // inside says (AMD-030 §4.1, §4.2).
        AssetReader reader = new(bytes, bigEndian: true);

        string signature = reader.ReadNullTerminatedString();
        if (!string.Equals(signature, Signature, StringComparison.Ordinal))
        {
            throw new UnityAssetFormatException(
                $"this is not a Unity asset bundle: the signature is \"{signature}\" and not \"{Signature}\".");
        }

        uint formatVersion = reader.ReadUInt32();
        if (formatVersion is < MinimumFormatVersion or > MaximumFormatVersion)
        {
            throw new UnityAssetFormatException(
                $"the bundle format version {formatVersion} is not supported; "
                + $"{MinimumFormatVersion} to {MaximumFormatVersion} are.");
        }

        string playerVersion = reader.ReadNullTerminatedString();
        string engineVersion = reader.ReadNullTerminatedString();

        long totalSize = reader.ReadInt64();
        uint compressedInfoSize = reader.ReadUInt32();
        uint uncompressedInfoSize = reader.ReadUInt32();
        uint flags = reader.ReadUInt32();

        if (totalSize != bytes.Length)
        {
            throw new UnityAssetFormatException(
                $"the bundle announces {totalSize} bytes and the file holds {bytes.Length}: "
                + "it is truncated or not the file it claims to be.");
        }

        if ((flags & FileListFlag) == 0)
        {
            throw new UnityAssetFormatException(
                "the bundle's information block carries no file list (flag 0x40 is clear), "
                + "so nothing can be extracted from it.");
        }

        // AMD-030 §4.1: from format version 7 on, what follows the header starts on a multiple
        // of 16 (the measured header ends at 50 and the information block starts at 64).
        if (formatVersion >= 7)
        {
            reader.Align(16);
        }

        if (uncompressedInfoSize > MaximumInfoSize)
        {
            throw new UnityAssetFormatException(
                $"the bundle announces an information block of {uncompressedInfoSize} bytes, "
                + $"more than the accepted {MaximumInfoSize}.");
        }

        ReadOnlySpan<byte> compressedInfo;
        int dataStart;
        int dataEnd;
        if ((flags & InfoAtEndFlag) != 0)
        {
            // The data blocks start right after the aligned header; the information block is the
            // tail of the file (AMD-030 §4.1 bit 0x80, §4.2 last paragraph).
            dataStart = reader.Position;
            long infoStart = bytes.Length - (long)compressedInfoSize;
            if (infoStart < dataStart)
            {
                throw new UnityAssetFormatException(
                    $"the information block of {compressedInfoSize} bytes does not fit at the end of "
                    + $"a {bytes.Length}-byte bundle whose data start at {dataStart}.");
            }

            dataEnd = (int)infoStart;
            compressedInfo = bytes.Slice(dataEnd, (int)compressedInfoSize);
        }
        else
        {
            compressedInfo = reader.ReadBytes(checked((int)compressedInfoSize));
            dataStart = reader.Position;
            dataEnd = bytes.Length;
        }

        byte[] info = Decompress(
            compressedInfo,
            checked((int)uncompressedInfoSize),
            CompressionOf(flags),
            "the bundle's information block");

        return ReadInfoAndBlocks(
            info, bytes, dataStart, dataEnd, formatVersion, playerVersion, engineVersion);
    }

    /// <summary>The compression bits of a flag word.</summary>
    private static UnityCompression CompressionOf(uint flags) => (UnityCompression)(flags & CompressionMask);

    private static UnityBundle ReadInfoAndBlocks(
        byte[] info,
        ReadOnlySpan<byte> bytes,
        int dataStart,
        int dataEnd,
        uint formatVersion,
        string playerVersion,
        string engineVersion)
    {
        AssetReader reader = new(info, bigEndian: true);

        // 16 bytes of hash, which nothing here needs (AMD-030 §4.2).
        reader.Skip(16);

        int blockCount = reader.ReadInt32();
        CheckTableLength(blockCount, "data blocks");

        (uint Uncompressed, uint Compressed, UnityCompression Compression)[] blocks =
            new (uint, uint, UnityCompression)[blockCount];
        long streamSize = 0;
        long compressedSize = 0;
        for (int index = 0; index < blockCount; index++)
        {
            uint uncompressed = reader.ReadUInt32();
            uint compressed = reader.ReadUInt32();
            ushort blockFlags = reader.ReadUInt16();
            blocks[index] = (uncompressed, compressed, CompressionOf(blockFlags));
            streamSize += uncompressed;
            compressedSize += compressed;
        }

        if (streamSize > MaximumStreamSize)
        {
            throw new UnityAssetFormatException(
                $"the bundle's blocks announce {streamSize} bytes once decompressed, "
                + $"more than the accepted {MaximumStreamSize}.");
        }

        if (dataStart + compressedSize > dataEnd)
        {
            throw new UnityAssetFormatException(
                $"the bundle's {blockCount} data blocks need {compressedSize} bytes from offset "
                + $"{dataStart} and only {dataEnd - dataStart} are available: the file is truncated "
                + "or its block table is inconsistent.");
        }

        int nodeCount = reader.ReadInt32();
        CheckTableLength(nodeCount, "file nodes");

        (long Offset, long Size, uint Flags, string Path)[] nodes = new (long, long, uint, string)[nodeCount];
        for (int index = 0; index < nodeCount; index++)
        {
            long offset = reader.ReadInt64();
            long size = reader.ReadInt64();
            uint nodeFlags = reader.ReadUInt32();
            string path = reader.ReadNullTerminatedString();
            nodes[index] = (offset, size, nodeFlags, path);
        }

        byte[] stream = Expand(blocks, bytes, dataStart, checked((int)streamSize));

        List<UnityBundleFile> files = new(nodeCount);
        foreach ((long offset, long size, uint nodeFlags, string path) in nodes)
        {
            if (offset < 0 || size < 0 || offset + size > stream.Length)
            {
                throw new UnityAssetFormatException(
                    $"the bundle node \"{path}\" claims {size} bytes at offset {offset} of a stream of "
                    + $"{stream.Length} bytes: the file table is out of range.");
            }

            byte[] data = new byte[size];
            Array.Copy(stream, offset, data, 0, size);
            files.Add(new UnityBundleFile(path, nodeFlags, data));
        }

        return new UnityBundle(formatVersion, playerVersion, engineVersion, files);
    }

    /// <summary>
    /// Decompresses the data blocks one after the other into a single continuous stream, which
    /// is where the file nodes are then read by offset and size (AMD-030 §4.2).
    /// </summary>
    private static byte[] Expand(
        (uint Uncompressed, uint Compressed, UnityCompression Compression)[] blocks,
        ReadOnlySpan<byte> bytes,
        int dataStart,
        int streamSize)
    {
        byte[] stream = new byte[streamSize];
        int source = dataStart;
        int written = 0;

        for (int index = 0; index < blocks.Length; index++)
        {
            (uint uncompressed, uint compressed, UnityCompression compression) = blocks[index];
            ReadOnlySpan<byte> block = bytes.Slice(source, checked((int)compressed));
            Span<byte> target = stream.AsSpan(written, checked((int)uncompressed));

            if (compression == UnityCompression.None)
            {
                if (compressed != uncompressed)
                {
                    throw new UnityAssetFormatException(
                        $"the uncompressed data block {index} announces {compressed} stored bytes for "
                        + $"{uncompressed} decompressed ones.");
                }

                block.CopyTo(target);
            }
            else
            {
                int produced = DecompressInto(block, target, compression, $"the data block {index}");
                if (produced != target.Length)
                {
                    throw new UnityAssetFormatException(
                        $"the data block {index} decodes to {produced} bytes and not the announced "
                        + $"{target.Length}.");
                }
            }

            source += checked((int)compressed);
            written += checked((int)uncompressed);
        }

        return stream;
    }

    private static byte[] Decompress(
        ReadOnlySpan<byte> source,
        int uncompressedSize,
        UnityCompression compression,
        string what)
    {
        byte[] target = new byte[uncompressedSize];
        if (compression == UnityCompression.None)
        {
            if (source.Length != uncompressedSize)
            {
                throw new UnityAssetFormatException(
                    $"{what} is stored uncompressed and announces {source.Length} bytes for "
                    + $"{uncompressedSize} decompressed ones.");
            }

            source.CopyTo(target);
            return target;
        }

        int produced = DecompressInto(source, target, compression, what);
        if (produced != uncompressedSize)
        {
            throw new UnityAssetFormatException(
                $"{what} decodes to {produced} bytes and not the announced {uncompressedSize}.");
        }

        return target;
    }

    private static int DecompressInto(
        ReadOnlySpan<byte> source,
        Span<byte> target,
        UnityCompression compression,
        string what)
    {
        switch (compression)
        {
            case UnityCompression.Lz4:
            case UnityCompression.Lz4Hc:
                return Lz4Block.Decode(source, target);

            case UnityCompression.Lzma:
                throw new UnityAssetFormatException(
                    $"{what} is compressed with LZMA, which this reader does not support; "
                    + "only uncompressed, LZ4 and LZ4HC bundles can be read.");

            default:
                throw new UnityAssetFormatException(
                    $"{what} uses the unknown compression code {(int)compression}.");
        }
    }

    private static void CheckTableLength(int count, string what)
    {
        if (count < 0 || count > MaximumTableLength)
        {
            throw new UnityAssetFormatException(
                $"the bundle announces {count} {what}, which is not a usable count.");
        }
    }
}
