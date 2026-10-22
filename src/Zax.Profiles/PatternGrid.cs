namespace Zax.Profiles;

/// <summary>
/// The rectangular tile grid of a pattern (SPEC-03 REQ-PROF-012): <c>Width × Height</c> cells
/// row by row, each either a tile or empty.
/// </summary>
/// <remarks>
/// Cell (cx, cy) is at index <c>cx + cy × Width</c>. The size comes from the pattern's
/// <c>tSize</c>; the 101 built-in profiles always hold exactly <c>Width × Height</c> cells, but
/// a file that holds another number is still read (the cell list is authoritative, and
/// <see cref="IsRectangular"/> says whether the two agree).
/// </remarks>
public sealed class PatternGrid
{
    private readonly ProfileTile?[] _cells;

    /// <summary>Creates a grid from its cells in file order, row by row.</summary>
    /// <param name="width">Width in tiles, from the pattern's <c>tSize.x</c>.</param>
    /// <param name="height">Height in tiles, from the pattern's <c>tSize.y</c>.</param>
    /// <param name="cells">Cells, row by row; a null entry is an empty cell.</param>
    public PatternGrid(int width, int height, IEnumerable<ProfileTile?> cells)
    {
        ArgumentNullException.ThrowIfNull(cells);
        Width = width;
        Height = height;
        _cells = [.. cells];
    }

    /// <summary>Width in tiles, as declared by the pattern.</summary>
    public int Width { get; }

    /// <summary>Height in tiles, as declared by the pattern.</summary>
    public int Height { get; }

    /// <summary>Cells in file order, row by row; a null entry is an empty cell.</summary>
    public IReadOnlyList<ProfileTile?> Cells => _cells;

    /// <summary>True when the number of cells is exactly <c>Width × Height</c>.</summary>
    public bool IsRectangular => _cells.Length == Width * Height;

    /// <summary>Number of cells that hold a tile.</summary>
    public int FilledCellCount
    {
        get
        {
            int count = 0;
            foreach (ProfileTile? cell in _cells)
            {
                if (cell is not null)
                {
                    count++;
                }
            }

            return count;
        }
    }

    /// <summary>
    /// The tile at (cx, cy), or <see langword="null"/> when the cell is empty or outside the
    /// stored cells.
    /// </summary>
    public ProfileTile? this[int cx, int cy]
    {
        get
        {
            if ((uint)cx >= (uint)Width || (uint)cy >= (uint)Height)
            {
                return null;
            }

            int index = cx + (cy * Width);
            return (uint)index < (uint)_cells.Length ? _cells[index] : null;
        }
    }
}
