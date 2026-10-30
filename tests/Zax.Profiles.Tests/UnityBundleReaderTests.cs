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
        ["3dSen 2.0: format 8, flag 0x200"] = ThreeDSen2,
        ["3dSen 2.0 flags with the info block at the end"] = ThreeDSen2AtEnd,
    };

    /// <summary>The header of 3dSen 2.0 (AMD-039 §2).</summary>
    private static SyntheticBundleOptions ThreeDSen2 => SyntheticUnityBundle.ThreeDSen2;

    /// <summary>
    /// The same flags with the information block at the end (bit 0x80), where AMD-039 §2 says
    /// bit 0x200 has no effect.
    /// </summary>
    private static SyntheticBundleOptions ThreeDSen2AtEnd => ThreeDSen2 with { InfoAtEnd = true };

    /// <summary>The names of <see cref="LayoutsByName"/>, one test case each.</summary>
    public static TheoryData<string> Layouts() => [.. LayoutsByName.Keys];

    [Theory] // AMD-030 §4, §6
    [MemberData(nameof(Layouts))]
    public void Every_layout_yields_the_same_text_assets(string layout)
    {
        byte[] bundle = SyntheticUnityBundle.Build(Assets, LayoutsByName[layout]);

        AssertAssetsRead(layout, ThreeDSenProfileBundle.ReadTextAssets(bundle));
    }

    /// <summary>
    /// AMD-039 §4, first bullet: a format 8 bundle with flags 0x243 whose compressed information
    /// block does not end on a multiple of 16 gives the expected <c>TextAsset</c>s — the data
    /// blocks are found at the next 16-byte boundary and not right after the block.
    /// </summary>
    [Fact] // AMD-039 §2, §4
    public void The_data_blocks_of_a_bundle_with_flag_0x200_start_on_a_multiple_of_16()
    {
        SyntheticBundleLayout layout = SyntheticUnityBundle.BuildWithLayout(Assets, ThreeDSen2);

        // The flags of the measured 2.0 header: LZ4HC (3), file list (0x40), alignment (0x200).
        Assert.Equal(0x243u, layout.Flags);

        // The case that matters: like the measured 1429-byte block ending at 1493, this one does
        // not end on a boundary, so the padding is what the reader must skip.
        Assert.NotEqual(0, layout.InfoEnd % 16);
        Assert.Equal((layout.InfoEnd + 15) / 16 * 16, layout.DataStart);
        Assert.Equal(
            new byte[layout.DataStart - layout.InfoEnd],
            layout.Bytes[layout.InfoEnd..layout.DataStart]);

        AssertAssetsRead("flag 0x200", ThreeDSenProfileBundle.ReadTextAssets(layout.Bytes));
    }

    /// <summary>
    /// AMD-039 §4, second bullet: the very same bytes with bit 0x200 taken out of the header —
    /// the padding still in front of the data — import no profile at all and do not crash
    /// (AMD-030 §5).
    /// </summary>
    [Fact] // AMD-039 §2, §4
    public void The_same_bundle_without_flag_0x200_is_refused_rather_than_misread()
    {
        SyntheticBundleLayout aligned = SyntheticUnityBundle.BuildWithLayout(Assets, ThreeDSen2);
        SyntheticBundleLayout same = SyntheticUnityBundle.BuildWithLayout(
            Assets, ThreeDSen2 with { AlignDataFlag = false });

        // Only the flag word differs: the bundle still carries its padding bytes.
        Assert.Equal(0x43u, same.Flags);
        Assert.Equal(aligned.Bytes.Length, same.Bytes.Length);
        Assert.Equal(aligned.DataStart, same.DataStart);

        // A refusal with a message, which is how AMD-030 §5 asks a bundle that cannot be read to
        // come out: no exception of another kind, and no profile.
        Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(same.Bytes));
    }

    /// <summary>
    /// AMD-039 §4, third bullet: with bits 0x80 and 0x200 both set, the data blocks follow the
    /// aligned header and bit 0x200 adds nothing.
    /// </summary>
    [Fact] // AMD-039 §2, §4
    public void Flag_0x200_has_no_effect_when_the_information_block_is_at_the_end()
    {
        SyntheticBundleLayout layout = SyntheticUnityBundle.BuildWithLayout(Assets, ThreeDSen2AtEnd);

        Assert.Equal(0x2C3u, layout.Flags);

        // The header of this bundle ends at 50 and its data start at 64, the aligned position of
        // AMD-030 §4.2: no further padding was inserted.
        Assert.Equal(64, layout.DataStart);

        AssertAssetsRead("flags 0x80 and 0x200", ThreeDSenProfileBundle.ReadTextAssets(layout.Bytes));
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
    /// Checks that <paramref name="read"/> is <see cref="Assets"/>, name for name and byte for
    /// byte; <paramref name="what"/> names the layout in the failure message.
    /// </summary>
    private static void AssertAssetsRead(string what, IReadOnlyList<UnityTextAsset> read)
    {
        Assert.Equal(Assets.Length, read.Count);
        for (int index = 0; index < Assets.Length; index++)
        {
            Assert.Equal(Assets[index].Name, read[index].Name);
            Assert.True(
                Assets[index].Script.AsSpan().SequenceEqual(read[index].Script.Span),
                $"{what}: the payload of \"{Assets[index].Name}\" came back changed.");
        }
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
