using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Import;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests of lot V2-1, written from <c>AMD-039</c> (import of the 3dSen 2.0
/// profile bundle) alone, which amends <c>AMD-030</c> §3, §4 and §6.
/// </summary>
/// <remarks>
/// <para>
/// AMD-039 introduces no <c>REQ-…</c> identifier of its own: what it states is a change of
/// bundle format (§2), a list of what does not change (§3) and three acceptance criteria that
/// need no installation plus two optional ones that do (§4). The tests below are named after
/// those paragraphs.
/// </para>
/// <para>
/// Everything a bundle is made of is built here, from the public description of §4 of AMD-030
/// and of §2 of AMD-039, and never from the reader under test: the header, the 16-byte
/// alignments, the LZ4 blocks, the serialized file and the <c>TextAsset</c> objects. The
/// payloads are profiles this test writes with the SPEC-03 encoder, so the bytes that come back
/// out can be compared with the bytes that went in.
/// </para>
/// </remarks>
public sealed class R4V2ImportTests
{
    /// <summary>Header flag bit 0x40: the information block carries the file list.</summary>
    private const uint FileListFlag = 0x40;

    /// <summary>Header flag bit 0x80: the information block sits at the end of the file.</summary>
    private const uint InfoAtEndFlag = 0x80;

    /// <summary>Header flag bit 0x200, new in 2.0 (AMD-039 §2).</summary>
    private const uint DataAlignedFlag = 0x200;

    /// <summary>Compression code 3, LZ4HC (AMD-030 §4.1).</summary>
    private const uint Lz4Hc = 3;

    /// <summary>The flags measured on the 2.0 bundle: LZ4HC, file list, bit 0x200.</summary>
    private const uint Flags20 = DataAlignedFlag | FileListFlag | Lz4Hc;

    /// <summary>The flags measured on the 1.1.2 bundle (AMD-030 §4.1).</summary>
    private const uint Flags112 = FileListFlag | Lz4Hc;

    /// <summary>Engine version measured on the 2.0 (AMD-039 §2).</summary>
    private const string Engine20 = "6000.3.21f1";

    /// <summary>Engine version measured on the 1.1.2 (AMD-030 §4.1).</summary>
    private const string Engine112 = "2019.4.41f2";

    /// <summary>Platform of the serialized file, 19 on the 2.0 and 5 on the 1.1.2.</summary>
    private const int Platform20 = 19;

    /// <summary>Platform of the serialized file on the 1.1.2.</summary>
    private const int Platform112 = 5;

    /// <summary>How many profiles a real installation holds (AMD-030 §6, AMD-039 §4).</summary>
    private const int RealProfileCount = 101;

    /// <summary>
    /// Three asset names, taken from the examples of AMD-030 §4.4, with one profile each.
    /// </summary>
    private static IReadOnlyList<(string Name, byte[] Payload)> SampleAssets { get; } =
    [
        ("Pinball", ProfileBytes("Pinball")),
        ("Blaster Master (U)", ProfileBytes("Blaster Master (U)")),
        ("pacman", ProfileBytes("pacman")),
    ];

    // ------------------------------------------------------------------ AMD-039 §4, criterion 1

    /// <summary>
    /// AMD-039 §4, first criterion, and §2 first bullet: a format 8 bundle with flags 0x243
    /// whose compressed information block does not end on a multiple of 16 gives the expected
    /// <c>TextAsset</c>s, the data blocks starting on the next 16-byte boundary.
    /// </summary>
    [Fact] // AMD-039 §2, §4
    public void Amd039_c1_a_format_8_bundle_with_bit_0x200_gives_the_expected_text_assets()
    {
        SyntheticBundle bundle = Bundle20();

        // The measured positions of §2: the header ends at 50, the information block is read at
        // 64 (the 16-byte alignment of formats ≥ 7), it does not end on a multiple of 16, and
        // the first data block starts on the next multiple of 16 — 11 bytes of padding, as on
        // the real 2.0 (1493 → 1504).
        Assert.Equal(50, bundle.HeaderEnd);
        Assert.Equal(64, bundle.InfoStart);
        Assert.NotEqual(0, bundle.InfoEnd % 16);
        Assert.Equal(5, bundle.InfoEnd % 16);
        Assert.Equal(bundle.InfoEnd + 11, bundle.DataStart);
        Assert.Equal(0, bundle.DataStart % 16);

        AssertSampleAssets(ThreeDSenProfileBundle.ReadTextAssets(bundle.Bytes));
    }

    // ------------------------------------------------------------------ AMD-039 §4, criterion 2

    /// <summary>
    /// AMD-039 §4, second criterion: the very same bundle with bit 0x200 cleared — the padding
    /// bytes still sitting in front of the data — imports no profile and does not crash
    /// (AMD-030 §5). Nothing is written, not even a partial file.
    /// </summary>
    [Fact] // AMD-039 §4, AMD-030 §5
    public void Amd039_c2_the_same_bundle_without_bit_0x200_imports_nothing_and_does_not_crash()
    {
        SyntheticBundle aligned = Bundle20();
        byte[] bytes = PatchedFlags(aligned, Flags20 & ~DataAlignedFlag);

        // The two files differ inside the flag word alone: the padding is still there.
        Assert.Equal(aligned.Bytes.Length, bytes.Length);
        Assert.DoesNotContain(
            DifferingBytes(aligned.Bytes, bytes),
            at => at < aligned.FlagsAt || at >= aligned.FlagsAt + 4);
        Assert.NotEmpty(DifferingBytes(aligned.Bytes, bytes));

        using TemporaryFolder folder = new();
        string bundlePath = folder.WriteBundle("StandaloneWindows64", bytes);
        ThreeDSenImporter importer = new(PhysicalProfileFileSystem.Instance, folder.Profiles);

        int imported;
        try
        {
            ThreeDSenImportSummary summary = importer.ImportFrom(bundlePath);
            imported = summary.Imported;
        }
        catch (UnityAssetFormatException exception)
        {
            // A refusal carrying a message is the other licit outcome of AMD-030 §5.
            Assert.NotEqual(string.Empty, exception.Message);
            imported = 0;
        }

        Assert.Equal(0, imported);
        Assert.Empty(folder.WrittenFiles());
    }

    // ------------------------------------------------------------------ AMD-039 §4, criterion 3

    /// <summary>
    /// AMD-039 §4, third criterion: a format 8 bundle carrying both bit 0x80 and bit 0x200,
    /// whose data follow the aligned header with no further padding, gives the expected
    /// <c>TextAsset</c>s.
    /// </summary>
    [Fact] // AMD-039 §2, §4
    public void Amd039_c3_a_format_8_bundle_with_bits_0x80_and_0x200_gives_the_expected_text_assets()
    {
        SyntheticBundle bundle = Build(
            formatVersion: 8,
            engineVersion: Engine20,
            flags: Flags20 | InfoAtEndFlag,
            serializedVersion: 22,
            platform: Platform20);

        // The data start right after the aligned header, with no padding of their own.
        Assert.Equal(64, bundle.DataStart);
        Assert.Equal(0, bundle.DataStart % 16);

        AssertSampleAssets(ThreeDSenProfileBundle.ReadTextAssets(bundle.Bytes));
    }

    // ------------------------------------------------------------------ AMD-039 §2, bullet 2

    /// <summary>
    /// AMD-039 §2, second bullet: with bit 0x80 the new bit is without effect. Reading the same
    /// bundle with and without bit 0x200 must give the very same assets.
    /// </summary>
    [Fact] // AMD-039 §2
    public void Amd039_bit_0x200_has_no_effect_when_the_information_block_is_at_the_end()
    {
        SyntheticBundle withBit = Build(
            formatVersion: 8,
            engineVersion: Engine20,
            flags: Flags20 | InfoAtEndFlag,
            serializedVersion: 22,
            platform: Platform20);
        byte[] without = PatchedFlags(withBit, (Flags20 | InfoAtEndFlag) & ~DataAlignedFlag);

        IReadOnlyList<UnityTextAsset> one = ThreeDSenProfileBundle.ReadTextAssets(withBit.Bytes);
        IReadOnlyList<UnityTextAsset> two = ThreeDSenProfileBundle.ReadTextAssets(without);

        AssertSampleAssets(one);
        AssertSampleAssets(two);
        Assert.Equal(one.Select(asset => asset.Name), two.Select(asset => asset.Name));
    }

    /// <summary>
    /// AMD-039 §2, first and second bullets together: « la position suivante alignée sur
    /// 16 octets » is the position itself when it already is a multiple of 16 — which is how the
    /// second bullet reasons about bit 0x80 (« position déjà multiple de 16 »). A bundle whose
    /// compressed information block ends on a boundary therefore has no padding at all.
    /// </summary>
    [Fact] // AMD-039 §2
    public void Amd039_an_information_block_ending_on_a_boundary_needs_no_padding()
    {
        SyntheticBundle bundle = Build(
            formatVersion: 8,
            engineVersion: Engine20,
            flags: Flags20,
            serializedVersion: 22,
            platform: Platform20,
            infoEndResidue: 0);

        Assert.Equal(0, bundle.InfoEnd % 16);
        Assert.Equal(bundle.InfoEnd, bundle.DataStart);

        AssertSampleAssets(ThreeDSenProfileBundle.ReadTextAssets(bundle.Bytes));
    }

    /// <summary>
    /// AMD-039 §2, third bullet: « les autres bits de drapeaux restent sans effet sur la
    /// lecture ». A bundle carrying an extra header bit the two amendments give no meaning to
    /// reads exactly like the same bundle without it.
    /// </summary>
    [Fact] // AMD-039 §2
    public void Amd039_a_header_bit_the_amendments_do_not_name_is_without_effect()
    {
        SyntheticBundle plain = Bundle20();
        byte[] withExtra = PatchedFlags(plain, Flags20 | 0x100);

        IReadOnlyList<UnityTextAsset> assets = ThreeDSenProfileBundle.ReadTextAssets(withExtra);

        AssertSampleAssets(assets);
        Assert.Equal(
            ThreeDSenProfileBundle.ReadTextAssets(plain.Bytes).Select(asset => asset.Name),
            assets.Select(asset => asset.Name));
    }

    // ------------------------------------------------------------------ AMD-039 §2, table

    /// <summary>
    /// AMD-039 §2, table and fourth bullet: the serialized file of the 2.0 is version 22, whose
    /// sizes and offsets are 64-bit fields, and its platform is 19 — ignored. The 32-bit header
    /// fields of a version 22 file are left at zero, so a reader that ignored the 64-bit ones
    /// would find no object at all.
    /// </summary>
    [Fact] // AMD-039 §2, AMD-030 §4.3
    public void Amd039_the_serialized_file_is_read_in_version_22_with_its_64_bit_fields()
    {
        SyntheticBundle bundle = Bundle20();
        UnityBundle container = UnityBundle.Read(bundle.Bytes);

        Assert.Equal(8u, container.FormatVersion);
        Assert.Equal(Engine20, container.EngineVersion);
        UnityBundleFile node = Assert.Single(container.Files);
        Assert.StartsWith("CAB-", node.Path, StringComparison.Ordinal);

        UnitySerializedFile serialized = UnitySerializedFile.Read(node.Data);

        Assert.Equal(22u, serialized.Version);
        Assert.Equal(Platform20, serialized.TargetPlatform);
        Assert.False(serialized.IsBigEndian);

        // Three TextAssets (class 49) and the AssetBundle object (142), which is ignored.
        Assert.Equal(SampleAssets.Count + 1, serialized.Objects.Count);
        Assert.Equal(
            SampleAssets.Count,
            serialized.Objects.Count(o => o.ClassId == UnityTextAsset.ClassId));
        Assert.Contains(serialized.Objects, o => o.ClassId == UnityTextAsset.AssetBundleClassId);
    }

    /// <summary>
    /// AMD-030 §4.3, unchanged by AMD-039: versions 17 to 22 are accepted and nothing else. A
    /// version 23 file is refused with a message instead of being read.
    /// </summary>
    [Fact] // AMD-030 §4.3, AMD-039 §2
    public void Amd039_a_serialized_file_of_version_23_is_refused_with_a_message()
    {
        SyntheticBundle bundle = Build(
            formatVersion: 8,
            engineVersion: Engine20,
            flags: Flags20,
            serializedVersion: 23,
            platform: Platform20);

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(bundle.Bytes));
        Assert.Contains("23", error.Message, StringComparison.Ordinal);
    }

    // ------------------------------------------------------------------ AMD-039 §2, bullet 3

    /// <summary>
    /// AMD-039 §2, third bullet: AMD-030 §4.1 still accepts format versions 6 to 8 and nothing
    /// else. Version 6 reads (it has no 16-byte alignment after the header), version 9 is
    /// refused with a message.
    /// </summary>
    [Fact] // AMD-030 §4.1, AMD-039 §2
    public void Amd039_format_versions_6_to_8_are_accepted_and_9_is_refused()
    {
        SyntheticBundle six = Build(
            formatVersion: 6,
            engineVersion: Engine112,
            flags: Flags112,
            serializedVersion: 21,
            platform: Platform112);

        // Format 6: no alignment, the information block follows the header straight away.
        Assert.Equal(six.HeaderEnd, six.InfoStart);
        AssertSampleAssets(ThreeDSenProfileBundle.ReadTextAssets(six.Bytes));

        SyntheticBundle nine = Build(
            formatVersion: 9,
            engineVersion: Engine20,
            flags: Flags20,
            serializedVersion: 22,
            platform: Platform20);
        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenProfileBundle.ReadTextAssets(nine.Bytes));
        Assert.Contains("9", error.Message, StringComparison.Ordinal);
    }

    // ------------------------------------------------------------------ AMD-039 §3

    /// <summary>
    /// AMD-039 §3: a 1.1.2 bundle imports as before — format 7, engine 2019.4.41f2, flags 0x43,
    /// serialized file version 21, platform 5, no bit 0x200 and no padding.
    /// </summary>
    [Fact] // AMD-039 §3
    public void Amd039_s3_a_1_1_2_shaped_bundle_still_gives_the_expected_text_assets()
    {
        SyntheticBundle bundle = Build(
            formatVersion: 7,
            engineVersion: Engine112,
            flags: Flags112,
            serializedVersion: 21,
            platform: Platform112);

        Assert.Equal(50, bundle.HeaderEnd);
        Assert.Equal(64, bundle.InfoStart);

        // Without bit 0x200 the data follow the information block with no padding at all.
        Assert.Equal(bundle.InfoEnd, bundle.DataStart);

        UnityBundle container = UnityBundle.Read(bundle.Bytes);
        Assert.Equal(7u, container.FormatVersion);
        Assert.Equal(Engine112, container.EngineVersion);
        Assert.Equal(21u, UnitySerializedFile.Read(container.Files[0].Data).Version);

        AssertSampleAssets(ThreeDSenProfileBundle.ReadTextAssets(bundle.Bytes));
    }

    // ------------------------------------------------------------------ AMD-039 §2, last bullet

    /// <summary>
    /// AMD-039 §2, last bullet: the path pattern of AMD-030 §3 does not change in 2.0 — the
    /// bundle is found under <c>StandaloneWindows64</c> from the installation folder, every
    /// profile is written unmodified under its asset name, and a second import of the same
    /// installation adds nothing (AMD-030 §2, AMD-039 §4 second part).
    /// </summary>
    [Fact] // AMD-030 §3, §5; AMD-039 §2, §4
    public void Amd039_the_2_0_installation_layout_imports_and_a_second_import_adds_nothing()
    {
        using TemporaryFolder folder = new();
        folder.WriteBundle("StandaloneWindows64", Bundle20().Bytes);

        ThreeDSenImporter importer = new(PhysicalProfileFileSystem.Instance, folder.Profiles);
        ThreeDSenImportSummary first = importer.ImportFrom(folder.Install);

        Assert.Equal(SampleAssets.Count, first.Imported);
        Assert.Equal(0, first.AlreadyPresent);
        Assert.Equal(0, first.Failed);
        Assert.Contains("StandaloneWindows64", first.BundlePath, StringComparison.Ordinal);

        // AMD-030 §5: the bytes written are the payload, unmodified, under <m_Name>.3dn.
        foreach ((string name, byte[] payload) in SampleAssets)
        {
            string path = Path.Combine(folder.Profiles, name + UserProfileFolder.Extension);
            Assert.True(File.Exists(path), path);
            Assert.Equal(payload, File.ReadAllBytes(path));
        }

        Assert.Equal(SampleAssets.Count, folder.WrittenFiles().Length);

        ThreeDSenImportSummary second = importer.ImportFrom(folder.Install);

        Assert.Equal(0, second.Imported);
        Assert.Equal(SampleAssets.Count, second.AlreadyPresent);
        Assert.Equal(0, second.Failed);
        Assert.Equal(SampleAssets.Count, folder.WrittenFiles().Length);
    }

    /// <summary>
    /// AMD-030 §5, unchanged by AMD-039: a truncated format 8 bundle produces a message, never a
    /// crash, and writes nothing — not even a partial file.
    /// </summary>
    [Fact] // AMD-030 §5, AMD-039 §3
    public void Amd039_a_truncated_format_8_bundle_gives_a_message_and_writes_nothing()
    {
        SyntheticBundle whole = Bundle20();
        byte[] truncated = whole.Bytes[..(whole.DataStart + 4)];

        using TemporaryFolder folder = new();
        string bundlePath = folder.WriteBundle("StandaloneWindows64", truncated);

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => new ThreeDSenImporter(PhysicalProfileFileSystem.Instance, folder.Profiles)
                .ImportFrom(bundlePath));

        Assert.NotEqual(string.Empty, error.Message);
        Assert.Empty(folder.WrittenFiles());
    }

    // ------------------------------------------------------------------ AMD-039 §4, real install

    /// <summary>
    /// AMD-039 §4, third bullet: on a real 3dSen 1.1.2 the criteria of AMD-030 §6 are unchanged,
    /// the byte-for-byte comparison with the reference inventory included. Skipped when no
    /// installation is at hand, and when the one at hand is a 2.0 (format version 8), whose
    /// profiles the inventory does not describe (AMD-039 §5).
    /// </summary>
    [RealInstallFact] // AMD-030 §6, AMD-039 §4
    public void Amd039_c5_a_real_1_1_2_installation_imports_the_101_profiles_of_the_inventory()
    {
        Assert.Equal(7u, RealInstall.BundleFormatVersion);

        using TemporaryFolder folder = new();
        ThreeDSenImporter importer = new(PhysicalProfileFileSystem.Instance, folder.Profiles);

        ThreeDSenImportSummary first = importer.ImportFrom(RealInstall.Root!);

        Assert.Equal(RealProfileCount, first.Imported);
        Assert.Equal(0, first.AlreadyPresent);
        Assert.Equal(0, first.Failed);

        string[] written = folder.WrittenFiles();
        Assert.Equal(RealProfileCount, written.Length);

        HashSet<string> inventory = RealInstall.InventoryHashes();
        Assert.Equal(RealProfileCount, inventory.Count);
        foreach (string path in written)
        {
            Assert.Contains(
                Convert.ToHexStringLower(SHA256.HashData(File.ReadAllBytes(path))),
                inventory);
        }

        ThreeDSenImportSummary second = importer.ImportFrom(RealInstall.Root!);
        Assert.Equal(0, second.Imported);
        Assert.Equal(RealProfileCount, second.AlreadyPresent);
        Assert.Equal(0, second.Failed);
    }

    /// <summary>
    /// AMD-039 §4, second bullet: on a real 3dSen 2.0, 101 profiles are imported and none fails,
    /// and a second import finds the 101 already present. The fingerprints are not compared with
    /// the reference inventory, which describes the 1.1.2 (§5). Skipped when the installation at
    /// hand is not a 2.0.
    /// </summary>
    [Version20Fact] // AMD-039 §4
    public void Amd039_c4_a_real_2_0_installation_imports_the_101_profiles()
    {
        Assert.Equal(8u, RealInstall.BundleFormatVersion);

        using TemporaryFolder folder = new();
        ThreeDSenImporter importer = new(PhysicalProfileFileSystem.Instance, folder.Profiles);

        ThreeDSenImportSummary first = importer.ImportFrom(RealInstall.Root!);
        Assert.Equal(RealProfileCount, first.Imported);
        Assert.Equal(0, first.Failed);
        Assert.Equal(RealProfileCount, folder.WrittenFiles().Length);

        ThreeDSenImportSummary second = importer.ImportFrom(RealInstall.Root!);
        Assert.Equal(0, second.Imported);
        Assert.Equal(RealProfileCount, second.AlreadyPresent);
        Assert.Equal(0, second.Failed);
    }

    // ================================================================== helpers

    private static void AssertSampleAssets(IReadOnlyList<UnityTextAsset> assets)
    {
        Assert.Equal(SampleAssets.Count, assets.Count);
        for (int index = 0; index < SampleAssets.Count; index++)
        {
            Assert.Equal(SampleAssets[index].Name, assets[index].Name);
            Assert.Equal(SampleAssets[index].Payload, assets[index].Script.ToArray());

            // AMD-030 §4.4: the payload is exactly a profile of SPEC-03.
            Assert.True(ProfileFormat.Load(assets[index].Script.Span).IsSuccess);
        }
    }

    /// <summary>A valid SPEC-03 profile whose title makes its bytes unlike the others'.</summary>
    private static byte[] ProfileBytes(string title)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = title;
        profile.Description = $"R4 V2-1 vector for {title}.";
        return ProfileFormat.Save(profile);
    }

    private static IEnumerable<int> DifferingBytes(byte[] left, byte[] right)
    {
        for (int at = 0; at < Math.Min(left.Length, right.Length); at++)
        {
            if (left[at] != right[at])
            {
                yield return at;
            }
        }
    }

    /// <summary>The same bundle with another flag word, the rest of the bytes untouched.</summary>
    private static byte[] PatchedFlags(SyntheticBundle bundle, uint flags)
    {
        byte[] copy = (byte[])bundle.Bytes.Clone();
        Blob.WriteU32(copy, bundle.FlagsAt, flags);
        return copy;
    }

    /// <summary>The bundle shape measured on 3dSen 2.0 (AMD-039 §2).</summary>
    private static SyntheticBundle Bundle20() => Build(
        formatVersion: 8,
        engineVersion: Engine20,
        flags: Flags20,
        serializedVersion: 22,
        platform: Platform20);

    /// <summary>
    /// Builds a whole bundle from the public description: header, alignment, LZ4HC information
    /// block, data blocks, serialized file and <c>TextAsset</c> objects.
    /// </summary>
    /// <remarks>
    /// The node path is lengthened one character at a time until the compressed information
    /// block ends <paramref name="infoEndResidue"/> bytes past a multiple of 16. The default, 5,
    /// is where the real 2.0 ends it (1493): the criterion of AMD-039 §4 needs an information
    /// block that does not end on a boundary, and 11 bytes of padding is what the measurement
    /// gives.
    /// </remarks>
    private static SyntheticBundle Build(
        uint formatVersion,
        string engineVersion,
        uint flags,
        uint serializedVersion,
        int platform,
        int infoEndResidue = 5)
    {
        byte[] serialized = BuildSerializedFile(
            serializedVersion, platform, engineVersion, SampleAssets, bigEndian: false);

        string node = "CAB-" + new string('a', 32);
        for (int extra = 0; extra < 32; extra++)
        {
            SyntheticBundle candidate = Assemble(
                formatVersion, engineVersion, flags, serialized, node + new string('b', extra));

            bool atEnd = (flags & InfoAtEndFlag) != 0;
            if (atEnd || candidate.InfoEnd % 16 == infoEndResidue)
            {
                return candidate;
            }
        }

        throw new InvalidOperationException("no node name gave the wanted information-block end.");
    }

    private static SyntheticBundle Assemble(
        uint formatVersion, string engineVersion, uint flags, byte[] serialized, string node)
    {
        // The data blocks: 1024 bytes each, LZ4HC, which decode as plain LZ4 (AMD-030 §4.1).
        const int BlockSize = 1024;
        List<byte[]> compressedBlocks = [];
        List<(uint Uncompressed, uint Compressed)> table = [];
        for (int at = 0; at < serialized.Length; at += BlockSize)
        {
            int length = Math.Min(BlockSize, serialized.Length - at);
            byte[] block = Lz4Literals(serialized.AsSpan(at, length));
            compressedBlocks.Add(block);
            table.Add(((uint)length, (uint)block.Length));
        }

        // The information block of AMD-030 §4.2.
        Blob info = new();
        info.Zeros(16);
        info.I32(table.Count);
        foreach ((uint uncompressed, uint compressed) in table)
        {
            info.U32(uncompressed);
            info.U32(compressed);
            info.U16((ushort)Lz4Hc);
        }

        info.I32(1);
        info.I64(0);
        info.I64(serialized.Length);
        info.U32(4);
        info.NullString(node);

        byte[] rawInfo = info.ToArray();
        byte[] compressedInfo = Lz4Literals(rawInfo);

        // The header of AMD-030 §4.1, big-endian.
        Blob bundle = new();
        bundle.NullString("UnityFS");
        bundle.U32(formatVersion);
        bundle.NullString("5.x.x");
        bundle.NullString(engineVersion);
        int totalAt = bundle.Length;
        bundle.I64(0);
        bundle.U32((uint)compressedInfo.Length);
        bundle.U32((uint)rawInfo.Length);
        int flagsAt = bundle.Length;
        bundle.U32(flags);
        int headerEnd = bundle.Length;

        if (formatVersion >= 7)
        {
            bundle.AlignTo(16);
        }

        int infoStart;
        int infoEnd;
        int dataStart;
        if ((flags & InfoAtEndFlag) == 0)
        {
            infoStart = bundle.Length;
            bundle.Bytes(compressedInfo);
            infoEnd = bundle.Length;

            if ((flags & DataAlignedFlag) != 0)
            {
                bundle.AlignTo(16);
            }

            dataStart = bundle.Length;
            foreach (byte[] block in compressedBlocks)
            {
                bundle.Bytes(block);
            }
        }
        else
        {
            dataStart = bundle.Length;
            foreach (byte[] block in compressedBlocks)
            {
                bundle.Bytes(block);
            }

            infoStart = bundle.Length;
            bundle.Bytes(compressedInfo);
            infoEnd = bundle.Length;
        }

        bundle.PatchI64(totalAt, bundle.Length);
        return new SyntheticBundle(
            bundle.ToArray(), flagsAt, headerEnd, infoStart, infoEnd, dataStart);
    }

    /// <summary>
    /// Builds a serialized file of AMD-030 §4.3 holding one <c>TextAsset</c> per asset plus the
    /// <c>AssetBundle</c> object the reader is to ignore.
    /// </summary>
    /// <remarks>
    /// In version 22 the three 32-bit header fields are left at zero and only the 64-bit ones
    /// carry the sizes and the data offset, which is what AMD-039 §2 calls « champs 64 bits »
    /// and what Q-071 settled the layout of.
    /// </remarks>
    private static byte[] BuildSerializedFile(
        uint version,
        int platform,
        string unityVersion,
        IReadOnlyList<(string Name, byte[] Payload)> assets,
        bool bigEndian)
    {
        // The objects, each in the byte order the file will announce (AMD-030 §4.4).
        List<(int ClassId, byte[] Data)> objects = [];
        foreach ((string name, byte[] payload) in assets)
        {
            Blob one = new() { BigEndian = bigEndian };
            byte[] nameBytes = Encoding.UTF8.GetBytes(name);
            one.I32(nameBytes.Length);
            one.Bytes(nameBytes);
            one.AlignTo(4);
            one.I32(payload.Length);
            one.Bytes(payload);
            objects.Add((UnityTextAsset.ClassId, one.ToArray()));
        }

        objects.Add((UnityTextAsset.AssetBundleClassId, [1, 2, 3, 4, 5, 6, 7, 8]));

        Blob data = new() { BigEndian = bigEndian };
        List<(long Start, long Size, int TypeIndex)> entries = [];
        foreach ((int classId, byte[] bytes) in objects)
        {
            data.AlignTo(8);
            entries.Add((
                data.Length,
                bytes.Length,
                classId == UnityTextAsset.ClassId ? 0 : 1));
            data.Bytes(bytes);
        }

        Blob file = new();
        int metadataSizeAt = file.Length;
        file.U32(0);
        int fileSizeAt = file.Length;
        file.U32(0);
        file.U32(version);
        int dataOffsetAt = file.Length;
        file.U32(0);
        file.U8((byte)(bigEndian ? 1 : 0));
        file.Zeros(3);

        int wideMetadataAt = -1;
        int wideFileSizeAt = -1;
        int wideDataOffsetAt = -1;
        if (version >= 22)
        {
            wideMetadataAt = file.Length;
            file.U32(0);
            wideFileSizeAt = file.Length;
            file.I64(0);
            wideDataOffsetAt = file.Length;
            file.I64(0);
            file.I64(0); // the reserved field of Q-071.
        }

        int headerEnd = file.Length;

        file.BigEndian = bigEndian;
        file.NullString(unityVersion);
        file.I32(platform);
        file.U8(0); // no type tree.
        file.I32(2);
        foreach (int classId in (int[])[UnityTextAsset.ClassId, UnityTextAsset.AssetBundleClassId])
        {
            file.I32(classId);
            file.U8(0); // stripped.
            file.I16(-1); // script type index.
            file.Zeros(16); // old type hash.
        }

        file.I32(entries.Count);
        file.AlignTo(4);
        long pathId = 1;
        foreach ((long start, long size, int typeIndex) in entries)
        {
            file.I64(pathId++);
            if (version >= 22)
            {
                file.I64(start);
            }
            else
            {
                file.U32((uint)start);
            }

            file.U32((uint)size);
            file.I32(typeIndex);
        }

        int metadataEnd = file.Length;
        file.BigEndian = true;
        file.AlignTo(16);
        int dataOffset = file.Length;
        file.Bytes(data.ToArray());
        int fileSize = file.Length;

        if (version >= 22)
        {
            file.PatchU32(wideMetadataAt, (uint)(metadataEnd - headerEnd));
            file.PatchI64(wideFileSizeAt, fileSize);
            file.PatchI64(wideDataOffsetAt, dataOffset);
        }
        else
        {
            file.PatchU32(metadataSizeAt, (uint)(metadataEnd - headerEnd));
            file.PatchU32(fileSizeAt, (uint)fileSize);
            file.PatchU32(dataOffsetAt, (uint)dataOffset);
        }

        return file.ToArray();
    }

    /// <summary>
    /// Encodes one LZ4 block holding nothing but literals: a token whose high nibble is the
    /// literal count — 15 plus extra bytes past 14 — then the bytes themselves, and no match.
    /// That is a licit output of the LZ4HC encoder, which AMD-030 §4.1 says decodes the same way.
    /// </summary>
    private static byte[] Lz4Literals(ReadOnlySpan<byte> data)
    {
        List<byte> block = [];
        if (data.Length < 15)
        {
            block.Add((byte)(data.Length << 4));
        }
        else
        {
            block.Add(0xF0);
            int extra = data.Length - 15;
            while (extra >= 255)
            {
                block.Add(0xFF);
                extra -= 255;
            }

            block.Add((byte)extra);
        }

        block.AddRange(data);
        return [.. block];
    }

    /// <summary>A bundle the tests built, with the positions the criteria talk about.</summary>
    private sealed record SyntheticBundle(
        byte[] Bytes, int FlagsAt, int HeaderEnd, int InfoStart, int InfoEnd, int DataStart);

    /// <summary>A byte buffer with a byte order, used to lay out every structure by hand.</summary>
    private sealed class Blob
    {
        private readonly List<byte> _bytes = [];

        public bool BigEndian { get; set; } = true;

        public int Length => _bytes.Count;

        public static void WriteU32(byte[] target, int at, uint value)
        {
            target[at] = (byte)(value >> 24);
            target[at + 1] = (byte)(value >> 16);
            target[at + 2] = (byte)(value >> 8);
            target[at + 3] = (byte)value;
        }

        public void U8(byte value) => _bytes.Add(value);

        public void Bytes(ReadOnlySpan<byte> value)
        {
            foreach (byte one in value)
            {
                _bytes.Add(one);
            }
        }

        public void Zeros(int count)
        {
            for (int index = 0; index < count; index++)
            {
                _bytes.Add(0);
            }
        }

        public void I16(short value) => Write((ulong)(ushort)value, 2);

        public void U16(ushort value) => Write(value, 2);

        public void I32(int value) => Write((uint)value, 4);

        public void U32(uint value) => Write(value, 4);

        public void I64(long value) => Write((ulong)value, 8);

        public void NullString(string value)
        {
            Bytes(Encoding.UTF8.GetBytes(value));
            _bytes.Add(0);
        }

        public void AlignTo(int alignment)
        {
            int padding = (alignment - (_bytes.Count % alignment)) % alignment;
            Zeros(padding);
        }

        public void PatchU32(int at, uint value)
        {
            byte[] bytes = new byte[4];
            WriteU32(bytes, 0, value);
            for (int index = 0; index < 4; index++)
            {
                _bytes[at + index] = bytes[index];
            }
        }

        public void PatchI64(int at, long value)
        {
            for (int index = 0; index < 8; index++)
            {
                _bytes[at + index] = (byte)((ulong)value >> (8 * (7 - index)));
            }
        }

        public byte[] ToArray() => [.. _bytes];

        private void Write(ulong value, int width)
        {
            for (int index = 0; index < width; index++)
            {
                int shift = BigEndian ? 8 * (width - 1 - index) : 8 * index;
                _bytes.Add((byte)(value >> shift));
            }
        }
    }

    /// <summary>An installation layout and a profile folder, both thrown away afterwards.</summary>
    private sealed class TemporaryFolder : IDisposable
    {
        private readonly string _root =
            Path.Combine(Path.GetTempPath(), "zax-r4-v2-1-" + Guid.NewGuid().ToString("N"));

        public TemporaryFolder()
        {
            Install = Path.Combine(_root, "install");
            Profiles = Path.Combine(_root, "profiles");
            Directory.CreateDirectory(Install);
            Directory.CreateDirectory(Profiles);
        }

        /// <summary>The folder standing for the one holding <c>3dSen.exe</c>.</summary>
        public string Install { get; }

        /// <summary>The folder of imported profiles (SPEC-09).</summary>
        public string Profiles { get; }

        /// <summary>
        /// Writes a bundle at the path pattern of AMD-030 §3, which AMD-039 §2 keeps unchanged:
        /// <c>&lt;install&gt;/3dSen_Data/StreamingAssets/aa/&lt;platform&gt;/defaultprofiles_assets_all_*.bundle</c>.
        /// </summary>
        public string WriteBundle(string platform, byte[] bytes)
        {
            string folder = Path.Combine(
                Install, "3dSen_Data", "StreamingAssets", "aa", platform);
            Directory.CreateDirectory(folder);
            string path = Path.Combine(
                folder, "defaultprofiles_assets_all_" + new string('0', 32) + ".bundle");
            File.WriteAllBytes(path, bytes);
            return path;
        }

        /// <summary>Everything that landed in the profile folder, partial files included.</summary>
        public string[] WrittenFiles() => Directory.GetFiles(Profiles);

        public void Dispose()
        {
            try
            {
                Directory.Delete(_root, recursive: true);
            }
            catch (IOException)
            {
                // A folder the system still holds open is not worth failing a test over.
            }
        }
    }

    /// <summary>
    /// The installation the optional criteria of AMD-039 §4 need: the one
    /// <c>ZAXNES_3DSEN_INSTALL</c> names, else the stand-in of <c>cleanroom/verify/p8-input</c>.
    /// </summary>
    internal static class RealInstall
    {
        /// <summary>The environment variable naming an installed 3dSen.</summary>
        public const string Variable = "ZAXNES_3DSEN_INSTALL";

        /// <summary>The installation folder, or null.</summary>
        public static string? Root { get; } = Locate();

        /// <summary>
        /// The bundle's format version — 7 on a 1.1.2 and 8 on a 2.0 (AMD-039 §2) — or null.
        /// </summary>
        public static uint? BundleFormatVersion { get; } = ReadFormatVersion();

        /// <summary>The SHA-256 of every <c>Profile</c> of <c>assets-neutral/inventory.json</c>.</summary>
        public static HashSet<string> InventoryHashes()
        {
            using FileStream stream = File.OpenRead(Path.Combine(NeutralAssets.Root, "inventory.json"));
            using JsonDocument document = JsonDocument.Parse(stream);

            HashSet<string> hashes = new(StringComparer.OrdinalIgnoreCase);
            foreach (JsonElement entry in document.RootElement.EnumerateArray())
            {
                if (entry.TryGetProperty("kind", out JsonElement kind)
                    && string.Equals(kind.GetString(), "Profile", StringComparison.Ordinal)
                    && entry.TryGetProperty("sha256", out JsonElement hash)
                    && hash.GetString() is { Length: > 0 } text)
                {
                    hashes.Add(text);
                }
            }

            return hashes;
        }

        private static string? Locate()
        {
            string? named = Environment.GetEnvironmentVariable(Variable);
            if (!string.IsNullOrWhiteSpace(named) && Directory.Exists(named))
            {
                return named;
            }

            DirectoryInfo? at = new(AppContext.BaseDirectory);
            while (at is not null)
            {
                string candidate = Path.Combine(
                    at.FullName, "verify", "p8-input", "fake-3dsen-install");
                if (Directory.Exists(candidate))
                {
                    return candidate;
                }

                at = at.Parent;
            }

            return null;
        }

        private static uint? ReadFormatVersion()
        {
            if (Root is null || ThreeDSenInstallation.FindBundle(Root) is not { } bundle)
            {
                return null;
            }

            try
            {
                using FileStream stream = File.OpenRead(bundle);
                byte[] head = new byte[12];
                stream.ReadExactly(head);
                return ((uint)head[8] << 24) | ((uint)head[9] << 16)
                    | ((uint)head[10] << 8) | head[11];
            }
            catch (IOException)
            {
                return null;
            }
        }
    }
}

/// <summary>
/// A fact needing a real 3dSen 1.1.2 and the reference inventory (AMD-039 §4, third bullet).
/// </summary>
public sealed class RealInstallFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public RealInstallFactAttribute()
    {
        if (R4V2ImportTests.RealInstall.Root is null)
        {
            Skip = "No 3dSen installation: neither ZAXNES_3DSEN_INSTALL nor verify/p8-input.";
        }
        else if (!NeutralAssets.Available)
        {
            Skip = "assets-neutral is not available, so the reference inventory cannot be read.";
        }
        else if (R4V2ImportTests.RealInstall.BundleFormatVersion != 7)
        {
            Skip = "The installation is not a 3dSen 1.1.2 (bundle format version 7).";
        }
    }
}

/// <summary>A fact needing a real 3dSen 2.0 (AMD-039 §4, second bullet).</summary>
public sealed class Version20FactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public Version20FactAttribute()
    {
        if (R4V2ImportTests.RealInstall.BundleFormatVersion != 8)
        {
            Skip = "No 3dSen 2.0 installation (bundle format version 8) is available.";
        }
    }
}
