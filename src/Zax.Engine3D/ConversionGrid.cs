namespace Zax.Engine3D;

/// <summary>
/// The fixed geometry of the conversion: grid sizes, scene height and the screen-to-cell
/// relations of SPEC-04 §2 and SPEC-04B REQ-E3D-410.
/// </summary>
public static class ConversionGrid
{
    /// <summary>
    /// Background grid width in cells: the core's 33 useful columns plus one cell of margin on
    /// each side (SPEC-02 REQ-CORE-031, SPEC-04 §2).
    /// </summary>
    public const int Width = 35;

    /// <summary>Background grid height in cells: the core's 31 useful rows plus two margins.</summary>
    public const int Height = 33;

    /// <summary>Number of cells of either grid.</summary>
    public const int CellCount = Width * Height;

    /// <summary>
    /// Scene height <c>Hs = 248</c>: the constant that flips the y axis (REQ-E3D-205,
    /// REQ-E3D-410).
    /// </summary>
    public const int SceneHeight = 248;

    /// <summary>Largest tile box width that still counts as "not big" (REQ-E3D-045).</summary>
    public const int BigWidth = 28;

    /// <summary>Largest tile box height that still counts as "not big" (REQ-E3D-045).</summary>
    public const int BigHeight = 26;

    /// <summary>
    /// The background cell of a tile whose top-left pixel is at
    /// (<paramref name="x"/>, <paramref name="y"/>): <c>(⌊(x+7)/8⌋ + 1, ⌊(y+7)/8⌋ + 1)</c>,
    /// division rounding towards −∞ (SPEC-04 §2, REQ-E3D-005).
    /// </summary>
    public static Cell BackgroundCell(int x, int y) => new(((x + 7) >> 3) + 1, ((y + 7) >> 3) + 1);

    /// <summary>
    /// The sprite cell of a sprite at (<paramref name="x"/>, <paramref name="y"/>):
    /// <c>(⌊x/8⌋, ⌊y/8⌋)</c>, without margin (SPEC-04 §2, REQ-E3D-005).
    /// </summary>
    public static Cell SpriteCell(int x, int y) => new(x >> 3, y >> 3);

    /// <summary>The index of a cell in a grid array.</summary>
    public static int Index(Cell cell) => cell.X + (cell.Y * Width);

    /// <summary>The index of a cell in a grid array.</summary>
    public static int Index(int x, int y) => x + (y * Width);

    /// <summary>True when the cell is inside the grid, margins included.</summary>
    public static bool Contains(Cell cell)
        => (uint)cell.X < (uint)Width && (uint)cell.Y < (uint)Height;

    /// <summary>
    /// The neighbour offsets in the examination order of REQ-E3D-090: right, down, up, left.
    /// Index <c>3 − d</c> is the opposite direction.
    /// </summary>
    public static ReadOnlySpan<int> NeighbourOffsets => [1, Width, -Width, -1];
}
