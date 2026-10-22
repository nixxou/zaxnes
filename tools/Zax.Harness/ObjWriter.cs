using System.Globalization;
using System.Text;
using Zax.Engine3D;

namespace Zax.Harness;

/// <summary>
/// Writes the abstract meshes of SPEC-04B REQ-E3D-640 as Wavefront OBJ, for review and for the
/// round-1 verification (REQ-ARCH-031).
/// </summary>
/// <remarks>
/// The scene frame of REQ-E3D-400 is left handed. In right-handed mode the writer negates z
/// and swaps two indices of every triangle, which is the conversion the note of REQ-E3D-400
/// describes; the file then opens the right way round in any viewer.
/// </remarks>
internal static class ObjWriter
{
    /// <summary>
    /// The tiles to draw of one frame, each placed in the scene by its own transform
    /// (REQ-E3D-406, REQ-E3D-641). The sprite depth nudge is not applied: that is the
    /// renderer's (REQ-SCN-037, DEC-R2-T4).
    /// </summary>
    public static string Scene(FrameDrawList list, bool rightHanded)
    {
        ArgumentNullException.ThrowIfNull(list);

        StringBuilder text = new();
        Header(text, rightHanded);
        text.Append(CultureInfo.InvariantCulture, $"# frame {list.FrameCounter}, time {list.Time:F4} s\n");
        text.Append(CultureInfo.InvariantCulture, $"# {list.Tiles.Count} tiles to draw\n");

        int baseIndex = 1;
        for (int n = 0; n < list.Tiles.Count; n++)
        {
            DrawTile tile = list.Tiles[n];
            if (tile.Mesh is not { IsEmpty: false } mesh)
            {
                continue;
            }

            text.Append(CultureInfo.InvariantCulture,
                $"g tile{n:00000}_{tile.Material}_{(tile.Sprite ? "sprite" : "bg")}\n");

            foreach (MeshVertex vertex in mesh.Vertices)
            {
                Vec3 position = tile.Transform.TransformPoint(vertex.Position);
                Vec3 normal = Direction(tile.Transform, vertex.Normal);
                Vertex(text, position, rightHanded);
                Normal(text, normal, rightHanded);
                Texture(text, vertex.Uv, tile.Texture);
            }

            Faces(text, mesh, baseIndex, rightHanded);
            baseIndex += mesh.VertexCount;
        }

        return text.ToString();
    }

    /// <summary>
    /// Every 3D object of the registry, its tile meshes in their local frame (REQ-E3D-504,
    /// REQ-E3D-522), one group per tile image.
    /// </summary>
    public static string Objects(ConversionEngine engine, GeometryBuilder geometry, bool rightHanded)
    {
        ArgumentNullException.ThrowIfNull(engine);
        ArgumentNullException.ThrowIfNull(geometry);

        StringBuilder text = new();
        Header(text, rightHanded);

        int baseIndex = 1;
        int pattern = 0;
        foreach (EnginePattern current in engine.Patterns.Permanent)
        {
            for (int instance = 0; instance < current.Instances.Count; instance++)
            {
                ObjectGeometry? built = geometry.Of(current.Instances[instance]);
                if (built is null)
                {
                    continue;
                }

                for (int image = 0; image < built.Meshes.Length; image++)
                {
                    TileMesh mesh = built.Meshes[image];
                    if (mesh.IsEmpty)
                    {
                        continue;
                    }

                    text.Append(CultureInfo.InvariantCulture,
                        $"g p{pattern:00000}_o{instance}_t{image}_{current.Instances[instance].Geometry}\n");

                    foreach (MeshVertex vertex in mesh.Vertices)
                    {
                        Vertex(text, vertex.Position, rightHanded);
                        Normal(text, vertex.Normal, rightHanded);
                        Texture(text, vertex.Uv, built.Texture);
                    }

                    Faces(text, mesh, baseIndex, rightHanded);
                    baseIndex += mesh.VertexCount;
                }
            }

            pattern++;
        }

        return text.ToString();
    }

    private static void Header(StringBuilder text, bool rightHanded)
    {
        text.Append("# Zaxnes harness mesh export (SPEC-04B REQ-E3D-640)\n");
        text.Append(rightHanded
            ? "# right-handed: z negated and winding swapped from the scene frame\n"
            : "# left-handed scene frame of REQ-E3D-400, written as is\n");
        text.Append("# one scene unit = one NES pixel\n");
    }

    private static void Vertex(StringBuilder text, Vec3 position, bool rightHanded)
    {
        float z = rightHanded ? -position.Z : position.Z;
        text.Append(CultureInfo.InvariantCulture, $"v {position.X:0.####} {position.Y:0.####} {z:0.####}\n");
    }

    private static void Normal(StringBuilder text, Vec3 normal, bool rightHanded)
    {
        float z = rightHanded ? -normal.Z : normal.Z;
        text.Append(CultureInfo.InvariantCulture, $"vn {normal.X:0.####} {normal.Y:0.####} {z:0.####}\n");
    }

    /// <summary>
    /// Texture coordinates, normalised into <c>[0, 1]</c> by the texture size so that a viewer
    /// can show the texture; the engine's own unit is the texel (REQ-E3D-506).
    /// </summary>
    private static void Texture(StringBuilder text, Vec2 uv, ObjectTexture? texture)
    {
        float width = texture is null ? 1f : texture.Width;
        float height = texture is null ? 1f : texture.Height;
        text.Append(CultureInfo.InvariantCulture, $"vt {uv.X / width:0.######} {1f - (uv.Y / height):0.######}\n");
    }

    private static void Faces(StringBuilder text, TileMesh mesh, int baseIndex, bool rightHanded)
    {
        ReadOnlySpan<int> triangles = mesh.Triangles;
        for (int n = 0; n < triangles.Length; n += 3)
        {
            int a = baseIndex + triangles[n];
            int b = baseIndex + triangles[n + 1];
            int c = baseIndex + triangles[n + 2];
            if (rightHanded)
            {
                (b, c) = (c, b);
            }

            text.Append(CultureInfo.InvariantCulture, $"f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n");
        }
    }

    /// <summary>Transforms a direction: the transform without its translation.</summary>
    private static Vec3 Direction(Mat4 transform, Vec3 direction)
    {
        Vec3 origin = transform.TransformPoint(Vec3.Zero);
        Vec3 moved = transform.TransformPoint(direction) - origin;
        float length = MathF.Sqrt(moved.SquaredLength);
        return length > 0f ? moved * (1f / length) : direction;
    }
}
