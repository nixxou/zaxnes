namespace Zax.Profiles.Import;

/// <summary>
/// Decoder of the LZ4 <b>block</b> format, the one Unity asset bundles use for their
/// compressed blocks (AMD-030 §4.1: compression codes 2 and 3).
/// </summary>
/// <remarks>
/// <para>
/// The block format is a sequence of « sequences ». Each one opens with a token byte whose high
/// nibble is a literal length and whose low nibble is a match length minus four; a nibble of 15
/// is followed by as many 255 bytes as needed plus a final byte below 255, all added together.
/// The literals are copied straight out of the input, then a two-byte little-endian offset says
/// how far back in the <b>output</b> the match starts. The last sequence of a block stops after
/// its literals and carries no match.
/// </para>
/// <para>
/// LZ4HC (code 3) is only a different <i>encoder</i>: it emits the same block format, so this
/// decoder reads both, which is what AMD-030 §4.1 states. Nothing here allocates, and every
/// length is checked against both buffers: a corrupt block raises
/// <see cref="UnityAssetFormatException"/> instead of reading or writing out of bounds
/// (AMD-030 §5, robustness).
/// </para>
/// </remarks>
public static class Lz4Block
{
    /// <summary>The minimum match length the format encodes, which the token's low nibble omits.</summary>
    private const int MinimumMatch = 4;

    /// <summary>
    /// Decodes one LZ4 block.
    /// </summary>
    /// <param name="source">The compressed block, whole.</param>
    /// <param name="destination">
    /// Where the decoded bytes go. Its length is the expected decoded size, which the caller
    /// knows from the bundle's block table.
    /// </param>
    /// <returns>How many bytes were written, which a well-formed block makes equal to
    /// <paramref name="destination"/>'s length.</returns>
    /// <exception cref="UnityAssetFormatException">
    /// The block is truncated, carries a back-reference before the start of the output, or asks
    /// for more room than <paramref name="destination"/> has.
    /// </exception>
    public static int Decode(ReadOnlySpan<byte> source, Span<byte> destination)
    {
        int input = 0;
        int output = 0;

        while (input < source.Length)
        {
            int token = source[input++];

            int literals = token >> 4;
            if (literals == 15)
            {
                literals += ReadExtraLength(source, ref input);
            }

            if (literals > 0)
            {
                if (input + literals > source.Length)
                {
                    throw Truncated("literals", literals, source.Length - input);
                }

                if (output + literals > destination.Length)
                {
                    throw TooLarge(output + literals, destination.Length);
                }

                source.Slice(input, literals).CopyTo(destination[output..]);
                input += literals;
                output += literals;
            }

            // The last sequence of a block ends on its literals: no offset, no match.
            if (input == source.Length)
            {
                break;
            }

            if (input + 2 > source.Length)
            {
                throw Truncated("the match offset", 2, source.Length - input);
            }

            int offset = source[input] | (source[input + 1] << 8);
            input += 2;

            if (offset == 0 || offset > output)
            {
                throw new UnityAssetFormatException(
                    $"the LZ4 block refers {offset} bytes back while only {output} bytes have been decoded.");
            }

            int match = token & 0x0F;
            if (match == 15)
            {
                match += ReadExtraLength(source, ref input);
            }

            match += MinimumMatch;
            if (output + match > destination.Length)
            {
                throw TooLarge(output + match, destination.Length);
            }

            // Byte by byte on purpose: a match may overlap itself, which is how the format
            // encodes runs (offset 1 repeats one byte).
            int from = output - offset;
            for (int index = 0; index < match; index++)
            {
                destination[output + index] = destination[from + index];
            }

            output += match;
        }

        return output;
    }

    private static int ReadExtraLength(ReadOnlySpan<byte> source, ref int input)
    {
        int extra = 0;
        while (true)
        {
            if (input >= source.Length)
            {
                throw Truncated("a length byte", 1, 0);
            }

            byte value = source[input++];
            extra += value;
            if (value != 0xFF)
            {
                return extra;
            }
        }
    }

    private static UnityAssetFormatException Truncated(string what, int wanted, int available)
        => new($"the LZ4 block ends early: {what} needs {wanted} byte(s), {available} left.");

    private static UnityAssetFormatException TooLarge(int wanted, int capacity)
        => new($"the LZ4 block decodes to more than the announced {capacity} bytes (at least {wanted}).");
}
