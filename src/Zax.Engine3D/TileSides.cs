namespace Zax.Engine3D;

/// <summary>
/// The four lateral sides of a tile a transparent background instance may have taken off
/// (AMD-063 REQ-SCN-087 step 2). Front and back are never removable.
/// </summary>
/// <remarks>
/// The names are the screen's: <see cref="Top"/> is the side the cell above shares, which is
/// <c>+y</c> in the tile's own frame, since <c>j</c> runs upwards there (REQ-E3D-420).
/// </remarks>
[Flags]
public enum TileSides
{
    /// <summary>Nothing removed: the whole mesh of REQ-E3D-501, as AMD-057 draws it.</summary>
    None = 0,

    /// <summary>The <c>−x</c> side, shared with the cell to the left.</summary>
    Left = 1,

    /// <summary>The <c>+x</c> side, shared with the cell to the right.</summary>
    Right = 2,

    /// <summary>The <c>−y</c> side, shared with the cell below on screen.</summary>
    Bottom = 4,

    /// <summary>The <c>+y</c> side, shared with the cell above on screen.</summary>
    Top = 8,

    /// <summary>All four lateral sides.</summary>
    All = Left | Right | Bottom | Top,
}

/// <summary>
/// Takes the lateral faces REQ-SCN-087 removes off the mesh of a <b>full</b> tile.
/// </summary>
/// <remarks>
/// <para>
/// The amendment leaves the choice between preparing the 15 mesh variants of a full tile and
/// removing the faces at draw time, the drawn result being the same (REQ-SCN-087 notes). This
/// is the first: <see cref="ObjectGeometry"/> memoises one variant per combination of sides.
/// </para>
/// <para>
/// For a full tile the work is a filter on the face normals and nothing more. A face of normal
/// <c>−x</c> exists at the plane <c>i</c> only where the voxel <c>i</c> carries its x slot and
/// the voxel <c>i − 1</c> does not (REQ-E3D-501); in a full tile every voxel of
/// <c>[0, 8) × [0, 8) × [0, D)</c> carries all three slots, so the only such plane is
/// <c>i = 0</c> — the tile's own boundary. The same holds for the three other lateral
/// directions. Dropping every triangle whose normal is that direction therefore removes
/// exactly the boundary wall, and nothing else, whichever greedy algorithm built the mesh.
/// </para>
/// </remarks>
internal static class TileSideRemoval
{
    /// <summary>
    /// The mesh of a full tile with <paramref name="sides"/> taken off, or
    /// <paramref name="mesh"/> itself when nothing is removed.
    /// </summary>
    /// <param name="mesh">The mesh REQ-E3D-501 built for the tile.</param>
    /// <param name="sides">The lateral sides to remove.</param>
    public static TileMesh Apply(TileMesh mesh, TileSides sides)
    {
        ArgumentNullException.ThrowIfNull(mesh);

        if (sides == TileSides.None || mesh.IsEmpty)
        {
            return mesh;
        }

        ReadOnlySpan<MeshVertex> vertices = mesh.Vertices;
        ReadOnlySpan<int> triangles = mesh.Triangles;

        List<MeshVertex> kept = new(vertices.Length);
        List<int> indices = new(triangles.Length);
        int[] moved = new int[vertices.Length];
        Array.Fill(moved, -1);

        for (int n = 0; n + 2 < triangles.Length; n += 3)
        {
            if (Removed(vertices[triangles[n]].Normal, sides))
            {
                continue;
            }

            for (int c = 0; c < 3; c++)
            {
                int source = triangles[n + c];
                if (moved[source] < 0)
                {
                    moved[source] = kept.Count;
                    kept.Add(vertices[source]);
                }

                indices.Add(moved[source]);
            }
        }

        return indices.Count == 0 ? TileMesh.Empty : new TileMesh(kept, indices);
    }

    /// <summary>
    /// Whether a face whose normal is <paramref name="normal"/> belongs to one of
    /// <paramref name="sides"/>. The greedy meshes carry axis-aligned unit normals
    /// (REQ-E3D-505), so an exact comparison is the right one.
    /// </summary>
    private static bool Removed(Vec3 normal, TileSides sides)
    {
        if (normal.Z != 0f)
        {
            return false;
        }

        if (normal.X < 0f)
        {
            return sides.HasFlag(TileSides.Left);
        }

        if (normal.X > 0f)
        {
            return sides.HasFlag(TileSides.Right);
        }

        if (normal.Y < 0f)
        {
            return sides.HasFlag(TileSides.Bottom);
        }

        if (normal.Y > 0f)
        {
            return sides.HasFlag(TileSides.Top);
        }

        return false;
    }
}
