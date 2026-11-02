namespace Zax.Settings.Tests;

/// <summary>
/// AMD-059 REQ-LUA-116, the catalogue side: the three settings the 2.0 members of
/// <c>Setting</c> persist, their defaults, their scope and their absence from both interfaces.
/// </summary>
/// <remarks>
/// <c>video.max_queued_frames</c> is not among them: REQ-LUA-116 says
/// <c>Setting.MaxQueuedFrames</c> <b>is</b> the setting AMD-052 REQ-LOOP-077 already added, so
/// its line is the existing one and no second key exists.
/// </remarks>
public sealed class Amd059SettingsTests
{
    /// <summary>The three keys REQ-LUA-116 adds exist, of scope U, so they are persisted.</summary>
    [Theory]
    [InlineData(SettingKeys.UiLibraryView)]
    [InlineData(SettingKeys.VrRefreshRate)]
    [InlineData(SettingKeys.VideoMsaa20)]
    public void The_three_new_settings_are_user_scoped_and_persisted_REQ_LUA_116(string key)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        Assert.Equal(SettingScope.User, descriptor.Scope);
        Assert.True(descriptor.IsPersisted);
    }

    /// <summary>
    /// REQ-LUA-116: no interface at all — no row in the options window and no core option.
    /// </summary>
    [Theory]
    [InlineData(SettingKeys.UiLibraryView)]
    [InlineData(SettingKeys.VrRefreshRate)]
    [InlineData(SettingKeys.VideoMsaa20)]
    public void The_three_new_settings_have_no_interface_REQ_LUA_116(string key)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        Assert.Null(descriptor.UiLabel);
        Assert.False(descriptor.IsLibretroExposed);
    }

    /// <summary>The defaults of the table of REQ-LUA-116: 0, 90 and 2.</summary>
    [Theory]
    [InlineData(SettingKeys.UiLibraryView, 0)]
    [InlineData(SettingKeys.VrRefreshRate, 90)]
    [InlineData(SettingKeys.VideoMsaa20, 2)]
    [InlineData(SettingKeys.VideoMaxQueuedFrames, 2)]
    public void The_defaults_are_the_ones_of_the_table_REQ_LUA_116(string key, int expected)
        => Assert.Equal(expected, SettingsCatalogue.Get(key).DefaultValue.AsInt);

    /// <summary>
    /// Each of the three announces its Lua name, so the mapping of SPEC-09 §4.1 holds and
    /// <c>Setting.Msaa</c> is not confused with <c>Setting.AntiAlias</c>.
    /// </summary>
    [Fact]
    public void Each_new_setting_announces_its_Lua_name_REQ_LUA_116()
    {
        Assert.Equal("LibraryView", SettingsCatalogue.Get(SettingKeys.UiLibraryView).LuaPath);
        Assert.Equal("RefreshRate", SettingsCatalogue.Get(SettingKeys.VrRefreshRate).LuaPath);
        Assert.Equal("Msaa", SettingsCatalogue.Get(SettingKeys.VideoMsaa20).LuaPath);
        Assert.Equal("AntiAlias", SettingsCatalogue.Get(SettingKeys.VideoAntiAlias).LuaPath);
    }

    /// <summary>
    /// <c>video.msaa_2_0</c> is a key of its own: writing it leaves <c>video.anti_alias</c>
    /// alone, which is the divergence (D) of AMD-059 §3 — a script must not change the user's
    /// anti-aliasing behind their back.
    /// </summary>
    [Fact]
    public void The_two_dot_oh_msaa_is_not_the_anti_aliasing_of_AMD_032()
    {
        SettingsModel model = new();
        int before = model.GetInt(SettingKeys.VideoAntiAlias);

        model.SetInt(SettingKeys.VideoMsaa20, 4);

        Assert.Equal(4, model.GetInt(SettingKeys.VideoMsaa20));
        Assert.Equal(before, model.GetInt(SettingKeys.VideoAntiAlias));
    }

    /// <summary>
    /// <c>ui.library_view</c> keeps whatever number it is given: REQ-LUA-116 says « v est
    /// mémorisé tel quel », so the domain bounds nothing.
    /// </summary>
    [Theory]
    [InlineData(7)]
    [InlineData(-3)]
    [InlineData(1000)]
    public void The_library_view_keeps_the_value_written_REQ_LUA_116(int value)
    {
        SettingsModel model = new();
        model.SetInt(SettingKeys.UiLibraryView, value);
        Assert.Equal(value, model.GetInt(SettingKeys.UiLibraryView));
    }
}
