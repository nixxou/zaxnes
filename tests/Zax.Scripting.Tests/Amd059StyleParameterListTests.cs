using Zax.Settings;

namespace Zax.Scripting.Tests;

/// <summary>
/// AMD-059 REQ-LUA-116, <c>Setting.VisualStyleParameters</c>: the list of the style parameters a
/// script has written since their last reset, its elements, the write of nil and the restore of
/// a list a read handed out.
/// </summary>
public sealed class Amd059StyleParameterListTests
{
    private static ScriptSession Session() => new ScriptProfileBuilder().Script("a", string.Empty).Start();

    /// <summary>REQ-LUA-116: before any write the list exists and is empty.</summary>
    [Fact] // REQ-LUA-116
    public void The_list_starts_empty()
    {
        using ScriptSession session = Session();

        Assert.Equal("userdata", session.Runtime.EvaluateText("return type(Setting.VisualStyleParameters)"));
        Assert.Equal(0, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
    }

    /// <summary>
    /// REQ-LUA-116 — the amendment criterion: after a reset of style 4 and two writes, the list
    /// holds two elements, the first one being the first parameter written, with its normalised
    /// value (40 lands on the rung 30).
    /// </summary>
    [Fact] // REQ-LUA-116
    public void The_list_holds_the_written_parameters_in_write_order()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("""
            Setting:ResetVisualStyleParameters(4)
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", 40)
            Setting:SetVisualStyleParameter(4, "spl.ao", 2)
            """);

        Assert.Equal(2, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
        Assert.Equal(4, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters[0].Style"));
        Assert.Equal(
            "spl.bend_zx",
            session.Runtime.EvaluateText("return Setting.VisualStyleParameters[0].ParameterId"));
        Assert.Equal(30, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters[0].Value"));

        Assert.Equal("spl.ao", session.Runtime.EvaluateText("return Setting.VisualStyleParameters[1].ParameterId"));
        Assert.Equal(2, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters[1].Value"));
    }

    /// <summary>
    /// REQ-LUA-116 — the second part of the criterion: <c>= nil</c> empties the list and puts
    /// every parameter of every style back to its default.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void Writing_nil_empties_the_list_and_resets_every_parameter()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("""
            Setting:ResetVisualStyleParameters(4)
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", 40)
            Setting:SetVisualStyleParameter(4, "spl.ao", 2)
            Setting.VisualStyleParameters = nil
            """);

        Assert.Equal(0, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
        Assert.Equal(
            0,
            session.Runtime.EvaluateNumber("return Setting:GetVisualStyleParameter(4, 'spl.bend_zx')"));
        Assert.Equal(0, session.Runtime.Settings.Model.GetInt(SettingKeys.SceneBendLeftRight));
    }

    /// <summary>
    /// REQ-LUA-116 — a second write of the same parameter updates its value and leaves it at its
    /// place: the order is that of the <b>first</b> write.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void A_second_write_of_one_parameter_updates_it_in_place()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("""
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", 30)
            Setting:SetVisualStyleParameter(4, "spl.ao", 2)
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", 90)
            """);

        Assert.Equal(2, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
        Assert.Equal(
            "spl.bend_zx",
            session.Runtime.EvaluateText("return Setting.VisualStyleParameters[0].ParameterId"));
        Assert.Equal(90, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters[0].Value"));
    }

    /// <summary>
    /// REQ-LUA-116 — a call that answers false records nothing: an unknown style and an unknown
    /// identifier leave the list alone (REQ-LUA-114).
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-LUA-114
    public void A_refused_write_records_nothing()
    {
        using ScriptSession session = Session();

        Assert.False(session.Runtime.EvaluateNumber(
            "return Setting:SetVisualStyleParameter(7, 'spl.bend_zx', 30) and 1 or 0") != 0d);
        Assert.False(session.Runtime.EvaluateNumber(
            "return Setting:SetVisualStyleParameter(4, 'inconnu', 30) and 1 or 0") != 0d);
        Assert.False(session.Runtime.EvaluateNumber(
            "return Setting:SetVisualStyleParameter(0, 'spl.bend_zx', 30) and 1 or 0") != 0d);

        Assert.Equal(0, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
    }

    /// <summary>
    /// REQ-LUA-116 with REQ-SCN-078: the style recorded is the one read back, so a write through
    /// the alias 1 gives an element of style 4 — and <c>ResetVisualStyleParameters(1)</c> takes
    /// it out again.
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-SCN-078
    public void The_alias_of_enhanced_records_the_style_read_back()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("Setting:SetVisualStyleParameter(1, 'spl.bend_zx', 60)");

        Assert.Equal(1, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
        Assert.Equal(4, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters[0].Style"));

        session.Runtime.Evaluate("Setting:ResetVisualStyleParameters(1)");
        Assert.Equal(0, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
    }

    /// <summary>
    /// REQ-LUA-116 — <c>ResetVisualStyleParameters(style)</c> takes out the elements of that
    /// style only. With a single style carrying parameters, that means all of them; the test
    /// still pins the wording, and that an unknown style takes nothing out.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void A_reset_of_an_unknown_style_takes_nothing_out()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("Setting:SetVisualStyleParameter(4, 'spl.ao', 2)");
        session.Runtime.Evaluate("Setting:ResetVisualStyleParameters(7)");
        Assert.Equal(1, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));

        session.Runtime.Evaluate("Setting:ResetVisualStyleParameters(0)");
        Assert.Equal(1, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));

        session.Runtime.Evaluate("Setting:ResetVisualStyleParameters(4)");
        Assert.Equal(0, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
    }

    /// <summary>
    /// REQ-LUA-116 — a list a read handed out puts exactly its elements back, and every other
    /// parameter returns to its default.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void Writing_back_a_list_that_was_read_restores_exactly_its_elements()
    {
        using ScriptSession session = Session();

        // 2 is « Depth » on the ladder 0 … 2 of spl.ao (AMD-055 REQ-SCN-079).
        session.Runtime.Evaluate("""
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", 60)
            Setting:SetVisualStyleParameter(4, "spl.ao", 2)
            kept = Setting.VisualStyleParameters
            """);

        // Move on: one parameter changed, another one added.
        session.Runtime.Evaluate("""
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", -30)
            Setting:SetVisualStyleParameter(4, "spl.arc_yx", 90)
            """);
        Assert.Equal(3, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));

        session.Runtime.Evaluate("Setting.VisualStyleParameters = kept");

        Assert.Equal(2, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
        Assert.Equal(
            60,
            session.Runtime.EvaluateNumber("return Setting:GetVisualStyleParameter(4, 'spl.bend_zx')"));
        Assert.Equal(2, session.Runtime.EvaluateNumber("return Setting:GetVisualStyleParameter(4, 'spl.ao')"));

        // The one that was not in the list is back to its default.
        Assert.Equal(
            0,
            session.Runtime.EvaluateNumber("return Setting:GetVisualStyleParameter(4, 'spl.arc_yx')"));
    }

    /// <summary>
    /// REQ-LUA-116 — the list a read gives is a snapshot: it does not follow a later write. That
    /// is what makes the restore above mean anything.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void A_list_that_was_read_does_not_follow_later_writes()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("""
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", 60)
            kept = Setting.VisualStyleParameters
            Setting:SetVisualStyleParameter(4, "spl.ao", 2)
            """);

        Assert.Equal(1, session.Runtime.EvaluateNumber("return kept.Count"));
        Assert.Equal(60, session.Runtime.EvaluateNumber("return kept[0].Value"));
        Assert.Equal(2, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters.Count"));
    }

    /// <summary>
    /// REQ-LUA-043 — two reads with nothing in between give the same Lua value; a write in
    /// between gives another one.
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-LUA-043
    public void Two_reads_with_nothing_in_between_give_the_same_value()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("Setting:SetVisualStyleParameter(4, 'spl.bend_zx', 60)");
        Assert.True(session.Runtime.EvaluateNumber(
            "return Setting.VisualStyleParameters == Setting.VisualStyleParameters and 1 or 0") != 0d);

        session.Runtime.Evaluate("before = Setting.VisualStyleParameters");
        session.Runtime.Evaluate("Setting:SetVisualStyleParameter(4, 'spl.ao', 2)");
        Assert.False(session.Runtime.EvaluateNumber(
            "return before == Setting.VisualStyleParameters and 1 or 0") != 0d);
    }

    /// <summary>
    /// REQ-LUA-116 — any other value raises, which is the one write of the requirement that may:
    /// the promise « aucune écriture ne lève d'erreur » covers the numeric members.
    /// </summary>
    [Theory] // REQ-LUA-116
    [InlineData("7")]
    [InlineData("'texte'")]
    [InlineData("{}")]
    [InlineData("true")]
    [InlineData("Setting")]
    [InlineData("Setting.Layer")]
    public void Writing_anything_else_raises(string value)
    {
        using ScriptSession session = Session();

        Assert.Throws<LuaScriptException>(
            () => session.Runtime.Evaluate($"Setting.VisualStyleParameters = {value}"));
    }

    /// <summary>REQ-LUA-045 — an index outside 0 … Count − 1 raises, as for every list.</summary>
    [Fact] // REQ-LUA-116, REQ-LUA-045
    public void An_index_out_of_range_raises()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("Setting:SetVisualStyleParameter(4, 'spl.bend_zx', 60)");

        Assert.Equal(60, session.Runtime.EvaluateNumber("return Setting.VisualStyleParameters[0].Value"));
        Assert.Throws<LuaScriptException>(
            () => session.Runtime.Evaluate("return Setting.VisualStyleParameters[1]"));
        Assert.Throws<LuaScriptException>(
            () => session.Runtime.Evaluate("return Setting.VisualStyleParameters[-1]"));
    }

    /// <summary>
    /// REQ-LUA-042 — a member of an element that the requirement does not name reads as nil,
    /// without an error, and an element is read only.
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-LUA-042
    public void An_element_names_three_members_and_is_read_only()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("Setting:SetVisualStyleParameter(4, 'spl.bend_zx', 60)");

        Assert.Equal("nil", session.Runtime.EvaluateText("return type(Setting.VisualStyleParameters[0].Autre)"));
        Assert.Throws<LuaScriptException>(
            () => session.Runtime.Evaluate("Setting.VisualStyleParameters[0].Value = 90"));
        Assert.Empty(session.Log.Errors);
    }

    /// <summary>
    /// The list and its element are in the API description the <c>lua-api</c> dump is built
    /// from, with the three members of the requirement (REQ-LUA-080).
    /// </summary>
    [Fact] // REQ-LUA-116
    public void The_list_and_its_element_are_described()
    {
        ScriptApiSurface api = ScriptApiSurface.Describe();

        ScriptApiType list = api.Types["Liste<ParamStyle>"];
        Assert.Equal("ParamStyle", list.ElementType);
        Assert.True(list.Indexable);
        Assert.True(list.Members.ContainsKey("Count"));

        ScriptApiType element = api.Types["ParamStyle"];
        Assert.Equal(
            new[] { "ParameterId", "Style", "Value" },
            element.Members.Keys.Order(StringComparer.Ordinal));
        Assert.All(element.Members.Values, member => Assert.False(member.Writable));

        ScriptApiMember property = api.Types["SettingAPI"].Members["VisualStyleParameters"];
        Assert.True(property.Writable);
        Assert.Equal("Liste<ParamStyle>", property.ResultType);
    }
}
