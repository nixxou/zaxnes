using Zax.Profiles.Hash;
using Zax.Profiles.Wire;

namespace Zax.Profiles.Tests;

/// <summary>
/// Reading a « 3dh » profile: recognition, envelope, validation, library header and the three
/// payload differences (AMD-062 REQ-PROF-070 to REQ-PROF-074).
/// </summary>
public sealed class HashProfileReaderTests
{
    // REQ-PROF-070: the 101 files of profiles-2.1 are recognised as « 3dh », the 101 of
    // profiles as 3dn.
    [HashAssetFact]
    public void Recognition_separates_the_two_formats()
    {
        IReadOnlyList<string> hashFiles = NeutralAssets.HashProfileFiles();
        Assert.Equal(101, hashFiles.Count);
        foreach (string file in hashFiles)
        {
            Assert.True(
                ProfileFormat.IsHashProfile(File.ReadAllBytes(file)),
                $"{Path.GetFileName(file)} should be a « 3dh » profile.");
        }

        IReadOnlyList<string> plainFiles = NeutralAssets.ProfileFiles();
        Assert.Equal(101, plainFiles.Count);
        foreach (string file in plainFiles)
        {
            Assert.False(
                ProfileFormat.IsHashProfile(File.ReadAllBytes(file)),
                $"{Path.GetFileName(file)} should be a 3dn profile.");
        }
    }

    // REQ-PROF-071: BalloonFight (2.1) has fields 1, 2, 3, 6, 7, 12 in that order,
    // Version = "5", 330 identifiers (5 280 bytes), PayloadHash = SHA-256 of Payload.
    [HashAssetFact]
    public void Envelope_of_BalloonFight_has_the_expected_shape()
    {
        byte[] bytes = NeutralAssets.ReadHashProfile("BalloonFight.bytes");
        Assert.Equal<int>([1, 2, 3, 6, 7, 12], HashEnvelopeCodec.FieldOrder(bytes));

        HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(bytes);
        Assert.Equal("3dh", envelope.Type);
        Assert.Equal("5", envelope.Version);
        Assert.Equal(330, envelope.Ids.Count);
        Assert.Equal(330 * 16, envelope.IdsByteLength);
        Assert.Equal(envelope.ComputePayloadHash(), envelope.PayloadHash);
        Assert.Empty(envelope.Hashes);
    }

    // REQ-PROF-072, notes: the 101 files are version "5", with an empty Hashes and no tile in
    // field 4; the manifest is therefore the identifier table alone.
    [HashAssetFact]
    public void Every_built_in_hash_profile_is_version_5_without_legacy_digests()
    {
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(File.ReadAllBytes(file));
            Assert.Equal("5", envelope.Version);
            Assert.Empty(envelope.Hashes);
            Assert.Equal(envelope.ComputePayloadHash(), envelope.PayloadHash);
        }
    }

    // REQ-PROF-072: the five refusals, each with its own log message, and the game starts
    // without a profile.
    [HashAssetFact]
    public void A_damaged_payload_is_refused()
    {
        byte[] bytes = NeutralAssets.ReadHashProfile("BalloonFight.bytes");
        HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(bytes);
        envelope.Payload![100] ^= 0x01;

        ProfileLoadResult result = ProfileFormat.Load(Rebuild(envelope));

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.DamagedHashPayload, result.Error);
        Assert.Equal("Damaged hash profile payload", result.Message);
        Assert.Empty(result.ProfileOrEmpty().PermanentPatterns);
    }

    [HashAssetFact]
    public void Envelope_version_4_is_refused_with_its_own_message()
    {
        ProfileLoadResult result = ProfileFormat.Load(WithVersion("4"));

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.UnsupportedHashEnvelope, result.Error);
        Assert.Equal(
            "Hash profile envelope version 4 is no longer supported: its tile identifiers "
            + "cannot be resolved. Import the original profile again.",
            result.Message);
        Assert.Empty(result.ProfileOrEmpty().PermanentPatterns);
    }

    [HashAssetFact]
    public void Envelope_version_3_is_refused_as_unsupported()
    {
        ProfileLoadResult result = ProfileFormat.Load(WithVersion("3"));

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.UnsupportedHashEnvelope, result.Error);
        Assert.Equal("Unsupported hash profile envelope", result.Message);
    }

    [HashAssetFact]
    public void Versions_1_2_and_5_are_accepted()
    {
        foreach (string version in (string[])["1", "2", "5"])
        {
            ProfileLoadResult result = ProfileFormat.Load(WithVersion(version));
            Assert.True(result.IsSuccess, $"version {version}: {result.Message}");
        }
    }

    [Fact]
    public void An_envelope_without_a_payload_is_refused()
    {
        ProfileLoadResult result = ProfileFormat.Load(
            SyntheticHashProfile.EncodeEnvelope(payload: null, writePayloadHash: false));

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.UnsupportedHashEnvelope, result.Error);
        Assert.Equal("Unsupported hash profile envelope", result.Message);
    }

    [HashAssetFact]
    public void A_duplicated_identifier_makes_the_manifest_invalid()
    {
        HashProfileEnvelope envelope = Envelope("BalloonFight.bytes");
        envelope.Ids.Add(envelope.Ids[0]);

        ProfileLoadResult result = ProfileFormat.Load(Rebuild(envelope));

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.InvalidHashManifest, result.Error);
        Assert.Equal("Invalid hash profile manifest", result.Message);
    }

    [HashAssetFact]
    public void An_identifier_also_listed_in_Hashes_makes_the_manifest_invalid()
    {
        HashProfileEnvelope envelope = Envelope("BalloonFight.bytes");
        envelope.Hashes.Add(envelope.Ids[0].ToText());

        ProfileLoadResult result = ProfileFormat.Load(Rebuild(envelope));

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.InvalidHashManifest, result.Error);
        Assert.Equal("Invalid hash profile manifest", result.Message);
    }

    [HashAssetFact]
    public void An_Ids_length_that_is_not_a_multiple_of_sixteen_makes_the_manifest_invalid()
    {
        HashProfileEnvelope envelope = Envelope("BalloonFight.bytes");
        envelope.IdsByteLength = (envelope.Ids.Count * 16) + 3;

        Assert.Null(envelope.BuildManifest(out string? error));
        Assert.Equal("Invalid hash profile manifest", error);
    }

    [Fact]
    public void A_32_digit_entry_of_Hashes_is_an_identifier_in_versions_2_and_5_only()
    {
        string text = TileId.Compute(new byte[16]).ToText();

        foreach (string version in (string[])["2", "5"])
        {
            HashProfileEnvelope accepted = new() { Version = version, Payload = [] };
            accepted.Hashes.Add(text);
            Assert.NotNull(accepted.BuildManifest(out _));
        }

        HashProfileEnvelope refused = new() { Version = "1", Payload = [] };
        refused.Hashes.Add(text);
        Assert.Null(refused.BuildManifest(out string? error));
        Assert.Equal("Invalid hash profile manifest", error);
    }

    [Fact]
    public void A_Hashes_entry_that_is_not_lowercase_hexadecimal_makes_the_manifest_invalid()
    {
        HashProfileEnvelope envelope = new() { Version = "5", Payload = [] };
        envelope.Hashes.Add(new string('A', 64));

        Assert.Null(envelope.BuildManifest(out string? error));
        Assert.Equal("Invalid hash profile manifest", error);
    }

    // REQ-PROF-073: the header of a « 3dh » comes from the envelope, and a file whose payload
    // is damaged still shows up with the Id and the Title of its envelope.
    [HashAssetFact]
    public void The_library_header_comes_from_the_envelope()
    {
        byte[] bytes = NeutralAssets.ReadHashProfile("Super Mario Bros.bytes");
        ProfileHeader header = Assert.NotNull(ProfileFormat.LoadHeader(bytes));

        Assert.Equal("Jhj6E7iOysxT", header.Id);
        Assert.True(header.IsHashEnvelope);
        Assert.True(header.HasAcceptableSignature);
        Assert.Equal(ProfileFormatVersion.Current, header.FormatVersion);

        ProfileHeader plain = Assert.NotNull(
            ProfileFormat.LoadHeader(NeutralAssets.ReadProfile("Super Mario Bros.bytes")));
        Assert.Equal(plain.Id, header.Id);
    }

    [HashAssetFact]
    public void A_refused_hash_profile_still_shows_its_header()
    {
        HashProfileEnvelope envelope = Envelope("Super Mario Bros.bytes");
        envelope.Payload![100] ^= 0x01;
        byte[] bytes = Rebuild(envelope);

        Assert.False(ProfileFormat.Load(bytes).IsSuccess);

        ProfileHeader header = Assert.NotNull(ProfileFormat.LoadHeader(bytes));
        Assert.Equal("Jhj6E7iOysxT", header.Id);
        Assert.Equal(envelope.Title, header.Title);
    }

    [HashAssetFact]
    public void Every_envelope_carries_an_identifier_and_matches_its_3dn_counterpart()
    {
        int withCounterpartId = 0;
        int withoutCounterpartId = 0;
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            ProfileHeader header = Assert.NotNull(ProfileFormat.LoadHeader(File.ReadAllBytes(file)));
            Assert.False(string.IsNullOrEmpty(header.Id), $"{name} has no envelope Id.");

            string counterpart = Path.Combine(
                NeutralAssets.ProfilesDirectory!, NeutralAssets.CounterpartName(name));
            ProfileHeader plain = Assert.NotNull(
                ProfileFormat.LoadHeader(File.ReadAllBytes(counterpart)));
            if (plain.HasId)
            {
                Assert.Equal(plain.Id, header.Id);
                withCounterpartId++;
            }
            else
            {
                withoutCounterpartId++;
            }
        }

        // REQ-PROF-073, notes: 56 of the 101 counterparts carry an Id, equal in every case.
        Assert.Equal(56, withCounterpartId);
        Assert.Equal(45, withoutCounterpartId);
    }

    // REQ-PROF-074 point 1 and 2, on Mega Man 2 (2.1).
    [HashAssetFact]
    public void Mega_Man_2_has_the_tile_and_variant_fields_of_the_hash_format()
    {
        ProfileDocument document = LoadHash("Mega Man 2.bytes");

        foreach (ProfileTile tile in document.HashTiles)
        {
            Assert.True(tile.IsPending);
            Assert.True(tile.Hash!.Value.StoredAsIdentifier);
            Assert.Equal<int>([3, 5], document.Layout.PresentFields(tile)!);
        }

        int withKind = 0;
        int kind1 = 0;
        int kind2 = 0;
        int withXor = 0;
        int withPlainTexture = 0;
        foreach (PatternVariant variant in AllVariants(document.Profile, includeSlots: false))
        {
            if (variant.TextureKind is { } kind)
            {
                withKind++;
                if (kind == 1)
                {
                    kind1++;
                }
                else if (kind == 2)
                {
                    kind2++;
                }
            }

            if (variant.XorTexture is not null)
            {
                withXor++;
            }

            if (document.Layout.IsPresent(variant, 19))
            {
                withPlainTexture++;
            }
        }

        Assert.Equal(1809, withKind);
        Assert.Equal(1663, kind1);
        Assert.Equal(146, kind2);
        Assert.Equal(146, withXor);
        Assert.Equal(0, withPlainTexture);
        Assert.Equal("3dn", document.Profile.Signature);
        Assert.Equal("2", document.Profile.VersionText);
    }

    // REQ-PROF-074: over the 101 files, 225 843 tiles, all of fields {3, 5}; 74 482 variants
    // outside the memorised slots, split 60 777 / 13 559 + 142 / 4; the 1 010 variants of the
    // memorised slots carry no field 25.
    [HashAssetFact]
    public void The_hundred_and_one_payloads_have_the_measured_partition()
    {
        int tiles = 0;
        int variants = 0;
        int kind0 = 0;
        int kind1 = 0;
        int kind2 = 0;
        int xorFormat0SameSize = 0;
        int xorFormat0OtherSize = 0;
        int xorFormat1 = 0;
        int slotVariants = 0;
        int slotVariantsWithKind = 0;

        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(file), sourceName: name);
            Assert.True(result.IsSuccess, $"{name}: {result.Message}");
            ProfileDocument document = result.Document!;
            tiles += document.HashTiles.Count;

            foreach (PatternVariant variant in AllVariants(document.Profile, includeSlots: false))
            {
                variants++;
                Assert.NotNull(variant.TextureKind);
                switch (variant.TextureKind)
                {
                    case 0:
                        kind0++;
                        break;
                    case 1:
                        kind1++;
                        break;
                    default:
                        kind2++;
                        break;
                }
            }

            foreach (ProfilePattern pattern in AllPatterns(document.Profile, includeSlots: false))
            {
                int patternWidth = PatternDefaultTexture.CellsX(pattern) * 8;
                int patternHeight = PatternDefaultTexture.CellsY(pattern) * 8;
                foreach (PatternVariant variant in pattern.Variants)
                {
                    if (variant.XorTexture is not { Pixels: { } pixels } stored)
                    {
                        continue;
                    }

                    Assert.Equal(2, variant.TextureKind);
                    if (pixels.Format == TextureFormat.Direct)
                    {
                        xorFormat1++;
                    }
                    else if (stored.Width == patternWidth && stored.Height == patternHeight)
                    {
                        xorFormat0SameSize++;
                    }
                    else
                    {
                        xorFormat0OtherSize++;
                    }
                }
            }

            foreach (PatternVariant variant in SlotVariants(document.Profile))
            {
                slotVariants++;
                if (variant.TextureKind is not null)
                {
                    slotVariantsWithKind++;
                }
            }
        }

        Assert.Equal(225_843, tiles);
        Assert.Equal(74_482, variants);
        Assert.Equal(0, kind0);
        Assert.Equal(60_777, kind1);

        // The four groups of REQ-PROF-074 add up to the 74 482 variants: 60 777 of
        // textureKind = 1, then the textureKind = 2 ones split by the shape of their field 24.
        Assert.Equal(13_559 + 4 + 142, kind2);
        Assert.Equal(13_559, xorFormat0SameSize);
        Assert.Equal(4, xorFormat0OtherSize);
        Assert.Equal(142, xorFormat1);
        Assert.Equal(1_010, slotVariants);
        Assert.Equal(0, slotVariantsWithKind);
    }

    // REQ-PROF-074 point 3: the root field order of the payload, over the 101 files.
    [HashAssetFact]
    public void The_payload_root_order_puts_the_message_fields_first()
    {
        HashSet<string> combinations = [];
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(File.ReadAllBytes(file));
            List<int> distinct = [];
            foreach (int field in HashEnvelopeCodec.FieldOrder(envelope.Payload!))
            {
                if (distinct.Count == 0 || distinct[^1] != field)
                {
                    distinct.Add(field);
                }
            }

            Assert.Equal(distinct.Distinct().Count(), distinct.Count);
            int settings = distinct.IndexOf(6);
            Assert.True(settings >= 0, $"{name} has no settings.");
            foreach (int field in distinct[..settings])
            {
                Assert.Contains(field, (int[])[3, 4, 5]);
            }

            List<int> head = [.. distinct[..settings]];
            List<int> sortedHead = [.. head];
            sortedHead.Sort();
            Assert.Equal(sortedHead, head);

            List<int> tail = [.. distinct[(settings + 1)..]];
            List<int> sortedTail = [.. tail];
            sortedTail.Sort();
            Assert.Equal(sortedTail, tail);
            foreach (int field in tail)
            {
                Assert.Contains(field, (int[])[1, 2, 7, 8, 9, 10, 11]);
            }

            combinations.Add(string.Join(",", distinct));
        }

        Assert.Equal(11, combinations.Count);
        Assert.Contains("3,6,1,2", combinations);
    }

    // REQ-PROF-074: a tile whose string is outside the manifest, and a bad textureKind.
    [Fact]
    public void A_tile_hash_outside_the_manifest_is_refused()
    {
        TileId absent = TileId.Compute(RawTile.FromPlanes(0xDEADBEEFUL, 0x1234UL));
        byte[] bytes = SyntheticHashProfile.Build(
            manifest: [TileId.Compute(new byte[16])],
            tileKeys: [absent.ToText()]);

        ProfileLoadResult result = ProfileFormat.Load(bytes);

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.InvalidHashPayload, result.Error);
        Assert.Equal($"Tile hash absent from profile manifest: {absent.ToText()}", result.Message);
    }

    [Fact]
    public void An_unusable_textureKind_is_refused()
    {
        TileId id = TileId.Compute(new byte[16]);

        ProfileLoadResult three = ProfileFormat.Load(SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], textureKind: 3));
        Assert.False(three.IsSuccess);
        Assert.Equal("Invalid profile XOR texture", three.Message);

        ProfileLoadResult twoWithoutTexture = ProfileFormat.Load(SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], textureKind: 2));
        Assert.False(twoWithoutTexture.IsSuccess);
        Assert.Equal("Invalid profile XOR texture", twoWithoutTexture.Message);
    }

    // REQ-PROF-074: the reader accepts the root fields in any order.
    [Fact]
    public void The_reader_accepts_the_root_fields_in_any_order()
    {
        TileId id = TileId.Compute(new byte[16]);
        byte[] ascending = SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], hashRootOrder: false);
        byte[] hashOrder = SyntheticHashProfile.Build(
            manifest: [id], tileKeys: [id.ToText()], hashRootOrder: true);

        Assert.NotEqual(ascending, hashOrder);

        ProfileLoadResult ascendingResult = ProfileFormat.Load(ascending);
        ProfileLoadResult hashResult = ProfileFormat.Load(hashOrder);
        Assert.True(ascendingResult.IsSuccess, ascendingResult.Message);
        Assert.True(hashResult.IsSuccess, hashResult.Message);

        ProfileDocument first = ascendingResult.Document!;
        ProfileDocument second = hashResult.Document!;
        Assert.Equal(first.Profile.PermanentPatterns.Count, second.Profile.PermanentPatterns.Count);
        Assert.Equal(first.HashTiles.Count, second.HashTiles.Count);
        Assert.Equal("3dn", second.Profile.Signature);
    }

    internal static ProfileDocument LoadHash(string fileName)
    {
        ProfileLoadResult result = ProfileFormat.Load(
            NeutralAssets.ReadHashProfile(fileName), sourceName: fileName);
        Assert.True(result.IsSuccess, $"{fileName}: {result.Message}");
        return result.Document!;
    }

    internal static IEnumerable<ProfilePattern> AllPatterns(GameProfile profile, bool includeSlots)
    {
        foreach (ProfilePattern pattern in profile.PermanentPatterns)
        {
            yield return pattern;
        }

        foreach (ProfilePattern pattern in profile.TemporaryPatterns)
        {
            yield return pattern;
        }

        if (!includeSlots || profile.Settings is not { } settings)
        {
            yield break;
        }

        foreach (RecordedPatternSlot slot in settings.SavedPatterns)
        {
            if (slot.Pattern is { } pattern)
            {
                yield return pattern;
            }
        }

        foreach (RecordedPatternSlot slot in settings.SavedAnimPatterns)
        {
            if (slot.Pattern is { } pattern)
            {
                yield return pattern;
            }
        }
    }

    internal static IEnumerable<PatternVariant> AllVariants(GameProfile profile, bool includeSlots)
    {
        foreach (ProfilePattern pattern in AllPatterns(profile, includeSlots))
        {
            foreach (PatternVariant variant in pattern.Variants)
            {
                yield return variant;
            }
        }
    }

    internal static IEnumerable<PatternVariant> SlotVariants(GameProfile profile)
    {
        if (profile.Settings is not { } settings)
        {
            yield break;
        }

        foreach (RecordedPatternSlot slot in settings.SavedPatterns)
        {
            foreach (PatternVariant variant in slot.Pattern?.Variants ?? [])
            {
                yield return variant;
            }
        }

        foreach (RecordedPatternSlot slot in settings.SavedAnimPatterns)
        {
            foreach (PatternVariant variant in slot.Pattern?.Variants ?? [])
            {
                yield return variant;
            }
        }
    }

    private static HashProfileEnvelope Envelope(string fileName)
        => HashEnvelopeCodec.Decode(NeutralAssets.ReadHashProfile(fileName));

    private static byte[] WithVersion(string version)
    {
        HashProfileEnvelope envelope = Envelope("BalloonFight.bytes");
        return Rebuild(envelope, version: version);
    }

    /// <summary>
    /// Re-encodes an envelope a test has altered, keeping the digest it carries so that a
    /// damaged payload stays detectable.
    /// </summary>
    private static byte[] Rebuild(
        HashProfileEnvelope envelope, string? version = null, string? payloadHash = null)
        => SyntheticHashProfile.EncodeEnvelope(
            envelope.Payload,
            type: envelope.Type,
            version: version ?? envelope.Version,
            hashes: envelope.Hashes,
            ids: envelope.Ids,
            payloadHashOverride: payloadHash ?? envelope.PayloadHash,
            id: envelope.Id,
            title: envelope.Title,
            author: envelope.Author,
            publisher: envelope.Publisher,
            description: envelope.Description);
}
