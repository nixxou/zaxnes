using System.Runtime.CompilerServices;

namespace Zax.Profiles;

/// <summary>
/// Pair of 32-bit integers (SPEC-03 §4.7, <c>IVec2</c>): a tile-grid size or a size
/// constraint.
/// </summary>
/// <param name="X">First component (width).</param>
/// <param name="Y">Second component (height).</param>
public readonly record struct Vec2I(int X, int Y)
{
    /// <summary>The value a missing <c>constraintSize</c> takes (REQ-PROF-034).</summary>
    public static Vec2I Unconstrained => new(99, 99);

    /// <inheritdoc/>
    public override string ToString() => $"({X}, {Y})";
}

/// <summary>Three single-precision floats (SPEC-03 §4.7, <c>Vec3</c>).</summary>
/// <remarks>
/// Equality is <b>bitwise</b>, not numeric: REQ-PROF-002 requires the bits of every float to
/// survive a load and a save unchanged (no NaN normalisation, no −0 → 0), so two values are
/// equal here exactly when they would be written as the same four bytes.
/// </remarks>
public readonly struct Vec3F : IEquatable<Vec3F>
{
    /// <summary>Creates a vector.</summary>
    public Vec3F(float x, float y, float z)
    {
        X = x;
        Y = y;
        Z = z;
    }

    /// <summary>First component.</summary>
    public float X { get; init; }

    /// <summary>Second component.</summary>
    public float Y { get; init; }

    /// <summary>Third component.</summary>
    public float Z { get; init; }

    /// <summary>All three components zero.</summary>
    public static Vec3F Zero => default;

    /// <inheritdoc/>
    public bool Equals(Vec3F other)
        => FloatBits.Same(X, other.X) && FloatBits.Same(Y, other.Y) && FloatBits.Same(Z, other.Z);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Vec3F other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode()
        => HashCode.Combine(BitConverter.SingleToInt32Bits(X), BitConverter.SingleToInt32Bits(Y),
            BitConverter.SingleToInt32Bits(Z));

    /// <summary>Bitwise equality.</summary>
    public static bool operator ==(Vec3F left, Vec3F right) => left.Equals(right);

    /// <summary>Bitwise inequality.</summary>
    public static bool operator !=(Vec3F left, Vec3F right) => !left.Equals(right);

    /// <inheritdoc/>
    public override string ToString() => $"({X}, {Y}, {Z})";
}

/// <summary>
/// Four single-precision floats (SPEC-03 §4.7, <c>Vec4</c>). Used for the image clipping
/// margins, whose indices are fixed by DEC-R2-T1: 0 = left, 1 = bottom, 2 = right, 3 = top.
/// </summary>
/// <remarks>Equality is bitwise, as for <see cref="Vec3F"/>.</remarks>
public readonly struct Vec4F : IEquatable<Vec4F>
{
    /// <summary>Creates a vector.</summary>
    public Vec4F(float x, float y, float z, float w)
    {
        X = x;
        Y = y;
        Z = z;
        W = w;
    }

    /// <summary>First component; clipping margin of the left edge (DEC-R2-T1).</summary>
    public float X { get; init; }

    /// <summary>Second component; clipping margin of the bottom edge.</summary>
    public float Y { get; init; }

    /// <summary>Third component; clipping margin of the right edge.</summary>
    public float Z { get; init; }

    /// <summary>Fourth component; clipping margin of the top edge.</summary>
    public float W { get; init; }

    /// <summary>Component by index, 0 to 3, in the order x, y, z, w.</summary>
    /// <exception cref="ArgumentOutOfRangeException">Index outside 0..3.</exception>
    public float this[int index] => index switch
    {
        0 => X,
        1 => Y,
        2 => Z,
        3 => W,
        _ => throw new ArgumentOutOfRangeException(nameof(index)),
    };

    /// <inheritdoc/>
    public bool Equals(Vec4F other)
        => FloatBits.Same(X, other.X) && FloatBits.Same(Y, other.Y)
        && FloatBits.Same(Z, other.Z) && FloatBits.Same(W, other.W);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Vec4F other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode()
        => HashCode.Combine(BitConverter.SingleToInt32Bits(X), BitConverter.SingleToInt32Bits(Y),
            BitConverter.SingleToInt32Bits(Z), BitConverter.SingleToInt32Bits(W));

    /// <summary>Bitwise equality.</summary>
    public static bool operator ==(Vec4F left, Vec4F right) => left.Equals(right);

    /// <summary>Bitwise inequality.</summary>
    public static bool operator !=(Vec4F left, Vec4F right) => !left.Equals(right);

    /// <inheritdoc/>
    public override string ToString() => $"({X}, {Y}, {Z}, {W})";
}

/// <summary>Storage of the sixteen cells of a <see cref="Mat4F"/>.</summary>
[InlineArray(Mat4F.CellCount)]
public struct Mat4Cells
{
    private float _element0;
}

/// <summary>
/// Four-by-four matrix of single-precision floats (SPEC-03 §4.7, <c>Mat4</c>), stored on the
/// wire as sixteen fields numbered <c>1 + 4 × row + column</c>.
/// </summary>
/// <remarks>
/// Deformation uses two of these per 3D variant; the fourth row of each also carries free
/// script data (<c>CData</c>, SPEC-03 §4.4). Equality is bitwise, as for <see cref="Vec3F"/>.
/// </remarks>
public readonly struct Mat4F : IEquatable<Mat4F>
{
    /// <summary>Number of cells, four rows of four.</summary>
    public const int CellCount = 16;

    private readonly Mat4Cells _cells;

    /// <summary>Creates a matrix from sixteen cells in row-major order.</summary>
    /// <exception cref="ArgumentException">The span does not hold exactly sixteen cells.</exception>
    public Mat4F(ReadOnlySpan<float> rowMajorCells)
    {
        if (rowMajorCells.Length != CellCount)
        {
            throw new ArgumentException($"A 4x4 matrix has {CellCount} cells.", nameof(rowMajorCells));
        }

        Mat4Cells cells = default;
        rowMajorCells.CopyTo(cells);
        _cells = cells;
    }

    /// <summary>Cell at a row-major index, 0 to 15.</summary>
    /// <exception cref="ArgumentOutOfRangeException">Index outside 0..15.</exception>
    public float this[int index]
    {
        get
        {
            if ((uint)index >= CellCount)
            {
                throw new ArgumentOutOfRangeException(nameof(index));
            }

            Mat4Cells cells = _cells;
            return cells[index];
        }
    }

    /// <summary>Cell at a row and a column, both 0 to 3.</summary>
    public float this[int row, int column]
    {
        get
        {
            if ((uint)row >= 4 || (uint)column >= 4)
            {
                throw new ArgumentOutOfRangeException(row >= 4 ? nameof(row) : nameof(column));
            }

            return this[(row * 4) + column];
        }
    }

    /// <summary>Copies the sixteen cells, row-major, into <paramref name="destination"/>.</summary>
    public void CopyTo(Span<float> destination)
    {
        Mat4Cells cells = _cells;
        ((ReadOnlySpan<float>)cells).CopyTo(destination);
    }

    /// <inheritdoc/>
    public bool Equals(Mat4F other)
    {
        for (int i = 0; i < CellCount; i++)
        {
            if (!FloatBits.Same(this[i], other[i]))
            {
                return false;
            }
        }

        return true;
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Mat4F other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode()
    {
        HashCode hash = default;
        for (int i = 0; i < CellCount; i++)
        {
            hash.Add(BitConverter.SingleToInt32Bits(this[i]));
        }

        return hash.ToHashCode();
    }

    /// <summary>Bitwise equality.</summary>
    public static bool operator ==(Mat4F left, Mat4F right) => left.Equals(right);

    /// <summary>Bitwise inequality.</summary>
    public static bool operator !=(Mat4F left, Mat4F right) => !left.Equals(right);
}

/// <summary>
/// Colour stored as one 32-bit integer, <c>a&lt;&lt;24 | r&lt;&lt;16 | g&lt;&lt;8 | b</c>
/// (SPEC-03 §4.7, <c>Argb</c>). The alpha component is ignored when read: profile colours are
/// opaque.
/// </summary>
/// <param name="Packed">The stored integer, alpha included.</param>
public readonly record struct ArgbColor(int Packed)
{
    /// <summary>Red component, 0 to 255.</summary>
    public byte R => (byte)(Packed >> 16);

    /// <summary>Green component, 0 to 255.</summary>
    public byte G => (byte)(Packed >> 8);

    /// <summary>Blue component, 0 to 255.</summary>
    public byte B => (byte)Packed;

    /// <summary>
    /// The stored alpha component, kept for lossless rewriting. Readers of the colour treat
    /// it as fully opaque (SPEC-03 §4.7).
    /// </summary>
    public byte StoredAlpha => (byte)(Packed >> 24);

    /// <inheritdoc/>
    public override string ToString() => $"#{R:X2}{G:X2}{B:X2}";
}

/// <summary>Bit-exact float comparison, used by the value types of this library.</summary>
internal static class FloatBits
{
    /// <summary>True when both floats have the same 32 bits.</summary>
    internal static bool Same(float left, float right)
        => BitConverter.SingleToInt32Bits(left) == BitConverter.SingleToInt32Bits(right);
}
