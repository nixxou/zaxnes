// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// A shape: a group of adjacent tiles of one frame and one layer, bound to exactly one
/// reference pattern (SPEC-04 §2, REQ-E3D-044 to 048).
/// </summary>
/// <remarks>
/// <para>
/// A shape also plays the part of the <b>frame pattern</b> of SPEC-05 (type <c>Pattern</c>):
/// its grid is its tile box, its instances are the frame instances, and the recognition
/// parameters a script reads through it (<see cref="ConstraintSize"/>,
/// <see cref="SelfConnected"/>, <see cref="SkipGeometry"/>) are per-frame copies of the
/// reference pattern's, so writing them affects this frame only.
/// </para>
/// <para>
/// Shapes are pooled by the frame, like tiles.
/// </para>
/// </remarks>
public sealed class FrameShape
{
    private readonly List<FrameTile> _tiles = [];
    private readonly List<TileImage> _images = [];
    private readonly List<FrameInstance> _instances = [];

    internal FrameShape()
    {
    }

    /// <summary>The tiles, in the order they were added; the first one is the germ.</summary>
    public IReadOnlyList<FrameTile> Tiles => _tiles;

    /// <summary>The distinct images of the shape, <c>Img(S)</c>, in the order first met.</summary>
    public IReadOnlyList<TileImage> Images => _images;

    /// <summary>The frame instances, one per instance of the reference pattern (REQ-E3D-190).</summary>
    public IReadOnlyList<FrameInstance> Instances => _instances;

    /// <summary>True for a background shape, false for a sprite shape.</summary>
    public bool Background { get; private set; }

    /// <summary>
    /// The frame that owns the shape and pools it. A shape restored from the pinned frame
    /// appears in another frame's list but stays owned by the pinned one (REQ-E3D-233).
    /// </summary>
    internal ConversionFrame? Owner { get; set; }

    /// <summary>The grid of the shape's layer, used by the "full" test of REQ-E3D-094.</summary>
    internal FrameTile?[] Grid { get; private set; } = [];

    /// <summary>Top-left corner of the tile box, in cells of the shape's grid.</summary>
    public Cell TileStart { get; private set; }

    /// <summary>Bottom-right corner of the tile box, inclusive.</summary>
    public Cell TileEnd { get; private set; }

    /// <summary>Size of the tile box, <c>tEnd − tStart + (1, 1)</c>.</summary>
    public Cell TileSize => new(TileEnd.X - TileStart.X + 1, TileEnd.Y - TileStart.Y + 1);

    /// <summary>Top-left corner of the pixel box, in screen pixels (REQ-E3D-044).</summary>
    public Cell PixelStart { get; private set; }

    /// <summary>Bottom-right corner of the pixel box, inclusive (REQ-E3D-044).</summary>
    public Cell PixelEnd { get; private set; }

    /// <summary>Size of the pixel box, <c>pEnd − pStart + (1, 1)</c> (SPEC-04 §4.4).</summary>
    public Cell PixelSize => new(PixelEnd.X - PixelStart.X + 1, PixelEnd.Y - PixelStart.Y + 1);

    /// <summary>Number of tiles.</summary>
    public int TileCount => _tiles.Count;

    /// <summary>Total number of non-transparent pixels of the shape's tiles.</summary>
    public int PixelCount { get; private set; }

    /// <summary>
    /// Palette of the shape: the germ's attribute, plus 4 for a sprite shape (REQ-E3D-047).
    /// </summary>
    public int Palette { get; private set; }

    /// <summary>
    /// Or of the <c>hidden</c> flags of the sprites; always false for a background shape
    /// (REQ-E3D-048).
    /// </summary>
    public bool Hidden { get; private set; }

    /// <summary>
    /// Screen centre <c>(pStart + pEnd + (1, 1)) / 2</c> (REQ-E3D-044), read by the tracker.
    /// </summary>
    public Vec3 Center { get; private set; }

    /// <summary>The reference pattern the shape is bound to, or null before binding.</summary>
    public EnginePattern? ReferencePattern { get; private set; }

    /// <summary>The script layer's handle cache (SPEC-05 REQ-LUA-043).</summary>
    public ScriptHandleSlot ScriptHandle;

    /// <summary>
    /// Whether the shape's instances are scriptable (REQ-E3D-220): true for a shape produced by
    /// the segmentation, false for one injected by a script or restored from the pinned frame.
    /// </summary>
    public bool Scriptable { get; internal set; }

    /// <summary>
    /// True when the shape was restored from the pinned frame: its tiles are drawn only where
    /// they overlap one of the frame's rectangles (REQ-E3D-233).
    /// </summary>
    public bool RestoredFromPinnedFrame { get; internal set; }

    /// <summary>Size constraint of the frame pattern, copied from the reference pattern.</summary>
    public Cell ConstraintSize { get; set; } = new(99, 99);

    /// <summary>Connectivity option of the frame pattern, copied from the reference pattern.</summary>
    public bool SelfConnected { get; set; } = true;

    /// <summary>Geometry-skip option of the frame pattern, copied from the reference pattern.</summary>
    public bool SkipGeometry { get; set; }

    /// <summary>
    /// The mask of the frame the shape belongs to, needed by <see cref="Inside"/>
    /// (REQ-E3D-045).
    /// </summary>
    internal Cell Mask { get; set; }

    /// <summary>
    /// True when the shape is fully away from the unreliable border bands (REQ-E3D-045):
    /// <c>pStart.x &gt; mX</c>, <c>pEnd.x + mX &lt; 255</c>, <c>pStart.y &gt; mY</c> and
    /// <c>pEnd.y + mY &lt; 239</c>.
    /// </summary>
    public bool Inside
        => PixelStart.X > Mask.X
        && PixelEnd.X + Mask.X < 255
        && PixelStart.Y > Mask.Y
        && PixelEnd.Y + Mask.Y < 239;

    /// <summary>The negation of <see cref="Inside"/> (REQ-E3D-045).</summary>
    public bool AtBorder => !Inside;

    /// <summary>
    /// True when the tile box is at least 28 cells wide or 26 cells high (REQ-E3D-045).
    /// </summary>
    public bool IsBig
    {
        get
        {
            Cell size = TileSize;
            return size.X >= ConversionGrid.BigWidth || size.Y >= ConversionGrid.BigHeight;
        }
    }

    /// <summary>True when the shape is <see cref="Inside"/> or <see cref="IsBig"/> (REQ-E3D-045).</summary>
    public bool IsFinished => Inside || IsBig;

    /// <summary>
    /// True when the shape shows every image of its reference pattern,
    /// <c>|Img(S)| = |Img(P)|</c> (REQ-E3D-046).
    /// </summary>
    public bool Completed
        => ReferencePattern is { } pattern && _images.Count == pattern.Images.Count;

    /// <summary>The age of a shape, which is always 1 (REQ-E3D-207).</summary>
    public static int Age => 1;

    /// <summary><c>BottomLeft = (pStart.x, Hs − pEnd.y − 1)</c> (SPEC-04 §4.4).</summary>
    public Vec2 BottomLeft => new(PixelStart.X, ConversionGrid.SceneHeight - PixelEnd.Y - 1);

    /// <summary><c>TopRight = (pEnd.x, Hs − pStart.y − 1)</c> (SPEC-04 §4.4).</summary>
    public Vec2 TopRight => new(PixelEnd.X, ConversionGrid.SceneHeight - PixelStart.Y - 1);

    /// <summary>The key of the shape in the pattern registry: its set of images (REQ-E3D-063).</summary>
    public ImageSetKey Key => new(_images);

    /// <summary>
    /// True when no cell strictly inside the tile box and outside the shape is
    /// <b>surrounded</b>, that is, has a tile of the shape somewhere to its left, right, above
    /// and below inside the box (REQ-E3D-094).
    /// </summary>
    public bool IsFull()
    {
        for (int x = TileStart.X + 1; x < TileEnd.X; x++)
        {
            for (int y = TileStart.Y + 1; y < TileEnd.Y; y++)
            {
                int index = ConversionGrid.Index(x, y);
                if (TileAt(index) is not null)
                {
                    continue;
                }

                if (IsSurrounded(x, y))
                {
                    return false;
                }
            }
        }

        return true;
    }

    /// <summary>
    /// The tile of this shape at grid index <paramref name="index"/>, or null when the cell
    /// holds no tile of this shape.
    /// </summary>
    internal FrameTile? TileAt(int index)
    {
        if ((uint)index >= (uint)Grid.Length)
        {
            return null;
        }

        FrameTile? tile = Grid[index];
        return tile is not null && ReferenceEquals(tile.Shape, this) ? tile : null;
    }

    internal void Reset(FrameTile?[] grid, bool background, Cell mask)
    {
        Grid = grid;
        Background = background;
        Mask = mask;
        _tiles.Clear();
        _images.Clear();
        _instances.Clear();
        PixelCount = 0;
        Palette = 0;
        Hidden = false;
        TileStart = default;
        TileEnd = default;
        PixelStart = default;
        PixelEnd = default;
        Center = default;
        ReferencePattern = null;
        Scriptable = false;
        RestoredFromPinnedFrame = false;
        ConstraintSize = new Cell(99, 99);
        SelfConnected = true;
        SkipGeometry = false;
    }

    internal void AddTile(FrameTile tile)
    {
        tile.Shape = this;
        if ((uint)tile.GridIndex < (uint)Grid.Length)
        {
            Grid[tile.GridIndex] = tile;
        }

        _tiles.Add(tile);
        PixelCount += tile.Image.PixelCount;

        if (!Background)
        {
            Hidden |= tile.Hidden;
        }

        TileImage image = tile.Image;
        if (!_images.Contains(image))
        {
            _images.Add(image);
        }

        if (_tiles.Count == 1)
        {
            Palette = tile.Palette + (Background ? 0 : 4);
            TileStart = tile.Position;
            TileEnd = tile.Position;
            PixelStart = new Cell(tile.X + image.PixelStart.X, tile.Y + image.PixelStart.Y);
            PixelEnd = new Cell(tile.X + image.PixelEnd.X, tile.Y + image.PixelEnd.Y);
            return;
        }

        TileStart = new Cell(
            Math.Min(TileStart.X, tile.Position.X),
            Math.Min(TileStart.Y, tile.Position.Y));
        TileEnd = new Cell(
            Math.Max(TileEnd.X, tile.Position.X),
            Math.Max(TileEnd.Y, tile.Position.Y));
        PixelStart = new Cell(
            Math.Min(PixelStart.X, tile.X + image.PixelStart.X),
            Math.Min(PixelStart.Y, tile.Y + image.PixelStart.Y));
        PixelEnd = new Cell(
            Math.Max(PixelEnd.X, tile.X + image.PixelEnd.X),
            Math.Max(PixelEnd.Y, tile.Y + image.PixelEnd.Y));
    }

    /// <summary>The tile box the shape would have if <paramref name="tile"/> joined it.</summary>
    internal Cell SizeWith(FrameTile tile)
    {
        if (_tiles.Count == 0)
        {
            return new Cell(1, 1);
        }

        int sx = Math.Min(TileStart.X, tile.Position.X);
        int sy = Math.Min(TileStart.Y, tile.Position.Y);
        int ex = Math.Max(TileEnd.X, tile.Position.X);
        int ey = Math.Max(TileEnd.Y, tile.Position.Y);
        return new Cell(ex - sx + 1, ey - sy + 1);
    }

    /// <summary>
    /// Binds the shape to <paramref name="pattern"/>: copies its recognition parameters into
    /// the frame pattern, creates one frame instance per pattern instance (REQ-E3D-190) and
    /// computes the screen centre.
    /// </summary>
    internal void Bind(EnginePattern pattern)
    {
        ReferencePattern = pattern;
        ConstraintSize = pattern.ConstraintSize;
        SelfConnected = pattern.SelfConnected;
        SkipGeometry = pattern.SkipGeometry;

        _instances.Clear();
        for (int k = 0; k < pattern.Instances.Count; k++)
        {
            FrameInstance instance = new(this);
            instance.CopyFrom(pattern.Instances[k]);
            instance.Reference = pattern.Instances[k];
            instance.Index = k;
            _instances.Add(instance);
        }

        Center = new Vec3(
            (PixelStart.X + PixelEnd.X + 1) / 2f,
            (PixelStart.Y + PixelEnd.Y + 1) / 2f,
            0f);
    }

    /// <summary>
    /// Releases the shape's tiles so that another germ can take them: the tiles of a shape
    /// rejected by pass 1 (REQ-E3D-092) or dropped by the band extraction of 2D mode
    /// (REQ-E3D-381), both of which are background passes.
    /// </summary>
    /// <remarks>
    /// The grid entry is left alone on purpose. For a background shape the grid <b>is</b> the
    /// list of tiles kept in the batch, which a release must not empty; for a sprite shape the
    /// entry is the cell occupancy of REQ-E3D-132, which lasts for the rest of the frame.
    /// </remarks>
    internal void ReleaseTiles()
    {
        foreach (FrameTile tile in _tiles)
        {
            if (ReferenceEquals(tile.Shape, this))
            {
                tile.Shape = null;
            }
        }

        _tiles.Clear();
        _images.Clear();
        PixelCount = 0;
    }

    /// <inheritdoc/>
    public override string ToString()
        => $"{(Background ? "bg" : "sp")} shape {TileCount} tiles, box {TileStart}-{TileEnd}";

    private bool IsSurrounded(int x, int y)
    {
        bool left = false;
        for (int i = TileStart.X; i < x && !left; i++)
        {
            left = TileAt(ConversionGrid.Index(i, y)) is not null;
        }

        if (!left)
        {
            return false;
        }

        bool right = false;
        for (int i = x + 1; i <= TileEnd.X && !right; i++)
        {
            right = TileAt(ConversionGrid.Index(i, y)) is not null;
        }

        if (!right)
        {
            return false;
        }

        bool above = false;
        for (int j = TileStart.Y; j < y && !above; j++)
        {
            above = TileAt(ConversionGrid.Index(x, j)) is not null;
        }

        if (!above)
        {
            return false;
        }

        for (int j = y + 1; j <= TileEnd.Y; j++)
        {
            if (TileAt(ConversionGrid.Index(x, j)) is not null)
            {
                return true;
            }
        }

        return false;
    }
}
