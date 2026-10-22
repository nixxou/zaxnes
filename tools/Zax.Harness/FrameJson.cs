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
    public static string Write(IReadOnlyList<TileRecord> records, IReadOnlyList<byte> palette)
    {
        ArgumentNullException.ThrowIfNull(records);
        ArgumentNullException.ThrowIfNull(palette);

        StringBuilder text = new();
        text.Append("{\n  \"palette\": [");
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
