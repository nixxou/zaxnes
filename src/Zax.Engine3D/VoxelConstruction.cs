// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// Builds the voxel fields of a 3D object, one per distinct tile image of its pattern, for
/// every geometry type of SPEC-04B §3.3 (REQ-E3D-420 to 452).
/// </summary>
/// <remarks>
/// <para>
/// The construction rules write on <b>pattern coordinates</b> <c>(x, y, k)</c>, <c>y = 0</c> at
/// the top of the pattern image; <see cref="Write"/> routes each write to the field of the
/// tile of cell <c>(⌊x/8⌋, ⌊y/8⌋)</c> at the voxel <c>(x mod 8, 7 − (y mod 8))</c>, and drops
/// it when the cell is empty or outside the grid (REQ-E3D-421). Cells that carry the same tile
/// image share one field, so all their writes pile up on it (REQ-E3D-425) — that is where the
/// inner walls of the original come from.
/// </para>
/// <para>
/// All reads of the texture go through <see cref="ObjectTexture.Texel"/>, hence modulo the
/// texture size (REQ-E3D-426).
/// </para>
/// </remarks>
public sealed class VoxelConstruction
{
    private readonly EnginePattern _pattern;
    private readonly ObjectTexture _texture;
    private readonly TileVoxels[] _fields;
    private readonly int[] _fieldOfImage;
    private readonly int _patternWidth;
    private readonly int _patternHeight;
    private readonly int _depth;
    private readonly int _emptyColor;
    private readonly bool _solid;
    private readonly bool _fullWarp;
    private readonly bool _flip;
    private readonly bool _antiShadow;
    private readonly float _fillRate;
    private readonly Geo3DType _geometry;
    private readonly bool _marching;

    private int _effectiveDepth;

    private VoxelConstruction(
        EnginePattern pattern,
        ObjectTexture texture,
        PatternInstance parameters,
        RenderAlgorithm algorithm,
        TileVoxels[] fields,
        int[] fieldOfImage)
    {
        _pattern = pattern;
        _texture = texture;
        _fields = fields;
        _fieldOfImage = fieldOfImage;
        _patternWidth = pattern.Width * 8;
        _patternHeight = pattern.Height * 8;
        _depth = Math.Clamp(parameters.Depth, 0, TileVoxels.MaxDepth);
        _emptyColor = ObjectTexture.EmptyColor(parameters.EmptyColor);
        _solid = parameters.Solid;
        _fullWarp = parameters.FullWarp;
        _flip = parameters.Flip;
        _antiShadow = parameters.AntiShadow;
        _fillRate = parameters.FillRate;
        _geometry = parameters.Geometry;
        _marching = algorithm == RenderAlgorithm.Marching;
        _effectiveDepth = _depth;
    }

    /// <summary>
    /// Builds one voxel field per distinct image of <paramref name="pattern"/> and returns
    /// them in the order of <see cref="EnginePattern.Images"/>.
    /// </summary>
    /// <param name="pattern">The pattern whose tiles are voxelised.</param>
    /// <param name="texture">The object's texture (REQ-E3D-426).</param>
    /// <param name="parameters">
    /// The 3D parameters as the build hook left them (REQ-E3D-455): geometry, depth, options,
    /// transparent colour and <c>fillrate</c>.
    /// </param>
    /// <param name="algorithm">The effective mesh algorithm (REQ-E3D-500).</param>
    /// <param name="effectiveDepth">
    /// The depth <c>D'</c> placement and deformation must use: <c>D</c> for the non-cylindrical
    /// types, <c>max(B + 1)</c> over the segments built otherwise (REQ-E3D-452).
    /// </param>
    public static TileVoxels[] Build(
        EnginePattern pattern,
        ObjectTexture texture,
        PatternInstance parameters,
        RenderAlgorithm algorithm,
        out int effectiveDepth)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        ArgumentNullException.ThrowIfNull(texture);
        ArgumentNullException.ThrowIfNull(parameters);

        int depth = Math.Clamp(parameters.Depth, 0, TileVoxels.MaxDepth);
        bool perAxisSlots = algorithm != RenderAlgorithm.Marching;

        TileVoxels[] fields = new TileVoxels[pattern.Images.Count];
        for (int n = 0; n < fields.Length; n++)
        {
            fields[n] = new TileVoxels(depth, perAxisSlots);
        }

        int[] fieldOfImage = BuildImageIndex(pattern);

        // D = 0: no voxel is built and the tile has no geometry (REQ-E3D-420); a cylinder
        // without a segment ends up with D' = 0 as well (REQ-E3D-452).
        if (depth == 0)
        {
            effectiveDepth = 0;
            return fields;
        }

        VoxelConstruction construction = new(pattern, texture, parameters, algorithm, fields, fieldOfImage);
        construction.Run();
        effectiveDepth = construction._effectiveDepth;
        return fields;
    }

    /// <summary>True for the four cylindrical geometry types (REQ-E3D-444).</summary>
    public static bool IsCylinder(Geo3DType geometry) => geometry
        is Geo3DType.VCylinder
        or Geo3DType.HCylinder
        or Geo3DType.HalfVCylinder
        or Geo3DType.HalfHCylinder;

    private static int[] BuildImageIndex(EnginePattern pattern)
    {
        int highest = 0;
        foreach (TileImage image in pattern.Images)
        {
            highest = Math.Max(highest, image.Id);
        }

        int[] index = new int[highest + 1];
        Array.Fill(index, -1);
        for (int n = 0; n < pattern.Images.Count; n++)
        {
            index[pattern.Images[n].Id] = n;
        }

        return index;
    }

    private void Run()
    {
        switch (_geometry)
        {
            case Geo3DType.Cube:
                BuildCube();
                break;

            case Geo3DType.VCylinder:
            case Geo3DType.HCylinder:
            case Geo3DType.HalfVCylinder:
            case Geo3DType.HalfHCylinder:
                BuildCylinder();
                break;

            default:
                BuildDefault();
                break;
        }
    }

    #region writes

    /// <summary>
    /// Assigns the texel (<paramref name="tx"/>, <paramref name="ty"/>) to the slot
    /// <paramref name="axis"/> of the pattern coordinate (<paramref name="x"/>,
    /// <paramref name="y"/>, <paramref name="k"/>) (REQ-E3D-421, 422, 423).
    /// </summary>
    private void Write(int x, int y, int k, int axis, int tx, int ty)
    {
        if ((uint)x >= (uint)_patternWidth || (uint)y >= (uint)_patternHeight)
        {
            return;
        }

        TileVoxels? field = FieldAt(x >> 3, y >> 3);
        field?.Write(x & 7, 7 - (y & 7), k, axis, tx, ty, IsTransparent(tx, ty));
    }

    /// <summary>Writes every slot of the voxel (REQ-E3D-422, "sur tous les axes").</summary>
    private void WriteAll(int x, int y, int k, int tx, int ty)
        => Write(x, y, k, TileVoxels.AllAxes, tx, ty);

    private TileVoxels? FieldAt(int cellX, int cellY)
    {
        TileImage? image = _pattern.ImageAt(cellX, cellY);
        if (image is null)
        {
            return null;
        }

        int index = (uint)image.Id < (uint)_fieldOfImage.Length ? _fieldOfImage[image.Id] : -1;
        return index < 0 ? null : _fields[index];
    }

    /// <summary>True when the texel is the transparent colour <c>e</c> (REQ-E3D-423, 424).</summary>
    private bool IsTransparent(int tx, int ty) => _texture.Texel(tx, ty) == _emptyColor;

    #endregion

    #region Default (REQ-E3D-440, 441, 442)

    private void BuildDefault()
    {
        // Canonical order: x ascending, then y, then k (REQ-E3D-440).
        for (int x = 0; x < _patternWidth; x++)
        {
            for (int y = 0; y < _patternHeight; y++)
            {
                if (IsTransparent(x, y))
                {
                    continue;
                }

                for (int k = 0; k < _depth; k++)
                {
                    if (!_fullWarp)
                    {
                        // Extrusion: every opaque pixel becomes a 1 x 1 x D prism.
                        WriteAll(x, y, k, x, y);
                    }
                    else if (_marching)
                    {
                        // REQ-E3D-442: the slice k takes the lowest opaque texel of the
                        // interval [y, y + k], which always exists since t(x, y) is opaque.
                        int wrapped = y + k;
                        while (IsTransparent(x, wrapped))
                        {
                            wrapped--;
                        }

                        WriteAll(x, y, k, x, wrapped);
                    }
                    else
                    {
                        // REQ-E3D-441, wrapped extrusion, in this order: Y, then X, then Z.
                        Write(x, y, k, TileVoxels.AxisY, x, _depth - 1 - k);
                        Write(x, y, k, TileVoxels.AxisX, _depth - 1 - k, y);
                        Write(x, y, k, TileVoxels.AxisZ, x, y);
                    }
                }
            }
        }
    }

    #endregion

    #region CUBE (REQ-E3D-443)

    private void BuildCube()
    {
        // The box covers the whole pattern image, empty cells included; writes that land in an
        // empty cell have no effect, so the box is open there (REQ-E3D-443 notes).
        for (int x = 0; x < _patternWidth; x++)
        {
            for (int k = 0; k < _depth; k++)
            {
                Write(x, _patternHeight - 1, k, TileVoxels.AxisY, x, k);
                Write(x, 0, k, TileVoxels.AxisY, x, _depth - 1 - k);
            }
        }

        for (int y = 0; y < _patternHeight; y++)
        {
            for (int k = 0; k < _depth; k++)
            {
                Write(0, y, k, TileVoxels.AxisX, _depth - 1 - k, y);
                Write(_patternWidth - 1, y, k, TileVoxels.AxisX, _depth - 1 - k, y);
            }
        }

        for (int x = 0; x < _patternWidth; x++)
        {
            for (int y = 0; y < _patternHeight; y++)
            {
                Write(x, y, _depth - 1, TileVoxels.AxisZ, x, y);
                Write(x, y, 0, TileVoxels.AxisZ, x, y);
            }
        }
    }

    #endregion

    #region cylinders (REQ-E3D-444 to 452)

    private bool Vertical => _geometry is Geo3DType.VCylinder or Geo3DType.HalfVCylinder;

    private bool Half => _geometry is Geo3DType.HalfVCylinder or Geo3DType.HalfHCylinder;

    private void BuildCylinder()
    {
        // Every segment contributes max(B + 1) to the effective depth; without a segment the
        // object ends up with D' = 0 (REQ-E3D-452).
        _effectiveDepth = 0;

        if (Vertical)
        {
            // V types: each row, x ascending.
            for (int y = 0; y < _patternHeight; y++)
            {
                for (int x = 0; x < _patternWidth; x++)
                {
                    if (IsTransparent(x, y))
                    {
                        continue;
                    }

                    int last = x;
                    while (last + 1 < _patternWidth && !IsTransparent(last + 1, y))
                    {
                        last++;
                    }

                    Segment(y, x, last);
                    x = last;
                }
            }

            return;
        }

        // H types: each column, y ascending.
        for (int x = 0; x < _patternWidth; x++)
        {
            for (int y = 0; y < _patternHeight; y++)
            {
                if (IsTransparent(x, y))
                {
                    continue;
                }

                int last = y;
                while (last + 1 < _patternHeight && !IsTransparent(x, last + 1))
                {
                    last++;
                }

                Segment(x, y, last);
                y = last;
            }
        }
    }

    /// <summary>
    /// Replaces one maximal segment of opaque pixels by an elliptic section in the plane
    /// (line, depth) (REQ-E3D-444 to 451). Doubled coordinates, in half pixels
    /// (REQ-E3D-445).
    /// </summary>
    /// <param name="line">The row (V types) or the column (H types) the segment lies on.</param>
    /// <param name="first">First pixel <c>s</c> of the segment along the line.</param>
    /// <param name="last">Last pixel <c>f</c> of the segment along the line.</param>
    private void Segment(int line, int first, int last)
    {
        int length = last - first;
        int centre;
        int major;
        int minor;

        if (!Half)
        {
            centre = first + last;
            major = length;
            minor = Math.Min(_depth - 1, length);
        }
        else
        {
            major = 2 * length;
            minor = Math.Min(_depth - 1, 2 * length);

            // The flat face sits at the end of the segment that carries the centre.
            bool centreAtFirst = (_geometry == Geo3DType.HalfVCylinder && !_flip)
                || (_geometry == Geo3DType.HalfHCylinder && _flip);
            centre = centreAtFirst ? 2 * first : 2 * last;
        }

        int centreZ = _depth - 1;
        _effectiveDepth = Math.Max(_effectiveDepth, minor + 1);

        if (major == 0 || minor == 0)
        {
            // Degenerate section: a single voxel at the centre (REQ-E3D-451).
            int u = FloorDiv(centre, 2);
            int z = FloorDiv(centreZ, 2);
            WriteSection(line, u, z, u);
            return;
        }

        long a2 = (long)major * major;
        long b2 = (long)minor * minor;
        long limit = a2 * b2;

        // Region 1, from (0, B): midpoint ellipse tracing in doubled coordinates.
        for (int p = 0, q = minor; b2 * p <= a2 * q;)
        {
            Quadrants(line, centre, centreZ, p, q, region2: false);
            int next = p + 1;
            if ((b2 * next * (next - 1)) + (a2 * q * (q - 1)) >= limit)
            {
                q--;
            }

            p = next;
        }

        // Region 2, from (A, 0).
        for (int p = major, q = 0; a2 * q <= b2 * p;)
        {
            Quadrants(line, centre, centreZ, p, q, region2: true);
            int next = q + 1;
            if ((a2 * next * (next - 1)) + (b2 * p * (p - 1)) > limit)
            {
                p--;
            }

            q = next;
        }
    }

    /// <summary>
    /// Turns one contour point into its four quadrant points and writes them, in the order
    /// Q0, Q1, Q2, Q3 (REQ-E3D-446, 447, 448, 449, 450).
    /// </summary>
    private void Quadrants(int line, int centre, int centreZ, int p, int q, bool region2)
    {
        int up = FloorDiv(centre + p, 2);
        int um = centre - up;
        int zp = region2 ? FloorDiv(centreZ + q - 1, 2) : FloorDiv(centreZ + q, 2);
        int zm = centreZ - zp;

        // Texel of each quadrant along the line (REQ-E3D-447). The full types mirror the image
        // on the back half; the half types take the quadrant's own position. The rule is
        // applied as written, region 2 included — which is what TV-Y1, TV-Y3, TV-Y6 and TV-Y7
        // show, and what the parenthetical of §5.3 contradicts for one case (Q-014).
        int texPlus = up;
        int texMinus = um;
        if (_antiShadow && region2)
        {
            // antiShadow only moves the texel choice of the region 2 points, never the voxels.
            texPlus = FloorDiv(centre + q, 2);
            texMinus = FloorDiv(centre - q + 1, 2);
        }

        int t0;
        int t1;
        int t2;
        int t3;
        if (!Half)
        {
            t0 = texMinus;
            t1 = texPlus;
            t2 = texPlus;
            t3 = texMinus;
        }
        else
        {
            t0 = texPlus;
            t1 = texPlus;
            t2 = texMinus;
            t3 = texMinus;
        }

        bool plusSide = !Half
            || (_geometry == Geo3DType.HalfVCylinder && !_flip)
            || (_geometry == Geo3DType.HalfHCylinder && _flip);
        bool q0 = plusSide;
        bool q1 = plusSide;
        bool q2 = !Half || !plusSide;
        bool q3 = q2;

        if (!_solid)
        {
            // Contour only: a quadrant point whose texel value is zero is skipped
            // (REQ-E3D-448), whatever the transparent colour is.
            WriteContour(line, q0, up, zp, t0);
            WriteContour(line, q1, up, zm, t1);
            WriteContour(line, q2, um, zp, t2);
            WriteContour(line, q3, um, zm, t3);
            return;
        }

        if (_fullWarp)
        {
            // REQ-E3D-450: fill each column of the section over the depth span of the point,
            // with the texel of the opposite quadrant and no zero test. The span runs between
            // the two bounds whichever way round they are, as TV-Y4 requires (Q-015).
            int from = Math.Min(zm, zp);
            int to = Math.Max(zm, zp);
            for (int z = from; z <= to; z++)
            {
                if (q0 && q1)
                {
                    WriteSection(line, up, z, t2);
                }

                if (q2 && q3)
                {
                    WriteSection(line, um, z, t0);
                }
            }

            return;
        }

        // REQ-E3D-449: Bresenham from the centre to Q0, mirrored into the active quadrants.
        FillToCentre(line, centre, centreZ, up, zp, q0, q1, q2, q3, t0, t1, t2, t3);
    }

    private void WriteContour(int line, bool active, int u, int z, int texelLine)
    {
        if (!active || TexelValue(line, texelLine) == 0)
        {
            return;
        }

        WriteSection(line, u, z, texelLine);
    }

    /// <summary>
    /// Writes one voxel of a section: pattern coordinate <c>(u, line, z)</c> for the V types,
    /// <c>(line, u, z)</c> for the H types, on all axes (REQ-E3D-448).
    /// </summary>
    private void WriteSection(int line, int u, int z, int texelLine)
    {
        if (Vertical)
        {
            WriteAll(u, line, z, texelLine, line);
        }
        else
        {
            WriteAll(line, u, z, line, texelLine);
        }
    }

    /// <summary>The texel value at the position <paramref name="texelLine"/> along the line.</summary>
    private int TexelValue(int line, int texelLine)
        => Vertical ? _texture.Texel(texelLine, line) : _texture.Texel(line, texelLine);

    /// <summary>
    /// Traces the discrete segment from the section centre <c>M</c> to <c>Q0</c> by the
    /// classic Bresenham algorithm and mirrors each traced point into the active quadrants
    /// (REQ-E3D-449).
    /// </summary>
    private void FillToCentre(
        int line,
        int centre,
        int centreZ,
        int targetU,
        int targetZ,
        bool q0,
        bool q1,
        bool q2,
        bool q3,
        int t0,
        int t1,
        int t2,
        int t3)
    {
        int centreU = FloorDiv(centre, 2);
        int midZ = FloorDiv(centreZ, 2);

        int deltaU = Math.Abs(targetU - centreU);
        int deltaZ = Math.Abs(targetZ - midZ);
        int majorSpan = Math.Max(deltaU, deltaZ);
        if (majorSpan == 0)
        {
            return;
        }

        bool majorIsU = deltaU >= deltaZ;
        int minorSpan = majorIsU ? deltaZ : deltaU;

        // Walked from the end of smallest major coordinate to the other one.
        int majorStart;
        int minorStart;
        int minorStep;
        if (majorIsU)
        {
            majorStart = Math.Min(centreU, targetU);
            bool fromCentre = centreU <= targetU;
            minorStart = fromCentre ? midZ : targetZ;
            minorStep = Math.Sign((fromCentre ? targetZ : midZ) - minorStart);
        }
        else
        {
            majorStart = Math.Min(midZ, targetZ);
            bool fromCentre = midZ <= targetZ;
            minorStart = fromCentre ? centreU : targetU;
            minorStep = Math.Sign((fromCentre ? targetU : centreU) - minorStart);
        }

        int majorTarget = majorIsU ? targetU : targetZ;
        int error = majorSpan / 2;
        int minor = minorStart;

        for (int step = 0; step <= majorSpan; step++)
        {
            int major = majorStart + step;
            int u = majorIsU ? major : minor;
            int z = majorIsU ? minor : major;

            // Only the points close enough to Q0 are drawn; with fillrate = 1 that excludes
            // the centre alone.
            if (Math.Abs(major - majorTarget) / (float)majorSpan < _fillRate)
            {
                WriteMirrored(line, centre, centreZ, u, z, q0, q1, q2, q3, t0, t1, t2, t3);
            }

            error -= minorSpan;
            if (error < 0)
            {
                minor += minorStep;
                error += majorSpan;
            }
        }
    }

    private void WriteMirrored(
        int line,
        int centre,
        int centreZ,
        int u,
        int z,
        bool q0,
        bool q1,
        bool q2,
        bool q3,
        int t0,
        int t1,
        int t2,
        int t3)
    {
        if (q0 && TexelValue(line, t0) != 0)
        {
            WriteSection(line, u, z, t0);
        }

        if (q1 && TexelValue(line, t1) != 0)
        {
            WriteSection(line, u, centreZ - z, t1);
        }

        if (q2 && TexelValue(line, t2) != 0)
        {
            WriteSection(line, centre - u, z, t2);
        }

        if (q3 && TexelValue(line, t3) != 0)
        {
            WriteSection(line, centre - u, centreZ - z, t3);
        }
    }

    #endregion

    /// <summary>Division rounding towards −∞, the <c>⌊⌋</c> of §3.3.</summary>
    private static int FloorDiv(int value, int divisor)
    {
        int quotient = value / divisor;
        return value % divisor != 0 && (value < 0) != (divisor < 0) ? quotient - 1 : quotient;
    }
}
