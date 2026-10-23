namespace Zax.Profiles.Tests;

/// <summary>
/// A small LZ4 <b>block</b> encoder, written for the tests alone: the production code only ever
/// reads LZ4 (AMD-030 §4.1), so building a compressed bundle needs an encoder that exists
/// nowhere else.
/// </summary>
/// <remarks>
/// Greedy, one four-byte key in a table, no second chance: the point is to emit real sequences —
/// literals, back-references, lengths above the four-bit nibble — so that
/// <see cref="Zax.Profiles.Import.Lz4Block"/> is exercised on more than a stream of literals. The
/// format rules an encoder must respect are kept: the last five bytes of a block are literals and
/// no match starts in them.
/// </remarks>
internal static class Lz4Encoder
{
    private const int MinimumMatch = 4;
    private const int LastLiterals = 5;
    private const int MaximumOffset = 0xFFFF;

    /// <summary>Compresses a block.</summary>
    public static byte[] Compress(ReadOnlySpan<byte> input)
    {
        List<byte> output = [];
        Dictionary<uint, int> table = [];

        int anchor = 0;
        int position = 0;
        int limit = input.Length - LastLiterals;

        while (position < limit)
        {
            uint key = KeyAt(input, position);
            if (table.TryGetValue(key, out int candidate)
                && position - candidate is > 0 and <= MaximumOffset
                && Matches(input, candidate, position, MinimumMatch))
            {
                int length = MinimumMatch;
                while (position + length < limit
                    && input[candidate + length] == input[position + length])
                {
                    length++;
                }

                EmitSequence(output, input[anchor..position], position - candidate, length);
                table[key] = position;
                position += length;
                anchor = position;
                continue;
            }

            table[key] = position;
            position++;
        }

        EmitLastLiterals(output, input[anchor..]);
        return [.. output];
    }

    private static uint KeyAt(ReadOnlySpan<byte> input, int position)
        => (uint)(input[position]
            | (input[position + 1] << 8)
            | (input[position + 2] << 16)
            | (input[position + 3] << 24));

    private static bool Matches(ReadOnlySpan<byte> input, int left, int right, int length)
    {
        for (int index = 0; index < length; index++)
        {
            if (input[left + index] != input[right + index])
            {
                return false;
            }
        }

        return true;
    }

    private static void EmitSequence(
        List<byte> output,
        ReadOnlySpan<byte> literals,
        int offset,
        int matchLength)
    {
        int match = matchLength - MinimumMatch;
        output.Add((byte)((Math.Min(literals.Length, 15) << 4) | Math.Min(match, 15)));

        if (literals.Length >= 15)
        {
            WriteExtra(output, literals.Length - 15);
        }

        foreach (byte value in literals)
        {
            output.Add(value);
        }

        output.Add((byte)(offset & 0xFF));
        output.Add((byte)((offset >> 8) & 0xFF));

        if (match >= 15)
        {
            WriteExtra(output, match - 15);
        }
    }

    private static void EmitLastLiterals(List<byte> output, ReadOnlySpan<byte> literals)
    {
        output.Add((byte)(Math.Min(literals.Length, 15) << 4));
        if (literals.Length >= 15)
        {
            WriteExtra(output, literals.Length - 15);
        }

        foreach (byte value in literals)
        {
            output.Add(value);
        }
    }

    private static void WriteExtra(List<byte> output, int remainder)
    {
        while (remainder >= 255)
        {
            output.Add(255);
            remainder -= 255;
        }

        output.Add((byte)remainder);
    }
}
