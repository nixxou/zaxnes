// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// The frame model built from one batch of the core: its tiles, its ordered shapes, its
/// instances, its palette, its mask and its counter (REQ-E3D-040).
/// </summary>
/// <remarks>
/// A frame owns pools for its tiles and its shapes and is reused from frame to frame, so a
/// full frame of 1 023 background tiles and 128 sprites costs no allocation once warm
/// (REQ-E3D-360).
/// </remarks>
public sealed class ConversionFrame
{
    private readonly FrameTile[] _backgroundCells = new FrameTile[ConversionGrid.CellCount];
    private readonly FrameTile?[] _backgroundGrid = new FrameTile?[ConversionGrid.CellCount];
    private readonly FrameTile?[] _spriteGrid = new FrameTile?[ConversionGrid.CellCount];
    private readonly List<FrameTile> _spritePool = [];
    private readonly List<FrameTile> _sprites = [];
    private readonly List<FrameTile> _injectedPool = [];
    private readonly List<FrameShape> _shapePool = [];
    private readonly List<FrameShape> _shapes = [];
    private readonly List<FrameInstance> _instances = [];
    private readonly List<FrameInstance> _scriptableInstances = [];
    private readonly List<Rect> _pinnedRects = [];
    private int _injectedCount;

    /// <summary>Creates an empty frame.</summary>
    public ConversionFrame()
    {
        for (int y = 0; y < ConversionGrid.Height; y++)
        {
            for (int x = 0; x < ConversionGrid.Width; x++)
            {
                int index = ConversionGrid.Index(x, y);
                _backgroundCells[index] = new FrameTile(new Cell(x, y), index, background: true);
            }
        }
    }

    /// <summary>
    /// The background grid: a non-null cell holds the tile kept in that cell for this frame
    /// (REQ-E3D-005, REQ-E3D-041).
    /// </summary>
    public IReadOnlyList<FrameTile?> BackgroundGrid => _backgroundGrid;

    /// <summary>The sprites kept, in the emission order of the core (REQ-E3D-006).</summary>
    public IReadOnlyList<FrameTile> Sprites => _sprites;

    /// <summary>
    /// The ordered shapes: every background shape first, then every sprite shape
    /// (REQ-E3D-040).
    /// </summary>
    public IReadOnlyList<FrameShape> Shapes => _shapes;

    /// <summary>How many of the first shapes are background shapes (REQ-E3D-040).</summary>
    public int BackgroundShapeCount { get; internal set; }

    /// <summary>The frame instances, in shape order then by instance index (REQ-E3D-220).</summary>
    public IReadOnlyList<FrameInstance> Instances => _instances;

    /// <summary>
    /// The scriptable instances: those of the shapes the segmentation produced, in the same
    /// order (REQ-E3D-220). Injected and restored shapes are absent.
    /// </summary>
    public IReadOnlyList<FrameInstance> ScriptableInstances => _scriptableInstances;

    /// <summary>The palette of the frame, 64 entries (REQ-E3D-250).</summary>
    public FramePalette Palette { get; } = new();

    /// <summary>The unreliable border widths of the batch (SPEC-02 REQ-CORE-046).</summary>
    public Cell Mask { get; internal set; }

    /// <summary>Number of emulated frames since the game started (REQ-E3D-043).</summary>
    public int Counter { get; internal set; }

    /// <summary>The frame time, <c>Counter / 60</c> seconds whatever the region (REQ-E3D-043).</summary>
    public float Time => Counter / 60f;

    /// <summary>Number of background tiles kept.</summary>
    public int BackgroundTileCount { get; internal set; }

    /// <summary>Total number of non-transparent background pixels kept.</summary>
    public int BackgroundPixelCount { get; internal set; }

    /// <summary>
    /// How many Lua errors were swallowed while converting this frame (REQ-E3D-324).
    /// </summary>
    public int ScriptErrorCount { get; internal set; }

    /// <summary>
    /// The rectangles added by <c>Frame:AddRectFromPinnedFrame</c>, in scene coordinates
    /// (REQ-E3D-233).
    /// </summary>
    public IReadOnlyList<Rect> PinnedRects => _pinnedRects;

    /// <summary>The background tile of a cell, or null when the cell holds none.</summary>
    public FrameTile? BackgroundAt(Cell cell)
        => ConversionGrid.Contains(cell) ? _backgroundGrid[ConversionGrid.Index(cell)] : null;

    /// <summary>The background tile of a grid index, or null.</summary>
    public FrameTile? BackgroundAt(int index)
        => (uint)index < (uint)_backgroundGrid.Length ? _backgroundGrid[index] : null;

    /// <summary>The sprite that occupies a cell of the sprite grid, or null (REQ-E3D-132).</summary>
    public FrameTile? SpriteOccupant(int index)
        => (uint)index < (uint)_spriteGrid.Length ? _spriteGrid[index] : null;

    /// <summary>Empties the frame: no shape, no tile, counter 0 (REQ-E3D-281).</summary>
    public void Clear()
    {
        ReclaimShapes();
        _shapes.Clear();
        _injectedCount = 0;
        _instances.Clear();
        _scriptableInstances.Clear();
        _pinnedRects.Clear();
        _sprites.Clear();
        Array.Clear(_backgroundGrid);
        Array.Clear(_spriteGrid);
        BackgroundShapeCount = 0;
        BackgroundTileCount = 0;
        BackgroundPixelCount = 0;
        ScriptErrorCount = 0;
        Counter = 0;
        Mask = default;
        Palette.Clear();

        foreach (FrameTile tile in _backgroundCells)
        {
            tile.Reset();
        }

        foreach (FrameTile tile in _spritePool)
        {
            tile.Reset();
        }
    }

    /// <summary>Starts a new batch: same pools, no shape, no tile.</summary>
    internal void BeginBatch(int counter, Cell mask)
    {
        ReclaimShapes();
        _shapes.Clear();
        _injectedCount = 0;
        _instances.Clear();
        _scriptableInstances.Clear();
        _pinnedRects.Clear();
        _sprites.Clear();
        Array.Clear(_backgroundGrid);
        Array.Clear(_spriteGrid);
        BackgroundShapeCount = 0;
        BackgroundTileCount = 0;
        BackgroundPixelCount = 0;
        ScriptErrorCount = 0;
        Counter = counter;
        Mask = mask;

        foreach (FrameTile tile in _backgroundCells)
        {
            tile.Reset();
        }

        foreach (FrameTile tile in _spritePool)
        {
            tile.Reset();
        }
    }

    /// <summary>
    /// Places a background tile in its cell (REQ-E3D-005). When two tiles of the batch land in
    /// the same cell — which a mid-frame scroll change allows (SPEC-02 REQ-CORE-040) — the
    /// later one wins and the cell germinates once (Q-012).
    /// </summary>
    internal FrameTile PlaceBackground(TileImage image, int x, int y, int palette, bool hidden)
    {
        Cell cell = ConversionGrid.BackgroundCell(x, y);
        int index = ConversionGrid.Index(cell);
        FrameTile tile = _backgroundCells[index];
        if (tile.Present)
        {
            BackgroundPixelCount -= tile.Image.PixelCount;
        }
        else
        {
            tile.Present = true;
            BackgroundTileCount++;
        }

        tile.Image = image;
        tile.X = x;
        tile.Y = y;
        tile.Palette = palette;
        tile.Hidden = hidden;
        tile.Ord = -1;
        tile.Shape = null;
        BackgroundPixelCount += image.PixelCount;
        _backgroundGrid[index] = tile;
        return tile;
    }

    /// <summary>Appends a sprite tile in emission order (REQ-E3D-006).</summary>
    internal FrameTile AddSprite(TileImage image, int x, int y, int palette, bool hidden, int ord)
    {
        int slot = _sprites.Count;
        while (_spritePool.Count <= slot)
        {
            _spritePool.Add(new FrameTile(default, 0, background: false));
        }

        FrameTile tile = _spritePool[slot];
        tile.Image = image;
        tile.X = x;
        tile.Y = y;
        tile.Palette = palette;
        tile.Hidden = hidden;
        tile.Ord = ord;
        tile.Position = ConversionGrid.SpriteCell(x, y);
        tile.GridIndex = ConversionGrid.Index(tile.Position);
        tile.Present = true;
        tile.Shape = null;
        _sprites.Add(tile);
        return tile;
    }

    /// <summary>
    /// Gives back the shapes this frame owns. A shape restored from the pinned frame belongs
    /// to that frame and must survive (REQ-E3D-233).
    /// </summary>
    private void ReclaimShapes()
    {
        foreach (FrameShape shape in _shapes)
        {
            if (ReferenceEquals(shape.Owner, this))
            {
                _shapePool.Add(shape);
            }
        }
    }

    /// <summary>
    /// Takes a shape from the pool, bound to the grid of its layer. Its mask is the frame's,
    /// which <see cref="FrameShape.Inside"/> needs (REQ-E3D-045).
    /// </summary>
    internal FrameShape TakeShape(bool background)
    {
        FrameShape shape;
        if (_shapePool.Count > 0)
        {
            shape = _shapePool[^1];
            _shapePool.RemoveAt(_shapePool.Count - 1);
        }
        else
        {
            shape = new FrameShape();
        }

        shape.Reset(background ? _backgroundGrid : _spriteGrid, background, Mask);
        shape.Owner = this;
        return shape;
    }

    /// <summary>
    /// Appends a tile that no batch produced: the tiles of a shape a script injects
    /// (REQ-E3D-230 to 232). They never take part in a segmentation.
    /// </summary>
    internal FrameTile AddInjectedTile(TileImage image, int x, int y, int palette)
    {
        while (_injectedPool.Count <= _injectedCount)
        {
            _injectedPool.Add(new FrameTile(default, 0, background: false));
        }

        FrameTile tile = _injectedPool[_injectedCount++];
        tile.Image = image;
        tile.X = x;
        tile.Y = y;
        tile.Palette = palette;
        tile.Hidden = false;
        tile.Ord = -1;
        tile.Present = true;
        tile.Shape = null;
        tile.Position = ConversionGrid.SpriteCell(x, y);
        tile.GridIndex = ConversionGrid.Index(tile.Position);
        return tile;
    }

    /// <summary>
    /// Adds a shape another frame owns — one restored from the pinned frame — as a
    /// non-scriptable shape of this frame, without duplicates (REQ-E3D-233).
    /// </summary>
    internal bool AddForeignShape(FrameShape shape)
    {
        if (_shapes.Contains(shape))
        {
            return false;
        }

        shape.Scriptable = false;
        shape.RestoredFromPinnedFrame = true;
        AddShape(shape);
        return true;
    }

    /// <summary>Gives a shape back to the pool without adding it to the frame.</summary>
    internal void ReturnShape(FrameShape shape)
    {
        shape.ReleaseTiles();
        _shapePool.Add(shape);
    }

    /// <summary>
    /// Adds a shape and its instances to the frame, in retention order (REQ-E3D-092,
    /// REQ-E3D-220).
    /// </summary>
    internal void AddShape(FrameShape shape)
    {
        _shapes.Add(shape);
        foreach (FrameInstance instance in shape.Instances)
        {
            _instances.Add(instance);
            if (shape.Scriptable)
            {
                _scriptableInstances.Add(instance);
            }
        }
    }

    /// <summary>Adds a rectangle to the pinned-frame list (REQ-E3D-233).</summary>
    internal void AddPinnedRect(Rect rectangle) => _pinnedRects.Add(rectangle);

    /// <summary>
    /// True when the 8 × 8 cell of <paramref name="tile"/> overlaps one of the frame's
    /// rectangles; the test a restored shape's tiles must pass to be drawn (REQ-E3D-233).
    /// </summary>
    /// <remarks>
    /// The tile box is in screen pixels and the rectangles are in scene coordinates, so the
    /// y axis of the two is reversed. The mismatch is the original's and is kept as is
    /// (DEC-04-Q05, Q-05).
    /// </remarks>
    public bool TileOverlapsPinnedRects(FrameTile tile)
    {
        ArgumentNullException.ThrowIfNull(tile);
        Rect box = new(tile.X, tile.Y, 8, 8);
        foreach (Rect rectangle in _pinnedRects)
        {
            if (box.Overlaps(rectangle))
            {
                return true;
            }
        }

        return false;
    }
}
