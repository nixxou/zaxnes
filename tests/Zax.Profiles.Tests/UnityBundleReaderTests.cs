using System.Text;
using Zax.Profiles.Import;

namespace Zax.Profiles.Tests;

/// <summary>
/// The reader of AMD-030 §4, on bundles the tests build themselves: no compression and LZ4, one
/// block and several, information block at the start and at the end, serialized file little- and
/// big-endian, and the refusals of AMD-030 §5 (truncated, bad signature, LZMA, offsets out of
/// range).
/// </summary>
public sealed class UnityBundleReaderTests
{
    /// <summary>The payloads the synthetic bundles carry, names of §4.4 included.</summary>
    private static readonly (string Name, byte[] Script)[] Assets =
    [
        ("Pinball", Payload(0x11, 300)),
        ("Blaster Master (U)", Payload(0x22, 70_000)),
        ("pacman", Payload(0x33, 5)),
    ];

    /// <summary>
    /// Every layout AMD-030 §6 asks a synthetic bundle to cover, by name so that the theory
    /// data stays serializable and each case shows up on its own in the run.
    /// </summary>
    private static readonly Dictionary<string, SyntheticBundleOptions> LayoutsByName = new(StringComparer.Ordinal)
    {
        ["stored, one block, info first"] = new SyntheticBundleOptions(),
        ["stored, info at the end"] = new SyntheticBundleOptions { InfoAtEnd = true },
        ["stored, 8 KiB blocks"] = new SyntheticBundleOptions { BlockSize = 8 * 1024 },
        ["LZ4 blocks, one block"] = new SyntheticBundleOptions
        {
            BlockCompression = UnityCompression.Lz4,
        },
        ["LZ4 blocks of 128 KiB, as measured"] = new SyntheticBundleOptions
        {
            BlockCompression = UnityCompression.Lz4,
            BlockSize = 128 * 1024,
        },
        ["LZ4 info and blocks, info at the end"] = new SyntheticBundleOptions
        {
            InfoCompression = UnityCompression.Lz4,
            BlockCompression = UnityCompression.Lz4,
            BlockSize = 16 * 1024,
            InfoAtEnd = true,
        },
        ["LZ4HC flagged blocks"] = new SyntheticBundleOptions
        {
            BlockCompression = UnityCompression.Lz4Hc,
            BlockSize = 32 * 1024,
        },
        ["big-endian serialized file"] = new SyntheticBundleOptions { BigEndian = true },
        ["big-endian, LZ4, several blocks, info at the end"] = new SyntheticBundleOptions
        {
            BigEndian = true,
            BlockCompression = UnityCompression.Lz4,
            InfoCompression = UnityCompression.Lz4,
            BlockSize = 4 * 1024,
            InfoAtEnd = true,
        },
        ["serialized file version 17"] = new SyntheticBundleOptions { SerializedVersion = 17 },
        ["serialized file version 19"] = new SyntheticBundleOptions { SerializedVersion = 19 },
        ["serialized file version 22"] = new SyntheticBundleOptions { SerializedVersion = 22 },
        ["serialized file version 22, big-endian"] = new SyntheticBundleOptions
        {
            SerializedVersion = 22,
            BigEndian = true,
        },
        ["no type tree"] = new SyntheticBundleOptions { TypeTree = false },
        ["bundle format version 6"] = new SyntheticBundleOptions { FormatVersion = 6 },
        ["bundle format version 8"] = new SyntheticBundleOptions { FormatVersion = 8 },
    };

    /// <summary>The names of <see cref="LayoutsByName"/>, one test case each.</summary>
    public static TheoryData<string> Layouts() => [.. LayoutsByName.Keys];

    [Theory] // AMD-030 §4, §6
    [MemberData(nameof(Layouts))]
    public void Every_layout_yields_the_same_text_assets(string layout)
    {
        byte[] bundle = SyntheticUnityBundle.Build(Assets, LayoutsByName[layout]);

        IReadOnlyList<UnityTextAsset> assets = ThreeDSenProfileBundle.ReadTextAssets(bundle);

        Assert.Equal(Assets.Length, assets.Count);
        for (int index = 0; index < Assets.Length; index++)
        {
            Assert.Equal(Assets[index].Name, assets[index].Name);
            Assert.True(
                Assets[index].Script.AsSpan().SequenceEqual(assets[index].Script.Span),
                $"{layout}: the payload of \"{Assets[index].Name}\" came back changed.");
        }
    }

    [Fact] // AMD-030 §4.1
    public void The_header_fields_are_read()
    {
        byte[] bundle = SyntheticUnityBundle.Build(Assets, new SyntheticBundleOptions());

        UnityBundle container = UnityBundle.Read(bundle);

        Assert.Equal(7u, container.FormatVersion);
        Assert.Equal("5.x.x", container.PlayerVersion);
        Assert.Equal(SyntheticUnityBundle.EngineVersion, container.EngineVersion);
        UnityBundleFile node = Assert.Single(container.Files);
        Assert.StartsWith("CAB-", node.Path, StringComparison.Ordinal);
        Assert.Equal(4u, node.Flags);
    }

    [Fact] // AMD-030 §4.1: the information block of a version 7 bundle starts on a multiple of 16.
    public void The_information_block_of_a_version_7_bundle_is_aligned_on_16_bytes()
    {
        byte[] bundle = SyntheticUnityBundle.Build(Assets, new SyntheticBundleOptions());

        // "UnityFS\0" + u32 + "5.x.x\0" + "2019.4.41f2\0" + i64 + 3 × u32 = 50 bytes, and the
        // block that follows starts at 64. The padding is what the bytes between show.
        Assert.Equal(64, Offset(bundle, 50));
        Assert.Equal(new byte[14], bundle[50..64]);

        // Reading it back proves the reader skips exactly the same padding.
        Assert.Equal(3, ThreeDSenProfileBundle.ReadTextAssets(bundle).Count);
    }

    [Fact] // AMD-030 §4.3, §4.4
    public void The_serialized_file_exposes_its_version_order_and_classes()
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets, new SyntheticBundleOptions { BigEndian = true, SerializedVersion = 21 });
        UnityBundle container = UnityBundle.Read(bundle);

        UnitySerializedFile serialized = UnitySerializedFile.Read(container.Files[0].Data);

        Assert.Equal(21u, serialized.Version);
        Assert.True(serialized.IsBigEndian);
        Assert.Equal(SyntheticUnityBundle.EngineVersion, serialized.UnityVersion);

        // The measured file holds one TextAsset per profile plus the AssetBundle object.
        Assert.Equal(Assets.Length + 1, serialized.Objects.Count);
        Assert.Equal(Assets.Length, serialized.Objects.Count(o => o.ClassId == UnityTextAsset.ClassId));
        Assert.Single(serialized.Objects, o => o.ClassId == UnityTextAsset.AssetBundleClassId);
    }

    [Fact] // AMD-030 §4.4: an empty payload is still a payload.
    public void An_empty_text_asset_is_read()
    {
        byte[] bundle = SyntheticUnityBundle.Build([("empty", [])], new SyntheticBundleOptions());

        UnityTextAsset asset = Assert.Single(ThreeDSenProfileBundle.ReadTextAssets(bundle));

        Assert.Equal("empty", asset.Name);
        Assert.Equal(0, asset.Script.Length);
    }

    [Fact] // AMD-030 §4.4: a name whose length is not a multiple of four exercises the alignment.
    public void Names_of_every_length_modulo_four_are_read()
    {
        (string Name, byte[] Script)[] assets =
        [
            ("a", [1]),
            ("ab", [2, 2]),
            ("abc", [3, 3, 3]),
            ("abcd", [4, 4, 4, 4]),
            ("accentué", [5]),
        ];

        IReadOnlyList<UnityTextAsset> read = ThreeDSenProfileBundle.ReadTextAssets(
            SyntheticUnityBundle.Build(assets, new SyntheticBundleOptions()));

        Assert.Equal(assets.Select(a => a.Name), read.Select(a => a.Name));
        for (int index = 0; index < assets.Length; index++)
        {
            Assert.True(assets[index].Script.AsSpan().SequenceEqual(read[index].Script.Span));
        }
    }

    [Fact] // AMD-030 §5: unknown signature.
    public void A_file_that_is_not_a_bundle_is_refused()
    {
        byte[] bytes = Encoding.ASCII.GetBytes("UnityWeb\0and then nothing useful at all......");

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bytes));

        Assert.Contains("not a Unity asset bundle", error.Message, StringComparison.Ordinal);
        Assert.Contains("UnityWeb", error.Message, StringComparison.Ordinal);
    }

    [Theory] // AMD-030 §5: a truncated file, cut at several places.
    [InlineData(0)]
    [InlineData(4)]
    [InlineData(30)]
    [InlineData(60)]
    [InlineData(200)]
    public void A_truncated_bundle_is_refused(int kept)
    {
        byte[] bundle = SyntheticUnityBundle.Build(Assets, new SyntheticBundleOptions());

        Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle.AsSpan(0, Math.Min(kept, bundle.Length))));
    }

    [Fact] // AMD-030 §5: cutting the tail off an otherwise well-formed bundle.
    public void A_bundle_cut_in_its_data_blocks_is_refused()
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets, new SyntheticBundleOptions { BlockSize = 8 * 1024 });

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle.AsSpan(0, bundle.Length - 4096)));

        Assert.Contains("truncated", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §4.1: LZMA is refused with a message that names it.
    public void An_lzma_information_block_is_refused()
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets,
            new SyntheticBundleOptions { InfoCompressionCode = (uint)UnityCompression.Lzma });

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle));

        Assert.Contains("LZMA", error.Message, StringComparison.Ordinal);
        Assert.Contains("information block", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §4.1
    public void Lzma_data_blocks_are_refused()
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets,
            new SyntheticBundleOptions { BlockCompressionCode = (ushort)UnityCompression.Lzma });

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle));

        Assert.Contains("LZMA", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §4.2: an unknown compression code is refused, not guessed.
    public void An_unknown_compression_code_is_refused()
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets, new SyntheticBundleOptions { BlockCompressionCode = 9 });

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle));

        Assert.Contains("unknown compression code 9", error.Message, StringComparison.Ordinal);
    }

    [Theory] // AMD-030 §5: a file table that points outside the decompressed stream.
    [InlineData(1_000_000, 0)]
    [InlineData(-8, 0)]
    [InlineData(0, 1_000_000)]
    [InlineData(0, -1_000_000)]
    public void A_file_node_out_of_range_is_refused(long offsetDelta, long sizeDelta)
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets,
            new SyntheticBundleOptions { NodeOffsetDelta = offsetDelta, NodeSizeDelta = sizeDelta });

        Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle));
    }

    [Fact] // AMD-030 §5: a block table announcing an absurd stream is refused, not allocated.
    public void A_block_table_announcing_more_than_the_cap_is_refused()
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets,
            new SyntheticBundleOptions { BlockUncompressedSizeCode = 0xF000_0000 });

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle));

        Assert.Contains("once decompressed", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §4.1: a format version outside 6 to 8.
    public void An_unsupported_bundle_version_is_refused()
    {
        byte[] bundle = SyntheticUnityBundle.Build(Assets, new SyntheticBundleOptions());

        // The version is the u32 right after "UnityFS\0".
        bundle[8] = 0;
        bundle[9] = 0;
        bundle[10] = 0;
        bundle[11] = 9;

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle));

        Assert.Contains("version 9 is not supported", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §4.1: bit 0x40 clear leaves nothing to extract.
    public void A_bundle_without_a_file_list_is_refused()
    {
        byte[] bundle = SyntheticUnityBundle.Build(
            Assets, new SyntheticBundleOptions { FileList = false });

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle));

        Assert.Contains("no file list", error.Message, StringComparison.Ordinal);
    }

    [Theory] // AMD-030 §4.3: versions 17 to 22 and nothing else.
    [InlineData(16u)]
    [InlineData(23u)]
    public void An_unsupported_serialized_file_version_is_refused(uint version)
    {
        byte[] serialized = SyntheticUnityBundle.BuildSerializedFile(
            Assets, new SyntheticBundleOptions { SerializedVersion = 21 });

        // The version is the third u32 of the big-endian header.
        serialized[8] = 0;
        serialized[9] = 0;
        serialized[10] = 0;
        serialized[11] = (byte)version;

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => UnitySerializedFile.Read(serialized));

        Assert.Contains($"version {version} is not supported", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §5: an object table pointing past the end of the file.
    public void A_serialized_file_whose_object_table_is_out_of_range_is_refused()
    {
        byte[] serialized = SyntheticUnityBundle.BuildSerializedFile(
            Assets, new SyntheticBundleOptions());

        // Shorten the file without touching its table: every object now ends out of range.
        byte[] cut = serialized[..(serialized.Length / 2)];

        Assert.Throws<UnityAssetFormatException>(() => UnitySerializedFile.Read(cut));
    }

    /// <summary>
    /// Where the next multiple of 16 after <paramref name="position"/> falls — the alignment of
    /// AMD-030 §4.1, computed here rather than taken from the reader.
    /// </summary>
    private static int Offset(byte[] bundle, int position)
    {
        Assert.True(position < bundle.Length);
        return (position + 15) / 16 * 16;
    }

    private static byte[] Payload(byte seed, int length)
    {
        byte[] bytes = new byte[length];

        // Compressible but not trivial: long runs with a changing pattern, which gives the LZ4
        // encoder both literals and matches.
        for (int index = 0; index < length; index++)
        {
            bytes[index] = (byte)(index % 97 < 40 ? seed : (seed + index) % 251);
        }

        return bytes;
    }
}
