using System.Globalization;
using System.Text;
using Zax.Nes;

namespace Zax.Harness;

/// <summary>
/// Writes one emulated frame in the JSON format <c>render --frames &lt;file.json&gt;</c> reads.
/// </summary>
/// <remarks>
/// It turns a ROM run into a self-contained scene: the same picture can then be rendered again
/// without the ROM, which is what a reproduction case attached to a report has to be.
/// </remarks>
internal static class FrameJson
{
    /// <summary>Serialises <paramref name="records"/> and the 32 palette bytes.</summary>
    /// <param name="records">The tile records of the frame.</param>
    /// <param name="palette">The 32 bytes of palette RAM.</param>
    /// <param name="runAhead">
    /// The run-ahead the frame was produced with (AMD-041). It is written down because a tile
    /// sequence is only comparable with another one at equal N (REQ-ARCH-012).
    /// </param>
    public static string Write(IReadOnlyList<TileRecord> records, IReadOnlyList<byte> palette, int runAhead = 0)
    {
        ArgumentNullException.ThrowIfNull(records);
        ArgumentNullException.ThrowIfNull(palette);

        StringBuilder text = new();
        text.Append(CultureInfo.InvariantCulture, $"{{\n  \"runAhead\": {runAhead},\n  \"palette\": [");
        for (int entry = 0; entry < palette.Count; entry++)
        {
            text.Append(entry == 0 ? string.Empty : ", ");
            text.Append(CultureInfo.InvariantCulture, $"{palette[entry]}");
        }

        text.Append("],\n  \"tiles\": [\n");
        for (int n = 0; n < records.Count; n++)
        {
            TileRecord record = records[n];
            text.Append(CultureInfo.InvariantCulture,
                $"    {{ \"low\": {record.Plane0}, \"high\": {record.Plane1}, "
                    + $"\"x\": {record.X}, \"y\": {record.Y}, \"palette\": {record.Palette}, "
                    + $"\"mirrorH\": {Bool(record.MirrorH)}, \"mirrorV\": {Bool(record.MirrorV)}, "
                    + $"\"hidden\": {Bool(record.Hidden)}, \"sprite\": {Bool(!record.Background)} }}");
            text.Append(n + 1 < records.Count ? ",\n" : "\n");
        }

        text.Append("  ]\n}\n");
        return text.ToString();
    }

    private static string Bool(bool value) => value ? "true" : "false";
}
