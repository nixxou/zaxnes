namespace Zax.App.Loop;

/// <summary>
/// The integer speed setting <c>s</c> of SPEC-07 REQ-LOOP-015 and the factor <c>k</c> it maps to.
/// </summary>
/// <remarks>
/// <c>k = 1 + s/4</c> for <c>s ≥ 0</c> and <c>k = 1 / (1 − s/4)</c> for <c>s &lt; 0</c>, so the
/// setting is differential rather than symmetric: −8 gives 1/3 and +8 gives 3. There is no
/// separate slow-motion mode; the negative half of the setting is it (REQ-LOOP-015, notes).
/// </remarks>
public static class SpeedSetting
{
    /// <summary>Lowest value of the setting (REQ-LOOP-015, DEC-07-Q1).</summary>
    public const int Minimum = -8;

    /// <summary>Highest value of the setting.</summary>
    public const int Maximum = 8;

    /// <summary>Default value, which is the factor 1.</summary>
    public const int Default = 0;

    /// <summary>
    /// The setting fast forward applies, whatever the user's setting is (REQ-LOOP-016,
    /// DEC-07-Q2): <c>s = 2</c>, so <c>k = 1.5</c>.
    /// </summary>
    public const int FastForward = 2;

    /// <summary>Brings a value back into <c>[−8 ; 8]</c> (SPEC-09 REQ-PERS-003 rule 3).</summary>
    public static int Clamp(int setting) => Math.Clamp(setting, Minimum, Maximum);

    /// <summary>The factor <c>k</c> of a setting, after clamping.</summary>
    public static double Factor(int setting)
    {
        int s = Clamp(setting);
        return s >= 0 ? 1.0 + (s / 4.0) : 1.0 / (1.0 - (s / 4.0));
    }

    /// <summary>The factor fast forward uses, 1.5.</summary>
    public static double FastForwardFactor => Factor(FastForward);
}
