using System.Buffers.Binary;
using System.Text;
using Zax.Profiles.Import;

namespace Zax.Profiles.Tests;

/// <summary>
/// How a synthetic bundle is laid out. Every field exists because AMD-030 §6 asks for that
/// variation: no compression and LZ4, one block and several, information block at the start and
/// at the end, serialized file little- and big-endian.
/// </summary>
internal sealed class SyntheticBundleOptions
{
    /// <summary>Bundle format version, 6 to 8 (7 is the measured one).</summary>
    public uint FormatVersion { get; init; } = 7;

    /// <summary>Compression of the information block.</summary>
    public UnityCompression InfoCompression { get; init; } = UnityCompression.None;

    /// <summary>Compression of the data blocks.</summary>
    public UnityCompression BlockCompression { get; init; } = UnityCompression.None;

    /// <summary>Largest decompressed size of one data block.</summary>
    public int BlockSize { get; init; } = int.MaxValue;

    /// <summary>Whether the information block goes at the end of the file (flag 0x80).</summary>
    public bool InfoAtEnd { get; init; }

    /// <summary>Whether the file list is announced (flag 0x40); clear makes the bundle unusable.</summary>
    public bool FileList { get; init; } = true;

    /// <summary>Version of the serialized file, 17 to 22.</summary>
    public uint SerializedVersion { get; init; } = 21;

    /// <summary>Byte order the serialized file announces.</summary>
    public bool BigEndian { get; init; }

    /// <summary>Whether the type tree is written.</summary>
    public bool TypeTree { get; init; } = true;

    /// <summary>The file node's name.</summary>
    public string NodePath { get; init; } = "CAB-0123456789abcdef0123456789abcdef";

    /// <summary>
    /// Compression code written in the header instead of the real one, to forge a bundle that
    /// claims LZMA or an unknown codec.
    /// </summary>
    public uint? InfoCompressionCode { get; init; }

    /// <summary>Same, for the data blocks.</summary>
    public ushort? BlockCompressionCode { get; init; }

    /// <summary>
    /// Written in the block table instead of each block's real decompressed size, to forge a
    /// bundle that announces an absurd stream.
    /// </summary>
    public uint? BlockUncompressedSizeCode { get; init; }

    /// <summary>Added to the node's offset, to forge a file table out of range.</summary>
    public long NodeOffsetDelta { get; init; }

    /// <summary>Added to the node's size, to forge a file table out of range.</summary>
    public long NodeSizeDelta { get; init; }
}

/// <summary>
/// Builds Unity « UnityFS » bundles from scratch, following the public format AMD-030 §4
/// describes, so the reader can be exercised without any file of 3dSen (AMD-030 §6, second
/// bullet).
/// </summary>
internal static class SyntheticUnityBundle
{
    /// <summary>The engine version the measured bundle carries.</summary>
    public const string EngineVersion = "2019.4.41f2";

    /// <summary>Builds a bundle holding the given <c>TextAsset</c>s.</summary>
    public static byte[] Build(
        IReadOnlyList<(string Name, byte[] Script)> assets,
        SyntheticBundleOptions? options = null)
    {
        options ??= new SyntheticBundleOptions();

        byte[] serialized = BuildSerializedFile(assets, options);
        return Wrap(serialized, options);
    }

    /// <summary>Builds the <c>CAB-…</c> serialized file alone (AMD-030 §4.3, §4.4).</summary>
    public static byte[] BuildSerializedFile(
        IReadOnlyList<(string Name, byte[] Script)> assets,
        SyntheticBundleOptions options)
    {
        // 1. The object data, each object aligned on 8 bytes inside the data area.
        ByteWriter data = new(options.BigEndian);
        List<(long Start, long Size, int TypeIndex)> table = [];
        foreach ((string name, byte[] script) in assets)
        {
            data.AlignTo(8);
            long start = data.Length;
            byte[] utf8 = Encoding.UTF8.GetBytes(name);
            data.Int32(utf8.Length);
            data.Bytes(utf8);
            data.AlignTo(4, (int)start);
            data.Int32(script.Length);
            data.Bytes(script);
            table.Add((start, data.Length - start, 0));
        }

        // The AssetBundle object of class 142, which AMD-030 §4.4 says to ignore: a plain name.
        data.AlignTo(8);
        long bundleStart = data.Length;
        byte[] bundleName = Encoding.UTF8.GetBytes("defaultprofiles_assets_all");
        data.Int32(bundleName.Length);
        data.Bytes(bundleName);
        data.AlignTo(4, (int)bundleStart);
        table.Add((bundleStart, data.Length - bundleStart, 1));

        byte[] objects = data.ToArray();

        // 2. The metadata, in the announced byte order.
        ByteWriter metadata = new(options.BigEndian);
        metadata.CString(EngineVersion);
        metadata.Int32(19); // Target platform: StandaloneWindows64.
        metadata.Byte(options.TypeTree ? (byte)1 : (byte)0);
        metadata.Int32(2);
        WriteType(metadata, UnityTextAsset.ClassId, options);
        WriteType(metadata, UnityTextAsset.AssetBundleClassId, options);
        metadata.Int32(table.Count);

        // 3. The header, big-endian, then the metadata, then the data at an aligned offset.
        int headerSize = options.SerializedVersion >= 22 ? 20 + 28 : 20;

        // The object table is aligned on 4 bytes from the start of the file (AMD-030 §4.3); the
        // header and the metadata written above already end on a 4-byte boundary or not, so the
        // padding is computed here and written as part of the metadata.
        int objectTableStart = Align(headerSize + metadata.Length, 4);
        int objectRecordSize = options.SerializedVersion >= 22 ? 24 : 20;
        int metadataEnd = objectTableStart + (table.Count * objectRecordSize);
        int dataOffset = Align(metadataEnd, 16);

        ByteWriter file = new(bigEndian: true);
        file.UInt32((uint)(metadataEnd - headerSize));
        file.UInt32((uint)(dataOffset + objects.Length));
        file.UInt32(options.SerializedVersion);
        file.UInt32((uint)dataOffset);
        file.Byte(options.BigEndian ? (byte)1 : (byte)0);
        file.Bytes([0, 0, 0]);

        if (options.SerializedVersion >= 22)
        {
            // Q-071: the public layout — u32 metadata size, i64 file size, i64 data offset and
            // one reserved i64.
            file.UInt32((uint)(metadataEnd - headerSize));
            file.Int64(dataOffset + objects.Length);
            file.Int64(dataOffset);
            file.Int64(0);
        }

        file.Bytes(metadata.ToArray());
        file.PadTo(objectTableStart);

        file.BigEndian = options.BigEndian;
        foreach ((long start, long size, int typeIndex) in table)
        {
            file.Int64(0x1000 + start);
            if (options.SerializedVersion >= 22)
            {
                file.Int64(start);
            }
            else
            {
                file.UInt32((uint)start);
            }

            file.UInt32((uint)size);
            file.Int32(typeIndex);
        }

        file.PadTo(dataOffset);
        file.Bytes(objects);
        return file.ToArray();
    }

    /// <summary>Wraps a serialized file into a bundle (AMD-030 §4.1, §4.2).</summary>
    public static byte[] Wrap(byte[] serialized, SyntheticBundleOptions options)
    {
        // The data blocks: the stream is the single file node, cut into blocks.
        int blockSize = Math.Min(options.BlockSize, Math.Max(serialized.Length, 1));
        List<(int Uncompressed, byte[] Stored)> blocks = [];
        for (int offset = 0; offset < serialized.Length; offset += blockSize)
        {
            int length = Math.Min(blockSize, serialized.Length - offset);
            byte[] plain = serialized[offset..(offset + length)];
            blocks.Add((length, Store(plain, options.BlockCompression)));
        }

        if (blocks.Count == 0)
        {
            blocks.Add((0, []));
        }

        // The information block.
        ByteWriter info = new(bigEndian: true);
        info.Bytes(new byte[16]); // The hash, ignored by the reader.
        info.Int32(blocks.Count);
        foreach ((int uncompressed, byte[] stored) in blocks)
        {
            info.UInt32(options.BlockUncompressedSizeCode ?? (uint)uncompressed);
            info.UInt32((uint)stored.Length);
            info.UInt16(options.BlockCompressionCode ?? (ushort)options.BlockCompression);
        }

        info.Int32(1);
        info.Int64(options.NodeOffsetDelta);
        info.Int64(serialized.Length + options.NodeSizeDelta);
        info.UInt32(4);
        info.CString(options.NodePath);

        byte[] plainInfo = info.ToArray();
        byte[] storedInfo = Store(plainInfo, options.InfoCompression);

        uint flags = options.InfoCompressionCode ?? (uint)options.InfoCompression;
        if (options.FileList)
        {
            flags |= 0x40;
        }

        if (options.InfoAtEnd)
        {
            flags |= 0x80;
        }

        ByteWriter header = new(bigEndian: true);
        header.CString(UnityBundle.Signature);
        header.UInt32(options.FormatVersion);
        header.CString("5.x.x");
        header.CString(EngineVersion);
        header.Int64(0); // Patched below, once the whole length is known.
        header.UInt32((uint)storedInfo.Length);
        header.UInt32((uint)plainInfo.Length);
        header.UInt32(flags);

        int sizeField = header.Length - 20;
        if (options.FormatVersion >= 7)
        {
            header.AlignTo(16);
        }

        if (!options.InfoAtEnd)
        {
            header.Bytes(storedInfo);
        }

        foreach ((int _, byte[] stored) in blocks)
        {
            header.Bytes(stored);
        }

        if (options.InfoAtEnd)
        {
            header.Bytes(storedInfo);
        }

        byte[] bundle = header.ToArray();
        BinaryPrimitives.WriteInt64BigEndian(bundle.AsSpan(sizeField, 8), bundle.Length);
        return bundle;
    }

    private static byte[] Store(byte[] plain, UnityCompression compression) => compression switch
    {
        UnityCompression.None => plain,
        UnityCompression.Lz4 or UnityCompression.Lz4Hc => Lz4Encoder.Compress(plain),

        // LZMA is never produced: the bundles that claim it carry stored bytes and an overridden
        // compression code, which is exactly the refusal AMD-030 §4.1 asks for.
        _ => plain,
    };

    private static void WriteType(ByteWriter metadata, int classId, SyntheticBundleOptions options)
    {
        metadata.Int32(classId);
        metadata.Byte(0); // Not a stripped type.
        metadata.Int16(-1); // No script type index.
        metadata.Bytes(new byte[16]); // The old type hash.

        if (!options.TypeTree)
        {
            return;
        }

        const int nodeCount = 3;
        byte[] strings = Encoding.ASCII.GetBytes("m_Name\0m_Script\0");
        metadata.Int32(nodeCount);
        metadata.Int32(strings.Length);
        metadata.Bytes(new byte[nodeCount * (options.SerializedVersion >= 19 ? 32 : 24)]);
        metadata.Bytes(strings);

        if (options.SerializedVersion >= 21)
        {
            metadata.Int32(1);
            metadata.Int32(classId);
        }
    }

    private static int Align(int value, int alignment)
        => (value + alignment - 1) / alignment * alignment;

    /// <summary>A growing buffer with a byte order, the mirror of the reader.</summary>
    private sealed class ByteWriter(bool bigEndian)
    {
        private readonly List<byte> _bytes = [];

        public bool BigEndian { get; set; } = bigEndian;

        public int Length => _bytes.Count;

        public void Byte(byte value) => _bytes.Add(value);

        public void Bytes(ReadOnlySpan<byte> value)
        {
            foreach (byte item in value)
            {
                _bytes.Add(item);
            }
        }

        public void CString(string value)
        {
            Bytes(Encoding.UTF8.GetBytes(value));
            _bytes.Add(0);
        }

        public void Int16(short value)
        {
            Span<byte> buffer = stackalloc byte[2];
            if (BigEndian)
            {
                BinaryPrimitives.WriteInt16BigEndian(buffer, value);
            }
            else
            {
                BinaryPrimitives.WriteInt16LittleEndian(buffer, value);
            }

            Bytes(buffer);
        }

        public void UInt16(ushort value)
        {
            Span<byte> buffer = stackalloc byte[2];
            if (BigEndian)
            {
                BinaryPrimitives.WriteUInt16BigEndian(buffer, value);
            }
            else
            {
                BinaryPrimitives.WriteUInt16LittleEndian(buffer, value);
            }

            Bytes(buffer);
        }

        public void Int32(int value)
        {
            Span<byte> buffer = stackalloc byte[4];
            if (BigEndian)
            {
                BinaryPrimitives.WriteInt32BigEndian(buffer, value);
            }
            else
            {
                BinaryPrimitives.WriteInt32LittleEndian(buffer, value);
            }

            Bytes(buffer);
        }

        public void UInt32(uint value)
        {
            Span<byte> buffer = stackalloc byte[4];
            if (BigEndian)
            {
                BinaryPrimitives.WriteUInt32BigEndian(buffer, value);
            }
            else
            {
                BinaryPrimitives.WriteUInt32LittleEndian(buffer, value);
            }

            Bytes(buffer);
        }

        public void Int64(long value)
        {
            Span<byte> buffer = stackalloc byte[8];
            if (BigEndian)
            {
                BinaryPrimitives.WriteInt64BigEndian(buffer, value);
            }
            else
            {
                BinaryPrimitives.WriteInt64LittleEndian(buffer, value);
            }

            Bytes(buffer);
        }

        public void AlignTo(int alignment, int origin = 0)
            => PadTo(origin + (Align(_bytes.Count - origin, alignment)));

        public void PadTo(int position)
        {
            while (_bytes.Count < position)
            {
                _bytes.Add(0);
            }
        }

        public byte[] ToArray() => [.. _bytes];
    }
}
