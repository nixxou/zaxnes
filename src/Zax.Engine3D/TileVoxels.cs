// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// The voxel field of one distinct tile image of a 3D object: <c>8 × 8 × D</c> voxels, each
/// carrying one slot per axis in the GREEDY algorithms and a single slot in MARCHING
/// (SPEC-04B REQ-E3D-420, REQ-E3D-422).
/// </summary>
/// <remarks>
/// <para>
/// A voxel is addressed by <c>(i, j, k)</c>: <c>i</c> the column (0 to 7, rightwards),
/// <c>j</c> the row (0 to 7, <b>upwards</b>) and <c>k</c> the depth slice (0 to <c>D − 1</c>,
/// <c>k = 0</c> on the viewer's side). A slot is either empty or the coordinates
/// <c>(tx, ty)</c> of a texel of the object's texture.
/// </para>
/// <para>
/// Writing a texel whose value is the transparent colour <c>e</c> leaves the slot
/// <b>empty</b>, and several writes on one slot resolve by "last written wins", including when
/// the last one empties it (REQ-E3D-423).
/// </para>
/// </remarks>
public sealed class TileVoxels
{
    /// <summary>Tile width in voxels.</summary>
    public const int SizeX = 8;

    /// <summary>Tile height in voxels.</summary>
    public const int SizeY = 8;

    /// <summary>Largest depth a 3D object can have (<c>sizeZ</c> is 0 to 96).</summary>
    public const int MaxDepth = 96;

    /// <summary>Slot index of the X axis.</summary>
    public const int AxisX = 0;

    /// <summary>Slot index of the Y axis.</summary>
    public const int AxisY = 1;

    /// <summary>Slot index of the Z axis.</summary>
    public const int AxisZ = 2;

    /// <summary>Writes every slot of the voxel, whatever the algorithm (REQ-E3D-422).</summary>
    public const int AllAxes = -1;

    private const int Empty = -1;

    private readonly int[] _slots;

    /// <summary>Creates an empty field.</summary>
    /// <param name="depth">Depth <c>D</c> in voxels, 0 to <see cref="MaxDepth"/>.</param>
    /// <param name="perAxisSlots">
    /// True for the GREEDY algorithms, where each voxel carries three independent slots; false
    /// for MARCHING, where it carries one.
    /// </param>
    public TileVoxels(int depth, bool perAxisSlots)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(depth);
        ArgumentOutOfRangeException.ThrowIfGreaterThan(depth, MaxDepth);

        Depth = depth;
        SlotsPerVoxel = perAxisSlots ? 3 : 1;
        _slots = new int[SizeX * SizeY * depth * SlotsPerVoxel];
        Array.Fill(_slots, Empty);
    }

    /// <summary>Depth <c>D</c> of the field in voxels.</summary>
    public int Depth { get; }

    /// <summary>Three in the GREEDY algorithms, one in MARCHING.</summary>
    public int SlotsPerVoxel { get; }

    /// <summary>True when the field holds no voxel at all (<c>D = 0</c>, REQ-E3D-420).</summary>
    public bool IsEmpty => _slots.Length == 0;

    /// <summary>True when <c>(i, j, k)</c> is inside <c>[0,8) × [0,8) × [0,D)</c>.</summary>
    public bool Contains(int i, int j, int k)
        => (uint)i < SizeX && (uint)j < SizeY && (uint)k < (uint)Depth;

    /// <summary>
    /// Assigns the texel (<paramref name="tx"/>, <paramref name="ty"/>) to the slot
    /// <paramref name="axis"/> of the voxel, or to all its slots when
    /// <paramref name="axis"/> is <see cref="AllAxes"/>. The slot is left <b>empty</b> when the
    /// texel value is the transparent colour (REQ-E3D-423). A write outside the field has no
    /// effect (REQ-E3D-421).
    /// </summary>
    public void Write(int i, int j, int k, int axis, int tx, int ty, bool transparent)
    {
        if (!Contains(i, j, k))
        {
            return;
        }

        int value = transparent ? Empty : Pack(tx, ty);
        int slot = Index(i, j, k);
        if (SlotsPerVoxel == 1 || axis == AllAxes)
        {
            for (int a = 0; a < SlotsPerVoxel; a++)
            {
                _slots[slot + a] = value;
            }

            return;
        }

        _slots[slot + axis] = value;
    }

    /// <summary>True when the slot <paramref name="axis"/> of the voxel is not empty.</summary>
    public bool Occupied(int i, int j, int k, int axis)
        => Contains(i, j, k) && _slots[Index(i, j, k) + SlotOf(axis)] != Empty;

    /// <summary>
    /// Reads the texel of a slot. Returns false when the voxel is outside the field or the slot
    /// is empty.
    /// </summary>
    public bool TryTexel(int i, int j, int k, int axis, out int tx, out int ty)
    {
        if (!Contains(i, j, k))
        {
            tx = 0;
            ty = 0;
            return false;
        }

        int value = _slots[Index(i, j, k) + SlotOf(axis)];
        if (value == Empty)
        {
            tx = 0;
            ty = 0;
            return false;
        }

        Unpack(value, out tx, out ty);
        return true;
    }

    private int SlotOf(int axis) => SlotsPerVoxel == 1 ? 0 : axis;

    private int Index(int i, int j, int k) => (((k * SizeY) + j) * SizeX + i) * SlotsPerVoxel;

    /// <summary>
    /// Packs a texel coordinate pair into one slot. Both components are clamped to
    /// <c>[0, 0x7FFF]</c>, which keeps the packed value positive and so distinct from
    /// <see cref="Empty"/>; the construction rules of §3.3 only ever write coordinates inside
    /// the pattern image, far below that bound.
    /// </summary>
    private static int Pack(int tx, int ty)
        => (Math.Clamp(ty, 0, 0x7FFF) << 16) | Math.Clamp(tx, 0, 0x7FFF);

    private static void Unpack(int value, out int tx, out int ty)
    {
        tx = value & 0xFFFF;
        ty = (value >> 16) & 0xFFFF;
    }
}
