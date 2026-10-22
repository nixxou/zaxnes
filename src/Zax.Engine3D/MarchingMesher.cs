// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// Turns a voxel field into a smoothed surface by marching cubes, one surface per colour
/// (SPEC-04B REQ-E3D-520 to 522).
/// </summary>
/// <remarks>
/// <para>
/// The grid is the <b>dual</b> of the voxels: a cell's eight corners are voxel centres, so the
/// surface floats half a voxel outside the field, which REQ-E3D-408 compensates by shrinking
/// the instance scale. Corners outside the tile are read in the neighbouring tile of the
/// pattern, which is what makes two adjacent tiles join without a wall (TV-M3).
/// </para>
/// <para>
/// Each vertex is placed <b>exactly on the exterior corner</b> of its edge rather than
/// interpolated, which is the behaviour of the reference; it produces zero-area triangles in
/// some configurations, which REQ-E3D-522 allows and REQ-E3D-523 ignores when comparing.
/// </para>
/// </remarks>
public static class MarchingMesher
{
    /// <summary>Builds the mesh of the tile of index <paramref name="imageIndex"/>.</summary>
    /// <param name="voxels">The fields of the whole pattern, for the neighbourhood.</param>
    /// <param name="imageIndex">The tile image to mesh.</param>
    /// <param name="emptyColor">The transparent colour <c>e</c> (REQ-E3D-424).</param>
    public static TileMesh Build(PatternVoxels voxels, int imageIndex, int emptyColor)
    {
        ArgumentNullException.ThrowIfNull(voxels);

        TileVoxels field = voxels.Field(imageIndex);
        if (field.IsEmpty)
        {
            return TileMesh.Empty;
        }

        int depth = field.Depth;
        TileMeshBuilder builder = new();

        Span<int> cornerValue = stackalloc int[MarchingCubesTables.CornerCount];
        Span<int> cornerTexelX = stackalloc int[MarchingCubesTables.CornerCount];
        Span<int> cornerTexelY = stackalloc int[MarchingCubesTables.CornerCount];
        Span<bool> cornerPresent = stackalloc bool[MarchingCubesTables.CornerCount];
        Span<int> cornerX = stackalloc int[MarchingCubesTables.CornerCount];
        Span<int> cornerY = stackalloc int[MarchingCubesTables.CornerCount];
        Span<int> cornerZ = stackalloc int[MarchingCubesTables.CornerCount];
        Span<int> colors = stackalloc int[MarchingCubesTables.CornerCount];
        Span<Vec3> edgeVertex = stackalloc Vec3[MarchingCubesTables.EdgeCount];
        Span<Vec3> edgeOutward = stackalloc Vec3[MarchingCubesTables.EdgeCount];
        Span<bool> interior = stackalloc bool[MarchingCubesTables.CornerCount];

        for (int k = -1; k < depth; k++)
        {
            for (int j = -1; j < TileVoxels.SizeY; j++)
            {
                for (int i = -1; i < TileVoxels.SizeX; i++)
                {
                    // The cells of the leftmost column belong to the leftmost occurrence of
                    // the tile only (REQ-E3D-520).
                    if (i == -1 && !voxels.MeshesLeftEdge(imageIndex, j))
                    {
                        continue;
                    }

                    int colorCount = 0;
                    for (int c = 0; c < MarchingCubesTables.CornerCount; c++)
                    {
                        (int dx, int dy, int dz) = MarchingCubesTables.CornerOffset(c);
                        cornerX[c] = i + dx;
                        cornerY[c] = j + dy;
                        cornerZ[c] = k + dz;

                        cornerPresent[c] = voxels.TryCorner(
                            imageIndex,
                            cornerX[c],
                            cornerY[c],
                            cornerZ[c],
                            out int tx,
                            out int ty);
                        cornerTexelX[c] = tx;
                        cornerTexelY[c] = ty;

                        // An empty corner counts as the value 0 (REQ-E3D-521).
                        cornerValue[c] = cornerPresent[c] ? voxels.Texture.Texel(tx, ty) : 0;

                        if (cornerValue[c] == emptyColor)
                        {
                            continue;
                        }

                        bool known = false;
                        for (int n = 0; n < colorCount; n++)
                        {
                            if (colors[n] == cornerValue[c])
                            {
                                known = true;
                                break;
                            }
                        }

                        if (!known)
                        {
                            colors[colorCount++] = cornerValue[c];
                        }
                    }

                    for (int n = 0; n < colorCount; n++)
                    {
                        Surface(
                            builder,
                            colors[n],
                            cornerValue,
                            cornerTexelX,
                            cornerTexelY,
                            cornerX,
                            cornerY,
                            cornerZ,
                            depth,
                            interior,
                            edgeVertex,
                            edgeOutward);
                    }
                }
            }
        }

        return builder.Build();
    }

    /// <summary>
    /// Emits the surface that separates the corners of value <paramref name="color"/> from the
    /// others, inside one cell (REQ-E3D-521, REQ-E3D-522).
    /// </summary>
    private static void Surface(
        TileMeshBuilder builder,
        int color,
        ReadOnlySpan<int> cornerValue,
        ReadOnlySpan<int> cornerTexelX,
        ReadOnlySpan<int> cornerTexelY,
        ReadOnlySpan<int> cornerX,
        ReadOnlySpan<int> cornerY,
        ReadOnlySpan<int> cornerZ,
        int depth,
        Span<bool> interior,
        Span<Vec3> edgeVertex,
        Span<Vec3> edgeOutward)
    {
        int configuration = 0;
        int highest = -1;
        for (int c = 0; c < MarchingCubesTables.CornerCount; c++)
        {
            interior[c] = cornerValue[c] == color;
            if (interior[c])
            {
                configuration |= 1 << c;
                highest = c;
            }
        }

        // A cell whose eight corners carry the colour produces nothing.
        if (configuration == 0xFF || highest < 0)
        {
            return;
        }

        // The texel of the interior corner of highest number (REQ-E3D-522).
        Vec2 uv = new(cornerTexelX[highest] + 0.5f, cornerTexelY[highest] + 0.5f);

        ReadOnlySpan<int> triangulation = MarchingCubesTables.Triangulation(configuration);
        for (int e = 0; e < MarchingCubesTables.EdgeCount; e++)
        {
            (int first, int second) = MarchingCubesTables.EdgeCorners(e);
            Vec3 a = Position(cornerX[first], cornerY[first], cornerZ[first], depth);
            Vec3 b = Position(cornerX[second], cornerY[second], cornerZ[second], depth);

            if (!interior[first])
            {
                edgeVertex[e] = a;
                edgeOutward[e] = interior[second] ? a - b : Vec3.Zero;
            }
            else if (!interior[second])
            {
                edgeVertex[e] = b;
                edgeOutward[e] = b - a;
            }
            else
            {
                edgeVertex[e] = (a + b) * 0.5f;
                edgeOutward[e] = Vec3.Zero;
            }
        }

        for (int n = 0; n + 2 < triangulation.Length && triangulation[n] != -1; n += 3)
        {
            int e0 = triangulation[n];
            int e1 = triangulation[n + 1];
            int e2 = triangulation[n + 2];

            // The surface must look away from the region of colour c; averaging the
            // interior-to-exterior directions of the three edges gives that side. REQ-E3D-522
            // states the intention, not a construction: the vertices sit on exterior corners,
            // so the triangle alone does not say which way is out (REQ-E3D-505, Q-016).
            Vec3 outward = edgeOutward[e0] + edgeOutward[e1] + edgeOutward[e2];
            builder.AddTriangle(edgeVertex[e0], edgeVertex[e1], edgeVertex[e2], uv, outward);
        }
    }

    /// <summary>
    /// The local position of a voxel centre: <c>(i, j, k) − (3.5, 3.5, (D − 1)/2)</c>
    /// (REQ-E3D-522).
    /// </summary>
    private static Vec3 Position(int i, int j, int k, int depth)
        => new(i - 3.5f, j - 3.5f, k - ((depth - 1) / 2f));
}
