namespace Zax.Scripting.Tests;

/// <summary>
/// AMD-059 REQ-LUA-117, the complement of the <c>CS</c> table of AMD-040 REQ-LUA-112:
/// <c>CS.Nes3D.CameraControl.DistanceMax</c>, <c>CS.Nes3D.SettingManager.DefaultMsaa</c> and
/// the two style identifiers of <c>CS.Nes3D.VisualStyle</c>.
/// </summary>
/// <remarks>
/// Before this lot, reading those paths indexed a nil and raised, which cost the faulty script
/// all its hooks (REQ-LUA-062). The rule of REQ-LUA-112 — every field the requirements do not
/// name reads nil, and a write anywhere is dropped — still holds for the three new tables.
/// </remarks>
public sealed class Amd059CsTableTests
{
    private static ScriptSession Session() => new ScriptProfileBuilder().Script("a", string.Empty).Start();

    /// <summary>REQ-LUA-117: the four new paths are tables and numbers, with their values.</summary>
    [Fact] // REQ-LUA-117
    public void The_three_new_tables_carry_the_values_of_the_requirement()
    {
        using ScriptSession session = Session();

        Assert.Equal("table", session.Runtime.EvaluateText("return type(CS.Nes3D.CameraControl)"));
        Assert.Equal("table", session.Runtime.EvaluateText("return type(CS.Nes3D.SettingManager)"));
        Assert.Equal("table", session.Runtime.EvaluateText("return type(CS.Nes3D.VisualStyle)"));

        Assert.Equal(512, session.Runtime.EvaluateNumber("return CS.Nes3D.CameraControl.DistanceMax"));
        Assert.Equal(2, session.Runtime.EvaluateNumber("return CS.Nes3D.SettingManager.DefaultMsaa"));
        Assert.Equal(0, session.Runtime.EvaluateNumber("return CS.Nes3D.VisualStyle.Classic"));
        Assert.Equal(4, session.Runtime.EvaluateNumber("return CS.Nes3D.VisualStyle.SoftPixelLight"));
    }

    /// <summary>
    /// REQ-LUA-117: the values are numbers, not enumerated objects — the divergence (D) of
    /// AMD-059 §3 — so arithmetic and comparison work on them.
    /// </summary>
    [Fact] // REQ-LUA-117
    public void The_four_values_are_numbers()
    {
        using ScriptSession session = Session();

        foreach (string path in (string[])
            [
                "CS.Nes3D.CameraControl.DistanceMax",
                "CS.Nes3D.SettingManager.DefaultMsaa",
                "CS.Nes3D.VisualStyle.Classic",
                "CS.Nes3D.VisualStyle.SoftPixelLight",
            ])
        {
            Assert.Equal("number", session.Runtime.EvaluateText($"return type({path})"));
        }
    }

    /// <summary>
    /// REQ-LUA-117 — the amendment criterion: the identifiers write and read back through
    /// <c>Setting.VisualStyle</c>, so the comparison a 2.0 script makes holds.
    /// </summary>
    [Fact] // REQ-LUA-117, REQ-SCN-078
    public void The_style_identifiers_round_trip_through_the_setting()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("Setting.VisualStyle = CS.Nes3D.VisualStyle.SoftPixelLight");
        Assert.True(session.Runtime.EvaluateNumber(
            "return Setting.VisualStyle == CS.Nes3D.VisualStyle.SoftPixelLight and 1 or 0") != 0d);

        session.Runtime.Evaluate("Setting.VisualStyle = CS.Nes3D.VisualStyle.Classic");
        Assert.True(session.Runtime.EvaluateNumber(
            "return Setting.VisualStyle == CS.Nes3D.VisualStyle.Classic and 1 or 0") != 0d);
    }

    /// <summary>
    /// REQ-LUA-117: the identifiers serve as arguments of the style-parameter methods
    /// (REQ-LUA-114) — Classic has no parameter, Enhanced has them all.
    /// </summary>
    [Fact] // REQ-LUA-117, REQ-LUA-114
    public void The_style_identifiers_serve_as_method_arguments()
    {
        using ScriptSession session = Session();

        Assert.True(session.Runtime.EvaluateNumber(
            "return Setting:SetVisualStyleParameter(CS.Nes3D.VisualStyle.SoftPixelLight, 'spl.bend_zx', 60)"
                + " and 1 or 0") != 0d);
        Assert.Equal(
            60,
            session.Runtime.EvaluateNumber(
                "return Setting:GetVisualStyleParameter(CS.Nes3D.VisualStyle.SoftPixelLight, 'spl.bend_zx')"));

        Assert.False(session.Runtime.EvaluateNumber(
            "return Setting:SetVisualStyleParameter(CS.Nes3D.VisualStyle.Classic, 'spl.bend_zx', 60)"
                + " and 1 or 0") != 0d);
    }

    /// <summary>
    /// REQ-LUA-117 — every other field of the three new tables reads nil, without an error, as
    /// REQ-LUA-112 already required of <c>Info</c>.
    /// </summary>
    [Fact] // REQ-LUA-117, REQ-LUA-112
    public void Every_other_field_of_the_three_tables_reads_as_nil()
    {
        using ScriptSession session = Session();

        foreach (string path in (string[])
            [
                "CS.Nes3D.CameraControl.Autre",
                "CS.Nes3D.CameraControl.Distance",
                "CS.Nes3D.SettingManager.Autre",
                "CS.Nes3D.SettingManager.Msaa",
                "CS.Nes3D.VisualStyle.Autre",
                "CS.Nes3D.VisualStyle.Enhanced",
                "CS.Nes3D.Autre",
                "CS.Autre",
            ])
        {
            Assert.Equal("nil", session.Runtime.EvaluateText($"return type({path})"));
        }

        Assert.Empty(session.Log.Errors);
    }

    /// <summary>
    /// REQ-LUA-117 — the tables of AMD-040 are still there: adding siblings to <c>Info</c> did
    /// not displace it.
    /// </summary>
    [Fact] // REQ-LUA-117, REQ-LUA-112
    public void The_info_table_of_AMD_040_is_untouched()
    {
        using ScriptSession session = Session();

        Assert.Equal("2.0", session.Runtime.EvaluateText("return CS.Nes3D.Info.Version"));
        Assert.Equal("Zaxnes", session.Runtime.EvaluateText("return CS.Nes3D.Info.Title"));
        Assert.Equal("nil", session.Runtime.EvaluateText("return type(CS.Nes3D.Info.AppId)"));
    }

    /// <summary>
    /// REQ-LUA-117 — « une écriture dans ces tables n'a aucun effet sur le programme » : it
    /// raises nothing and does not even change what the next read gives. The metatables stay
    /// hidden, so the shield cannot be taken off.
    /// </summary>
    [Fact] // REQ-LUA-117, REQ-LUA-112
    public void A_write_into_the_three_tables_has_no_effect_and_no_error()
    {
        using ScriptSession session = Session();
        int antiAlias = session.Runtime.Settings.AntiAlias;

        session.Runtime.Evaluate("""
            CS.Nes3D.CameraControl.DistanceMax = 1
            CS.Nes3D.SettingManager.DefaultMsaa = 8
            CS.Nes3D.VisualStyle.Classic = 9
            CS.Nes3D.VisualStyle.SoftPixelLight = 9
            CS.Nes3D.CameraControl = nil
            CS.Nes3D.Quelque = 1
            """);

        Assert.Equal(512, session.Runtime.EvaluateNumber("return CS.Nes3D.CameraControl.DistanceMax"));
        Assert.Equal(2, session.Runtime.EvaluateNumber("return CS.Nes3D.SettingManager.DefaultMsaa"));
        Assert.Equal(0, session.Runtime.EvaluateNumber("return CS.Nes3D.VisualStyle.Classic"));
        Assert.Equal(4, session.Runtime.EvaluateNumber("return CS.Nes3D.VisualStyle.SoftPixelLight"));
        Assert.Equal("nil", session.Runtime.EvaluateText("return type(CS.Nes3D.Quelque)"));

        // Nothing of the program moved: DefaultMsaa is not the setting, it only announces it.
        Assert.Equal(2, session.Runtime.Settings.Msaa);
        Assert.Equal(antiAlias, session.Runtime.Settings.AntiAlias);
        Assert.Empty(session.Log.Errors);

        Assert.Equal("CS", session.Runtime.EvaluateText("return getmetatable(CS.Nes3D.CameraControl)"));
        Assert.Equal("CS", session.Runtime.EvaluateText("return getmetatable(CS.Nes3D.VisualStyle)"));
    }

    /// <summary>
    /// <c>DefaultMsaa</c> announces the default of <c>Setting.Msaa</c> of REQ-LUA-116: the two
    /// must not drift apart.
    /// </summary>
    [Fact] // REQ-LUA-117, REQ-LUA-116
    public void The_announced_default_is_the_default_of_the_setting()
    {
        using ScriptSession session = Session();

        Assert.True(session.Runtime.EvaluateNumber(
            "return Setting.Msaa == CS.Nes3D.SettingManager.DefaultMsaa and 1 or 0") != 0d);
    }
}
