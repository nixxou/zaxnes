namespace Zax.Render;

/// <summary>
/// The critical smoothing filter of SPEC-06 REQ-SCN-019: a critically damped spring that never
/// overshoots, with characteristic time <c>Tₛ</c>.
/// </summary>
/// <remarks>
/// One instance holds the internal velocity of one scalar. The camera applies it separately to
/// <c>rx</c>, <c>ry</c>, <c>rz</c> (without folding modulo 360°), to the distance and to the
/// three components of the target point (REQ-SCN-019).
/// </remarks>
public struct CriticalSmoothing
{
    /// <summary>The default characteristic time, 0.4 s (REQ-SCN-019).</summary>
    public const float DefaultSmoothTime = 0.4f;

    private float _velocity;

    /// <summary>The internal velocity, zero before the first step.</summary>
    public float Velocity => _velocity;

    /// <summary>Forgets the velocity, so the next step starts from rest.</summary>
    public void Reset() => _velocity = 0f;

    /// <summary>
    /// Moves <paramref name="value"/> towards <paramref name="target"/> over
    /// <paramref name="deltaSeconds"/>, with the characteristic time
    /// <paramref name="smoothTime"/>.
    /// </summary>
    /// <returns>The new value.</returns>
    public float Step(float value, float target, float smoothTime, float deltaSeconds)
    {
        if (deltaSeconds <= 0f)
        {
            return value;
        }

        // ω = 2 / Tₛ, x = ω·Δt, k = 1 / (1 + x + 0.48 x² + 0.235 x³) (REQ-SCN-019).
        float omega = 2f / MathF.Max(smoothTime, 1e-4f);
        float x = omega * deltaSeconds;
        float k = 1f / (1f + x + (0.48f * x * x) + (0.235f * x * x * x));

        float delta = value - target;
        float w = (_velocity + (omega * delta)) * deltaSeconds;
        _velocity = (_velocity - (omega * w)) * k;
        float result = target + ((delta + w) * k);

        // The setpoint must never be overshot (REQ-SCN-019).
        if ((target - value > 0f) == (result > target))
        {
            _velocity = 0f;
            return target;
        }

        return result;
    }
}
