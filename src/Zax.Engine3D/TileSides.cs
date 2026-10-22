namespace Zax.Engine3D;

/// <summary>
/// The four lateral sides of a tile — the ones a neighbouring tile of the same object can be
/// against — as a bit mask (Q-067).
/// </summary>
/// <remarks>
/// SPEC-04B REQ-E3D-501 closes every tile mesh on itself, so two full tiles side by side keep a
/// pair of coincident walls between them, inside the solid the two form. Those walls are what
/// shows at every joint of a flat surface: a dark line where they win the depth test along the
/// shared edge, and a fin sticking out of the silhouette wherever the surface ends (SPEC-06
/// REQ-SCN-065). A side listed in a mask is one the draw list has proved covered, so the mesh
/// variant of Q-067 leaves its wall out.
/// </remarks>
public static class TileSides
{
    /// <summary>The <c>x = 0</c> side of the tile, towards the neighbour on its left.</summary>
    public const int MinusX = 1 << 0;

    /// <summary>The <c>x = 8</c> side, towards the neighbour on its right.</summary>
    public const int PlusX = 1 << 1;

    /// <summary>The <c>y = 0</c> side, towards the neighbour below it in the scene.</summary>
    public const int MinusY = 1 << 2;

    /// <summary>The <c>y = 8</c> side, towards the neighbour above it in the scene.</summary>
    public const int PlusY = 1 << 3;

    /// <summary>Every lateral side; the Z sides are never sealed.</summary>
    public const int All = MinusX | PlusX | MinusY | PlusY;

    /// <summary>The bit of the side at the <paramref name="low"/> end of <paramref name="axis"/>.</summary>
    /// <param name="axis">0 for X, 1 for Y; anything else has no bit.</param>
    /// <param name="low">True for the <c>0</c> side, false for the <c>size</c> side.</param>
    public static int Bit(int axis, bool low) => axis switch
    {
        0 => low ? MinusX : PlusX,
        1 => low ? MinusY : PlusY,
        _ => 0,
    };

    /// <summary>The side opposite <paramref name="side"/>, which the neighbour presents.</summary>
    public static int Opposite(int side) => side switch
    {
        MinusX => PlusX,
        PlusX => MinusX,
        MinusY => PlusY,
        PlusY => MinusY,
        _ => 0,
    };

    /// <summary>
    /// Whether the voxel at index <paramref name="index"/> along <paramref name="axis"/>, which
    /// is outside the tile when it leaves <c>[0, size)</c>, belongs to a sealed neighbour.
    /// </summary>
    /// <param name="sealedSides">The mask of sealed sides.</param>
    /// <param name="axis">The axis the index runs along.</param>
    /// <param name="index">The voxel index, possibly out of the tile.</param>
    /// <param name="size">The size of the tile along <paramref name="axis"/>.</param>
    public static bool IsSealed(int sealedSides, int axis, int index, int size)
    {
        if (sealedSides == 0 || axis > 1)
        {
            return false;
        }

        if (index < 0)
        {
            return (sealedSides & Bit(axis, low: true)) != 0;
        }

        return index >= size && (sealedSides & Bit(axis, low: false)) != 0;
    }

    /// <summary>
    /// The sides of <paramref name="field"/> whose whole plane is occupied: those are the only
    /// ones a neighbour can seal without opening a hole, since the two walls then coincide
    /// cell for cell (Q-067).
    /// </summary>
    /// <param name="field">The voxel field of the tile.</param>
    /// <returns>A mask of <see cref="MinusX"/>, <see cref="PlusX"/>, <see cref="MinusY"/>, <see cref="PlusY"/>.</returns>
    public static int FullSides(TileVoxels field)
    {
        ArgumentNullException.ThrowIfNull(field);
        if (field.IsEmpty || field.Depth <= 0)
        {
            return 0;
        }

        int mask = All;
        for (int k = 0; k < field.Depth; k++)
        {
            for (int n = 0; n < TileVoxels.SizeY; n++)
            {
                if (!field.Occupied(0, n, k, TileVoxels.AxisX))
                {
                    mask &= ~MinusX;
                }

                if (!field.Occupied(TileVoxels.SizeX - 1, n, k, TileVoxels.AxisX))
                {
                    mask &= ~PlusX;
                }
            }

            for (int n = 0; n < TileVoxels.SizeX; n++)
            {
                if (!field.Occupied(n, 0, k, TileVoxels.AxisY))
                {
                    mask &= ~MinusY;
                }

                if (!field.Occupied(n, TileVoxels.SizeY - 1, k, TileVoxels.AxisY))
                {
                    mask &= ~PlusY;
                }
            }

            if (mask == 0)
            {
                return 0;
            }
        }

        return mask;
    }
}
