using Zax.Profiles.Import;

namespace Zax.Profiles.Tests;

/// <summary>
/// The LZ4 block decoder of AMD-030 §4.1: round trips through the test encoder, the hand-written
/// sequences the format allows, and the refusals a corrupt block must produce.
/// </summary>
public sealed class Lz4BlockTests
{
    [Theory] // AMD-030 §4.1
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(5)]
    [InlineData(15)]
    [InlineData(16)]
    [InlineData(270)]
    [InlineData(65_536)]
    [InlineData(131_072)]
    public void Every_block_the_encoder_produces_decodes_back(int length)
    {
        byte[] plain = Pattern(length);
        byte[] compressed = Lz4Encoder.Compress(plain);
        byte[] decoded = new byte[plain.Length];

        int produced = Lz4Block.Decode(compressed, decoded);

        Assert.Equal(plain.Length, produced);
        Assert.Equal(plain, decoded);
    }

    [Fact] // AMD-030 §4.1: the whole point of LZ4 is that it is shorter than the plain bytes.
    public void A_repetitive_block_compresses()
    {
        byte[] plain = new byte[100_000];
        Array.Fill(plain, (byte)0x5A);

        byte[] compressed = Lz4Encoder.Compress(plain);

        Assert.True(
            compressed.Length < plain.Length / 10,
            $"{compressed.Length} bytes for {plain.Length}: the block is not compressed.");

        byte[] decoded = new byte[plain.Length];
        Assert.Equal(plain.Length, Lz4Block.Decode(compressed, decoded));
        Assert.Equal(plain, decoded);
    }

    [Fact] // AMD-030 §4.1: a match may overlap itself, which is how a run is encoded.
    public void An_overlapping_match_repeats_the_bytes_it_covers()
    {
        // Token 0x1F: one literal, match length 15 + extra; literal 'A'; offset 1; extra 0 →
        // a match of 19 bytes one byte back, i.e. 'A' twenty times.
        byte[] block = [0x1F, (byte)'A', 0x01, 0x00, 0x00];
        byte[] decoded = new byte[20];

        int produced = Lz4Block.Decode(block, decoded);

        Assert.Equal(20, produced);
        Assert.Equal(new string('A', 20), System.Text.Encoding.ASCII.GetString(decoded));
    }

    [Fact] // AMD-030 §4.1: literal lengths above 14 are continued with 255 bytes.
    public void A_long_literal_run_reads_its_continuation_bytes()
    {
        byte[] literals = Pattern(300);
        List<byte> block = [0xF0, 255, 30]; // 15 + 255 + 30 = 300 literals, no match.
        block.AddRange(literals);
        byte[] decoded = new byte[300];

        int produced = Lz4Block.Decode([.. block], decoded);

        Assert.Equal(300, produced);
        Assert.Equal(literals, decoded);
    }

    [Fact] // AMD-030 §5: a back-reference before the start of the output.
    public void A_match_pointing_before_the_output_is_refused()
    {
        byte[] block = [0x10, (byte)'A', 0x05, 0x00]; // One literal, offset 5: nothing is there.
        byte[] decoded = new byte[16];

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => Lz4Block.Decode(block, decoded));

        Assert.Contains("refers 5 bytes back", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §5: a literal run that runs past the end of the block.
    public void A_truncated_literal_run_is_refused()
    {
        byte[] block = [0x50, (byte)'A', (byte)'B']; // Five literals announced, two given.
        byte[] decoded = new byte[16];

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => Lz4Block.Decode(block, decoded));

        Assert.Contains("ends early", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §5: a continuation that never ends.
    public void A_length_that_runs_off_the_block_is_refused()
    {
        byte[] block = [0xF0, 255, 255]; // Continuation bytes with nothing after them.
        byte[] decoded = new byte[16];

        Assert.Throws<UnityAssetFormatException>(() => Lz4Block.Decode(block, decoded));
    }

    [Fact] // AMD-030 §5: more output than the block table announced.
    public void A_block_that_decodes_to_more_than_announced_is_refused()
    {
        byte[] block = [0x80, 1, 2, 3, 4, 5, 6, 7, 8]; // Eight literals.
        byte[] decoded = new byte[4];

        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => Lz4Block.Decode(block, decoded));

        Assert.Contains("more than the announced 4 bytes", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §5: a match longer than the room left.
    public void A_match_longer_than_the_output_buffer_is_refused()
    {
        byte[] block = [0x1F, (byte)'A', 0x01, 0x00, 0xFF, 0xFF, 0x00];
        byte[] decoded = new byte[32];

        Assert.Throws<UnityAssetFormatException>(() => Lz4Block.Decode(block, decoded));
    }

    private static byte[] Pattern(int length)
    {
        byte[] bytes = new byte[length];
        for (int index = 0; index < length; index++)
        {
            bytes[index] = (byte)(index % 11 == 0 ? index % 251 : 0x7E);
        }

        return bytes;
    }
}
