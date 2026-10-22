using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>
/// Direction of a neighbour relative to a tile, in the examination order of REQ-E3D-090
/// (right, down, up, left). <c>3 − d</c> is the opposite direction.
/// </summary>
public enum GrowDirection
{
    /// <summary>The neighbour is to the right.</summary>
    Right = 0,

    /// <summary>The neighbour is below (screen y grows downwards).</summary>
    Down = 1,

    /// <summary>The neighbour is above.</summary>
    Up = 2,

    /// <summary>The neighbour is to the left.</summary>
    Left = 3,
}

/// <summary>
/// The key of a pattern in the registry: its set of distinct images (REQ-E3D-063). Layout,
/// grid size, number of occurrences and parameters are ignored.
/// </summary>
public readonly struct ImageSetKey : IEquatable<ImageSetKey>
{
    private readonly int[] _sortedIds;
    private readonly int _hash;

    /// <summary>Builds the key of a set of images, in any order.</summary>
    public ImageSetKey(IEnumerable<TileImage> images)
    {
        ArgumentNullException.ThrowIfNull(images);
        List<int> ids = [];
        foreach (TileImage image in images)
        {
            if (!ids.Contains(image.Id))
            {
                ids.Add(image.Id);
            }
        }

        ids.Sort();
        _sortedIds = [.. ids];

        HashCode hash = default;
        foreach (int id in _sortedIds)
        {
            hash.Add(id);
        }

        _hash = hash.ToHashCode();
    }

    /// <summary>Number of distinct images, <c>|Img(P)|</c>.</summary>
    public int Count => _sortedIds?.Length ?? 0;

    /// <summary>The image identities, ascending.</summary>
    public ReadOnlySpan<int> Ids => _sortedIds ?? [];

    /// <summary>True when this set is a subset of <paramref name="other"/> (REQ-E3D-065).</summary>
    public bool IsSubsetOf(ImageSetKey other)
    {
        ReadOnlySpan<int> mine = Ids;
        ReadOnlySpan<int> theirs = other.Ids;
        int j = 0;
        foreach (int id in mine)
        {
            while (j < theirs.Length && theirs[j] < id)
            {
                j++;
            }

            if (j >= theirs.Length || theirs[j] != id)
            {
                return false;
            }

            j++;
        }

        return true;
    }

    /// <inheritdoc/>
    public bool Equals(ImageSetKey other)
    {
        if (_hash != other._hash || Count != other.Count)
        {
            return false;
        }

        return Ids.SequenceEqual(other.Ids);
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is ImageSetKey other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode() => _hash;

    /// <summary>Equality.</summary>
    public static bool operator ==(ImageSetKey left, ImageSetKey right) => left.Equals(right);

    /// <summary>Inequality.</summary>
    public static bool operator !=(ImageSetKey left, ImageSetKey right) => !left.Equals(right);
}

/// <summary>
/// A pattern of the registry: a grid of images, its recognition parameters and its ordered
/// instances (SPEC-04 §2, REQ-E3D-060 to 065).
/// </summary>
public sealed class EnginePattern
{
    private readonly TileImage?[] _cells;
    private readonly List<TileImage> _images = [];
    private readonly HashSet<long> _horizontal = [];
    private readonly HashSet<long> _vertical = [];
    private readonly List<PatternInstance> _instances = [];

    /// <summary>Builds a pattern over a grid of images given row by row.</summary>
    /// <param name="width">Grid width in cells.</param>
    /// <param name="height">Grid height in cells.</param>
    /// <param name="cells">Cells, row by row; a null entry is an empty cell.</param>
    /// <param name="permanent">Whether the pattern is permanent.</param>
    public EnginePattern(int width, int height, IEnumerable<TileImage?> cells, bool permanent)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(width);
        ArgumentOutOfRangeException.ThrowIfNegative(height);
        ArgumentNullException.ThrowIfNull(cells);

        Width = width;
        Height = height;
        Permanent = permanent;
        _cells = new TileImage?[width * height];
        int index = 0;
        foreach (TileImage? cell in cells)
        {
            if (index >= _cells.Length)
            {
                break;
            }

            _cells[index++] = cell;
        }

        foreach (TileImage? cell in _cells)
        {
            if (cell is not null && !_images.Contains(cell))
            {
                _images.Add(cell);
            }
        }

        Key = new ImageSetKey(_images);
        BuildRelations();
        BuildPixelBox();
    }

    /// <summary>Grid width in cells (<c>W</c>).</summary>
    public int Width { get; }

    /// <summary>Grid height in cells (<c>H</c>).</summary>
    public int Height { get; }

    /// <summary>Whether the pattern is permanent (as opposed to temporary).</summary>
    public bool Permanent { get; }

    /// <summary>The registry key: the set of distinct images (REQ-E3D-063).</summary>
    public ImageSetKey Key { get; }

    /// <summary>
    /// The distinct images of the grid, in first-occurrence order; <c>Img(P)</c>.
    /// <c>Count</c> is <c>|Img(P)|</c>, the tie-break of REQ-E3D-090.
    /// </summary>
    public IReadOnlyList<TileImage> Images => _images;

    /// <summary>The ordered instances; a pattern of a file always has at least one.</summary>
    public IReadOnlyList<PatternInstance> Instances => _instances;

    /// <summary>Internal connectivity option (<c>selfConnected</c>, REQ-E3D-061).</summary>
    public bool SelfConnected { get; private set; } = true;

    /// <summary>Skip the geometric adjacency test (<c>skipGeo</c>, REQ-E3D-062).</summary>
    public bool SkipGeometry { get; set; }

    /// <summary>Size constraint; <c>(99, 99)</c> means unconstrained (REQ-E3D-062).</summary>
    public Cell ConstraintSize { get; set; } = new(99, 99);

    /// <summary>
    /// Script flag that takes the pattern out of the candidates (REQ-E3D-093). Never saved.
    /// </summary>
    public bool Disabled { get; set; }

    /// <summary>
    /// Order of registration in the registry, which is the candidate order of REQ-E3D-091.
    /// </summary>
    public int RegistrationIndex { get; internal set; }

    /// <summary>Top-left corner of the pattern pixel box, in grid pixels (REQ-E3D-064).</summary>
    public Cell PixelStart { get; private set; }

    /// <summary>Bottom-right corner of the pattern pixel box, inclusive (REQ-E3D-064).</summary>
    public Cell PixelEnd { get; private set; }

    /// <summary>
    /// <c>BottomLeft(P) = (pStart.x, 8H − pEnd.y − 1)</c> (REQ-E3D-064), used by
    /// <c>Frame:InsertShape</c> (REQ-E3D-232).
    /// </summary>
    public Vec2 BottomLeft => new(PixelStart.X, (8 * Height) - PixelEnd.Y - 1);

    /// <summary>
    /// <c>TopRight(P) = (pEnd.x, 8W − pStart.y − 1)</c> (REQ-E3D-064). The y component uses the
    /// grid <b>width</b>: a quirk of the original, kept as is (DEC-04-Q04, Q-04).
    /// </summary>
    public Vec2 TopRight => new(PixelEnd.X, (8 * Width) - PixelStart.Y - 1);

    /// <summary>The script layer's handle cache (SPEC-05 REQ-LUA-043).</summary>
    public ScriptHandleSlot ScriptHandle;

    /// <summary>Stored pixel count of the profile, kept as read (REQ-E3D-341).</summary>
    public int StoredPixelCount { get; set; }

    /// <summary>Stored colour flags of the profile, kept as read (REQ-E3D-341).</summary>
    public int StoredColorFlags { get; set; }

    /// <summary>Stored tile density of the profile, kept as read (REQ-E3D-341).</summary>
    public int StoredTileDensity { get; set; }

    /// <summary>Stored pixel density of the profile, kept as read (REQ-E3D-341).</summary>
    public int StoredPixelDensity { get; set; }

    /// <summary>The image of cell (<paramref name="i"/>, <paramref name="j"/>), or null.</summary>
    public TileImage? ImageAt(int i, int j)
    {
        if ((uint)i >= (uint)Width || (uint)j >= (uint)Height)
        {
            return null;
        }

        return _cells[i + (j * Width)];
    }

    /// <summary>Adds an instance at the end of the list.</summary>
    public void AddInstance(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        instance.Index = _instances.Count;
        instance.Reference = instance;
        _instances.Add(instance);
    }

    /// <summary>True when the pattern may be a candidate at all (REQ-E3D-093).</summary>
    public bool IsCandidate => Permanent && !Disabled;

    /// <summary>
    /// Whether the pattern <b>allows</b> adding a neighbour of image <paramref name="target"/>
    /// in direction <paramref name="direction"/> from a tile of image
    /// <paramref name="source"/> (REQ-E3D-062, conditions 1 and 2). The size constraint of
    /// condition 3 is <see cref="FitsConstraint"/>.
    /// </summary>
    public bool AllowsDirection(TileImage source, TileImage target, GrowDirection direction)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(target);

        if (SkipGeometry)
        {
            return _images.Contains(target);
        }

        return direction switch
        {
            GrowDirection.Right => _horizontal.Contains(Pair(source.Id, target.Id)),
            GrowDirection.Left => _horizontal.Contains(Pair(target.Id, source.Id)),
            GrowDirection.Down => _vertical.Contains(Pair(source.Id, target.Id)),
            _ => _vertical.Contains(Pair(target.Id, source.Id)),
        };
    }

    /// <summary>The size constraint of REQ-E3D-062 condition 3.</summary>
    public bool FitsConstraint(int width, int height)
        => width <= ConstraintSize.X && height <= ConstraintSize.Y;

    /// <summary>True when <c>(source, target)</c> is in the horizontal relation R(P).</summary>
    public bool HasHorizontalPair(TileImage source, TileImage target)
        => _horizontal.Contains(Pair(source.Id, target.Id));

    /// <summary>True when <c>(source, target)</c> is in the vertical relation D(P).</summary>
    public bool HasVerticalPair(TileImage source, TileImage target)
        => _vertical.Contains(Pair(source.Id, target.Id));

    /// <summary>Number of pairs in the horizontal relation R(P).</summary>
    public int HorizontalPairCount => _horizontal.Count;

    /// <summary>Number of pairs in the vertical relation D(P).</summary>
    public int VerticalPairCount => _vertical.Count;

    /// <summary>
    /// Reads the recognition parameters of a profile pattern (SPEC-03 §4.3) and builds the
    /// registry pattern, interning its images on the way (REQ-E3D-060).
    /// </summary>
    public static EnginePattern FromProfile(ProfilePattern pattern, ImageRegistry images, bool permanent)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        ArgumentNullException.ThrowIfNull(images);

        PatternGrid? grid = pattern.Grid;
        int width = grid?.Width ?? pattern.TileSize?.X ?? 0;
        int height = grid?.Height ?? pattern.TileSize?.Y ?? 0;
        TileImage?[] cells = new TileImage?[Math.Max(0, width * height)];
        if (grid is not null)
        {
            for (int j = 0; j < height; j++)
            {
                for (int i = 0; i < width; i++)
                {
                    ProfileTile? tile = grid[i, j];
                    if (tile is null)
                    {
                        continue;
                    }

                    cells[i + (j * width)] = images.Intern(
                        tile.LowPlane,
                        tile.HighPlane,
                        tile.FlipX,
                        tile.FlipY);
                }
            }
        }

        EnginePattern result = new(width, height, cells, permanent)
        {
            SkipGeometry = pattern.SkipGeometry,
            ConstraintSize = new Cell(pattern.ConstraintSize.X, pattern.ConstraintSize.Y),
            StoredPixelCount = pattern.PixelCount,
            StoredColorFlags = (int)pattern.ColorFlags,
            StoredTileDensity = (int)pattern.TileDensity,
            StoredPixelDensity = (int)pattern.PixelDensity,
        };

        result.SetSelfConnected(pattern.SelfConnected);

        foreach (PatternVariant variant in pattern.Variants)
        {
            result.AddInstance(PatternInstance.FromVariant(variant));
        }

        return result;
    }

    /// <summary>
    /// Sets <c>selfConnected</c> and rebuilds the adjacency relations, which depend on it
    /// (REQ-E3D-061).
    /// </summary>
    public void SetSelfConnected(bool value)
    {
        if (SelfConnected == value)
        {
            return;
        }

        SelfConnected = value;
        _horizontal.Clear();
        _vertical.Clear();
        BuildRelations();
    }

    /// <inheritdoc/>
    public override string ToString()
        => $"{(Permanent ? "pattern" : "temp")} #{RegistrationIndex} {Width}x{Height}, |Img|={Key.Count}";

    private static long Pair(int a, int b) => ((long)a << 32) | (uint)b;

    private void BuildRelations()
    {
        for (int j = 0; j < Height; j++)
        {
            for (int i = 0; i < Width; i++)
            {
                TileImage? here = ImageAt(i, j);
                if (here is null)
                {
                    continue;
                }

                AddPair(_horizontal, here, ImageAt(i + 1, j));
                AddPair(_vertical, here, ImageAt(i, j + 1));
                if (SelfConnected)
                {
                    AddPair(_horizontal, here, ImageAt((i + 1) % Width, j));
                    AddPair(_vertical, here, ImageAt(i, (j + 1) % Height));
                }
            }
        }
    }

    private static void AddPair(HashSet<long> relation, TileImage source, TileImage? target)
    {
        if (target is not null)
        {
            relation.Add(Pair(source.Id, target.Id));
        }
    }

    private void BuildPixelBox()
    {
        int sx = int.MaxValue;
        int sy = int.MaxValue;
        int ex = int.MinValue;
        int ey = int.MinValue;

        for (int j = 0; j < Height; j++)
        {
            for (int i = 0; i < Width; i++)
            {
                TileImage? image = ImageAt(i, j);
                if (image is null || image.IsEmpty)
                {
                    continue;
                }

                sx = Math.Min(sx, (8 * i) + image.PixelStart.X);
                sy = Math.Min(sy, (8 * j) + image.PixelStart.Y);
                ex = Math.Max(ex, (8 * i) + image.PixelEnd.X);
                ey = Math.Max(ey, (8 * j) + image.PixelEnd.Y);
            }
        }

        if (sx == int.MaxValue)
        {
            PixelStart = new Cell(0, 0);
            PixelEnd = new Cell(0, 0);
            return;
        }

        PixelStart = new Cell(sx, sy);
        PixelEnd = new Cell(ex, ey);
    }
}
