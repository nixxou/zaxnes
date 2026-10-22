namespace Zax.Engine3D;

/// <summary>
/// A three-component vector of the conversion engine, in scene coordinates unless said
/// otherwise (x right, y up, z away from the viewer; SPEC-04B REQ-E3D-400).
/// </summary>
/// <remarks>
/// The fields are mutable on purpose. Scripts change one component of an instance's
/// transform in place (<c>shape.Offset.x = shape.Offset.x + 5</c>, SPEC-05 REQ-LUA-044), so
/// the parameters of <see cref="PatternInstance"/> are public mutable fields of this type and
/// the Lua layer binds a live reference to them.
/// </remarks>
public struct Vec3 : IEquatable<Vec3>
{
    /// <summary>X component.</summary>
    public float X;

    /// <summary>Y component.</summary>
    public float Y;

    /// <summary>Z component.</summary>
    public float Z;

    /// <summary>Creates a vector.</summary>
    public Vec3(float x, float y, float z)
    {
        X = x;
        Y = y;
        Z = z;
    }

    /// <summary>The zero vector.</summary>
    public static Vec3 Zero => default;

    /// <summary>The vector of all ones.</summary>
    public static Vec3 One => new(1f, 1f, 1f);

    /// <summary>The default pivot, <c>(0.5, 0.5, 0.5)</c> (SPEC-04 §4.3).</summary>
    public static Vec3 Half => new(0.5f, 0.5f, 0.5f);

    /// <summary>Component-by-component product (the <c>⊙</c> of SPEC-04 REQ-E3D-205).</summary>
    public static Vec3 Scale(Vec3 a, Vec3 b) => new(a.X * b.X, a.Y * b.Y, a.Z * b.Z);

    /// <summary>Sum.</summary>
    public static Vec3 operator +(Vec3 a, Vec3 b) => new(a.X + b.X, a.Y + b.Y, a.Z + b.Z);

    /// <summary>Difference.</summary>
    public static Vec3 operator -(Vec3 a, Vec3 b) => new(a.X - b.X, a.Y - b.Y, a.Z - b.Z);

    /// <summary>Product by a scalar.</summary>
    public static Vec3 operator *(Vec3 a, float k) => new(a.X * k, a.Y * k, a.Z * k);

    /// <summary>Euclidean distance between two vectors.</summary>
    public static float Distance(Vec3 a, Vec3 b)
    {
        float dx = a.X - b.X;
        float dy = a.Y - b.Y;
        float dz = a.Z - b.Z;
        return MathF.Sqrt((dx * dx) + (dy * dy) + (dz * dz));
    }

    /// <summary>Squared length.</summary>
    public float SquaredLength => (X * X) + (Y * Y) + (Z * Z);

    /// <inheritdoc/>
    public bool Equals(Vec3 other) => X.Equals(other.X) && Y.Equals(other.Y) && Z.Equals(other.Z);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Vec3 other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(X, Y, Z);

    /// <summary>Equality.</summary>
    public static bool operator ==(Vec3 left, Vec3 right) => left.Equals(right);

    /// <summary>Inequality.</summary>
    public static bool operator !=(Vec3 left, Vec3 right) => !left.Equals(right);

    /// <inheritdoc/>
    public override string ToString() => $"({X}, {Y}, {Z})";
}

/// <summary>A two-component vector, mutable like <see cref="Vec3"/>.</summary>
public struct Vec2 : IEquatable<Vec2>
{
    /// <summary>X component.</summary>
    public float X;

    /// <summary>Y component.</summary>
    public float Y;

    /// <summary>Creates a vector.</summary>
    public Vec2(float x, float y)
    {
        X = x;
        Y = y;
    }

    /// <summary>The zero vector.</summary>
    public static Vec2 Zero => default;

    /// <summary>The vector of all ones.</summary>
    public static Vec2 One => new(1f, 1f);

    /// <inheritdoc/>
    public bool Equals(Vec2 other) => X.Equals(other.X) && Y.Equals(other.Y);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Vec2 other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(X, Y);

    /// <summary>Equality.</summary>
    public static bool operator ==(Vec2 left, Vec2 right) => left.Equals(right);

    /// <summary>Inequality.</summary>
    public static bool operator !=(Vec2 left, Vec2 right) => !left.Equals(right);

    /// <inheritdoc/>
    public override string ToString() => $"({X}, {Y})";
}

/// <summary>A pair of integers: a grid cell, a tile box corner or a screen pixel.</summary>
/// <param name="X">First component.</param>
/// <param name="Y">Second component.</param>
public readonly record struct Cell(int X, int Y)
{
    /// <summary>Sum.</summary>
    public static Cell operator +(Cell a, Cell b) => new(a.X + b.X, a.Y + b.Y);

    /// <summary>Difference.</summary>
    public static Cell operator -(Cell a, Cell b) => new(a.X - b.X, a.Y - b.Y);

    /// <inheritdoc/>
    public override string ToString() => $"({X}, {Y})";
}

/// <summary>
/// An axis-aligned rectangle of origin <c>(X, Y)</c> and size <c>(Width, Height)</c>, with
/// the strict overlap test of SPEC-05 §4.3 (type <c>Rect</c>).
/// </summary>
/// <param name="X">Origin x.</param>
/// <param name="Y">Origin y.</param>
/// <param name="Width">Width; a negative width is normalised by <see cref="Overlaps"/>.</param>
/// <param name="Height">Height; a negative height is normalised by <see cref="Overlaps"/>.</param>
public readonly record struct Rect(float X, float Y, float Width, float Height)
{
    /// <summary>Strict overlap: <c>xMax &gt; r.x</c>, <c>x &lt; r.xMax</c>, and the same on y.</summary>
    public bool Overlaps(Rect other)
    {
        (float x0, float x1) = Normalise(X, Width);
        (float y0, float y1) = Normalise(Y, Height);
        (float ox0, float ox1) = Normalise(other.X, other.Width);
        (float oy0, float oy1) = Normalise(other.Y, other.Height);
        return x1 > ox0 && x0 < ox1 && y1 > oy0 && y0 < oy1;
    }

    private static (float Min, float Max) Normalise(float origin, float size)
        => size < 0f ? (origin + size, origin) : (origin, origin + size);
}

/// <summary>
/// A 4 × 4 matrix of floats in row-major order, applied to column vectors
/// (<c>P = M · (v, 1)</c>), the convention of SPEC-04B REQ-E3D-406 and REQ-E3D-407.
/// </summary>
public struct Mat4 : IEquatable<Mat4>
{
    private float _m00, _m01, _m02, _m03;
    private float _m10, _m11, _m12, _m13;
    private float _m20, _m21, _m22, _m23;
    private float _m30, _m31, _m32, _m33;

    /// <summary>The identity matrix.</summary>
    public static Mat4 Identity
    {
        get
        {
            Mat4 m = default;
            m._m00 = 1f;
            m._m11 = 1f;
            m._m22 = 1f;
            m._m33 = 1f;
            return m;
        }
    }

    /// <summary>Cell at (<paramref name="row"/>, <paramref name="column"/>), both 0 to 3.</summary>
    /// <exception cref="ArgumentOutOfRangeException">A coordinate is outside 0..3.</exception>
    public float this[int row, int column]
    {
        get
        {
            CheckIndex(row, column);
            return Read(row, column);
        }

        set
        {
            CheckIndex(row, column);
            Write(row, column, value);
        }
    }

    /// <summary>Builds the translation by <paramref name="t"/>.</summary>
    public static Mat4 Translation(Vec3 t)
    {
        Mat4 m = Identity;
        m._m03 = t.X;
        m._m13 = t.Y;
        m._m23 = t.Z;
        return m;
    }

    /// <summary>Builds the diagonal scale by <paramref name="s"/>.</summary>
    public static Mat4 ScaleMatrix(Vec3 s)
    {
        Mat4 m = Identity;
        m._m00 = s.X;
        m._m11 = s.Y;
        m._m22 = s.Z;
        return m;
    }

    /// <summary>Rotation about X by <paramref name="degrees"/> (REQ-E3D-407).</summary>
    public static Mat4 RotationX(float degrees)
    {
        float r = degrees * MathF.PI / 180f;
        float c = MathF.Cos(r);
        float s = MathF.Sin(r);
        Mat4 m = Identity;
        m._m11 = c;
        m._m12 = -s;
        m._m21 = s;
        m._m22 = c;
        return m;
    }

    /// <summary>Rotation about Y by <paramref name="degrees"/> (REQ-E3D-407).</summary>
    public static Mat4 RotationY(float degrees)
    {
        float r = degrees * MathF.PI / 180f;
        float c = MathF.Cos(r);
        float s = MathF.Sin(r);
        Mat4 m = Identity;
        m._m00 = c;
        m._m02 = s;
        m._m20 = -s;
        m._m22 = c;
        return m;
    }

    /// <summary>Rotation about Z by <paramref name="degrees"/> (REQ-E3D-407).</summary>
    public static Mat4 RotationZ(float degrees)
    {
        float r = degrees * MathF.PI / 180f;
        float c = MathF.Cos(r);
        float s = MathF.Sin(r);
        Mat4 m = Identity;
        m._m00 = c;
        m._m01 = -s;
        m._m10 = s;
        m._m11 = c;
        return m;
    }

    /// <summary>Matrix product.</summary>
    public static Mat4 operator *(Mat4 a, Mat4 b)
    {
        Mat4 r = default;
        for (int i = 0; i < 4; i++)
        {
            for (int j = 0; j < 4; j++)
            {
                float sum = 0f;
                for (int k = 0; k < 4; k++)
                {
                    sum += a.Read(i, k) * b.Read(k, j);
                }

                r.Write(i, j, sum);
            }
        }

        return r;
    }

    /// <summary>Applies the matrix to the point <paramref name="v"/> (w = 1).</summary>
    public Vec3 TransformPoint(Vec3 v) => new(
        (_m00 * v.X) + (_m01 * v.Y) + (_m02 * v.Z) + _m03,
        (_m10 * v.X) + (_m11 * v.Y) + (_m12 * v.Z) + _m13,
        (_m20 * v.X) + (_m21 * v.Y) + (_m22 * v.Z) + _m23);

    /// <summary>Copies the sixteen cells in row-major order.</summary>
    public void CopyTo(Span<float> destination)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(destination.Length, 16, nameof(destination));
        for (int i = 0; i < 4; i++)
        {
            for (int j = 0; j < 4; j++)
            {
                destination[(i * 4) + j] = Read(i, j);
            }
        }
    }

    /// <inheritdoc/>
    public bool Equals(Mat4 other)
    {
        for (int i = 0; i < 4; i++)
        {
            for (int j = 0; j < 4; j++)
            {
                if (!Read(i, j).Equals(other.Read(i, j)))
                {
                    return false;
                }
            }
        }

        return true;
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Mat4 other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(_m00, _m11, _m22, _m33, _m03, _m13, _m23);

    /// <summary>Equality.</summary>
    public static bool operator ==(Mat4 left, Mat4 right) => left.Equals(right);

    /// <summary>Inequality.</summary>
    public static bool operator !=(Mat4 left, Mat4 right) => !left.Equals(right);

    private static void CheckIndex(int row, int column)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)row, 3u, nameof(row));
        ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)column, 3u, nameof(column));
    }

    private float Read(int row, int column) => (row, column) switch
    {
        (0, 0) => _m00,
        (0, 1) => _m01,
        (0, 2) => _m02,
        (0, 3) => _m03,
        (1, 0) => _m10,
        (1, 1) => _m11,
        (1, 2) => _m12,
        (1, 3) => _m13,
        (2, 0) => _m20,
        (2, 1) => _m21,
        (2, 2) => _m22,
        (2, 3) => _m23,
        (3, 0) => _m30,
        (3, 1) => _m31,
        (3, 2) => _m32,
        _ => _m33,
    };

    private void Write(int row, int column, float value)
    {
        switch ((row, column))
        {
            case (0, 0): _m00 = value; break;
            case (0, 1): _m01 = value; break;
            case (0, 2): _m02 = value; break;
            case (0, 3): _m03 = value; break;
            case (1, 0): _m10 = value; break;
            case (1, 1): _m11 = value; break;
            case (1, 2): _m12 = value; break;
            case (1, 3): _m13 = value; break;
            case (2, 0): _m20 = value; break;
            case (2, 1): _m21 = value; break;
            case (2, 2): _m22 = value; break;
            case (2, 3): _m23 = value; break;
            case (3, 0): _m30 = value; break;
            case (3, 1): _m31 = value; break;
            case (3, 2): _m32 = value; break;
            default: _m33 = value; break;
        }
    }
}
