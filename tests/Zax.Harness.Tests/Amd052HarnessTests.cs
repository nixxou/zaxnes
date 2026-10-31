using Zax.Harness;
using Zax.Settings;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-052 — the two instruments the amendment's criteria measure with: the deliberately slow
/// display path of REQ-LOOP-077 and the anti-aliasing default of REQ-PERS-089, which
/// <c>Zax.Harness render</c> has to share with a fresh installation for its criterion to read as
/// written ("Zax.Harness render reports 4 granted samples").
/// </summary>
public sealed class Amd052HarnessTests
{
    /// <summary>Both are declared in the usage text of the <c>render</c> command.</summary>
    [Fact] // REQ-LOOP-077, REQ-PERS-089
    public void The_two_instruments_are_documented()
    {
        Assert.Contains("--slow-render <ms>", RenderExport.Usage, StringComparison.Ordinal);
        Assert.Contains("1 MSAA 4x (default)", RenderExport.Usage, StringComparison.Ordinal);
    }

    /// <summary>
    /// REQ-PERS-089: the default of the option is the default of the setting, so a plain run
    /// asks for MSAA 4×.
    /// </summary>
    [Fact] // REQ-PERS-089
    public void The_anti_alias_option_defaults_to_the_setting_default()
        => Assert.Equal(
            1,
            SettingsCatalogue.Get(SettingKeys.VideoAntiAlias).DefaultValue.AsInt);
}
