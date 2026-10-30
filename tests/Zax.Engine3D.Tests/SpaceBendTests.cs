namespace Zax.Engine3D.Tests;

/// <summary>
/// The space bending of AMD-043 REQ-SCN-067: the thirteen test vectors of the amendment, the
/// continuity at θ = 0, the inverse REQ-INP-124 relies on and the Jacobian REQ-SCN-068
/// transports the normal with.
/// </summary>
public sealed class SpaceBendTests
{
    /// <summary>The tolerance the amendment's table is given with, ±0,05 scene unit.</summary>
    private const float Tolerance = 0.05f;

    /// <summary>
    /// The thirteen vectors of REQ-SCN-067, in the order the amendment lists them: the three
    /// axes on their own, the depth, both signs of curvature, the two arc centres and the two
    /// cases where the 95 % depth limit bites.
    /// </summary>
    public static TheoryData<int, int, int, bool, float, float, float, float, float, float> Vectors => new()
    {
        { 60, 0, 0, false, 128f, 120f, 0f, 128f, 120f, -8.43f },
        { 60, 0, 0, false, 256f, 120f, 0f, 251.64f, 120f, 24.70f },
        { 60, 0, 0, false, 0f, 120f, 0f, 4.36f, 120f, 24.70f },
        { 60, 0, 0, false, 256f, 120f, 10f, 246.58f, 120f, 33.46f },
        { -60, 0, 0, false, 256f, 120f, 0f, 251.64f, 120f, -24.70f },
        { -180, 0, 0, false, 256f, 120f, -200f, 132.53f, 120f, -64.00f },
        { 0, 60, 0, false, 128f, 240f, 0f, 128f, 235.91f, 23.16f },
        { 0, 60, 0, false, 128f, 0f, 0f, 128f, 4.09f, 23.16f },
        { 0, 0, 60, false, 0f, 120f, 0f, 4.36f, 95.30f, 0f },
        { 0, 0, 60, false, 128f, 240f, 0f, 128f, 249.81f, 0f },
        { 0, 0, 60, true, 0f, 0f, 0f, 4.36f, -24.70f, 0f },
        { 0, 0, 60, true, 128f, 240f, 0f, 128f, 251.19f, 0f },
        { 0, 0, -180, false, 128f, 240f, 0f, 128f, 179.47f, 0f },
    };

    /// <summary>REQ-SCN-067: the thirteen vectors of the amendment, to ±0,05 unit.</summary>
    [Theory] // REQ-SCN-067
    [MemberData(nameof(Vectors))]
    public void REQ_SCN_067_the_amendment_vectors(
        int bendLeftRight,
        int bendTopBottom,
        int arcHorizontal,
        bool arcAtEdge,
        float x,
        float y,
        float z,
        float expectedX,
        float expectedY,
        float expectedZ)
    {
        SpaceBend bend = new(bendLeftRight, bendTopBottom, arcHorizontal, arcAtEdge);
        Vec3 image = bend.Apply(new Vec3(x, y, z));

        Assert.Equal(expectedX, image.X, Tolerance);
        Assert.Equal(expectedY, image.Y, Tolerance);
        Assert.Equal(expectedZ, image.Z, Tolerance);
    }

    /// <summary>
    /// REQ-SCN-067: an axis whose angle is zero transforms nothing, and <c>B</c> is continuous
    /// in θ — the limits <c>k → 0</c>, <c>f → 1</c>, <c>h → 0</c> make it the identity there.
    /// </summary>
    [Fact] // REQ-SCN-067
    public void REQ_SCN_067_zero_is_the_identity_and_B_is_continuous_in_theta()
    {
        Assert.True(SpaceBend.None.IsIdentity);
        Assert.True(new SpaceBend(0f, 0f, 0f, arcAtEdge: false).IsIdentity);

        Vec3 point = new(200f, 60f, 12f);
        Assert.Equal(point, SpaceBend.None.Apply(point));

        // Approaching zero from both sides, the image converges on the point itself.
        foreach (float angle in (float[])[1f, 0.1f, 0.01f, -0.01f, -0.1f, -1f])
        {
            Vec3 image = new SpaceBend(angle, angle, angle, arcAtEdge: true).Apply(point);
            float drift = Vec3.Distance(image, point);
            Assert.True(drift < 2f * MathF.Abs(angle), $"θ = {angle}° moved the point by {drift}.");
        }

        // Under the threshold the amendment allows, the axis is exactly the identity.
        SpaceBend tiny = new(1e-4f, 0f, 0f, arcAtEdge: false);
        Assert.True(tiny.IsIdentity);
    }

    /// <summary>
    /// REQ-SCN-067: a positive angle pushes the edges away (bulge), a negative one brings them
    /// closer (wrapped around the player). The z axis is the depth of the scene.
    /// </summary>
    [Fact] // REQ-SCN-067
    public void REQ_SCN_067_the_sign_of_the_angle_picks_the_direction_of_the_bulge()
    {
        Vec3 edge = new(256f, 120f, 0f);
        Assert.True(new SpaceBend(60f, 0f, 0f, false).Apply(edge).Z > 0f);
        Assert.True(new SpaceBend(-60f, 0f, 0f, false).Apply(edge).Z < 0f);
    }

    /// <summary>
    /// AMD-043 §4: <c>B</c> is invertible wherever the depth limit does not apply, which is
    /// what the Zapper aim of REQ-INP-124 is built on.
    /// </summary>
    [Fact] // REQ-INP-124, REQ-SCN-067
    public void REQ_INP_124_B_is_invertible_away_from_the_depth_limit()
    {
        SpaceBend bend = new(90f, -30f, 60f, arcAtEdge: true);

        for (float x = -16f; x <= 272f; x += 32f)
        {
            for (float y = -16f; y <= 256f; y += 32f)
            {
                foreach (float z in (float[])[-24f, 0f, 16f])
                {
                    Vec3 point = new(x, y, z);
                    Assert.True(bend.TryInvert(bend.Apply(point), out Vec3 back), $"{point}");
                    Assert.True(
                        Vec3.Distance(point, back) < 0.01f,
                        $"{point} came back as {back}.");
                }
            }
        }
    }

    /// <summary>
    /// REQ-SCN-067 step 2 and AMD-043 §4: past 95 % of the radius the depth is held back, so
    /// the transform stops being injective and the inverse refuses to answer.
    /// </summary>
    [Fact] // REQ-SCN-067, REQ-INP-124
    public void REQ_SCN_067_the_depth_limit_is_where_the_inverse_stops()
    {
        SpaceBend bend = new(-180f, 0f, 0f, arcAtEdge: false);

        // The sixth vector of the table: the limit is active there.
        Vec3 far = new(256f, 120f, -200f);
        Vec3 farther = new(256f, 120f, -400f);
        Vec3 image = bend.Apply(far);
        Assert.True(Vec3.Distance(image, bend.Apply(farther)) < 0.05f, "both are held at the limit.");
        Assert.False(bend.TryInvert(image, out _));

        // A point well short of the limit is still invertible.
        Assert.True(bend.TryInvert(bend.Apply(new Vec3(256f, 120f, -20f)), out Vec3 back));
        Assert.True(Vec3.Distance(back, new Vec3(256f, 120f, -20f)) < 0.01f);
    }

    /// <summary>
    /// REQ-SCN-068 (amending REQ-SCN-028): the shading normal is the mesh normal transported by
    /// the curvature. The Jacobian it is built from agrees with finite differences of
    /// <see cref="SpaceBend.Apply"/>.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-028
    public void REQ_SCN_068_the_jacobian_agrees_with_finite_differences()
    {
        SpaceBend bend = new(90f, -60f, 30f, arcAtEdge: true);
        Span<float> jacobian = stackalloc float[9];

        foreach (Vec3 point in (Vec3[])
        [
            new(0f, 8f, 0f),
            new(128f, 120f, 0f),
            new(256f, 232f, 12f),
            new(64f, 40f, -30f),
        ])
        {
            bend.Jacobian(point, jacobian);
            const float step = 0.05f;
            for (int axis = 0; axis < 3; axis++)
            {
                Vec3 offset = new(axis == 0 ? step : 0f, axis == 1 ? step : 0f, axis == 2 ? step : 0f);
                Vec3 forward = bend.Apply(point + offset);
                Vec3 backward = bend.Apply(point - offset);
                Vec3 slope = (forward - backward) * (1f / (2f * step));

                Assert.Equal(slope.X, jacobian[axis], 0.002f);
                Assert.Equal(slope.Y, jacobian[3 + axis], 0.002f);
                Assert.Equal(slope.Z, jacobian[6 + axis], 0.002f);
            }
        }
    }

    /// <summary>
    /// REQ-SCN-068: the transported normal stays perpendicular to the bent surface — here the
    /// image of the plane <c>z = 0</c>, whose tangents are the images of two directions of the
    /// plane.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-028
    public void REQ_SCN_068_the_transported_normal_stays_perpendicular_to_the_surface()
    {
        SpaceBend bend = new(90f, 30f, 0f, arcAtEdge: false);
        Vec3 point = new(224f, 72f, 0f);
        Vec3 normal = bend.TransformNormal(point, new Vec3(0f, 0f, -1f));

        const float step = 0.1f;
        Vec3 alongX = bend.Apply(point + new Vec3(step, 0f, 0f)) - bend.Apply(point - new Vec3(step, 0f, 0f));
        Vec3 alongY = bend.Apply(point + new Vec3(0f, step, 0f)) - bend.Apply(point - new Vec3(0f, step, 0f));

        Assert.Equal(1f, MathF.Sqrt(normal.SquaredLength), 1e-3f);
        Assert.Equal(0f, Dot(normal, Normalised(alongX)), 1e-3f);
        Assert.Equal(0f, Dot(normal, Normalised(alongY)), 1e-3f);

        // An identity bend leaves the normal exactly as it was.
        Assert.Equal(new Vec3(0f, 0f, -1f), SpaceBend.None.TransformNormal(point, new Vec3(0f, 0f, -1f)));
    }

    /// <summary>
    /// REQ-SCN-067: the scene scale <c>s</c> divides the curvature, so a scaled scene bends by
    /// the same total angle over its own width.
    /// </summary>
    [Fact] // REQ-SCN-067, REQ-SCN-002
    public void REQ_SCN_067_the_scene_scale_divides_the_curvature()
    {
        SpaceBend desktop = new(60f, 0f, 0f, arcAtEdge: false);
        SpaceBend scaled = new(60f, 0f, 0f, arcAtEdge: false, sceneScale: 2f);

        Assert.Equal(desktop.Curvature.X, 2f * scaled.Curvature.X, 1e-6f);
        Assert.Equal(desktop.Recentring.X, scaled.Recentring.X, 1e-6f);
    }

    private static float Dot(Vec3 a, Vec3 b) => (a.X * b.X) + (a.Y * b.Y) + (a.Z * b.Z);

    private static Vec3 Normalised(Vec3 v)
    {
        float length = MathF.Sqrt(v.SquaredLength);
        return length <= 1e-9f ? v : v * (1f / length);
    }
}
