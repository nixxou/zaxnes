namespace Zax.Platform.Input;

/// <summary>
/// Turns raw stick and trigger values into the 0-to-1 values the digital threshold of
/// SPEC-08 REQ-INP-030 is applied to, with the dead zones of REQ-INP-031 (DEC-08-8).
/// </summary>
/// <remarks>
/// <para>
/// Sticks get a <b>radial</b> dead zone: the magnitude of the two axes together decides whether
/// the stick is at rest, so a diagonal push is treated the same as an axis-aligned one.
/// </para>
/// <para>
/// AMD-021 R2-011 settles what the 0.34 of REQ-INP-030 is measured on, and <b>replaces AMD-020
/// for Q-048</b>: the threshold applies to the <b>shaped</b> value of REQ-INP-031 — radial dead
/// zone 0.2 and saturation 0.9 for a stick, dead zone 0.2 and saturation 0.9 for a trigger, with
/// a linear re-spread between the two. The press decision therefore reads exactly the same
/// number an analog consumer reads, which is what closes Q-056: the shaping is observable again.
/// R2-011 also amends the trigger dead zone of REQ-INP-031 from 0.1 to 0.2.
/// </para>
/// <para>
/// None of these values is a user setting, exactly as in the original (REQ-INP-031, notes).
/// </para>
/// </remarks>
public static class AnalogCalibration
{
    /// <summary>
    /// The digital threshold of REQ-INP-030: an analog command bound to an action is pressed
    /// from 0.34 of its <b>shaped</b> travel (AMD-021 R2-011).
    /// </summary>
    public const float PressThreshold = 0.34f;

    /// <summary>Radial dead zone of a stick, below which the value is zero (REQ-INP-031).</summary>
    public const float StickDeadZone = 0.2f;

    /// <summary>Radial saturation of a stick, above which the value is one (REQ-INP-031).</summary>
    public const float StickSaturation = 0.9f;

    /// <summary>Dead zone of a trigger (REQ-INP-031 amended by AMD-021 R2-011).</summary>
    public const float TriggerDeadZone = 0.2f;

    /// <summary>Saturation of a trigger (AMD-021 R2-011).</summary>
    public const float TriggerSaturation = 0.9f;

    /// <summary>
    /// The travel of the half-axis <paramref name="direction"/> of a stick whose two raw axes are
    /// <paramref name="raw"/> and <paramref name="other"/>, in 0 to 1: the shaped travel of that
    /// half (R2-011), or 0 when the stick is inside its radial dead zone.
    /// </summary>
    /// <param name="raw">Raw value of the axis being read, −1 to 1.</param>
    /// <param name="other">Raw value of the stick's other axis, −1 to 1.</param>
    /// <param name="direction">+1 for the positive half, −1 for the negative half.</param>
    // R2-011 (supersedes AMD-020 / Q-048), Q-054
    public static float HalfAxis(float raw, float other, int direction)
    {
        float shaped = Stick(raw, other);
        return direction < 0 ? MathF.Max(0f, -shaped) : MathF.Max(0f, shaped);
    }

    /// <summary>
    /// The <b>shaped</b> value of one axis of a stick, −1 to 1, after the radial dead zone and
    /// the saturation of REQ-INP-031: the magnitude is re-spread linearly between the two bounds
    /// and the direction is kept.
    /// </summary>
    /// <param name="raw">Raw value of the axis being read.</param>
    /// <param name="other">Raw value of the stick's other axis.</param>
    public static float Stick(float raw, float other)
    {
        float magnitude = Magnitude(raw, other);
        if (magnitude <= StickDeadZone)
        {
            return 0f;
        }

        if (magnitude >= StickSaturation)
        {
            // At or past saturation the direction is kept but the length becomes exactly 1.
            return raw / magnitude;
        }

        float rescaled = (magnitude - StickDeadZone) / (StickSaturation - StickDeadZone);
        return raw / magnitude * rescaled;
    }

    /// <summary>
    /// The shaped value of the half-axis <paramref name="direction"/>, 0 to 1. Since R2-011 this
    /// is the value the press decision reads, so it is <see cref="HalfAxis"/> itself.
    /// </summary>
    public static float ShapedHalfAxis(float raw, float other, int direction)
        => HalfAxis(raw, other, direction);

    /// <summary>
    /// The travel of a trigger, 0 to 1: its shaped travel, the dead zone and the saturation of
    /// R2-011 re-spread linearly over the whole range.
    /// </summary>
    /// <param name="raw">Raw trigger travel, 0 to 1.</param>
    // R2-011 (supersedes AMD-020 / Q-048), Q-054
    public static float Trigger(float raw)
    {
        if (raw <= TriggerDeadZone)
        {
            return 0f;
        }

        if (raw >= TriggerSaturation)
        {
            return 1f;
        }

        return (raw - TriggerDeadZone) / (TriggerSaturation - TriggerDeadZone);
    }

    /// <summary>
    /// The shaped value of a trigger, 0 to 1. Since R2-011 this is the value the press decision
    /// reads, so it is <see cref="Trigger"/> itself.
    /// </summary>
    public static float ShapedTrigger(float raw) => Trigger(raw);

    /// <summary>
    /// Whether an analog travel counts as pressed (REQ-INP-030): 0.34 or more of the shaped
    /// value (AMD-021 R2-011).
    /// </summary>
    public static bool IsPressed(float travel) => travel >= PressThreshold;

    private static float Magnitude(float raw, float other)
        => MathF.Sqrt((raw * raw) + (other * other));
}
