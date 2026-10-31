using System.Globalization;
using Zax.Nes;

namespace Zax.Harness;

/// <summary>
/// The per-frame log of <c>render --rom --frame-log &lt;file.jsonl&gt;</c>: one JSON object per
/// <b>real</b> frame, one object per line (JSON Lines), so that a long replay can be diffed
/// frame by frame without re-running it.
/// </summary>
/// <remarks>
/// <para>
/// The line is appended once the real frame has run and been published, which is why
/// <c>coreFrame</c> is the core's frame counter <i>after</i> the frame: a soft reset (AMD-044)
/// or a rollback (AMD-042) applied after that frame shows up on the <b>next</b> line, where the
/// counter restarts, while <c>frame</c> and <c>input</c> keep the harness's own real-frame
/// numbering throughout.
/// </para>
/// <para>
/// <c>tileHash</c> is the FNV-1a 64 of the tile records of the frame — the very tiles
/// <c>--dump-frame</c> writes, taken field by field in the order that file lists them
/// (<c>low</c>, <c>high</c>, <c>x</c>, <c>y</c>, <c>palette</c>, <c>mirrorH</c>,
/// <c>mirrorV</c>, <c>hidden</c>, <c>sprite</c>), each integer little-endian and each flag as
/// one byte, 0 or 1. It is written as a 16-digit hexadecimal string because a 64-bit integer
/// does not survive a JSON reader that parses numbers as doubles.
/// </para>
/// </remarks>
internal sealed class FrameLog
{
    /// <summary>FNV-1a 64 offset basis.</summary>
    private const ulong OffsetBasis = 14695981039346656037UL;

    /// <summary>FNV-1a 64 prime.</summary>
    private const ulong Prime = 1099511628211UL;

    private readonly List<string> _lines = [];

    /// <summary>The lines written so far, each one a complete JSON object.</summary>
    public IReadOnlyList<string> Lines => _lines;

    /// <summary>
    /// What a frame the core emitted no tile for measures: no tile and the bare offset basis.
    /// It is what a line reports if the video callback did not fire, which only a core holding
    /// no ROM could manage.
    /// </summary>
    public static (int NonEmptyTiles, ulong Hash) Empty => Measure([]);

    /// <summary>
    /// The two per-frame measurements of a log line: how many tile records carry a non-empty
    /// pattern — at least one pixel that is not colour 0 — and the hash of the whole record
    /// list, empty tiles included.
    /// </summary>
    /// <param name="records">The tile records of the frame, as the core emitted them.</param>
    public static (int NonEmptyTiles, ulong Hash) Measure(ReadOnlySpan<TileRecord> records)
    {
        int nonEmpty = 0;
        ulong hash = OffsetBasis;

        for (int n = 0; n < records.Length; n++)
        {
            TileRecord record = records[n];
            if ((record.Plane0 | record.Plane1) != 0)
            {
                nonEmpty++;
            }

            hash = Feed(hash, record.Plane0);
            hash = Feed(hash, record.Plane1);
            hash = Feed(hash, unchecked((ushort)record.X));
            hash = Feed(hash, unchecked((ushort)record.Y));
            hash = Feed(hash, record.Palette);
            hash = Feed(hash, record.MirrorH);
            hash = Feed(hash, record.MirrorV);
            hash = Feed(hash, record.Hidden);

            // '--dump-frame' writes the sprite flag, not the background one it is the negation of.
            hash = Feed(hash, !record.Background);
        }

        return (nonEmpty, hash);
    }

    /// <summary>Appends the line of one real frame.</summary>
    /// <param name="frame">The real frame number, the one <c>--press</c> counts.</param>
    /// <param name="input">The port 1 input word of REQ-CORE-022 the frame ran with.</param>
    /// <param name="coreFrame">The core's frame counter once the frame has run.</param>
    /// <param name="tiles">Non-empty tile records of the frame.</param>
    /// <param name="tileHash">FNV-1a 64 of the frame's tile records.</param>
    public void Add(int frame, ushort input, long coreFrame, int tiles, ulong tileHash)
        => _lines.Add(string.Create(
            CultureInfo.InvariantCulture,
            $"{{ \"frame\": {frame}, \"input\": {input}, \"coreFrame\": {coreFrame}, "
                + $"\"tiles\": {tiles}, \"tileHash\": \"{tileHash:x16}\" }}"));

    /// <summary>
    /// Writes the log to <paramref name="path"/>, one object per line with LF endings and no
    /// trailing comma: the JSON Lines a reader can consume line by line.
    /// </summary>
    public void Write(string path)
        => File.WriteAllText(path, string.Concat(_lines.Select(static line => line + "\n")));

    private static ulong Feed(ulong hash, ulong value)
    {
        for (int shift = 0; shift < 64; shift += 8)
        {
            hash = Feed(hash, (byte)(value >> shift));
        }

        return hash;
    }

    private static ulong Feed(ulong hash, ushort value)
        => Feed(Feed(hash, (byte)value), (byte)(value >> 8));

    private static ulong Feed(ulong hash, bool value) => Feed(hash, value ? (byte)1 : (byte)0);

    private static ulong Feed(ulong hash, byte value) => (hash ^ value) * Prime;
}
