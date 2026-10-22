namespace Zax.Engine3D;

/// <summary>
/// The identity of a tile image: its two bit planes and its flips (SPEC-04 §2).
/// </summary>
/// <remarks>
/// Two tiles have the same image if and only if the four components are equal, so the same
/// CHR data shown flipped is <b>another</b> image. This is the unit of identity of the whole
/// recognition (REQ-E3D-007, TV-E3D-10).
/// </remarks>
/// <param name="Low">Low bit plane, packed as in SPEC-02 REQ-CORE-034.</param>
/// <param name="High">High bit plane.</param>
/// <param name="FlipX">Horizontal flip.</param>
/// <param name="FlipY">Vertical flip.</param>
public readonly record struct TileImageKey(ulong Low, ulong High, bool FlipX, bool FlipY)
{
    /// <summary>The flips as the two-bit value of SPEC-03 (bit 0 horizontal, bit 1 vertical).</summary>
    public int Flip => (FlipX ? 1 : 0) | (FlipY ? 2 : 0);

    /// <summary>True when both planes are zero: the image draws nothing.</summary>
    public bool IsEmpty => (Low | High) == 0UL;

    /// <summary>
    /// Colour (0 to 3) of the pixel <b>displayed</b> at (<paramref name="px"/>,
    /// <paramref name="py"/>), flips applied. Colour 0 is transparent (SPEC-04 §2).
    /// </summary>
    public int ColorAt(int px, int py)
    {
        int sx = FlipX ? 7 - px : px;
        int sy = FlipY ? 7 - py : py;
        int bit = (sy << 3) | (7 - sx);
        return (int)((Low >> bit) & 1UL) | (int)(((High >> bit) & 1UL) << 1);
    }
}

/// <summary>Colours used by an image; colour 0 is transparent and has no flag.</summary>
[Flags]
public enum ImageColors
{
    /// <summary>No colour.</summary>
    None = 0,

    /// <summary>Colour 1 occurs.</summary>
    Color1 = 1 << 1,

    /// <summary>Colour 2 occurs.</summary>
    Color2 = 1 << 2,

    /// <summary>Colour 3 occurs.</summary>
    Color3 = 1 << 3,
}

/// <summary>Density class of an image (REQ-E3D-021).</summary>
public enum ImageDensity
{
    /// <summary>No opaque pixel.</summary>
    Empty = 0,

    /// <summary>1 to 15 opaque pixels.</summary>
    Low = 1,

    /// <summary>16 to 31.</summary>
    Average = 2,

    /// <summary>32 to 47.</summary>
    Medium = 3,

    /// <summary>48 to 63.</summary>
    High = 4,

    /// <summary>All 64 pixels.</summary>
    Solid = 5,
}

/// <summary>2D shape class of an image (REQ-E3D-021).</summary>
public enum ImageShape2D
{
    /// <summary>None of the other classes.</summary>
    Unidentified = 0,

    /// <summary>One colour and fewer than 48 opaque pixels; used only by 2D mode (REQ-E3D-381).</summary>
    Char = 1,

    /// <summary>The opaque pixels exactly fill the pixel box.</summary>
    Rect = 2,

    /// <summary>At least 60 opaque pixels.</summary>
    FullRect = 3,
}

/// <summary>
/// An image of the session's image registry: its identity, its derived attributes
/// (REQ-E3D-020, REQ-E3D-021) and the registry patterns that contain it.
/// </summary>
/// <remarks>
/// <para>
/// The lists of patterns are the recognition index: <see cref="PermanentPatterns"/> is the
/// initial candidate list of a tile carrying this image, in registration order
/// (REQ-E3D-091), and it decides whether the image is <see cref="IsKnown">known</see>
/// (SPEC-04 §2). Both lists are kept ordered, never sorted, so the result of the segmentation
/// does not depend on a hash order.
/// </para>
/// <para>
/// AMD-021 R2-008 amends REQ-E3D-091 and closes DEC-04-Q02: registration order is not the
/// whole story. When a pattern is taken out of the registry and another pattern carrying the
/// same image is registered afterwards, the newcomer takes <em>the place of the last pattern
/// removed</em> from this image rather than the end of the list. One place is remembered per
/// list, and filling it uses it up, so a second newcomer goes to the end again.
/// </para>
/// </remarks>
public sealed class TileImage
{
    private readonly List<EnginePattern> _permanentPatterns = [];
    private readonly List<EnginePattern> _temporaryPatterns = [];

    /// <summary>
    /// Place left free by the last permanent pattern removed from this image, or -1 when the
    /// next newcomer belongs at the end (R2-008).
    /// </summary>
    private int _freedPermanentPlace = -1;

    /// <summary>Same, for the temporary list.</summary>
    private int _freedTemporaryPlace = -1;

    internal TileImage(TileImageKey key, int id)
    {
        Key = key;
        Id = id;

        int count = 0;
        int sx = 7;
        int sy = 7;
        int ex = 0;
        int ey = 0;
        ImageColors colors = ImageColors.None;

        for (int px = 0; px < 8; px++)
        {
            for (int py = 0; py < 8; py++)
            {
                int color = key.ColorAt(px, py);
                if (color == 0)
                {
                    continue;
                }

                count++;
                sx = Math.Min(sx, px);
                sy = Math.Min(sy, py);
                ex = Math.Max(ex, px);
                ey = Math.Max(ey, py);
                colors |= (ImageColors)(1 << color);
            }
        }

        PixelCount = count;
        Colors = colors;
        PixelStart = new Cell(sx, sy);
        PixelEnd = new Cell(ex, ey);
        Density = ClassifyDensity(count);
        Shape2D = ClassifyShape(count, colors, PixelStart, PixelEnd);
    }

    /// <summary>The image identity.</summary>
    public TileImageKey Key { get; }

    /// <summary>
    /// Identity in the session's image registry: a small stable integer, in the order the
    /// images were first met (REQ-E3D-007).
    /// </summary>
    public int Id { get; }

    /// <summary>Number of non-transparent pixels, 0 to 64 (REQ-E3D-020).</summary>
    public int PixelCount { get; }

    /// <summary>The colours used, a subset of {1, 2, 3} (REQ-E3D-020).</summary>
    public ImageColors Colors { get; }

    /// <summary>
    /// Top-left corner of the pixel box; <c>(7, 7)</c> for an empty image (REQ-E3D-020).
    /// </summary>
    public Cell PixelStart { get; }

    /// <summary>
    /// Bottom-right corner of the pixel box, inclusive; <c>(0, 0)</c> for an empty image.
    /// </summary>
    public Cell PixelEnd { get; }

    /// <summary>Density class (REQ-E3D-021).</summary>
    public ImageDensity Density { get; }

    /// <summary>2D shape class (REQ-E3D-021).</summary>
    public ImageShape2D Shape2D { get; }

    /// <summary>True when the image has no opaque pixel.</summary>
    public bool IsEmpty => PixelCount == 0;

    /// <summary>Number of distinct colours used, 0 to 3.</summary>
    public int ColorCount => System.Numerics.BitOperations.PopCount((uint)Colors);

    /// <summary>
    /// Permanent registry patterns that contain this image, in registration order: the initial
    /// candidate list of REQ-E3D-091.
    /// </summary>
    public IReadOnlyList<EnginePattern> PermanentPatterns => _permanentPatterns;

    /// <summary>Temporary registry patterns that contain this image, in registration order.</summary>
    public IReadOnlyList<EnginePattern> TemporaryPatterns => _temporaryPatterns;

    /// <summary>
    /// True when at least one <b>permanent</b> pattern of the registry contains this image,
    /// disabled patterns included (SPEC-04 §2, REQ-E3D-093).
    /// </summary>
    public bool IsKnown => _permanentPatterns.Count > 0;

    /// <summary>Colour (0 to 3) of the displayed pixel at (x, y).</summary>
    public int ColorAt(int px, int py) => Key.ColorAt(px, py);

    internal void Attach(EnginePattern pattern)
    {
        List<EnginePattern> list = pattern.Permanent ? _permanentPatterns : _temporaryPatterns;
        if (list.Contains(pattern))
        {
            return;
        }

        ref int freed = ref (pattern.Permanent ? ref _freedPermanentPlace : ref _freedTemporaryPlace);

        // R2-008: the place of the last pattern removed from this image, if one is still free.
        // Q-059: occupying it uses it up, so a second newcomer goes to the end.
        if (freed >= 0 && freed <= list.Count)
        {
            list.Insert(freed, pattern);
            freed = -1;
            return;
        }

        freed = -1;
        list.Add(pattern);
    }

    internal void Detach(EnginePattern pattern)
    {
        List<EnginePattern> list = pattern.Permanent ? _permanentPatterns : _temporaryPatterns;
        int place = list.IndexOf(pattern);
        if (place < 0)
        {
            return;
        }

        list.RemoveAt(place);

        // R2-008: this is now the place the next newcomer takes.
        if (pattern.Permanent)
        {
            _freedPermanentPlace = place;
        }
        else
        {
            _freedTemporaryPlace = place;
        }
    }

    /// <inheritdoc/>
    public override string ToString()
        => $"Image #{Id} (n={PixelCount}, flip={Key.Flip})";

    private static ImageDensity ClassifyDensity(int count) => count switch
    {
        0 => ImageDensity.Empty,
        < 16 => ImageDensity.Low,
        < 32 => ImageDensity.Average,
        < 48 => ImageDensity.Medium,
        < 64 => ImageDensity.High,
        _ => ImageDensity.Solid,
    };

    private static ImageShape2D ClassifyShape(int count, ImageColors colors, Cell start, Cell end)
    {
        if (count >= 60)
        {
            return ImageShape2D.FullRect;
        }

        if (System.Numerics.BitOperations.PopCount((uint)colors) == 1 && count < 48)
        {
            return ImageShape2D.Char;
        }

        int area = (end.X - start.X + 1) * (end.Y - start.Y + 1);
        return count > 0 && count == area ? ImageShape2D.Rect : ImageShape2D.Unidentified;
    }
}

/// <summary>
/// The session's image registry (REQ-E3D-007): every image met in a batch or in a loaded
/// pattern gets one unique, stable <see cref="TileImage"/>.
/// </summary>
/// <remarks>
/// Interning is a dictionary lookup, so getting the candidates of an image is constant time
/// amortised, as REQ-E3D-360 requires. The registry is part of the learned state a savestate
/// carries (REQ-E3D-008, DEC-R2-T5).
/// </remarks>
public sealed class ImageRegistry
{
    private readonly Dictionary<TileImageKey, TileImage> _byKey = [];
    private readonly List<TileImage> _byId = [];

    /// <summary>Number of distinct images met so far.</summary>
    public int Count => _byId.Count;

    /// <summary>The images in registry order; index = <see cref="TileImage.Id"/>.</summary>
    public IReadOnlyList<TileImage> Images => _byId;

    /// <summary>The image of <paramref name="key"/>, creating it on first sight.</summary>
    public TileImage Intern(TileImageKey key)
    {
        if (_byKey.TryGetValue(key, out TileImage? image))
        {
            return image;
        }

        image = new TileImage(key, _byId.Count);
        _byKey.Add(key, image);
        _byId.Add(image);
        return image;
    }

    /// <summary>The image of the given planes and flips, creating it on first sight.</summary>
    public TileImage Intern(ulong low, ulong high, bool flipX, bool flipY)
        => Intern(new TileImageKey(low, high, flipX, flipY));

    /// <summary>The image of registry identity <paramref name="id"/>.</summary>
    public TileImage ById(int id) => _byId[id];

    /// <summary>Forgets every image. Done when the game changes (REQ-E3D-008).</summary>
    public void Clear()
    {
        _byKey.Clear();
        _byId.Clear();
    }

    /// <summary>
    /// Forgets the images of identity <paramref name="count"/> and above, keeping the
    /// identities of the others. Used before a savestate's learned state is put back: the
    /// images the profile interned keep their identity, the ones met while playing are
    /// replaced by the recorded ones (REQ-E3D-008, DEC-R2-T5).
    /// </summary>
    public void TruncateTo(int count)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(count);
        for (int id = _byId.Count - 1; id >= count; id--)
        {
            _byKey.Remove(_byId[id].Key);
            _byId.RemoveAt(id);
        }
    }
}
