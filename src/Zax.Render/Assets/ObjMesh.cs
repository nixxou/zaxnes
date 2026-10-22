using System.Globalization;
using Zax.Engine3D;

namespace Zax.Render.Assets;

/// <summary>
/// A reader for the Wavefront OBJ files of <c>assets-neutral/meshes</c>, the mesh half of
/// <see cref="ISceneAssets"/>.
/// </summary>
/// <remarks>
/// <para>
/// Only what those files use is read: <c>v</c>, <c>vt</c>, <c>vn</c>, <c>f</c> and <c>g</c>.
/// Faces are triangulated as a fan, which is exact for the triangles and quads the exported
/// files contain. A missing normal is replaced by the face normal, a missing texture
/// coordinate by <c>(0, 0)</c>.
/// </para>
/// <para>
/// A <c>g</c> line starts a new group, which is how an export writes the submeshes of one
/// model. <see cref="Load"/> keeps the first group that has faces and drops the rest (Q-023);
/// <see cref="ParseGroups"/> returns all of them.
/// </para>
/// </remarks>
public static class ObjMesh
{
    /// <summary>Reads the OBJ file <paramref name="path"/>, keeping its first non-empty group.</summary>
    public static TileMesh Load(string path)
    {
        // Q-023: one material per model, so one group per model.
        return Parse(File.ReadAllText(path));
    }

    /// <summary>Reads OBJ text, keeping its first non-empty group.</summary>
    public static TileMesh Parse(string text)
    {
        List<(string Name, TileMesh Mesh)> groups = ParseGroups(text);
        return groups.Count > 0 ? groups[0].Mesh : TileMesh.Empty;
    }

    /// <summary>Reads OBJ text as one mesh per non-empty group, in file order.</summary>
    public static List<(string Name, TileMesh Mesh)> ParseGroups(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        List<Vec3> positions = [];
        List<Vec2> uvs = [];
        List<Vec3> normals = [];
        List<(string Name, TileMesh Mesh)> groups = [];

        string name = string.Empty;
        List<MeshVertex> vertices = [];
        List<int> triangles = [];
        Dictionary<(int P, int T, int N), int> seen = [];

        void Flush()
        {
            if (triangles.Count > 0)
            {
                groups.Add((name, new TileMesh(vertices, triangles)));
            }

            vertices = [];
            triangles = [];
            seen = [];
        }

        foreach (ReadOnlySpan<char> rawLine in text.AsSpan().EnumerateLines())
        {
            ReadOnlySpan<char> line = rawLine.Trim();
            if (line.IsEmpty || line[0] == '#')
            {
                continue;
            }

            if (line.StartsWith("v "))
            {
                positions.Add(ReadVec3(line[2..]));
            }
            else if (line.StartsWith("vt "))
            {
                Vec3 uv = ReadVec3(line[3..]);
                uvs.Add(new Vec2(uv.X, uv.Y));
            }
            else if (line.StartsWith("vn "))
            {
                normals.Add(ReadVec3(line[3..]));
            }
            else if (line.StartsWith("g ") || line.SequenceEqual("g"))
            {
                Flush();
                name = line.Length > 2 ? line[2..].Trim().ToString() : string.Empty;
            }
            else if (line.StartsWith("f "))
            {
                AddFace(line[2..], positions, uvs, normals, vertices, triangles, seen);
            }
        }

        Flush();
        return groups;
    }

    private static void AddFace(
        ReadOnlySpan<char> line,
        List<Vec3> positions,
        List<Vec2> uvs,
        List<Vec3> normals,
        List<MeshVertex> vertices,
        List<int> triangles,
        Dictionary<(int P, int T, int N), int> seen)
    {
        Span<int> corners = stackalloc int[16];
        int count = 0;

        foreach (Range part in Split(line))
        {
            ReadOnlySpan<char> token = line[part];
            if (token.IsEmpty || count == corners.Length)
            {
                continue;
            }

            (int p, int t, int n) = ReadCorner(token, positions.Count, uvs.Count, normals.Count);
            if (p < 0 || p >= positions.Count)
            {
                return;
            }

            if (!seen.TryGetValue((p, t, n), out int index))
            {
                index = vertices.Count;
                vertices.Add(new MeshVertex(
                    positions[p],
                    n >= 0 && n < normals.Count ? normals[n] : Vec3.Zero,
                    t >= 0 && t < uvs.Count ? uvs[t] : Vec2.Zero));
                seen.Add((p, t, n), index);
            }

            corners[count++] = index;
        }

        for (int i = 2; i < count; i++)
        {
            triangles.AddRange([corners[0], corners[i - 1], corners[i]]);
        }
    }

    private static (int Position, int Texture, int Normal) ReadCorner(
        ReadOnlySpan<char> token,
        int positionCount,
        int uvCount,
        int normalCount)
    {
        int position = -1;
        int texture = -1;
        int normal = -1;
        int field = 0;

        while (!token.IsEmpty)
        {
            int slash = token.IndexOf('/');
            ReadOnlySpan<char> value = slash < 0 ? token : token[..slash];
            token = slash < 0 ? [] : token[(slash + 1)..];

            if (int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int number))
            {
                int count = field switch { 0 => positionCount, 1 => uvCount, _ => normalCount };
                int index = number > 0 ? number - 1 : count + number;
                switch (field)
                {
                    case 0:
                        position = index;
                        break;

                    case 1:
                        texture = index;
                        break;

                    default:
                        normal = index;
                        break;
                }
            }

            field++;
            if (field > 2)
            {
                break;
            }
        }

        return (position, texture, normal);
    }

    private static Vec3 ReadVec3(ReadOnlySpan<char> line)
    {
        Span<float> values = [0f, 0f, 0f];
        int count = 0;
        foreach (Range part in Split(line))
        {
            ReadOnlySpan<char> token = line[part];
            if (token.IsEmpty || count == 3)
            {
                continue;
            }

            values[count++] = float.TryParse(token, NumberStyles.Float, CultureInfo.InvariantCulture, out float v)
                ? v
                : 0f;
        }

        return new Vec3(values[0], values[1], values[2]);
    }

    private static List<Range> Split(ReadOnlySpan<char> line)
    {
        List<Range> parts = [];
        int start = -1;
        for (int i = 0; i <= line.Length; i++)
        {
            bool space = i == line.Length || line[i] is ' ' or '\t';
            if (space)
            {
                if (start >= 0)
                {
                    parts.Add(new Range(start, i));
                    start = -1;
                }
            }
            else if (start < 0)
            {
                start = i;
            }
        }

        return parts;
    }
}
