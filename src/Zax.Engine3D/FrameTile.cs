namespace Zax.Engine3D;

/// <summary>
/// One tile kept in a frame: its image, where it is on screen, its palette attribute and the
/// shape it ended up in (REQ-E3D-005, REQ-E3D-042).
/// </summary>
/// <remarks>
/// Tiles are pooled: a frame holds one instance per background cell and one per sprite slot,
/// reused from frame to frame, so a full frame costs no allocation (REQ-E3D-360).
/// </remarks>
public sealed class FrameTile
{
    internal FrameTile(Cell cell, int gridIndex, bool background)
    {
        Position = cell;
        GridIndex = gridIndex;
        Background = background;
        Image = null!;
    }

    /// <summary>The image of the tile.</summary>
    public TileImage Image { get; internal set; }

    /// <summary>Screen x of the top-left pixel.</summary>
    public int X { get; internal set; }

    /// <summary>Screen y of the top-left pixel.</summary>
    public int Y { get; internal set; }

    /// <summary>Palette attribute, 0 to 3.</summary>
    public int Palette { get; internal set; }

    /// <summary>Sprite priority bit "behind the background"; always false for background.</summary>
    public bool Hidden { get; internal set; }

    /// <summary>Sprite slot, 0 to 127 (REQ-CORE-043); −1 for a background tile.</summary>
    public int Ord { get; internal set; } = -1;

    /// <summary>The cell of the tile in the grid of its layer.</summary>
    public Cell Position { get; internal set; }

    /// <summary>Index of <see cref="Position"/> in a grid array.</summary>
    public int GridIndex { get; internal set; }

    /// <summary>True for a background tile, false for a sprite.</summary>
    public bool Background { get; }

    /// <summary>True when the tile was kept in the batch of the current frame.</summary>
    public bool Present { get; internal set; }

    /// <summary>The shape the tile belongs to, or null while it is still free.</summary>
    public FrameShape? Shape { get; internal set; }

    /// <summary>True when the tile is not in a shape yet (REQ-E3D-090 point 3).</summary>
    public bool IsFree => Shape is null;

    /// <summary>Colour (0 to 3) of the displayed pixel at (x, y) of the tile.</summary>
    public int ColorAt(int px, int py) => Image.ColorAt(px, py);

    internal void Reset()
    {
        Present = false;
        Shape = null;
    }

    /// <inheritdoc/>
    public override string ToString()
        => $"{(Background ? "bg" : "sp")} tile ({X}, {Y}) cell {Position} pal {Palette}";
}
