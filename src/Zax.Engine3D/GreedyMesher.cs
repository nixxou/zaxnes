// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// Turns a voxel field into rectangular faces merged greedily (SPEC-04B REQ-E3D-501 to 506):
/// <c>GREEDY</c> merges without looking at colour, <c>GREEDY_MULTICOLOR</c> only merges cells
/// of equal texel value.
/// </summary>
/// <remarks>
/// <para>
/// The voxels of the neighbouring tiles are never consulted: each tile mesh is closed on
/// itself, which is why two full tiles side by side keep the wall between them
/// (REQ-E3D-501 notes, TV-G7).
/// </para>
/// <para>
/// Q-067: the <c>sealedSides</c> mask is the one exception, and it is a <b>rendering</b>
/// variant, not a change to REQ-E3D-501. A side listed there is one the draw list has found to
/// be entirely covered by the neighbouring tile of the same object; the wall between the two
/// is then strictly inside the solid they form, and the mesh of that variant leaves it out so
/// that it cannot show at the joint. The default mask of 0 gives back the mesh REQ-E3D-501
/// and REQ-E3D-523 describe, bit for bit.
/// </para>
/// </remarks>
public static class GreedyMesher
{
    private const int Empty = -1;

    /// <summary>Builds the mesh of one tile.</summary>
    /// <param name="field">The voxel field of the tile (three slots per voxel).</param>
    /// <param name="texture">The object's texture, read for the multicolour merge.</param>
    /// <param name="multicolor">True for <c>GREEDY_MULTICOLOR</c> (REQ-E3D-503).</param>
    /// <param name="sealedSides">
    /// Lateral sides a neighbouring tile covers entirely, as the bits of
    /// <see cref="TileSides"/>; 0 closes the tile on itself as REQ-E3D-501 states (Q-067).
    /// </param>
    public static TileMesh Build(
        TileVoxels field,
        ObjectTexture texture,
        bool multicolor,
        int sealedSides = 0)
    {
        ArgumentNullException.ThrowIfNull(field);
        ArgumentNullException.ThrowIfNull(texture);

        if (field.IsEmpty)
        {
            return TileMesh.Empty;
        }

        TileMeshBuilder builder = new();
        int[] sizes = [TileVoxels.SizeX, TileVoxels.SizeY, field.Depth];
        int maxCells = sizes[0] * sizes[1];
        maxCells = Math.Max(maxCells, sizes[1] * sizes[2]);
        maxCells = Math.Max(maxCells, sizes[2] * sizes[0]);
        int[] mask = new int[maxCells];

        for (int axis = 0; axis < 3; axis++)
        {
            int u = (axis + 1) % 3;
            int v = (axis + 2) % 3;
            int su = sizes[u];
            int sv = sizes[v];

            for (int sign = -1; sign <= 1; sign += 2)
            {
                for (int plane = 0; plane <= sizes[axis]; plane++)
                {
                    if (!FillMask(field, texture, mask, axis, u, v, su, sv, plane, sign, sealedSides, sizes[axis]))
                    {
                        continue;
                    }

                    Partition(builder, texture, mask, axis, u, v, su, sv, plane, sign, field.Depth, multicolor);
                }
            }
        }

        return builder.Build();
    }

    /// <summary>
    /// Fills <paramref name="mask"/> with the face cells of one plane and one direction
    /// (REQ-E3D-501), and answers whether there is at least one.
    /// </summary>
    private static bool FillMask(
        TileVoxels field,
        ObjectTexture texture,
        int[] mask,
        int axis,
        int u,
        int v,
        int su,
        int sv,
        int plane,
        int sign,
        int sealedSides,
        int axisSize)
    {
        bool any = false;
        Span<int> cell = stackalloc int[3];
        int n = 0;

        for (int vi = 0; vi < sv; vi++)
        {
            for (int ui = 0; ui < su; ui++, n++)
            {
                cell[u] = ui;
                cell[v] = vi;

                // The owner of the face is the voxel on the side the normal comes from.
                cell[axis] = sign > 0 ? plane - 1 : plane;
                bool owner = field.Occupied(cell[0], cell[1], cell[2], axis);
                int tx = 0;
                int ty = 0;
                if (owner)
                {
                    field.TryTexel(cell[0], cell[1], cell[2], axis, out tx, out ty);
                }

                cell[axis] = sign > 0 ? plane : plane - 1;

                // Q-067: on a sealed side the tile next door fills the whole plane, so the
                // voxel just outside counts as occupied and no wall is emitted there.
                bool neighbour = TileSides.IsSealed(sealedSides, axis, cell[axis], axisSize)
                    || field.Occupied(cell[0], cell[1], cell[2], axis);

                if (owner && !neighbour)
                {
                    mask[n] = Pack(tx, ty);
                    any = true;
                }
                else
                {
                    mask[n] = Empty;
                }
            }
        }

        return any;
    }

    /// <summary>
    /// Groups the face cells of a plane into maximal rectangles, <c>v</c> ascending then
    /// <c>u</c> ascending (REQ-E3D-502, REQ-E3D-503), and emits each one.
    /// </summary>
    private static void Partition(
        TileMeshBuilder builder,
        ObjectTexture texture,
        int[] mask,
        int axis,
        int u,
        int v,
        int su,
        int sv,
        int plane,
        int sign,
        int depth,
        bool multicolor)
    {
        for (int vi = 0; vi < sv; vi++)
        {
            int row = vi * su;
            for (int ui = 0; ui < su;)
            {
                int start = row + ui;
                if (mask[start] == Empty)
                {
                    ui++;
                    continue;
                }

                // GREEDY merges any two cells that both exist; GREEDY_MULTICOLOR also demands
                // the texel value of the rectangle's first cell. A position without a cell is
                // never merged, which leaves out the legacy "missing counts as 0" case of the
                // REQ-E3D-503 notes (P3, class D, DEC-04B-Q6).
                int reference = multicolor ? Value(texture, mask[start]) : 0;

                int width = 0;
                while (ui + width < su && Compatible(texture, mask[start + width], multicolor, reference))
                {
                    width++;
                }

                int height = 1;
                while (vi + height < sv)
                {
                    int candidate = start + (su * height);
                    bool ok = true;
                    for (int c = 0; c < width; c++)
                    {
                        if (!Compatible(texture, mask[candidate + c], multicolor, reference))
                        {
                            ok = false;
                            break;
                        }
                    }

                    if (!ok)
                    {
                        break;
                    }

                    height++;
                }

                int topRow = start + (su * (height - 1));
                EmitRectangle(
                    builder,
                    axis,
                    u,
                    v,
                    plane,
                    sign,
                    depth,
                    ui,
                    vi,
                    width,
                    height,
                    mask[start],
                    mask[start + width - 1],
                    mask[topRow],
                    mask[topRow + width - 1]);

                for (int c = 0; c < width; c++)
                {
                    for (int r = 0; r < height; r++)
                    {
                        mask[start + c + (su * r)] = Empty;
                    }
                }

                ui += width;
            }
        }
    }

    /// <summary>
    /// Emits one rectangle: its four corners in local coordinates (REQ-E3D-504) and their
    /// texture coordinates (REQ-E3D-506).
    /// </summary>
    private static void EmitRectangle(
        TileMeshBuilder builder,
        int axis,
        int u,
        int v,
        int plane,
        int sign,
        int depth,
        int u0,
        int v0,
        int width,
        int height,
        int cornerBottomLeft,
        int cornerBottomRight,
        int cornerTopLeft,
        int cornerTopRight)
    {
        Vec3 bottomLeft = Corner(axis, u, v, plane, depth, u0, v0);
        Vec3 topLeft = Corner(axis, u, v, plane, depth, u0, v0 + height);
        Vec3 topRight = Corner(axis, u, v, plane, depth, u0 + width, v0 + height);
        Vec3 bottomRight = Corner(axis, u, v, plane, depth, u0 + width, v0);

        Unpack(cornerBottomLeft, out int blx, out int bly);
        Unpack(cornerBottomRight, out int brx, out int bry);
        Unpack(cornerTopLeft, out int tlx, out int tly);
        Unpack(cornerTopRight, out int trx, out int trY);

        bool axisIsZ = axis == 2;

        // Column component (REQ-E3D-506).
        if (trx >= blx)
        {
            trx++;
            if (axisIsZ)
            {
                brx++;
            }
            else
            {
                tlx++;
            }
        }
        else
        {
            blx++;
            if (axisIsZ)
            {
                tlx++;
            }
            else
            {
                brx++;
            }
        }

        // Row component (REQ-E3D-506).
        if (trY >= bly)
        {
            trY++;
            if (axisIsZ)
            {
                tly++;
            }
            else
            {
                bry++;
            }
        }
        else
        {
            bly++;
            if (axisIsZ)
            {
                bry++;
            }
            else
            {
                tly++;
            }
        }

        Vec3 normal = axis switch
        {
            0 => new Vec3(sign, 0f, 0f),
            1 => new Vec3(0f, sign, 0f),
            _ => new Vec3(0f, 0f, sign),
        };

        builder.AddQuad(
            bottomLeft,
            topLeft,
            topRight,
            bottomRight,
            new Vec2(blx, bly),
            new Vec2(tlx, tly),
            new Vec2(trx, trY),
            new Vec2(brx, bry),
            normal);
    }

    /// <summary>
    /// The local position of the voxel corner of integer indices
    /// <c>(i, j, k)</c>: <c>(i − 4, j − 4, k − D/2)</c> (REQ-E3D-504).
    /// </summary>
    private static Vec3 Corner(int axis, int u, int v, int plane, int depth, int ui, int vi)
    {
        Span<int> index = stackalloc int[3];
        index[axis] = plane;
        index[u] = ui;
        index[v] = vi;
        return new Vec3(index[0] - 4f, index[1] - 4f, index[2] - (depth / 2f));
    }

    private static bool Compatible(ObjectTexture texture, int cell, bool multicolor, int reference)
        => cell != Empty && (!multicolor || Value(texture, cell) == reference);

    private static int Value(ObjectTexture texture, int cell)
    {
        Unpack(cell, out int tx, out int ty);
        return texture.Texel(tx, ty);
    }

    private static int Pack(int tx, int ty)
        => (Math.Clamp(ty, 0, 0x7FFF) << 16) | Math.Clamp(tx, 0, 0x7FFF);

    private static void Unpack(int value, out int tx, out int ty)
    {
        tx = value & 0xFFFF;
        ty = (value >> 16) & 0xFFFF;
    }
}
