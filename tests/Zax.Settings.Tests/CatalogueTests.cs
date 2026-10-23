using System.Globalization;
using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>
/// The catalogue against the transcription of SPEC-09 §4.1 (REQ-PERS-001, REQ-PERS-002,
/// REQ-PERS-008, REQ-PERS-080, REQ-PERS-084, REQ-ARCH-021).
/// </summary>
public sealed class CatalogueTests
{
    /// <summary>REQ-PERS-001: every setting of §4.1 is in the model.</summary>
    [Theory]
    [MemberData(nameof(CatalogueTable.Theory), MemberType = typeof(CatalogueTable))]
    public void Every_setting_of_the_tables_is_present_with_its_kind_default_scope_and_effect(CatalogueRow row)
    {
        SettingDescriptor? descriptor = SettingsCatalogue.Find(row.Key);
        Assert.NotNull(descriptor);

        Assert.Equal(row.Kind, descriptor.Kind);
        Assert.Equal(row.Scope, descriptor.Scope);
        Assert.Equal(row.Effect, descriptor.Effect);
        Assert.Equal(row.Lua, descriptor.LuaPath);
        Assert.Equal(row.Default, Describe(descriptor.DefaultValue));
    }

    /// <summary>
    /// REQ-PERS-001: the enumeration returns at least the keys of §4.1. Anything beyond them is a
    /// setting of this reimplementation, which must carry a new key.
    /// </summary>
    [Fact]
    public void The_enumeration_covers_the_tables_and_names_its_additions()
    {
        HashSet<string> specified = [.. CatalogueTable.Rows.Select(static row => row.Key)];
        HashSet<string> model = [.. SettingsCatalogue.Keys];

        Assert.Empty(specified.Except(model));
        Assert.Equal(
            new[] { SettingKeys.AudioMute, SettingKeys.UiShowFps, SettingKeys.UiLanguage }.Order(),
            model.Except(specified).Order());
    }

    /// <summary>REQ-PERS-080: every key matches the published grammar and appears once.</summary>
    [Fact]
    public void Keys_are_stable_and_unique()
    {
        List<string> keys = [.. SettingsCatalogue.Keys];
        Assert.Equal(keys.Count, keys.Distinct(StringComparer.Ordinal).Count());
        Assert.All(keys, key => Assert.True(SettingDescriptor.IsValidKey(key), key));
    }

    /// <summary>
    /// REQ-PERS-080: the key set against the versioned reference list. A key that disappears
    /// without being recorded as withdrawn fails here.
    /// </summary>
    [Fact]
    public void No_published_key_disappeared()
    {
        string[] reference = File.ReadAllLines("SettingKeys.reference.txt")
            .Select(static line => line.Trim())
            .Where(static line => line.Length > 0 && !line.StartsWith('#'))
            .ToArray();

        Assert.NotEmpty(reference);
        Assert.Empty(reference.Except(SettingsCatalogue.Keys, StringComparer.Ordinal));
    }

    /// <summary>
    /// REQ-PERS-002: on a first launch — no settings file — the acceptance criterion's eight
    /// values hold.
    /// </summary>
    [Fact]
    public void First_launch_values_are_the_defaults_of_the_tables()
    {
        SettingsModel model = new();

        Assert.Equal(70f, model.GetFloat(SettingKeys.CameraFov));
        Assert.Equal(90, model.GetInt(SettingKeys.VideoFrameRate));
        Assert.Equal(-1, model.GetInt(SettingKeys.SceneSkybox));
        Assert.Equal(1f, model.GetFloat(SettingKeys.AudioVolume));
        Assert.True(model.GetBool(SettingKeys.RenderShadow));
        Assert.Equal(new SettingVector3(1f, -1f, 1f), model.GetVector3(SettingKeys.LightDirection));
        Assert.Equal(0f, model.GetFloat(SettingKeys.LayerOffset(2)));
        Assert.Equal(36f, model.GetFloat(SettingKeys.LayerOffset(5)));
    }

    /// <summary>REQ-PERS-008: the Lua aliases are exposed as well as the main names.</summary>
    [Theory]
    [InlineData("SamePalette", SettingKeys.ConvSameBgPalette)]
    [InlineData("SameBgPalette", SettingKeys.ConvSameBgPalette)]
    [InlineData("TopDownCam", SettingKeys.CameraLocalCam)]
    [InlineData("LocalCam", SettingKeys.CameraLocalCam)]
    public void Lua_aliases_designate_the_same_setting(string path, string key)
        => Assert.Equal(key, SettingsCatalogue.FindByLuaPath(path)?.Key);

    /// <summary>
    /// REQ-PERS-008, §5.6: the access paths the built-in profiles use all resolve. The six
    /// method calls — <c>GetInputType</c> and <c>SetInputType</c> — are not properties and are
    /// checked by SPEC-08's lot instead.
    /// </summary>
    [Theory]
    [InlineData("EmptyTile")]
    [InlineData("Clipping[0]")]
    [InlineData("Clipping[1]")]
    [InlineData("Clipping[2]")]
    [InlineData("Clipping[3]")]
    [InlineData("Layer[2].CastShadow")]
    [InlineData("Layer[3].CastShadow")]
    [InlineData("Layer[4].CastShadow")]
    [InlineData("Layer[5].CastShadow")]
    [InlineData("BgColor")]
    [InlineData("Light.Direction")]
    [InlineData("CustomBgColor")]
    [InlineData("SamePalette")]
    [InlineData("Light.Color")]
    [InlineData("LocalCam")]
    [InlineData("GameSpeed")]
    [InlineData("VrRuntime")]
    [InlineData("Layer[3].Offset")]
    [InlineData("Layer[3].Aligned")]
    [InlineData("TopDownCam")]
    [InlineData("Layer[2].RecvShadow")]
    [InlineData("SmoothTime")]
    [InlineData("SameBgPalette")]
    [InlineData("SceneRatio")]
    public void The_paths_the_built_in_profiles_use_all_resolve(string path)
        => Assert.NotNull(SettingsCatalogue.FindByLuaPath(path));

    /// <summary>REQ-PERS-007: the members marked "lecture" in §4.2 are not assignable.</summary>
    [Theory]
    [InlineData(SettingKeys.VideoTargetFramerate)]
    [InlineData(SettingKeys.VideoTargetTimePerFrame)]
    [InlineData(SettingKeys.SceneSkyboxEnable)]
    [InlineData(SettingKeys.InputZapperEnable)]
    [InlineData(SettingKeys.VideoResScaleFactor)]
    [InlineData(SettingKeys.EmuTvData)]
    public void Read_only_members_are_not_writable_from_lua(string key)
        => Assert.False(SettingsCatalogue.Get(key).IsLuaWritable);

    /// <summary>
    /// REQ-PERS-007: <c>SceneScale</c> is the one derived member Lua may assign, because writing
    /// it writes <c>SceneRatio</c> (REQ-PERS-013 point 3).
    /// </summary>
    [Fact]
    public void Scene_scale_is_the_one_writable_derived_member()
    {
        SettingDescriptor[] writableDerived =
        [
            .. SettingsCatalogue.OfScope(SettingScope.Derived).Where(static d => d.IsLuaWritable),
        ];

        Assert.Equal([SettingKeys.SceneSceneScale], writableDerived.Select(static d => d.Key));
    }

    /// <summary>REQ-PERS-084: the three examples of the acceptance criterion.</summary>
    [Fact]
    public void The_moment_of_effect_is_exposed_as_metadata()
    {
        Assert.Equal(SettingEffect.Immediate, SettingsCatalogue.Get(SettingKeys.CameraFov).Effect);
        Assert.Equal(SettingEffect.NextRomLoad, SettingsCatalogue.Get(SettingKeys.EmuAutoTvMode).Effect);
        Assert.Equal(SettingEffect.Restart, SettingsCatalogue.Get(SettingKeys.UiWindowRect).Effect);
    }

    /// <summary>
    /// REQ-PERS-020: scope U, and <c>ui.fullscreen</c> among them, is what gets persisted;
    /// nothing else is.
    /// </summary>
    [Fact]
    public void Only_user_preferences_are_persisted()
    {
        Assert.True(SettingsCatalogue.Get(SettingKeys.UiFullscreen).IsPersisted);
        Assert.All(
            SettingsCatalogue.All,
            descriptor => Assert.Equal(descriptor.Scope == SettingScope.User, descriptor.IsPersisted));
    }

    /// <summary>
    /// REQ-PROF-041: the profile-scoped settings of §4.1 are exactly the list SPEC-03 publishes,
    /// which is authoritative where the two differ.
    /// </summary>
    [Fact]
    public void The_profile_scoped_settings_are_the_list_of_REQ_PROF_041()
    {
        List<string> expected =
        [
            SettingKeys.LightDirection,
            SettingKeys.LightColor,
            .. SettingKeys.Clipping,
            SettingKeys.SceneMapEnable,
            SettingKeys.LayerDefaultLayer,
            SettingKeys.LayerAuto,
            SettingKeys.VideoGraphicQuality,
            SettingKeys.EditorSavedPatterns,
            SettingKeys.EditorSavedAnimPatterns,
            SettingKeys.ConvSameBgPalette,
            SettingKeys.ConvSameSpPalette,
            SettingKeys.ConvSpriteTolerance,
            SettingKeys.ConvScanDirection,
            SettingKeys.ConvEmptyTile,
            SettingKeys.CameraLocalCam,
            SettingKeys.EmuTvMode,
            SettingKeys.SceneUseNesBgColor,
        ];

        for (int i = 0; i < SettingKeys.LayerCount; i++)
        {
            expected.Add(SettingKeys.LayerOffset(i));
            expected.Add(SettingKeys.LayerAligned(i));
            expected.Add(SettingKeys.LayerCastShadow(i));
            expected.Add(SettingKeys.LayerRecvShadow(i));
        }

        Assert.Equal(
            expected.Order(StringComparer.Ordinal),
            SettingsCatalogue.OfScope(SettingScope.Profile).Select(static d => d.Key).Order(StringComparer.Ordinal));
    }

    /// <summary>
    /// REQ-PROF-041 second list: the user keeps these across a profile load, the light
    /// intensity included.
    /// </summary>
    [Theory]
    [InlineData(SettingKeys.LightIntensity)]
    [InlineData(SettingKeys.RenderMeshAlgo)]
    [InlineData(SettingKeys.SceneSkybox)]
    [InlineData(SettingKeys.EmuGameSpeed)]
    [InlineData(SettingKeys.RenderShadow)]
    [InlineData(SettingKeys.AudioVolume)]
    [InlineData(SettingKeys.SceneCustomBgColor)]
    [InlineData(SettingKeys.CameraFov)]
    public void The_preserved_settings_are_user_scoped(string key)
        => Assert.Equal(SettingScope.User, SettingsCatalogue.Get(key).Scope);

    /// <summary>
    /// REQ-ARCH-021: the enumeration gives key, type, values or bounds and default for every
    /// setting.
    /// </summary>
    [Fact]
    public void Every_setting_announces_a_type_and_a_domain_or_bounds()
    {
        Assert.All(SettingsCatalogue.All, descriptor =>
        {
            Assert.False(string.IsNullOrWhiteSpace(descriptor.Key));
            Assert.NotNull(descriptor.Domain);
            Assert.Equal(descriptor.Kind, descriptor.DefaultValue.Kind);
        });
    }

    private static string Describe(SettingValue value) => value.Kind switch
    {
        SettingKind.Vector3 => string.Format(
            CultureInfo.InvariantCulture,
            "{0},{1},{2}",
            value.AsVector3.X,
            value.AsVector3.Y,
            value.AsVector3.Z),
        SettingKind.Rect => string.Format(
            CultureInfo.InvariantCulture,
            "{0},{1},{2},{3}",
            value.AsRect.X,
            value.AsRect.Y,
            value.AsRect.Width,
            value.AsRect.Height),
        SettingKind.Opaque => string.Empty,
        _ => value.ToString(),
    };
}
