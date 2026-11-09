using Zax.Profiles.Hash;
using Zax.Profiles.Wire;

namespace Zax.Profiles.Tests;

/// <summary>
/// The lossless rewrite of a « 3dh » profile (AMD-062 REQ-PROF-084): a « 3dh » stays a « 3dh »,
/// byte for byte, resolved or not; a 3dn stays a 3dn.
/// </summary>
public sealed class HashProfileRewriteTests
{
    private static readonly string[] NotPairable = ["DQ3_DW3.bytes", "Final_Fantasy_USA.bytes"];

    // REQ-PROF-084: decoding then rewriting each of the 101 files gives the same bytes, with no
    // ROM at all — no tile resolved.
    [HashAssetFact]
    public void Every_hash_profile_is_rewritten_byte_for_byte_without_a_rom()
    {
        int rewritten = 0;
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            byte[] bytes = File.ReadAllBytes(file);
            ProfileDocument document = Load(bytes, name);

            Assert.True(document.IsHashProfile);
            Assert.Equal(document.Tiles!.TotalCount, document.Tiles.PendingCount);
            Assert.Equal(bytes, document.Save());
            rewritten++;
        }

        Assert.Equal(101, rewritten);
    }

    // REQ-PROF-084: the same, with every tile resolved — the synthetic ROMs of REQ-PROF-078 for
    // the 99 pairable profiles. A resolved tile is written back by its identifier, never by its
    // planes, so the bytes do not move.
    [HashAssetFact]
    public void Every_pairable_hash_profile_is_rewritten_byte_for_byte_with_a_rom()
    {
        int rewritten = 0;
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            if (NotPairable.Contains(name))
            {
                continue;
            }

            byte[] bytes = File.ReadAllBytes(file);
            ProfileDocument document = Load(bytes, name);

            string counterpart = Path.Combine(
                NeutralAssets.ProfilesDirectory!, NeutralAssets.CounterpartName(name));
            byte[] rom = SyntheticRom.WithTiles(
                SyntheticRom.DistinctRawTiles(Load(File.ReadAllBytes(counterpart), counterpart).Profile));

            document.Tiles!.ResolveFromRom(rom);
            Assert.Equal(0, document.Tiles.PendingCount);
            document.Tiles.RebuildAllTextures();

            Assert.Equal(bytes, document.Save());
            rewritten++;
        }

        Assert.Equal(99, rewritten);
    }

    // REQ-PROF-084: DQ3_DW3 and Final_Fantasy_USA keep manifest strings pending for want of a
    // known raw tile, and the rewrite is identical in both cases.
    //
    // The note of REQ-PROF-084 says 173 and 67. Measured here: 173 and 66. One manifest string
    // of Final_Fantasy_USA is the **mirror** of a tile its 1.x counterpart carries, and step 2
    // of REQ-PROF-078 — which is normative — resolves the four mirrorings of every window. With
    // the un-flipped tiles alone the count is exactly 67, which is what the note measured; the
    // rule the amendment prescribes gives 66. See Q-178 §3.
    [HashAssetFact]
    public void The_two_unpairable_profiles_are_rewritten_identically()
    {
        (string File, int Pending)[] cases = [("DQ3_DW3.bytes", 173), ("Final_Fantasy_USA.bytes", 66)];
        foreach ((string name, int pending) in cases)
        {
            byte[] bytes = NeutralAssets.ReadHashProfile(name);

            ProfileDocument bare = Load(bytes, name);
            Assert.Equal(bytes, bare.Save());

            ProfileDocument resolved = Load(bytes, name);
            string counterpart = Path.Combine(
                NeutralAssets.ProfilesDirectory!, NeutralAssets.CounterpartName(name));
            byte[] rom = SyntheticRom.WithTiles(
                SyntheticRom.DistinctRawTiles(Load(File.ReadAllBytes(counterpart), counterpart).Profile));

            resolved.Tiles!.ResolveFromRom(rom);
            Assert.Equal(pending, resolved.Tiles.PendingCount);
            resolved.Tiles.RebuildAllTextures();

            Assert.Equal(bytes, resolved.Save());
        }
    }

    // REQ-PROF-084: the canonical envelope writes fields 1, 2, 3, 4, 6 to 12 in that order, with
    // null strings and empty lists left out and the payload digest recomputed.
    [HashAssetFact]
    public void The_canonical_envelope_writes_its_fields_in_order()
    {
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            byte[] rewritten = Load(File.ReadAllBytes(file), file).Save();
            IReadOnlyList<int> order = HashEnvelopeCodec.FieldOrder(rewritten);

            List<int> sorted = [.. order];
            sorted.Sort();
            Assert.Equal(sorted, order);
            Assert.DoesNotContain(5, order);
            Assert.Contains(1, order);
            Assert.Contains(2, order);
            Assert.Contains(3, order);
            Assert.Contains(6, order);

            HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(rewritten);
            Assert.Equal("5", envelope.Version);
            Assert.Equal(envelope.ComputePayloadHash(), envelope.PayloadHash);
        }
    }

    // REQ-PROF-084: the identifiers of field 12 are sorted by their text form (ordinal).
    [HashAssetFact]
    public void The_identifier_table_is_sorted_by_text_form()
    {
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(File.ReadAllBytes(file));
            List<string> texts = [.. envelope.Ids.Select(static id => id.ToText())];
            List<string> sorted = [.. texts];
            sorted.Sort(StringComparer.Ordinal);
            Assert.Equal(sorted, texts);
        }
    }

    // REQ-PROF-084: the payload writes field 12 (skipGeo) for every pattern of the root lists 4
    // and 5, even false, where a 3dn reproduces the absence it read.
    [HashAssetFact]
    public void Every_listed_pattern_of_a_hash_payload_carries_skipGeo()
    {
        int patterns = 0;
        int plainPatternsWithout = 0;
        int slotPatternsWithout = 0;
        int slotPatterns = 0;

        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            ProfileDocument document = Load(File.ReadAllBytes(file), file);
            foreach (ProfilePattern pattern in
                document.Profile.PermanentPatterns.Concat(document.Profile.TemporaryPatterns))
            {
                patterns++;
                Assert.True(document.Layout.IsPresent(pattern, 12));
            }

            foreach (ProfilePattern pattern in SlotPatterns(document.Profile))
            {
                slotPatterns++;
                if (!document.Layout.IsPresent(pattern, 12))
                {
                    slotPatternsWithout++;
                }
            }
        }

        foreach (string file in NeutralAssets.ProfileFiles())
        {
            ProfileDocument document = NeutralAssets.LoadProfile(Path.GetFileName(file));
            foreach (ProfilePattern pattern in
                document.Profile.PermanentPatterns.Concat(document.Profile.TemporaryPatterns))
            {
                if (!document.Layout.IsPresent(pattern, 12))
                {
                    plainPatternsWithout++;
                }
            }
        }

        Assert.True(patterns > 0);

        // REQ-PROF-084: 27 330 patterns of the 101 1.x profiles leave skipGeo out, and 480 of
        // the 1 010 memorised slots do on both sides.
        Assert.Equal(27_330, plainPatternsWithout);
        Assert.Equal(1_010, slotPatterns);
        Assert.Equal(480, slotPatternsWithout);
    }

    // REQ-PROF-084, notes: a 3dn stays a 3dn. The byte-identity of the 101 1.x profiles is the
    // subject of ProfileRoundTripTests; what matters here is that the format does not change.
    [AssetFact]
    public void A_3dn_profile_still_rewrites_as_a_3dn()
    {
        foreach (string file in NeutralAssets.ProfileFiles())
        {
            string name = Path.GetFileName(file);
            byte[] bytes = File.ReadAllBytes(file);
            ProfileDocument document = NeutralAssets.LoadProfile(name);

            Assert.False(document.IsHashProfile);
            Assert.Null(document.Tiles);

            byte[] rewritten = document.Save();
            Assert.False(ProfileFormat.IsHashProfile(rewritten));
            Assert.Equal(bytes, rewritten);
        }
    }

    // REQ-PROF-084: a tile carried by its legacy digest is written back in field 4, and its
    // digest stays in the envelope's Hashes.
    [Fact]
    public void A_tile_carried_by_a_legacy_digest_keeps_its_field()
    {
        byte[] raw = RawTile.FromPlanes(0x1234_5678UL, 0xABCDUL);
        TileId id = TileId.Compute(raw);
        string digest = RawTile.LegacyDigest(raw);

        byte[] bytes = SyntheticHashProfile.Build(
            manifest: [id],
            hashes: [digest],
            cells: [new SyntheticCell(id.ToText()), new SyntheticCell(digest, AsIdentifier: false)]);

        ProfileDocument document = Load(bytes, "synthetic");
        byte[] rewritten = document.Save();

        HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(rewritten);
        Assert.Equal<string>([digest], envelope.Hashes);
        Assert.Equal<string>([id.ToText()], [.. envelope.Ids.Select(static each => each.ToText())]);

        // Resolving changes nothing of the written bytes: a resolved tile is written by its
        // identifier, never by its planes.
        ProfileDocument resolved = Load(bytes, "synthetic");
        resolved.Tiles!.ResolveFromFrameTile(0x1234_5678UL, 0xABCDUL);
        Assert.Equal(0, resolved.Tiles.PendingCount);
        Assert.Equal(rewritten, resolved.Save());
    }

    // REQ-PROF-084: an entry of Hashes no tile refers to any more is dropped.
    [Fact]
    public void An_unreferenced_legacy_digest_is_dropped()
    {
        TileId id = TileId.Compute(new byte[16]);
        string orphan = RawTile.LegacyDigest(RawTile.FromPlanes(0x99UL, 0UL));

        byte[] bytes = SyntheticHashProfile.Build(
            manifest: [id], hashes: [orphan], tileKeys: [id.ToText()]);

        HashProfileEnvelope before = HashEnvelopeCodec.Decode(bytes);
        Assert.Equal<string>([orphan], before.Hashes);

        HashProfileEnvelope after = HashEnvelopeCodec.Decode(Load(bytes, "synthetic").Save());
        Assert.Empty(after.Hashes);
    }

    private static IEnumerable<ProfilePattern> SlotPatterns(GameProfile profile)
    {
        if (profile.Settings is not { } settings)
        {
            yield break;
        }

        foreach (RecordedPatternSlot slot in settings.SavedPatterns.Concat(settings.SavedAnimPatterns))
        {
            if (slot.Pattern is { } pattern)
            {
                yield return pattern;
            }
        }
    }

    private static ProfileDocument Load(byte[] bytes, string name)
    {
        ProfileLoadResult result = ProfileFormat.Load(bytes, sourceName: name);
        Assert.True(result.IsSuccess, $"{name}: {result.Message}");
        return result.Document!;
    }
}
