using Zax.App.Ui;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// AMD-059 REQ-LUA-116, the host side: the three settings the 2.0 members of <c>Setting</c>
/// persist carry « UI : aucune », so the options window shows none of them — and the settings
/// file keeps all three, since they are of scope U.
/// </summary>
public sealed class Amd059NoInterfaceTests
{
    /// <summary>REQ-LUA-116: none of the three is a row of the options window.</summary>
    [Theory] // REQ-LUA-116
    [InlineData(SettingKeys.UiLibraryView)]
    [InlineData(SettingKeys.VrRefreshRate)]
    [InlineData(SettingKeys.VideoMsaa20)]
    public void The_options_window_shows_none_of_the_three(string key)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        bool shown = OptionsModel.IsShown(descriptor) && OptionsModel.TabOf(descriptor) is not null;

        Assert.False(shown, key);
        Assert.DoesNotContain(key, new OptionsModel().AllRows().Select(static row => row.Descriptor.Key));
    }

    /// <summary>
    /// The anti-aliasing row of AMD-032 is still there: <c>Setting.Msaa</c> took nothing away
    /// from the window, which is the divergence (D) of AMD-059 §3.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void The_anti_alias_row_of_AMD_032_is_untouched()
        => Assert.Contains(
            SettingKeys.VideoAntiAlias,
            new OptionsModel().AllRows().Select(static row => row.Descriptor.Key));

    /// <summary>
    /// REQ-LUA-116: a row the window does not show is still reachable through the Lua name the
    /// catalogue publishes, so <c>Setting.Msaa</c> and <c>Setting.AntiAlias</c> stay two
    /// distinct settings for the host as well.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void The_two_anti_aliasing_settings_stay_distinct_for_the_host()
    {
        Assert.NotEqual(
            SettingsCatalogue.Get(SettingKeys.VideoMsaa20).Key,
            SettingsCatalogue.Get(SettingKeys.VideoAntiAlias).Key);
        Assert.Equal("Msaa", SettingsCatalogue.Get(SettingKeys.VideoMsaa20).LuaPath);
        Assert.Equal("AntiAlias", SettingsCatalogue.Get(SettingKeys.VideoAntiAlias).LuaPath);
    }
}
