namespace Zax.Engine3D;

/// <summary>
/// The space bending <c>B</c> of SPEC-06 REQ-SCN-067 (AMD-043): the purely geometric curvature
/// the renderer applies to the game scene — the bulged glass of a cathode-ray television, or a
/// side-scrolling level rolled onto a tiny planet.
/// </summary>
/// <remarks>
/// <para>
/// It lives here, in the platform-free geometry library, for two reasons: it is a closed-form
/// function of a point that has to be unit-tested without a GL context, and both
/// <c>Zax.Render</c> (which mirrors it in GLSL) and <c>Zax.App</c> (which inverts it for the
/// Zapper aim of REQ-INP-124) need it. It takes part in no conversion: AMD-043 §1 keeps the
/// curvature out of the profiles and out of the 2D → 3D conversion.
/// </para>
/// <para>
/// <b>Not</b> the animated deformation of REQ-SCN-043: that one moves the vertices of a motif in
/// its own local frame, before the object transform; this one is applied last, to the final
/// scene position (REQ-SCN-068).
/// </para>
/// </remarks>
public readonly struct SpaceBend : IEquatable<SpaceBend>
{
    /// <summary>Reference length of the left-right bend and of the horizontal arc, 256.</summary>
    public const float HorizontalLength = 256f;

    /// <summary>Reference length of the top-bottom bend, 240.</summary>
    public const float VerticalLength = 240f;

    /// <summary>
    /// How much of the cylinder radius a depth may reach before it is held back, 0.95
    /// (REQ-SCN-067 step 2).
    /// </summary>
    public const float DepthLimit = 0.95f;

    /// <summary>
    /// Angles under this many radians are treated as zero, which REQ-SCN-067 allows up to
    /// 10⁻⁴ rad. <c>B</c> is continuous in θ and is the identity at θ = 0.
    /// </summary>
    public const float AngleThreshold = 1e-4f;

    /// <summary>
    /// The arc pivot when the centre is the bottom edge of the reference frame: <c>−120 · s</c>
    /// relative to <c>C</c>, that is <c>y = 0</c>, eight units below the bottom of the NES image
    /// (REQ-SCN-066).
    /// </summary>
    public const float EdgePivotOffset = -120f;

    private readonly Axis _leftRight;
    private readonly Axis _topBottom;
    private readonly Axis _arc;
    private readonly float _pivot;

    /// <summary>Builds the transform of the four settings of REQ-SCN-066.</summary>
    /// <param name="bendLeftRight">
    /// <c>scene.bend_lr</c> in degrees: positive bulges the scene outwards, negative curves it
    /// around the player.
    /// </param>
    /// <param name="bendTopBottom"><c>scene.bend_tb</c> in degrees.</param>
    /// <param name="arcHorizontal"><c>scene.arc_h</c> in degrees.</param>
    /// <param name="arcAtEdge">
    /// <c>scene.arc_center</c>: false for the screen centre, true for the bottom edge of the
    /// reference frame.
    /// </param>
    /// <param name="sceneScale">The scene scale <c>s</c> of REQ-SCN-002; 1 on desktop.</param>
    public SpaceBend(
        float bendLeftRight,
        float bendTopBottom,
        float arcHorizontal,
        bool arcAtEdge,
        float sceneScale = 1f)
    {
        float scale = sceneScale is > 0f and <= float.MaxValue ? sceneScale : 1f;
        _leftRight = new Axis(bendLeftRight, HorizontalLength, scale);
        _topBottom = new Axis(bendTopBottom, VerticalLength, scale);
        _arc = new Axis(arcHorizontal, HorizontalLength, scale);
        _pivot = arcAtEdge ? EdgePivotOffset * scale : 0f;
    }

    /// <summary>The identity: no curvature at all.</summary>
    public static SpaceBend None => default;

    /// <summary>
    /// The centre <c>C = (128, 120, 0)</c> of the reference frame, which is the fixed point of
    /// every axis (REQ-E3D-410, REQ-SCN-067).
    /// </summary>
    public static Vec3 Center => new(128f, 120f, 0f);

    /// <summary>Whether no axis bends anything, in which case <c>B</c> is the identity.</summary>
    public bool IsIdentity => !_leftRight.Active && !_topBottom.Active && !_arc.Active;

    /// <summary>The image <c>B(P)</c> of a scene point (REQ-SCN-067).</summary>
    /// <param name="point">A point of the scene, in scene units.</param>
    public Vec3 Apply(Vec3 point)
    {
        if (IsIdentity)
        {
            return point;
        }

        Vec3 c = Center;
        float u = point.X - c.X;
        float v = point.Y - c.Y;
        float w = point.Z - c.Z;

        if (_leftRight.Active)
        {
            Wrap(_leftRight, ref u, ref w, out _);
        }

        if (_topBottom.Active)
        {
            Wrap(_topBottom, ref v, ref w, out _);
        }

        if (_arc.Active)
        {
            float s = _pivot - v;
            Wrap(_arc, ref u, ref s, out _);
            v = _pivot - s;
        }

        return new Vec3(u + c.X, v + c.Y, w + c.Z);
    }

    /// <summary>
    /// The preimage of <paramref name="bent"/>: the point <c>P</c> such that
    /// <c>B(P) = bent</c> (REQ-INP-124).
    /// </summary>
    /// <returns>
    /// False where the depth limit of REQ-SCN-067 step 2 was reached, which is exactly where
    /// <c>B</c> stops being injective and where AMD-043 §4 leaves the aim unspecified.
    /// </returns>
    public bool TryInvert(Vec3 bent, out Vec3 point)
    {
        point = bent;
        if (IsIdentity)
        {
            return true;
        }

        Vec3 c = Center;
        float u = bent.X - c.X;
        float v = bent.Y - c.Y;
        float w = bent.Z - c.Z;

        // The three steps are undone in the reverse order of REQ-SCN-067 step 3. Each one acts
        // on a pair of components and leaves the third alone, so the chain unwinds exactly.
        if (_arc.Active)
        {
            float s = _pivot - v;
            if (!Unwrap(_arc, ref u, ref s))
            {
                return false;
            }

            v = _pivot - s;
        }

        if (_topBottom.Active && !Unwrap(_topBottom, ref v, ref w))
        {
            return false;
        }

        if (_leftRight.Active && !Unwrap(_leftRight, ref u, ref w))
        {
            return false;
        }

        point = new Vec3(u + c.X, v + c.Y, w + c.Z);
        return true;
    }

    /// <summary>
    /// The Jacobian of <c>B</c> at <paramref name="point"/>, row by row: cell
    /// <c>3 · i + j</c> is <c>∂Bᵢ/∂Pⱼ</c>.
    /// </summary>
    /// <param name="point">A point of the scene, in scene units.</param>
    /// <param name="rowMajor">Nine cells to fill.</param>
    public void Jacobian(Vec3 point, Span<float> rowMajor)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(rowMajor.Length, 9, nameof(rowMajor));

        // Identity to start with, then one 3 × 3 factor per active axis, applied on the left in
        // the order the axes are applied.
        Span<float> j = stackalloc float[9];
        j.Clear();
        j[0] = 1f;
        j[4] = 1f;
        j[8] = 1f;

        Vec3 c = Center;
        float u = point.X - c.X;
        float v = point.Y - c.Y;
        float w = point.Z - c.Z;
        Span<float> step = stackalloc float[9];

        if (_leftRight.Active)
        {
            Wrap(_leftRight, ref u, ref w, out PairDerivatives d);

            // (u, w) move together; v passes through.
            step.Clear();
            step[0] = d.Uu;
            step[2] = d.Uw;
            step[4] = 1f;
            step[6] = d.Wu;
            step[8] = d.Ww;
            Compose(step, j);
        }

        if (_topBottom.Active)
        {
            Wrap(_topBottom, ref v, ref w, out PairDerivatives d);

            step.Clear();
            step[0] = 1f;
            step[4] = d.Uu;
            step[5] = d.Uw;
            step[7] = d.Wu;
            step[8] = d.Ww;
            Compose(step, j);
        }

        if (_arc.Active)
        {
            float s = _pivot - v;
            Wrap(_arc, ref u, ref s, out PairDerivatives d);

            // The pair is (u, p − v) and the result is v ← p − s, so the two cross terms and
            // the second diagonal term change sign twice and once respectively.
            step.Clear();
            step[0] = d.Uu;
            step[1] = -d.Uw;
            step[3] = -d.Wu;
            step[4] = d.Ww;
            step[8] = 1f;
            Compose(step, j);
        }

        j.CopyTo(rowMajor);
    }

    /// <summary>
    /// The mesh normal <paramref name="normal"/> transported by the curvature (REQ-SCN-028 as
    /// amended by REQ-SCN-068): the cofactor matrix of the Jacobian applied to it, normalised.
    /// </summary>
    /// <param name="point">The scene point the normal belongs to, before bending.</param>
    /// <param name="normal">The mesh normal in scene coordinates, before bending.</param>
    public Vec3 TransformNormal(Vec3 point, Vec3 normal)
    {
        if (IsIdentity)
        {
            return normal;
        }

        Span<float> j = stackalloc float[9];
        Jacobian(point, j);

        // The cofactor matrix is det(J) · J⁻ᵗ, built here from the cross products of the
        // columns so that a singular Jacobian needs no special case.
        Vec3 c0 = new(j[0], j[3], j[6]);
        Vec3 c1 = new(j[1], j[4], j[7]);
        Vec3 c2 = new(j[2], j[5], j[8]);
        Vec3 k0 = Cross(c1, c2);
        Vec3 k1 = Cross(c2, c0);
        Vec3 k2 = Cross(c0, c1);

        Vec3 transported = new(
            (k0.X * normal.X) + (k1.X * normal.Y) + (k2.X * normal.Z),
            (k0.Y * normal.X) + (k1.Y * normal.Y) + (k2.Y * normal.Z),
            (k0.Z * normal.X) + (k1.Z * normal.Y) + (k2.Z * normal.Z));

        float length = MathF.Sqrt(transported.SquaredLength);
        return length <= 1e-12f ? normal : transported * (1f / length);
    }

    /// <summary>The curvature <c>k</c> of the left-right bend, for the shader uniforms.</summary>
    public Vec3 Curvature => new(_leftRight.K, _topBottom.K, _arc.K);

    /// <summary>The recentring factors <c>f</c> of the three axes.</summary>
    public Vec3 Recentring => new(_leftRight.F, _topBottom.F, _arc.F);

    /// <summary>The offsets <c>h</c> of the three axes.</summary>
    public Vec3 Offset => new(_leftRight.H, _topBottom.H, _arc.H);

    /// <summary>1 for an axis that bends, 0 for one that does not.</summary>
    public Vec3 Active => new(
        _leftRight.Active ? 1f : 0f,
        _topBottom.Active ? 1f : 0f,
        _arc.Active ? 1f : 0f);

    /// <summary>The arc pivot <c>p</c>, relative to <c>C</c>.</summary>
    public float ArcPivot => _pivot;

    /// <inheritdoc/>
    public bool Equals(SpaceBend other)
        => _leftRight.Equals(other._leftRight)
        && _topBottom.Equals(other._topBottom)
        && _arc.Equals(other._arc)
        && _pivot.Equals(other._pivot);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is SpaceBend other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(_leftRight, _topBottom, _arc, _pivot);

    /// <summary>Equality.</summary>
    public static bool operator ==(SpaceBend left, SpaceBend right) => left.Equals(right);

    /// <summary>Inequality.</summary>
    public static bool operator !=(SpaceBend left, SpaceBend right) => !left.Equals(right);

    /// <summary>
    /// The wrapping <c>E(k, f, h)</c> of REQ-SCN-067 step 2: the pair <c>(u, w)</c> — the
    /// coordinate along the bent axis and the depth — rolled onto a cylinder of radius
    /// <c>1 / k</c>.
    /// </summary>
    private static void Wrap(in Axis axis, ref float u, ref float w, out PairDerivatives d)
    {
        float k = axis.K;
        float f = axis.F;

        // Neither direction of curvature goes past 95 % of the radius.
        bool held = w * k > DepthLimit;
        float depth = held ? DepthLimit / k : w;
        float slope = held ? 0f : 1f;

        float a = u * k;
        float sin = MathF.Sin(a);
        float cos = MathF.Cos(a);
        float radial = 1f - (depth * k);

        d = new PairDerivatives(
            f * radial * cos,
            -f * slope * sin,
            f * radial * sin,
            f * slope * cos);

        u = f * radial * sin / k;
        w = f * (((1f - cos) / k) + (depth * cos) - axis.H);
    }

    /// <summary>
    /// The inverse of <see cref="Wrap"/>. With <c>r = 1 / k</c> and <c>ρ = r − w</c>, the pair
    /// comes out as <c>(f ρ sin a, f (r − ρ cos a) − f h)</c>, so <c>ρ</c> and <c>a</c> are read
    /// back with a square root and an arc tangent.
    /// </summary>
    private static bool Unwrap(in Axis axis, ref float u, ref float w)
    {
        float k = axis.K;
        float f = axis.F;
        float r = 1f / k;

        float along = u / f;
        float deep = r - axis.H - (w / f);
        float magnitude = MathF.Sqrt((along * along) + (deep * deep));
        if (magnitude <= 1e-9f)
        {
            return false;
        }

        float sign = k < 0f ? -1f : 1f;
        float radius = magnitude * sign;
        float depth = r - radius;

        // Where the forward pass held the depth back, every point of a whole half-line maps to
        // the same image: the inverse does not exist there (AMD-043 §4).
        if (depth * k >= DepthLimit - 1e-5f)
        {
            return false;
        }

        float a = MathF.Atan2(along * sign, deep * sign);
        u = a / k;
        w = depth;
        return true;
    }

    /// <summary>Left-multiplies <paramref name="target"/> by <paramref name="step"/>, in place.</summary>
    private static void Compose(ReadOnlySpan<float> step, Span<float> target)
    {
        Span<float> product = stackalloc float[9];
        for (int row = 0; row < 3; row++)
        {
            for (int column = 0; column < 3; column++)
            {
                float sum = 0f;
                for (int inner = 0; inner < 3; inner++)
                {
                    sum += step[(row * 3) + inner] * target[(inner * 3) + column];
                }

                product[(row * 3) + column] = sum;
            }
        }

        product.CopyTo(target);
    }

    private static Vec3 Cross(Vec3 a, Vec3 b) => new(
        (a.Y * b.Z) - (a.Z * b.Y),
        (a.Z * b.X) - (a.X * b.Z),
        (a.X * b.Y) - (a.Y * b.X));

    /// <summary>The four partial derivatives of one <c>E</c>, in the order uu, uw, wu, ww.</summary>
    private readonly record struct PairDerivatives(float Uu, float Uw, float Wu, float Ww);

    /// <summary>
    /// One bent axis: the curvature <c>k = θ / (L · s)</c>, the recentring factor
    /// <c>f = φ / sin φ</c> and the offset <c>h = 2 sin²(φ / 2) / k</c>, with <c>φ = θ / 4</c>
    /// (REQ-SCN-067 step 1).
    /// </summary>
    private readonly record struct Axis
    {
        public Axis(float degrees, float length, float sceneScale)
        {
            float theta = degrees * MathF.PI / 180f;
            if (!float.IsFinite(theta) || MathF.Abs(theta) < AngleThreshold || length <= 0f)
            {
                K = 0f;
                F = 1f;
                H = 0f;
                Active = false;
                return;
            }

            float phi = theta / 4f;
            K = theta / (length * sceneScale);
            F = phi / MathF.Sin(phi);

            float half = MathF.Sin(phi / 2f);
            H = 2f * half * half / K;
            Active = true;
        }

        /// <summary>Curvature <c>k</c>; zero when the axis bends nothing.</summary>
        public float K { get; }

        /// <summary>Recentring factor <c>f</c>; one when the axis bends nothing.</summary>
        public float F { get; }

        /// <summary>Offset <c>h</c>; zero when the axis bends nothing.</summary>
        public float H { get; }

        /// <summary>Whether the axis bends anything at all.</summary>
        public bool Active { get; }
    }
}
