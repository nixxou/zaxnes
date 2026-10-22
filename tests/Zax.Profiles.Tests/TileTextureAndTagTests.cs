namespace Zax.Profiles.Tests;

/// <summary>
/// Tiles, textures and tags: REQ-PROF-031, REQ-PROF-032, REQ-PROF-035, REQ-PROF-036 and
/// REQ-PROF-037.
/// </summary>
public sealed class TileTextureAndTagTests
{
    [Fact]
    public void REQ_PROF_031_the_flip_bits_mirror_the_displayed_pixels()
    {
        // A single pixel at the top-left corner: bit y*8 + (7 - x) = bit 7 of the low plane.
        const ulong topLeft = 1UL << 7;
        ProfileTile plain = new(topLeft, 0, TileFlip.None);
        ProfileTile mirroredX = new(topLeft, 0, TileFlip.Horizontal);
        ProfileTile mirroredY = new(topLeft, 0, TileFlip.Vertical);
        ProfileTile mirroredBoth = new(topLeft, 0, TileFlip.Horizontal | TileFlip.Vertical);

        Assert.Equal(1, plain.ColorAt(0, 0));
        Assert.Equal(1, mirroredX.ColorAt(7, 0));
        Assert.Equal(0, mirroredX.ColorAt(0, 0));
        Assert.Equal(1, mirroredY.ColorAt(0, 7));
        Assert.Equal(1, mirroredBoth.ColorAt(7, 7));

        Assert.True(mirroredX.FlipX);
        Assert.False(mirroredX.FlipY);
        Assert.True(mirroredY.FlipY);
    }

    [Fact]
    public void REQ_PROF_031_the_colour_of_a_pixel_is_two_times_the_high_bit_plus_the_low_bit()
    {
        const ulong bit = 1UL << 7;
        Assert.Equal(0, new ProfileTile(0, 0, TileFlip.None).ColorAt(0, 0));
        Assert.Equal(1, new ProfileTile(bit, 0, TileFlip.None).ColorAt(0, 0));
        Assert.Equal(2, new ProfileTile(0, bit, TileFlip.None).ColorAt(0, 0));
        Assert.Equal(3, new ProfileTile(bit, bit, TileFlip.None).ColorAt(0, 0));
    }

    [Fact]
    public void REQ_PROF_032_two_tiles_are_the_same_tile_when_planes_and_flip_agree()
    {
        ProfileTile left = new(0x1234, 0x5678, TileFlip.Horizontal);
        ProfileTile right = new(0x1234, 0x5678, TileFlip.Horizontal);
        ProfileTile otherFlip = new(0x1234, 0x5678, TileFlip.Vertical);

        Assert.Equal(left, right);
        Assert.Equal(left.GetHashCode(), right.GetHashCode());
        Assert.NotEqual(left, otherFlip);
        Assert.False(ReferenceEquals(left, right));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_032_cells_carried_by_different_envelopes_may_still_be_the_same_tile()
    {
        // Across a whole profile the same tile value turns up under several keys: value
        // equality and object sharing are two different things.
        GameProfile profile = NeutralAssets.LoadProfile("Super Mario Bros.bytes").Profile;
        List<ProfileTile> objects = [.. profile.DistinctTileObjects];
        HashSet<ProfileTile> values = [.. objects];

        Assert.Equal(783, objects.Count);
        Assert.True(values.Count <= objects.Count);
    }

    [Fact]
    public void REQ_PROF_035_the_null_marker_and_an_explicit_texture_are_told_apart()
    {
        VariantTexture marker = VariantTexture.NullMarker();
        Assert.True(marker.IsNullMarker);
        Assert.True(marker.HasConsistentLength());

        VariantTexture indexed = new(8, 8, new TexturePixels(TextureFormat.Indexed, new int[4]));
        Assert.False(indexed.IsNullMarker);
        Assert.True(indexed.HasConsistentLength());

        VariantTexture tooShort = new(8, 8, new TexturePixels(TextureFormat.Indexed, new int[3]));
        Assert.False(tooShort.HasConsistentLength());

        VariantTexture direct = new(2, 3, new TexturePixels(TextureFormat.Direct, new int[6]));
        Assert.True(direct.HasConsistentLength());
    }

    [Fact]
    public void REQ_PROF_035_an_indexed_payload_needs_one_word_per_sixteen_pixels()
    {
        TexturePixels indexed = new(TextureFormat.Indexed);
        Assert.Equal(1, indexed.ExpectedWordCount(1, 1));
        Assert.Equal(1, indexed.ExpectedWordCount(4, 4));
        Assert.Equal(2, indexed.ExpectedWordCount(4, 5));
        Assert.Equal(4, indexed.ExpectedWordCount(8, 8));

        TexturePixels direct = new(TextureFormat.Direct);
        Assert.Equal(64, direct.ExpectedWordCount(8, 8));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_035_every_explicit_texture_of_the_corpus_has_the_stated_length()
    {
        int explicitTextures = 0;
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadProfile(Path.GetFileName(path)).Profile;
            foreach (ProfilePattern pattern in profile.AllPatterns)
            {
                foreach (PatternVariant variant in pattern.Variants)
                {
                    if (variant.Texture is not { } texture || texture.IsNullMarker)
                    {
                        continue;
                    }

                    explicitTextures++;
                    Assert.True(
                        texture.HasConsistentLength(),
                        $"{Path.GetFileName(path)}: a {texture.Width}x{texture.Height} texture has "
                        + $"{texture.Pixels?.Data.Count} words");
                }
            }
        }

        // 13 539 indexed plus 142 direct textures (§5.5).
        Assert.Equal(13681, explicitTextures);
    }

    [Fact]
    public void REQ_PROF_036_a_tag_string_is_split_trimmed_and_emptied_of_blanks()
    {
        PatternVariant variant = new() { TagString = " char, ,!cast " };

        Assert.Equal(["char", "!cast"], variant.Tags);
        Assert.True(variant.HasTag("char"));
        Assert.True(variant.HasTag("!cast"));
        Assert.False(variant.HasTag("Char"));
    }

    [Fact]
    public void REQ_PROF_036_the_special_variant_tags_are_the_five_of_the_spec()
    {
        Assert.Equal(
            ["!cast", "!receiv", "!recv", "!static", "_gaxis"],
            ProfileTags.SpecialVariantTags);
    }

    [Fact]
    public void REQ_PROF_036_an_absent_tag_string_gives_no_tag()
    {
        Assert.Empty(ProfileTags.Parse(null));
        Assert.Empty(ProfileTags.Parse(string.Empty));
        Assert.Empty(ProfileTags.Parse(" , , "));
    }

    [Theory]
    [InlineData(0, 0)]
    [InlineData(4, 4)]
    [InlineData(-1, 4)]
    [InlineData(-5, 0)]
    [InlineData(-2, 3)]
    public void REQ_PROF_037_a_negative_empty_colour_is_brought_back_into_0_to_4(int stored, int expected)
    {
        PatternVariant variant = new() { StoredEmptyColor = stored };

        Assert.Equal(expected, variant.EmptyColor);
        Assert.Equal(stored, variant.StoredEmptyColor);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_037_no_built_in_profile_stores_a_negative_empty_colour()
    {
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadProfile(Path.GetFileName(path)).Profile;
            foreach (ProfilePattern pattern in profile.AllPatterns)
            {
                foreach (PatternVariant variant in pattern.Variants)
                {
                    Assert.InRange(variant.StoredEmptyColor, 0, 4);
                }
            }
        }
    }

    [Fact]
    public void REQ_PROF_034_the_custom_data_of_a_variant_comes_from_the_last_row_of_the_two_matrices()
    {
        Span<float> speed = stackalloc float[Mat4F.CellCount];
        Span<float> offset = stackalloc float[Mat4F.CellCount];
        for (int i = 0; i < Mat4F.CellCount; i++)
        {
            speed[i] = i;
            offset[i] = 100 + i;
        }

        PatternVariant variant = new()
        {
            DeformSpeed = new Mat4F(speed),
            DeformOffset = new Mat4F(offset),
        };

        // Row 3 of deltaSpeed is CData[0..3], row 3 of deltaOffset is CData[4..7].
        Assert.Equal(12f, variant.CustomData(0));
        Assert.Equal(15f, variant.CustomData(3));
        Assert.Equal(112f, variant.CustomData(4));
        Assert.Equal(115f, variant.CustomData(7));
        Assert.Throws<ArgumentOutOfRangeException>(() => variant.CustomData(8));

        PatternVariant empty = new();
        Assert.Equal(0f, empty.CustomData(0));
    }

    [Fact]
    public void REQ_PROF_002_float_bits_survive_a_round_trip_including_negative_zero()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        ProfilePattern pattern = new();
        pattern.Variants.Add(new PatternVariant
        {
            Alpha = -0f,
            Scale = new Vec3F(float.NaN, -0f, float.NegativeInfinity),
        });
        profile.PermanentPatterns.Add(pattern);

        ProfileLoadResult result = ProfileFormat.Load(ProfileFormat.Save(profile));
        PatternVariant restored = result.Profile!.PermanentPatterns[0].Variants[0];

        Assert.Equal(
            BitConverter.SingleToInt32Bits(-0f),
            BitConverter.SingleToInt32Bits(restored.Alpha));
        Assert.Equal(
            BitConverter.SingleToInt32Bits(float.NaN),
            BitConverter.SingleToInt32Bits(restored.Scale!.Value.X));
        Assert.Equal(
            BitConverter.SingleToInt32Bits(-0f),
            BitConverter.SingleToInt32Bits(restored.Scale.Value.Y));
        Assert.Equal(float.NegativeInfinity, restored.Scale.Value.Z);

        // Bitwise equality: -0 is not 0 here, which is what keeps a rewrite byte-identical.
        Assert.NotEqual(new Vec3F(0f, 0f, 0f), new Vec3F(-0f, 0f, 0f));
    }
}
