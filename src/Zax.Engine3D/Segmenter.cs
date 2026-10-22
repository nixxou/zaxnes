// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// The segmentation of a frame: the three background passes (REQ-E3D-092), the two sprite
/// passes (REQ-E3D-131) and the constrained growth they share (REQ-E3D-090).
/// </summary>
/// <remarks>
/// <para>
/// Every buffer is reused between frames; the segmentation of a full frame allocates nothing
/// once warm (REQ-E3D-360).
/// </para>
/// <para>
/// The greedy shrinking of the candidate set depends on the order the tiles are explored. That
/// is deliberate and required for the built-in profiles to be recognised as they are
/// (REQ-E3D-090 notes).
/// </para>
/// </remarks>
internal sealed class Segmenter
{
    private static readonly Comparison<FrameTile> ByRowThenColumn =
        static (a, b) => a.Y != b.Y ? a.Y.CompareTo(b.Y) : a.X.CompareTo(b.X);

    private static readonly Comparison<FrameTile> ByColumnThenRow =
        static (a, b) => a.X != b.X ? a.X.CompareTo(b.X) : a.Y.CompareTo(b.Y);

    private readonly PatternRegistry _registry;
    private readonly ConversionSettings _settings;
    private readonly Func<FrameShape, bool, EnginePattern> _learn;

    private readonly List<FrameTile> _germs = [];
    private readonly Queue<FrameTile> _queue = new();
    private readonly List<FrameShape> _rejected = [];
    private List<EnginePattern> _candidates = [];
    private List<EnginePattern> _nextCandidates = [];

    /// <summary>Creates a segmenter.</summary>
    /// <param name="registry">The pattern registry.</param>
    /// <param name="settings">The settings the pipeline consumes.</param>
    /// <param name="learn">
    /// Learns a pattern from a shape (REQ-E3D-161): registers it, runs the build hook and
    /// starts its geometry. The second argument says whether the pattern is permanent.
    /// </param>
    public Segmenter(
        PatternRegistry registry,
        ConversionSettings settings,
        Func<FrameShape, bool, EnginePattern> learn)
    {
        _registry = registry;
        _settings = settings;
        _learn = learn;
    }

    /// <summary>
    /// Segments the background then the sprites of <paramref name="frame"/> (REQ-E3D-001 c).
    /// </summary>
    public void Run(ConversionFrame frame, bool twoDimensionalMode)
    {
        SegmentBackground(frame, twoDimensionalMode);
        frame.BackgroundShapeCount = frame.Shapes.Count;
        SegmentSprites(frame, twoDimensionalMode);
    }

    private void SegmentBackground(ConversionFrame frame, bool twoDimensionalMode)
    {
        BuildGerminationOrder(frame);

        // Pass 1: large known regions. A shape is kept when it is full with at least 8 tiles
        // and 2 distinct images, or when it is big; otherwise it is rejected and its tiles stay
        // unavailable until the end of the pass (REQ-E3D-092).
        _rejected.Clear();
        foreach (FrameTile germ in _germs)
        {
            if (!germ.IsFree || !germ.Image.IsKnown || !InitialiseCandidates(germ))
            {
                continue;
            }

            FrameShape shape = GrowBackground(frame, germ);
            bool keep = (shape.IsFull() && shape.TileCount >= 8 && shape.Images.Count >= 2)
                || shape.IsBig;
            if (keep)
            {
                BindToChosenCandidate(shape);
                frame.AddShape(shape);
            }
            else
            {
                _rejected.Add(shape);
            }
        }

        foreach (FrameShape shape in _rejected)
        {
            frame.ReturnShape(shape);
        }

        _rejected.Clear();

        // Pass 2: tiles whose image is unknown at the moment they are visited (REQ-E3D-092).
        foreach (FrameTile germ in _germs)
        {
            if (!germ.IsFree || germ.Image.IsKnown)
            {
                continue;
            }

            FrameShape shape = frame.TakeShape(background: true);
            shape.Scriptable = true;
            shape.AddTile(germ);

            if (twoDimensionalMode)
            {
                GrowUnknownBackgroundIn2DMode(frame, shape, germ);
                shape = ExtractBand(frame, shape);
                if (shape.TileCount == 0)
                {
                    frame.ReturnShape(shape);
                    continue;
                }
            }

            BindUnknownShape(shape);
            frame.AddShape(shape);
        }

        // Pass 3: the rest of the known tiles; always kept (REQ-E3D-092, REQ-E3D-095).
        foreach (FrameTile germ in _germs)
        {
            if (!germ.IsFree || !germ.Image.IsKnown || !InitialiseCandidates(germ))
            {
                continue;
            }

            FrameShape shape = GrowBackground(frame, germ);
            BindToChosenCandidate(shape);
            frame.AddShape(shape);
        }
    }

    private void SegmentSprites(ConversionFrame frame, bool twoDimensionalMode)
    {
        // Pass S1: groups of sprites whose image is unknown when they are visited; free growth
        // with an exact adjacency and the same palette (REQ-E3D-131).
        foreach (FrameTile germ in frame.Sprites)
        {
            if (!germ.IsFree || germ.Image.IsKnown)
            {
                continue;
            }

            FrameShape shape = frame.TakeShape(background: false);
            shape.Scriptable = true;
            shape.AddTile(germ);

            _queue.Clear();
            _queue.Enqueue(germ);
            while (_queue.Count > 0)
            {
                FrameTile tile = _queue.Dequeue();
                foreach (FrameTile neighbour in frame.Sprites)
                {
                    if (!neighbour.IsFree
                        || neighbour.Palette != tile.Palette
                        || Adjacency(tile, neighbour, 0) < 0
                        || frame.SpriteOccupant(neighbour.GridIndex) is not null)
                    {
                        continue;
                    }

                    // 2D mode keeps one tile per image in the group (REQ-E3D-381).
                    if (twoDimensionalMode
                        && (neighbour.Image.IsKnown || shape.Images.Contains(neighbour.Image)))
                    {
                        continue;
                    }

                    shape.AddTile(neighbour);
                    _queue.Enqueue(neighbour);
                }
            }

            BindUnknownShape(shape);
            frame.AddShape(shape);
        }

        // Pass S2: every sprite still free, constrained growth over all free sprites
        // (REQ-E3D-131).
        foreach (FrameTile germ in frame.Sprites)
        {
            if (!germ.IsFree || !InitialiseCandidates(germ))
            {
                continue;
            }

            FrameShape shape = frame.TakeShape(background: false);
            shape.Scriptable = true;
            shape.AddTile(germ);

            _queue.Clear();
            _queue.Enqueue(germ);
            while (_queue.Count > 0)
            {
                FrameTile tile = _queue.Dequeue();
                foreach (FrameTile neighbour in frame.Sprites)
                {
                    if (!neighbour.IsFree)
                    {
                        continue;
                    }

                    int direction = Adjacency(tile, neighbour, _settings.SpriteTolerance);
                    if (direction < 0)
                    {
                        continue;
                    }

                    if (_settings.SameSpritePalette && neighbour.Palette != tile.Palette)
                    {
                        continue;
                    }

                    if (!ShrinkCandidates(shape, tile, neighbour, (GrowDirection)direction))
                    {
                        continue;
                    }

                    shape.AddTile(neighbour);
                    _queue.Enqueue(neighbour);
                }
            }

            BindToChosenCandidate(shape);
            frame.AddShape(shape);
        }
    }

    /// <summary>
    /// The germination order of the background tiles (REQ-E3D-006): by y then x when
    /// <c>ScanDirection</c> is 0, by x then y when it is 1.
    /// </summary>
    private void BuildGerminationOrder(ConversionFrame frame)
    {
        _germs.Clear();
        IReadOnlyList<FrameTile?> grid = frame.BackgroundGrid;
        for (int i = 0; i < grid.Count; i++)
        {
            if (grid[i] is { } tile)
            {
                _germs.Add(tile);
            }
        }

        // Two tiles of one batch that land in the same cell share one slot (Q-012), so the
        // sort keys are unique and the order is total.
        _germs.Sort(_settings.ScanDirection == 1 ? ByColumnThenRow : ByRowThenColumn);
    }

    /// <summary>
    /// Constrained growth over the background grid: the 4-neighbourhood in the order right,
    /// down, up, left, a FIFO queue, and the greedy shrinking of the candidate set
    /// (REQ-E3D-090).
    /// </summary>
    private FrameShape GrowBackground(ConversionFrame frame, FrameTile germ)
    {
        FrameShape shape = frame.TakeShape(background: true);
        shape.Scriptable = true;
        shape.AddTile(germ);

        _queue.Clear();
        _queue.Enqueue(germ);
        ReadOnlySpan<int> offsets = ConversionGrid.NeighbourOffsets;
        while (_queue.Count > 0)
        {
            FrameTile tile = _queue.Dequeue();
            for (int d = 0; d < offsets.Length; d++)
            {
                FrameTile? neighbour = frame.BackgroundAt(tile.GridIndex + offsets[d]);
                if (neighbour is null || !neighbour.IsFree)
                {
                    continue;
                }

                if (_settings.SameBackgroundPalette && neighbour.Palette != tile.Palette)
                {
                    continue;
                }

                if (!ShrinkCandidates(shape, tile, neighbour, (GrowDirection)d))
                {
                    continue;
                }

                shape.AddTile(neighbour);
                _queue.Enqueue(neighbour);
            }
        }

        return shape;
    }

    /// <summary>
    /// Growth of an unknown background tile in 2D mode (REQ-E3D-381): 4-neighbourhood, same
    /// palette, free, unknown, not CHAR, and an image the shape does not already carry.
    /// </summary>
    private void GrowUnknownBackgroundIn2DMode(ConversionFrame frame, FrameShape shape, FrameTile germ)
    {
        if (germ.Image.Shape2D == ImageShape2D.Char)
        {
            return;
        }

        _queue.Clear();
        _queue.Enqueue(germ);
        ReadOnlySpan<int> offsets = ConversionGrid.NeighbourOffsets;
        while (_queue.Count > 0)
        {
            FrameTile tile = _queue.Dequeue();
            for (int d = 0; d < offsets.Length; d++)
            {
                FrameTile? neighbour = frame.BackgroundAt(tile.GridIndex + offsets[d]);
                if (neighbour is null
                    || !neighbour.IsFree
                    || neighbour.Image.IsKnown
                    || neighbour.Image.Shape2D == ImageShape2D.Char
                    || neighbour.Palette != tile.Palette
                    || shape.Images.Contains(neighbour.Image))
                {
                    continue;
                }

                shape.AddTile(neighbour);
                _queue.Enqueue(neighbour);
            }
        }
    }

    /// <summary>
    /// The band extraction of 2D mode (REQ-E3D-381): when the shape is big and does not fill
    /// its tile box, keep the first run of entirely full rows (or columns), release the other
    /// tiles and extend the band to free tiles whose image it already carries.
    /// </summary>
    /// <returns>The shape to bind, which may be a new one.</returns>
    private FrameShape ExtractBand(ConversionFrame frame, FrameShape shape)
    {
        Cell size = shape.TileSize;
        if (!shape.IsBig || shape.TileCount == size.X * size.Y)
        {
            return shape;
        }

        FrameShape band = frame.TakeShape(background: true);
        band.Scriptable = true;

        bool begun = false;
        if (size.X >= size.Y)
        {
            for (int y = shape.TileStart.Y; y <= shape.TileEnd.Y; y++)
            {
                bool full = true;
                for (int x = shape.TileStart.X; x <= shape.TileEnd.X && full; x++)
                {
                    full = shape.TileAt(ConversionGrid.Index(x, y)) is not null;
                }

                if (full)
                {
                    for (int x = shape.TileStart.X; x <= shape.TileEnd.X; x++)
                    {
                        band.AddTile(shape.TileAt(ConversionGrid.Index(x, y))!);
                    }

                    begun = true;
                }
                else if (begun)
                {
                    break;
                }
            }
        }
        else
        {
            for (int x = shape.TileStart.X; x <= shape.TileEnd.X; x++)
            {
                bool full = true;
                for (int y = shape.TileStart.Y; y <= shape.TileEnd.Y && full; y++)
                {
                    full = shape.TileAt(ConversionGrid.Index(x, y)) is not null;
                }

                if (full)
                {
                    for (int y = shape.TileStart.Y; y <= shape.TileEnd.Y; y++)
                    {
                        band.AddTile(shape.TileAt(ConversionGrid.Index(x, y))!);
                    }

                    begun = true;
                }
                else if (begun)
                {
                    break;
                }
            }
        }

        if (band.TileCount == 0)
        {
            frame.ReturnShape(band);
            return shape;
        }

        frame.ReturnShape(shape);
        ExpandBand(frame, band);
        return band;
    }

    private void ExpandBand(ConversionFrame frame, FrameShape band)
    {
        _queue.Clear();
        foreach (FrameTile tile in band.Tiles)
        {
            _queue.Enqueue(tile);
        }

        ReadOnlySpan<int> offsets = ConversionGrid.NeighbourOffsets;
        while (_queue.Count > 0)
        {
            FrameTile tile = _queue.Dequeue();
            for (int d = 0; d < offsets.Length; d++)
            {
                FrameTile? neighbour = frame.BackgroundAt(tile.GridIndex + offsets[d]);
                if (neighbour is null || !neighbour.IsFree || !band.Images.Contains(neighbour.Image))
                {
                    continue;
                }

                band.AddTile(neighbour);
                _queue.Enqueue(neighbour);
            }
        }
    }

    /// <summary>
    /// The initial candidate list of a germ: the non-disabled permanent patterns that contain
    /// its image, in registration order (REQ-E3D-091). False when there is none, in which case
    /// the germ stays free and is not displayed (REQ-E3D-093).
    /// </summary>
    private bool InitialiseCandidates(FrameTile germ)
    {
        _candidates.Clear();
        foreach (EnginePattern pattern in germ.Image.PermanentPatterns)
        {
            if (pattern.IsCandidate)
            {
                _candidates.Add(pattern);
            }
        }

        return _candidates.Count > 0;
    }

    /// <summary>
    /// Keeps the candidates that allow adding <paramref name="neighbour"/> from
    /// <paramref name="tile"/> (REQ-E3D-062). The set is only replaced when it stays non-empty
    /// (REQ-E3D-090 point 4).
    /// </summary>
    private bool ShrinkCandidates(
        FrameShape shape,
        FrameTile tile,
        FrameTile neighbour,
        GrowDirection direction)
    {
        Cell size = shape.SizeWith(neighbour);
        _nextCandidates.Clear();
        foreach (EnginePattern pattern in _candidates)
        {
            if (pattern.AllowsDirection(tile.Image, neighbour.Image, direction)
                && pattern.FitsConstraint(size.X, size.Y))
            {
                _nextCandidates.Add(pattern);
            }
        }

        if (_nextCandidates.Count == 0)
        {
            return false;
        }

        (_candidates, _nextCandidates) = (_nextCandidates, _candidates);
        return true;
    }

    /// <summary>
    /// Binds the shape to the candidate with the fewest distinct images, the first in candidate
    /// order on a tie (REQ-E3D-090, REQ-E3D-095).
    /// </summary>
    private void BindToChosenCandidate(FrameShape shape)
    {
        EnginePattern chosen = _candidates[0];
        foreach (EnginePattern pattern in _candidates)
        {
            if (pattern.Images.Count < chosen.Images.Count)
            {
                chosen = pattern;
            }
        }

        shape.Bind(chosen);
    }

    /// <summary>
    /// Binds a shape built from unknown tiles (background pass 2 or sprite pass S1), learning a
    /// permanent or a temporary pattern when the registry has none (REQ-E3D-160).
    /// </summary>
    private void BindUnknownShape(FrameShape shape)
    {
        ImageSetKey key = shape.Key;
        if (shape.IsFinished)
        {
            EnginePattern pattern = _registry.FindPermanent(key) ?? _learn(shape, true);
            shape.Bind(pattern);
            return;
        }

        EnginePattern temporary = _registry.FindTemporary(key)
            ?? _registry.FindTemporaryContaining(key)
            ?? _learn(shape, false);
        shape.Bind(temporary);
    }

    /// <summary>
    /// The direction of <paramref name="neighbour"/> relative to <paramref name="tile"/> for
    /// two sprites with tolerance <paramref name="tolerance"/>, or −1 when they are not
    /// adjacent (REQ-E3D-130). The four tests are made in the order right, left, down, up and
    /// the first that fits wins.
    /// </summary>
    private static int Adjacency(FrameTile tile, FrameTile neighbour, int tolerance)
    {
        int dx = Math.Abs(neighbour.X - tile.X);
        int dy = Math.Abs(neighbour.Y - tile.Y);

        if (dy <= tolerance && Math.Abs(tile.X + 8 - neighbour.X) <= tolerance)
        {
            return (int)GrowDirection.Right;
        }

        if (dy <= tolerance && Math.Abs(neighbour.X + 8 - tile.X) <= tolerance)
        {
            return (int)GrowDirection.Left;
        }

        if (dx <= tolerance && Math.Abs(tile.Y + 8 - neighbour.Y) <= tolerance)
        {
            return (int)GrowDirection.Down;
        }

        if (dx <= tolerance && Math.Abs(neighbour.Y + 8 - tile.Y) <= tolerance)
        {
            return (int)GrowDirection.Up;
        }

        return -1;
    }
}
