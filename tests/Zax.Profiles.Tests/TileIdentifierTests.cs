using Zax.Profiles.Hash;

namespace Zax.Profiles.Tests;

/// <summary>
/// The tile identifier, the legacy digest, the mixing prefix and the mirrorings of a raw tile
/// (AMD-062 REQ-PROF-075, 076, 077, 079), on the spec's own vectors.
/// </summary>
public sealed class TileIdentifierTests
{
    // REQ-PROF-075 and REQ-PROF-077: the four vectors, prefix, binary form and text form.
    [Theory]
    [InlineData(
        "00000000000000000000000000000000",
        0x06e28b2dU,
        "2d8be206b548f95b38bf8a3e2d7f50f9",
        "f9507f2d3e8abf385bf948b506e28b2d")]
    [InlineData(
        "ffffffffffffffffffffffffffffffff",
        0x5343027fU,
        "7f024353624f541ff1301725c2461716",
        "161746c2251730f11f544f625343027f")]
    [InlineData(
        "000102030405060708090a0b0c0d0e0f",
        0x870dcae2U,
        "e2ca0d87ba7804d4c5cccdc426124e91",
        "914e1226c4cdccc5d40478ba870dcae2")]
    [InlineData(
        "ffffffffffffffff0000000000000000",
        0x1dbbcceaU,
        "eaccbb1d67be65f246453106c3c0a5d3",
        "d3a5c0c306314546f265be671dbbccea")]
    public void The_identifier_of_a_raw_tile_follows_the_vectors(
        string rawHex, uint prefix, string binaryHex, string text)
    {
        byte[] raw = Convert.FromHexString(rawHex);
        (ulong low, ulong high) = RawTile.ToPlanes(raw);

        Assert.Equal(prefix, TileId.ComputePrefix(low, high));
        Assert.Equal(prefix, TileId.ComputePrefix(raw));

        TileId id = TileId.Compute(raw);
        Assert.Equal(prefix, id.Prefix);
        Assert.Equal(binaryHex, Convert.ToHexStringLower(id.ToBinary()));
        Assert.Equal(text, id.ToText());

        // The binary form and the text form are two readings of the same pair.
        Assert.Equal(id, TileId.FromBinary(id.ToBinary()));
        Assert.True(TileId.TryParseText(text, out TileId parsed));
        Assert.Equal(id, parsed);
    }

    // REQ-PROF-076: the legacy digest of the null tile and of 00 01 … 0f.
    [Theory]
    [InlineData(
        "00000000000000000000000000000000",
        "374708fff7719dd5979ec875d56cd2286f6d3cf7ec317a3b25632aab28ec37bb")]
    [InlineData(
        "000102030405060708090a0b0c0d0e0f",
        "be45cb2605bf36bebde684841a28f0fd43c69850a3dce5fedba69928ee3a8991")]
    public void The_legacy_digest_follows_the_vectors(string rawHex, string digest)
        => Assert.Equal(digest, RawTile.LegacyDigest(Convert.FromHexString(rawHex)));

    // REQ-PROF-079: the three mirrorings of 00 01 … 0f.
    [Theory]
    [InlineData(TileFlip.Horizontal, "008040c020a060e0109050d030b070f0")]
    [InlineData(TileFlip.Vertical, "07060504030201000f0e0d0c0b0a0908")]
    [InlineData((TileFlip)3, "e060a020c0408000f070b030d0509010")]
    public void The_mirrorings_follow_the_vectors(TileFlip flip, string expected)
    {
        byte[] raw = Convert.FromHexString("000102030405060708090a0b0c0d0e0f");
        Assert.Equal(expected, Convert.ToHexStringLower(RawTile.Flipped(raw, flip)));
    }

    [Fact]
    public void A_mirroring_of_zero_leaves_the_tile_alone()
    {
        byte[] raw = Convert.FromHexString("000102030405060708090a0b0c0d0e0f");
        Assert.Equal(raw, RawTile.Flipped(raw, TileFlip.None));
    }

    [Fact]
    public void Every_mirroring_is_its_own_inverse()
    {
        byte[] raw = Convert.FromHexString("123456789abcdef0fedcba9876543210");
        foreach (TileFlip flip in (TileFlip[])
                 [TileFlip.None, TileFlip.Horizontal, TileFlip.Vertical, (TileFlip)3])
        {
            Assert.Equal(raw, RawTile.Flipped(RawTile.Flipped(raw, flip), flip));
        }
    }

    // REQ-PROF-075: the planes read from the raw bytes are the planes in clear of SPEC-03, so a
    // raw tile and the profile tile of the same content show the same pixels.
    [Fact]
    public void The_planes_of_a_raw_tile_are_the_planes_in_clear()
    {
        byte[] raw = Convert.FromHexString("ff00000000000000ff00000000000000");
        (ulong low, ulong high) = RawTile.ToPlanes(raw);
        ProfileTile tile = new(low, high, TileFlip.None);

        for (int x = 0; x < 8; x++)
        {
            Assert.Equal(3, tile.ColorAt(x, 0));
            Assert.Equal(0, tile.ColorAt(x, 1));
        }

        Assert.Equal(raw, RawTile.FromPlanes(low, high));
    }

    [Fact]
    public void The_text_form_only_accepts_thirty_two_lowercase_digits()
    {
        Assert.False(TileId.TryParseText(null, out _));
        Assert.False(TileId.TryParseText(string.Empty, out _));
        Assert.False(TileId.TryParseText(new string('0', 31), out _));
        Assert.False(TileId.TryParseText(new string('0', 33), out _));
        Assert.False(TileId.TryParseText(new string('A', 32), out _));
        Assert.True(TileId.TryParseText(new string('a', 32), out _));
    }
}
