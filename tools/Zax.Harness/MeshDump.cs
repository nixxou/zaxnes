using System.Globalization;
using System.Text;
using Zax.Engine3D;

namespace Zax.Harness;

/// <summary>
/// The <c>--dump-meshes</c> export of the <c>render</c> command: one entry per drawn tile
/// instance of the captured frame — every tile of every active shape the renderer draws, the
/// HUD included and marked (SPEC-04B REQ-E3D-641, SPEC-01 REQ-ARCH-031).
/// </summary>
/// <remarks>
/// <para>
/// It is the per-tile counterpart of <c>--dump-frame</c>: that one writes what the core emitted,
/// this one what the conversion made of it, tile by tile, so that a geometry question can be
/// answered from the capture alone. The tile position is the core's, in the very convention
/// <c>--dump-frame</c> uses: the screen pixel of the tile's top-left corner.
/// </para>
/// <para>
/// The local frame is the one of REQ-E3D-504 — the tile's own, <b>centred</b> on the tile: the
/// voxel corner of integer indices <c>(i, j, k)</c> sits at <c>(i − 4, j − 4, k − D/2)</c>. A
/// full 8 × 8 tile therefore spans <c>[−4, 4]</c> on x and y, eight units across, and
/// <c>[−D/2, D/2]</c> on z. The world bounds are the same box carried through
/// <see cref="DrawTile.Transform"/>, the complete object transform of REQ-E3D-406: the scene
/// transformation and the space bending of SPEC-06 come after it and are not included. Q-081.
/// </para>
/// </remarks>
internal static class MeshDump
{
    /// <summary>UTF-8 without a byte order mark, so the dump stays plain text.</summary>
    private static readonly UTF8Encoding Utf8 = new(encoderShouldEmitUTF8Identifier: false);

    /// <summary>Writes the dump of <paramref name="list"/> to <paramref name="path"/>.</summary>
    /// <param name="path">The file to write.</param>
    /// <param name="list">The draw list of the captured frame.</param>
    /// <param name="global">The global mesh algorithm, which a tile may override (REQ-E3D-500).</param>
    /// <param name="scene">The name of the capture, written down with the tiles.</param>
    public static void WriteFile(string path, FrameDrawList list, RenderAlgorithm global, string scene)
        => File.WriteAllText(path, Write(list, global, scene), Utf8);

    /// <summary>Builds the dump of <paramref name="list"/>.</summary>
    /// <param name="list">The draw list of the captured frame.</param>
    /// <param name="global">The global mesh algorithm, which a tile may override (REQ-E3D-500).</param>
    /// <param name="scene">The name of the capture, written down with the tiles.</param>
    public static string Write(FrameDrawList list, RenderAlgorithm global, string scene)
    {
        ArgumentNullException.ThrowIfNull(list);

        StringBuilder text = new();
        text.Append("{\n");
        text.Append(CultureInfo.InvariantCulture, $"  \"scene\": \"{Escape(scene)}\",\n");
        text.Append(CultureInfo.InvariantCulture, $"  \"frameCounter\": {list.FrameCounter},\n");
        text.Append(CultureInfo.InvariantCulture, $"  \"time\": {list.Time:0.######},\n");
        text.Append(CultureInfo.InvariantCulture, $"  \"algorithm\": \"{global}\",\n");
        text.Append(CultureInfo.InvariantCulture, $"  \"tileCount\": {list.Tiles.Count},\n");
        text.Append("  \"tiles\": [\n");

        for (int n = 0; n < list.Tiles.Count; n++)
        {
            DrawTile tile = list.Tiles[n];

            // A tile with no geometry (sizeZ = 0, REQ-E3D-420) carries the empty mesh, whose
            // bounds are the single point (0, 0, 0): it is drawn, so it is listed, with zero
            // vertices and a degenerate box at the tile's own position.
            Vec3 localMin = tile.Mesh?.BoundsMin ?? Vec3.Zero;
            Vec3 localMax = tile.Mesh?.BoundsMax ?? Vec3.Zero;
            (Vec3 worldMin, Vec3 worldMax) = WorldBounds(tile.Transform, localMin, localMax);

            text.Append("    { ");
            text.Append(CultureInfo.InvariantCulture, $"\"tile\": [{tile.Tile.X}, {tile.Tile.Y}], ");
            text.Append(CultureInfo.InvariantCulture, $"\"sprite\": {Bool(tile.Sprite)}, ");

            // REQ-SCN-044: the HUD is the interface material, the one the renderer draws last
            // and lights in the camera frame. It is marked, never left out.
            text.Append(CultureInfo.InvariantCulture,
                $"\"hud\": {Bool(tile.Material == MaterialClass.Interface)}, ");
            text.Append(CultureInfo.InvariantCulture, $"\"vertices\": {tile.Mesh?.VertexCount ?? 0}, ");
            text.Append(CultureInfo.InvariantCulture, $"\"triangles\": {tile.Mesh?.TriangleCount ?? 0}, ");
            Vector(text, "localMin", localMin);
            Vector(text, "localMax", localMax);
            Vector(text, "worldMin", worldMin);
            Vector(text, "worldMax", worldMax);
            text.Append(CultureInfo.InvariantCulture, $"\"depth\": {tile.Instance.Depth}, ");
            text.Append(CultureInfo.InvariantCulture,
                $"\"mesh\": \"{tile.Instance.EffectiveAlgorithm(global)}\", ");
            text.Append(CultureInfo.InvariantCulture, $"\"tags\": \"{Escape(tile.Instance.TagString)}\" }}");
            text.Append(n + 1 < list.Tiles.Count ? ",\n" : "\n");
        }

        text.Append("  ]\n");
        text.Append("}\n");
        return text.ToString();
    }

    /// <summary>
    /// Where the dump of <paramref name="scene"/> goes: <paramref name="path"/> as given when
    /// the command captures one scene, and the same path with the scene name inserted before the
    /// extension when it captures several, so that no capture overwrites another's dump.
    /// </summary>
    /// <param name="path">The path the command line gave.</param>
    /// <param name="scene">The name of the capture.</param>
    /// <param name="single">True when the command captures a single scene.</param>
    public static string PathFor(string path, string scene, bool single)
    {
        ArgumentNullException.ThrowIfNull(path);

        if (single)
        {
            return path;
        }

        string directory = Path.GetDirectoryName(path) ?? string.Empty;
        string stem = Path.GetFileNameWithoutExtension(path);
        string extension = Path.GetExtension(path);
        return Path.Combine(directory, $"{stem}.{scene}{extension}");
    }

    /// <summary>
    /// The axis-aligned box the local box <paramref name="min"/>…<paramref name="max"/> occupies
    /// once <paramref name="transform"/> has been applied: the hull of its eight transformed
    /// corners, which is also how the renderer measures an object's extent (AMD-012).
    /// </summary>
    private static (Vec3 Min, Vec3 Max) WorldBounds(Mat4 transform, Vec3 min, Vec3 max)
    {
        Vec3 low = default;
        Vec3 high = default;
        for (int corner = 0; corner < 8; corner++)
        {
            Vec3 point = transform.TransformPoint(new Vec3(
                (corner & 1) == 0 ? min.X : max.X,
                (corner & 2) == 0 ? min.Y : max.Y,
                (corner & 4) == 0 ? min.Z : max.Z));

            if (corner == 0)
            {
                low = point;
                high = point;
                continue;
            }

            low = new Vec3(
                MathF.Min(low.X, point.X),
                MathF.Min(low.Y, point.Y),
                MathF.Min(low.Z, point.Z));
            high = new Vec3(
                MathF.Max(high.X, point.X),
                MathF.Max(high.Y, point.Y),
                MathF.Max(high.Z, point.Z));
        }

        return (low, high);
    }

    private static void Vector(StringBuilder text, string name, Vec3 value)
        => text.Append(CultureInfo.InvariantCulture,
            $"\"{name}\": [{value.X:0.######}, {value.Y:0.######}, {value.Z:0.######}], ");

    private static string Bool(bool value) => value ? "true" : "false";

    private static string Escape(string value)
        => value.Replace("\\", "\\\\", StringComparison.Ordinal)
            .Replace("\"", "\\\"", StringComparison.Ordinal);
}
