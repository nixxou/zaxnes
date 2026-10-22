using System.Buffers.Binary;

namespace Zax.Scripting;

/// <summary>
/// The persistent-data block a savestate carries: 64 little-endian IEEE-754 single-precision
/// reals, <c>PData[1]</c> to <c>PData[64]</c> (SPEC-05 REQ-LUA-073, §4.6).
/// </summary>
public static class PersistentData
{
    /// <summary>How many entries are saved, the value of the global <c>pdata_count</c>.</summary>
    public const int EntryCount = 64;

    /// <summary>Size of the block in bytes, always the same.</summary>
    public const int BlockSize = EntryCount * 4;

    /// <summary>
    /// The 1-based <c>PData</c> index <c>GetPData(i)</c> and <c>SetPData(i, v)</c> address:
    /// <c>(i mod 64) + 1</c>, the modulo giving a positive result (REQ-LUA-070).
    /// </summary>
    public static int WrapIndex(int index)
    {
        int wrapped = index % EntryCount;
        if (wrapped < 0)
        {
            wrapped += EntryCount;
        }

        return wrapped + 1;
    }

    /// <summary>Writes the 64 values as the 256-byte block of §4.6.</summary>
    public static byte[] Write(ReadOnlySpan<float> values)
    {
        byte[] block = new byte[BlockSize];
        for (int i = 0; i < EntryCount; i++)
        {
            float value = i < values.Length ? values[i] : 0f;
            BinaryPrimitives.WriteSingleLittleEndian(block.AsSpan(i * 4, 4), value);
        }

        return block;
    }

    /// <summary>
    /// Reads the 64 values from a block. A short or absent block gives zeros for the entries it
    /// does not cover, so a truncated savestate never throws (REQ-ACC-041).
    /// </summary>
    public static float[] Read(ReadOnlySpan<byte> block)
    {
        float[] values = new float[EntryCount];
        for (int i = 0; i < EntryCount; i++)
        {
            int offset = i * 4;
            if (offset + 4 > block.Length)
            {
                break;
            }

            values[i] = BinaryPrimitives.ReadSingleLittleEndian(block.Slice(offset, 4));
        }

        return values;
    }
}
