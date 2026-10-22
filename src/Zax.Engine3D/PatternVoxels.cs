// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// The voxel fields of one 3D object: one per distinct tile image of its pattern, plus the
/// pattern-level neighbourhood MARCHING needs (SPEC-04B REQ-E3D-425, REQ-E3D-520).
/// </summary>
/// <remarks>
/// <para>
/// MARCHING reads corners outside the tile from the neighbouring tile of the pattern. When a
/// tile image occupies several cells there is no single neighbourhood, so the occurrence
/// closest to the side being looked at is used (REQ-E3D-520). The nine positions kept per
/// image are exactly that: for each of the nine (left, middle, right) x (top, middle, bottom)
/// directions, the extreme occurrence in that direction.
/// </para>
/// </remarks>
public sealed class PatternVoxels
{
    private const int Missing = int.MinValue;

    private readonly TileVoxels[] _fields;
    private readonly int[] _fieldOfImage;
    private readonly int[] _cornerX;
    private readonly int[] _cornerY;
    private readonly int _patternWidth;
    private readonly int _patternHeight;

    private PatternVoxels(
        EnginePattern pattern,
        ObjectTexture texture,
        TileVoxels[] fields,
        int[] fieldOfImage,
        int[] cornerX,
        int[] cornerY)
    {
        Pattern = pattern;
        Texture = texture;
        _fields = fields;
        _fieldOfImage = fieldOfImage;
        _cornerX = cornerX;
        _cornerY = cornerY;
        _patternWidth = pattern.Width * 8;
        _patternHeight = pattern.Height * 8;
    }

    /// <summary>The pattern the fields belong to.</summary>
    public EnginePattern Pattern { get; }

    /// <summary>The object's texture.</summary>
    public ObjectTexture Texture { get; }

    /// <summary>Number of distinct tile images, hence of fields.</summary>
    public int Count => _fields.Length;

    /// <summary>The field of the image of index <paramref name="imageIndex"/>.</summary>
    public TileVoxels Field(int imageIndex) => _fields[imageIndex];

    /// <summary>The image of index <paramref name="imageIndex"/>.</summary>
    public TileImage Image(int imageIndex) => Pattern.Images[imageIndex];

    /// <summary>Groups the fields of <paramref name="fields"/> with their neighbourhood tables.</summary>
    public static PatternVoxels Create(EnginePattern pattern, ObjectTexture texture, TileVoxels[] fields)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        ArgumentNullException.ThrowIfNull(texture);
        ArgumentNullException.ThrowIfNull(fields);

        int images = pattern.Images.Count;
        int highest = 0;
        foreach (TileImage image in pattern.Images)
        {
            highest = Math.Max(highest, image.Id);
        }

        int[] fieldOfImage = new int[highest + 1];
        Array.Fill(fieldOfImage, -1);
        for (int n = 0; n < images; n++)
        {
            fieldOfImage[pattern.Images[n].Id] = n;
        }

        int[] cornerX = new int[images * 9];
        int[] cornerY = new int[images * 9];
        Array.Fill(cornerX, Missing);
        Array.Fill(cornerY, Missing);

        for (int j = 0; j < pattern.Height; j++)
        {
            for (int i = 0; i < pattern.Width; i++)
            {
                TileImage? image = pattern.ImageAt(i, j);
                if (image is null)
                {
                    continue;
                }

                SetOccurrence(cornerX, cornerY, fieldOfImage[image.Id] * 9, i * 8, j * 8);
            }
        }

        return new PatternVoxels(pattern, texture, fields, fieldOfImage, cornerX, cornerY);
    }

    /// <summary>
    /// True when the cells at <c>i = −1</c> must be meshed for this image, that is when its
    /// leftmost occurrence sits in the leftmost column of the pattern (REQ-E3D-520).
    /// </summary>
    /// <param name="imageIndex">The image.</param>
    /// <param name="cellRow">
    /// The row of voxels the cell starts at, which selects the occurrence used.
    /// </param>
    public bool MeshesLeftEdge(int imageIndex, int cellRow)
        => _cornerX[(imageIndex * 9) + DirectionIndex(-1, cellRow)] == 0;

    /// <summary>
    /// Reads the single slot of a corner of the MARCHING grid, in the tile frame of the image
    /// <paramref name="imageIndex"/>. Coordinates outside <c>[0,8)</c> on x or y are resolved
    /// in the neighbouring tile of the pattern; outside the pattern image, outside
    /// <c>[0, D)</c> in depth or on an empty cell, the corner is empty (REQ-E3D-520).
    /// </summary>
    public bool TryCorner(int imageIndex, int i, int j, int k, out int tx, out int ty)
    {
        tx = 0;
        ty = 0;

        int at = (imageIndex * 9) + DirectionIndex(i, j);
        int originX = _cornerX[at];
        if (originX == Missing)
        {
            return false;
        }

        int x = i + originX;
        int y = (7 - j) + _cornerY[at];
        if ((uint)x >= (uint)_patternWidth || (uint)y >= (uint)_patternHeight)
        {
            return false;
        }

        TileImage? image = Pattern.ImageAt(x >> 3, y >> 3);
        if (image is null)
        {
            return false;
        }

        int index = (uint)image.Id < (uint)_fieldOfImage.Length ? _fieldOfImage[image.Id] : -1;
        if (index < 0)
        {
            return false;
        }

        return _fields[index].TryTexel(x & 7, 7 - (y & 7), k, 0, out tx, out ty);
    }

    /// <summary>
    /// The direction slot of a tile coordinate: 0 to 2 for x (left, inside, right) plus 0, 3 or
    /// 6 for y (above, inside, below), the layout of the nine occurrences.
    /// </summary>
    private static int DirectionIndex(int i, int j)
    {
        int index = i == -1 ? 0 : i == TileVoxels.SizeX ? 2 : 1;
        if (j == -1)
        {
            index += 6;
        }
        else if (j != TileVoxels.SizeY)
        {
            index += 3;
        }

        return index;
    }

    private static void SetOccurrence(int[] cornerX, int[] cornerY, int at, int x, int y)
    {
        // Slot 4 is "any occurrence"; the eight others keep the extreme one in their
        // direction, x towards the left or the right and y towards the top or the bottom of
        // the pattern image.
        Keep(cornerX, cornerY, at + 0, x, y, smallerX: true, smallerY: true);
        Keep(cornerX, cornerY, at + 1, x, y, smallerX: null, smallerY: true);
        Keep(cornerX, cornerY, at + 2, x, y, smallerX: false, smallerY: true);
        Keep(cornerX, cornerY, at + 3, x, y, smallerX: true, smallerY: null);
        cornerX[at + 4] = x;
        cornerY[at + 4] = y;
        Keep(cornerX, cornerY, at + 5, x, y, smallerX: false, smallerY: null);
        Keep(cornerX, cornerY, at + 6, x, y, smallerX: true, smallerY: false);
        Keep(cornerX, cornerY, at + 7, x, y, smallerX: null, smallerY: false);
        Keep(cornerX, cornerY, at + 8, x, y, smallerX: false, smallerY: false);
    }

    private static void Keep(int[] cornerX, int[] cornerY, int at, int x, int y, bool? smallerX, bool? smallerY)
    {
        if (cornerX[at] != Missing
            && !(Accepts(cornerX[at], x, smallerX) && Accepts(cornerY[at], y, smallerY)))
        {
            return;
        }

        cornerX[at] = x;
        cornerY[at] = y;
    }

    private static bool Accepts(int kept, int candidate, bool? smaller) => smaller switch
    {
        true => kept >= candidate,
        false => kept <= candidate,
        _ => true,
    };
}
