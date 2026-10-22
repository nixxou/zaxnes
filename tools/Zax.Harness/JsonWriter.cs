using System.Globalization;
using System.Text;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Harness;

/// <summary>
/// Writes the instance list of SPEC-04B REQ-E3D-641 and a build summary as JSON, next to the
/// OBJ files (REQ-ARCH-031).
/// </summary>
internal static class JsonWriter
{
    /// <summary>The tiles to draw of one frame, with every field of REQ-E3D-641.</summary>
    public static string Frame(FrameDrawList list, int number)
    {
        ArgumentNullException.ThrowIfNull(list);

        StringBuilder text = new();
        text.Append("{\n");
        Field(text, 1, "frame", number);
        Field(text, 1, "frameCounter", list.FrameCounter);
        Field(text, 1, "time", list.Time);
        Field(text, 1, "tileCount", list.Tiles.Count);

        text.Append("  \"paletteRgb\": [");
        for (int entry = 0; entry < list.Palette.Length; entry++)
        {
            text.Append(entry == 0 ? string.Empty : ", ");
            text.Append(CultureInfo.InvariantCulture, $"\"{list.Palette[entry]}\"");
        }

        text.Append("],\n");
        text.Append("  \"tiles\": [\n");

        for (int n = 0; n < list.Tiles.Count; n++)
        {
            DrawTile tile = list.Tiles[n];
            float[] matrix = new float[16];
            tile.Transform.CopyTo(matrix);

            text.Append("    {\n");
            Field(text, 3, "index", n);
            Field(text, 3, "tileX", tile.Tile.X);
            Field(text, 3, "tileY", tile.Tile.Y);
            Field(text, 3, "imageId", tile.Tile.Image.Id);

            // The tile identity of SPEC-04 §2, so that a synthetic layout can be replayed as a
            // `render --frames <file.json>` frame (the two planes are what that format takes).
            TileImageKey key = tile.Tile.Image.Key;
            Field(text, 3, "low", key.Low);
            Field(text, 3, "high", key.High);
            Field(text, 3, "mirrorH", key.FlipX);
            Field(text, 3, "mirrorV", key.FlipY);
            Field(text, 3, "geo", tile.Instance.Geometry.ToString());
            Field(text, 3, "sizeZ", tile.Instance.Depth);
            Field(text, 3, "fullWarp", tile.Instance.FullWarp);
            Field(text, 3, "emptyColor", tile.Instance.EmptyColor);
            Field(text, 3, "meshTriangles", tile.Mesh?.TriangleCount ?? 0);
            Field(text, 3, "meshVertices", tile.Mesh?.VertexCount ?? 0);
            Field(text, 3, "container", tile.Container.ToString());
            Field(text, 3, "material", tile.Material.ToString());
            Field(text, 3, "alpha", tile.Alpha);
            Field(text, 3, "textureFormat", tile.TextureFormat.ToString());
            Field(text, 3, "textureWidth", tile.Texture?.Width ?? 0);
            Field(text, 3, "textureHeight", tile.Texture?.Height ?? 0);
            Field(text, 3, "paletteTable", tile.PaletteTable);
            Field(text, 3, "texOffset", tile.TexOffset);
            Field(text, 3, "texTilting", tile.TexTilting);
            Field(text, 3, "castShadow", tile.CastShadow);
            Field(text, 3, "receiveShadow", tile.ReceiveShadow);
            Field(text, 3, "sprite", tile.Sprite);
            Field(text, 3, "behindBackground", tile.BehindBackground);
            Field(text, 3, "anchor", tile.Anchor);
            Field(text, 3, "tileCenter", tile.TileCenter);

            text.Append("      \"transform\": [");
            for (int cell = 0; cell < matrix.Length; cell++)
            {
                text.Append(cell == 0 ? string.Empty : ", ");
                text.Append(CultureInfo.InvariantCulture, $"{matrix[cell]:0.######}");
            }

            text.Append(']');

            if (tile.Deform is { } deform)
            {
                text.Append(",\n      \"deform\": {\n");
                Field(text, 4, "amplitude", deform.Amplitude);
                Field(text, 4, "size", deform.Size);
                Field(text, 4, "tileRelativeToPivot", deform.TileRelativeToPivot);
                Matrix(text, 4, "speed", deform.Speed);
                Matrix(text, 4, "phase", deform.Phase, last: true);
                text.Append("      }");
            }

            text.Append('\n');
            text.Append(n + 1 < list.Tiles.Count ? "    },\n" : "    }\n");
        }

        text.Append("  ]\n");
        text.Append("}\n");
        return text.ToString();
    }

    /// <summary>What was loaded and built, with the per-geometry counts of annex A.</summary>
    public static string Summary(
        string profileName,
        GameProfile profile,
        ConversionEngine engine,
        GeometryBuilder geometry,
        MeshExport.Options options,
        double loadMilliseconds,
        int frames,
        int tiles)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ArgumentNullException.ThrowIfNull(engine);
        ArgumentNullException.ThrowIfNull(geometry);
        ArgumentNullException.ThrowIfNull(options);

        Dictionary<Geo3DType, int> objectsByGeometry = [];
        long triangles = 0;
        long vertices = 0;
        int objects = 0;
        int meshes = 0;

        foreach (EnginePattern pattern in engine.Patterns.Permanent.Concat(engine.Patterns.Temporary))
        {
            foreach (PatternInstance instance in pattern.Instances)
            {
                ObjectGeometry? built = geometry.Of(instance);
                if (built is null)
                {
                    continue;
                }

                objects++;
                objectsByGeometry[instance.Geometry] = objectsByGeometry.GetValueOrDefault(instance.Geometry) + 1;
                foreach (TileMesh mesh in built.Meshes)
                {
                    meshes++;
                    triangles += mesh.TriangleCount;
                    vertices += mesh.VertexCount;
                }
            }
        }

        StringBuilder text = new();
        text.Append("{\n");
        Field(text, 1, "profile", profileName);
        Field(text, 1, "formatVersion", profile.VersionText ?? string.Empty);
        Field(text, 1, "isV2Format", profile.IsV2Format);
        Field(text, 1, "algorithm", options.Algorithm.ToString());
        Field(text, 1, "handedness", options.RightHanded ? "right" : "left");
        Field(text, 1, "patterns", engine.Patterns.Permanent.Count + engine.Patterns.Temporary.Count);
        Field(text, 1, "objects", objects);
        Field(text, 1, "tileMeshes", meshes);
        Field(text, 1, "triangles", triangles);
        Field(text, 1, "vertices", vertices);
        Field(text, 1, "buildMilliseconds", (float)loadMilliseconds);
        Field(text, 1, "loadProgress", engine.LoadProgress);
        Field(text, 1, "framesExported", frames);
        Field(text, 1, "tilesDrawn", tiles);

        text.Append("  \"objectsByGeometry\": {\n");
        Geo3DType[] kinds = Enum.GetValues<Geo3DType>();
        for (int n = 0; n < kinds.Length; n++)
        {
            text.Append(CultureInfo.InvariantCulture,
                $"    \"{kinds[n]}\": {objectsByGeometry.GetValueOrDefault(kinds[n])}");
            text.Append(n + 1 < kinds.Length ? ",\n" : "\n");
        }

        text.Append("  }\n");
        text.Append("}\n");
        return text.ToString();
    }

    private static void Field(StringBuilder text, int depth, string name, string value)
        => text.Append(CultureInfo.InvariantCulture, $"{Indent(depth)}\"{name}\": \"{Escape(value)}\",\n");

    private static void Field(StringBuilder text, int depth, string name, int value)
        => text.Append(CultureInfo.InvariantCulture, $"{Indent(depth)}\"{name}\": {value},\n");

    private static void Field(StringBuilder text, int depth, string name, long value)
        => text.Append(CultureInfo.InvariantCulture, $"{Indent(depth)}\"{name}\": {value},\n");

    private static void Field(StringBuilder text, int depth, string name, ulong value)
        => text.Append(CultureInfo.InvariantCulture, $"{Indent(depth)}\"{name}\": {value},\n");

    private static void Field(StringBuilder text, int depth, string name, bool value)
        => text.Append(CultureInfo.InvariantCulture, $"{Indent(depth)}\"{name}\": {(value ? "true" : "false")},\n");

    private static void Field(StringBuilder text, int depth, string name, float value)
        => text.Append(CultureInfo.InvariantCulture, $"{Indent(depth)}\"{name}\": {value:0.######},\n");

    private static void Field(StringBuilder text, int depth, string name, Vec3 value)
        => text.Append(CultureInfo.InvariantCulture,
            $"{Indent(depth)}\"{name}\": [{value.X:0.######}, {value.Y:0.######}, {value.Z:0.######}],\n");

    private static void Field(StringBuilder text, int depth, string name, Vec2 value)
        => text.Append(CultureInfo.InvariantCulture,
            $"{Indent(depth)}\"{name}\": [{value.X:0.######}, {value.Y:0.######}],\n");

    private static void Matrix(StringBuilder text, int depth, string name, Mat4 value, bool last = false)
    {
        text.Append(CultureInfo.InvariantCulture, $"{Indent(depth)}\"{name}\": [");
        for (int row = 0; row < 4; row++)
        {
            for (int column = 0; column < 4; column++)
            {
                text.Append(row == 0 && column == 0 ? string.Empty : ", ");
                text.Append(CultureInfo.InvariantCulture, $"{value[row, column]:0.######}");
            }
        }

        text.Append(last ? "]\n" : "],\n");
    }

    private static string Indent(int depth) => new(' ', 2 * depth);

    private static string Escape(string value)
        => value.Replace("\\", "\\\\", StringComparison.Ordinal)
            .Replace("\"", "\\\"", StringComparison.Ordinal);
}
