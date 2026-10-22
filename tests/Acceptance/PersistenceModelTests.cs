using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// SPEC-09 §3.1 and §3.9: the settings model itself — the catalogue, the defaults, the
/// validation, the scopes, the derived values and the libretro exposure.
/// </summary>
public sealed class PersistenceModelTests
{
    /// <summary>
    /// REQ-PERS-001: every key of the tables §4.1.1 to §4.1.10 is in the model, with the scope
    /// the document gives it. The expected list is read out of SPEC-09 itself.
    /// </summary>
    [PersFact]
    public void REQ_PERS_001_every_key_of_the_tables_is_in_the_model()
    {
        List<string> missing = [];
        List<string> wrongScope = [];

        foreach (PersSettingRow row in PersTables.Settings())
        {
            if (SettingsCatalogue.Find(row.Key) is not { } descriptor)
            {
                missing.Add($"{row.Key} ({row.Section})");
                continue;
            }

            char scope = descriptor.Scope switch
            {
                SettingScope.User => 'U',
                SettingScope.Profile => 'P',
                SettingScope.Session => 'S',
                _ => 'D',
            };

            if (scope != row.Scope)
            {
                wrongScope.Add($"{row.Key}: document {row.Scope}, model {scope}");
            }
        }

        Assert.Empty(missing);
        Assert.Empty(wrongScope);
        Assert.True(PersTables.Settings().Count >= 60, "the table parser found too few rows");
    }

    /// <summary>
    /// REQ-PERS-008: the Lua names of the column « Lua » are exposed exactly, aliases included.
    /// The names that are a plain <c>Setting.&lt;Name&gt;</c> are checked against the catalogue's
    /// own index; the compound ones belong to the <c>Light</c>, <c>Layer</c> and <c>Clipping</c>
    /// objects and are checked by the criterion below.
    /// </summary>
    [PersFact]
    public void REQ_PERS_008_the_lua_names_are_exposed_exactly()
    {
        List<string> missing = [];
        List<string> writable = [];

        foreach (PersSettingRow row in PersTables.Settings())
        {
            if (row.Lua is not { } path)
            {
                continue;
            }

            foreach (string name in (string[])[path, .. row.Aliases])
            {
                if (name.Contains('(', StringComparison.Ordinal)
                    || name.Contains('.', StringComparison.Ordinal)
                    || name.Contains('[', StringComparison.Ordinal))
                {
                    continue;
                }

                // The two static predicates of app.engine_mode are not settings of their own;
                // they are checked from Lua below.
                if (name is "IsProfileMode" or "IsRomMode")
                {
                    continue;
                }

                if (SettingsCatalogue.FindByLuaPath(name) is not { } found)
                {
                    missing.Add($"{row.Key} → Setting.{name}");
                    continue;
                }

                if (row.ReadOnlyLua && found.IsLuaWritable)
                {
                    writable.Add(name);
                }
            }
        }

        Assert.Empty(missing);
        Assert.Empty(writable);

        // The two aliases the requirement names explicitly.
        Assert.Equal(
            SettingKeys.ConvSameBgPalette,
            SettingsCatalogue.FindByLuaPath("SamePalette")?.Key);
        Assert.Equal(
            SettingKeys.CameraLocalCam,
            SettingsCatalogue.FindByLuaPath("TopDownCam")?.Key);
    }

    /// <summary>
    /// REQ-PERS-008, the criterion itself: the 22 access paths the embedded profiles use (§5.6)
    /// run without a Lua error, in reading and in writing.
    /// </summary>
    [Fact]
    public void REQ_PERS_008_the_twenty_two_paths_of_paragraph_5_6_run_without_error()
    {
        using LuaRun run = new LuaProfile().Script("probe", "-- no hook").Load();

        string[] reads =
        [
            "Setting.EmptyTile",
            "Setting.Clipping[0]", "Setting.Clipping.y", "Clipping[1]", "Clipping.target",
            "Setting.Layer[2].CastShadow", "Setting.Layer[3].CastShadow",
            "Setting.BgColor",
            "Light.Direction", "Setting.Light.Direction",
            "Setting.CustomBgColor",
            "Setting.SamePalette", "Setting.SameBgPalette",
            "Light.Color",
            "Setting.Layer[2]", "Layer[2]",
            "Setting.LocalCam", "Setting.TopDownCam",
            "Setting:GetInputType(0)", "Setting:GetInputType(1)",
            "Setting.GameSpeed",
            "Setting.VrRuntime",
            "Setting.Layer[4].Offset",
            "Setting.Layer[5].Aligned",
            "Setting.Layer[2].RecvShadow",
            "Setting.SmoothTime",
            "Setting.SceneRatio",
        ];

        foreach (string path in reads)
        {
            Assert.Null(run.Error($"local v = {path}"));
        }

        string[] writes =
        [
            "Setting.EmptyTile = true",
            "Setting.Clipping[0] = 2",
            "Clipping[1] = 16",
            "Setting.Layer[3].CastShadow = true",
            "Setting.BgColor = false",
            "Light.Direction = Vector3(0, -1, 1)",
            "Setting.CustomBgColor = Color(1, 0, 0, 1)",
            "Setting.SamePalette = true",
            "Light.Color = Color(1, 1, 1, 1)",
            "Setting.LocalCam = true",
            "Setting:SetInputType(0, 0)",
            "Setting.GameSpeed = 2",
            "Setting.VrRuntime = 1",
            "Setting.Layer[4].Offset = 24",
            "Setting.Layer[5].Aligned = 2",
            "Setting.Layer[2].RecvShadow = true",
            "Setting.SmoothTime = 0.7",
            "Setting.SameBgPalette = true",
            "Setting.SceneRatio = 1",
            "Setting.TopDownCam = false",
        ];

        foreach (string write in writes)
        {
            Assert.Null(run.Error(write));
        }

        // The two aliases name the same setting as their canonical form.
        run.Do("Setting.SamePalette = true");
        Assert.True(run.Flag("Setting.SameBgPalette"));
        run.Do("Setting.TopDownCam = true");
        Assert.True(run.Flag("Setting.LocalCam"));

    }

    /// <summary>
    /// REQ-PERS-008, remaining clause: the column « Lua » of §4.1.10 publishes three static
    /// members for <c>app.engine_mode</c> — <c>EngineMode</c>, <c>IsProfileMode</c> and
    /// <c>IsRomMode</c> — which the requirement says must be exposed exactly.
    /// </summary>
    /// <remarks>
    /// Round 1: none of the three was exposed, and SPEC-05 §4 listed none of them either, which is
    /// what <c>Q-052</c> asked about. AMD-020 answers that the three are <b>required</b>, read
    /// only, with the semantics of SPEC-09 §4.1.10; the <c>R1-fixes</c> batch exposes them through
    /// <c>ScriptSettings</c> and <c>LuaApi.Settings</c>. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Fact] // REQ-PERS-008 — AMD-020 (Q-052)
    public void REQ_PERS_008_the_static_engine_mode_members_must_be_exposed()
    {
        using LuaRun run = new LuaProfile().Script("probe", "-- no hook").Load();

        Assert.Null(run.Error("local v = Setting.EngineMode"));
        Assert.Equal("number", run.TypeOf("Setting.EngineMode"));
        Assert.Equal(1.0, run.Num("Setting.EngineMode"), 6);
        Assert.True(run.Flag("Setting.IsProfileMode"));
        Assert.False(run.Flag("Setting.IsRomMode"));

        // AMD-020 (Q-052): the three are required **read only**.
        Assert.NotNull(run.Error("Setting.EngineMode = 0"));
        Assert.NotNull(run.Error("Setting.IsProfileMode = false"));
        Assert.NotNull(run.Error("Setting.IsRomMode = true"));
    }

    /// <summary>
    /// REQ-PERS-007: the globals designate the <b>living</b> model — a write is visible to the
    /// very next read — <c>Layer[i]</c> and <c>Clipping[i]</c> behave as the requirement says,
    /// and a read-only member refuses an assignment.
    /// </summary>
    [Fact]
    public void REQ_PERS_007_the_globals_designate_the_living_model()
    {
        using LuaRun run = new LuaProfile().Script("probe", "-- no hook").Load();

        // Light, Layer and Clipping are the very objects Setting carries.
        run.Do("Clipping[1] = 16");
        Assert.Equal(16.0, run.Num("Setting.Clipping.y"), 4);
        Assert.Equal(16.0, run.Num("Clipping.y"), 4);
        Assert.Equal(16.0, run.Num("Setting.Clipping[1]"), 4);

        run.Do("Setting.Layer[3].Offset = 24");
        Assert.Equal(24.0, run.Num("Layer[3].Offset"), 4);
        Assert.Equal(24.0, run.Num("Setting.Layer[3].Offset"), 4);
        Assert.Equal(24f, run.Settings.Model.GetFloat(SettingKeys.LayerOffset(3)));

        run.Do("Light.Intensity = 1.25");
        Assert.Equal(1.25, run.Num("Setting.Light.Intensity"), 4);

        // Layer[i] accepts 0 to 5 and is not reassignable; its fields are.
        for (int layer = 0; layer < 6; layer++)
        {
            Assert.Null(run.Error($"local v = Layer[{layer}].Offset"));
        }

        Assert.NotNull(run.Error("Layer[6]"));
        Assert.NotNull(run.Error("Setting.Light = nil"));

        // A derived member is read-only (REQ-PERS-013).
        Assert.NotNull(run.Error("Setting.TargetFramerate = 1"));
        Assert.NotNull(run.Error("Setting.ZapperEnable = true"));
    }

    /// <summary>
    /// REQ-PERS-015: <c>Setting:Save()</c> and <c>Setting:Load()</c> reach the host's settings
    /// file, and <c>Setting:SetRuntime(n)</c> is inert on desktop.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_PERS_015_save_load_and_set_runtime()
    {
        Zax.Profiles.GameProfile profile = Zax.Profiles.GameProfile.CreateEmpty();
        profile.Title = "verify-settings-file";
        profile.Scripts.Add(new Zax.Profiles.ProfileScript(
            "s",
            true,
            """
            function Start()
              Setting:SetRuntime(1)
              runtime = Setting.VrRuntime
              Setting:Save()
              Setting:Load()
            end
            """,
            "*"));

        using HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle("verify-settings-file"), host.WriteRom(LoopRom.Counter()));

        Assert.Empty(host.ScriptLog.Errors);
        Assert.Equal(0.0, host.Session!.Scripts.EvaluateNumber("return runtime"), 6);
    }

    /// <summary>
    /// REQ-PERS-002: with no settings file every setting holds the value of the column
    /// « Défaut ». The eight values the criterion names are checked one by one.
    /// </summary>
    [Fact]
    public void REQ_PERS_002_the_defaults_are_the_ones_of_the_criterion()
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

    /// <summary>REQ-PERS-002, the rest of the table: the other documented defaults.</summary>
    [Fact]
    public void REQ_PERS_002_the_other_documented_defaults()
    {
        SettingsModel model = new();

        Assert.Equal(0, model.GetInt(SettingKeys.VideoGraphicQuality));
        Assert.Equal(0, model.GetInt(SettingKeys.VideoVSync));
        Assert.True(model.GetBool(SettingKeys.VideoSkipFrame));
        Assert.Equal(0, model.GetInt(SettingKeys.VideoRenderMethod));
        Assert.Equal(0, model.GetInt(SettingKeys.VideoResScale));
        Assert.Equal(0, model.GetInt(SettingKeys.VideoAntiAlias));
        Assert.Equal(0, model.GetInt(SettingKeys.RenderMeshAlgo));
        Assert.False(model.GetBool(SettingKeys.UiFullscreen));
        Assert.False(model.GetBool(SettingKeys.CameraLocalCam));
        Assert.Equal(0.4f, model.GetFloat(SettingKeys.CameraSmoothTime));
        Assert.Equal(new SettingColor(1f, 1f, 1f, 1f), model.GetColor(SettingKeys.LightColor));
        Assert.Equal(1f, model.GetFloat(SettingKeys.LightIntensity));
        Assert.Equal(new SettingColor(0f, 0f, 0f, 1f), model.GetColor(SettingKeys.SceneCustomBgColor));
        Assert.True(model.GetBool(SettingKeys.SceneUseNesBgColor));
        Assert.False(model.GetBool(SettingKeys.SceneMapEnable));

        Assert.Equal(0.1f, model.GetFloat(SettingKeys.ClipLeft), 4);
        Assert.Equal(8.1f, model.GetFloat(SettingKeys.ClipBottom), 4);
        Assert.Equal(8.1f, model.GetFloat(SettingKeys.ClipRight), 4);
        Assert.Equal(0.1f, model.GetFloat(SettingKeys.ClipTop), 4);

        Assert.Equal(2, model.GetInt(SettingKeys.LayerDefaultLayer));
        Assert.False(model.GetBool(SettingKeys.LayerAuto));
        float[] offsets = [0f, 0f, 0f, 12f, 24f, 36f];
        int[] alignments = [0, 1, 0, 0, 0, 0];
        for (int layer = 0; layer < 6; layer++)
        {
            Assert.Equal(offsets[layer], model.GetFloat(SettingKeys.LayerOffset(layer)));
            Assert.Equal(alignments[layer], model.GetInt(SettingKeys.LayerAligned(layer)));
            Assert.Equal(layer == 2, model.GetBool(SettingKeys.LayerCastShadow(layer)));
            Assert.Equal(layer >= 2, model.GetBool(SettingKeys.LayerRecvShadow(layer)));
        }

        Assert.Equal(0, model.GetInt(SettingKeys.EmuGameSpeed));
        Assert.Equal(0, model.GetInt(SettingKeys.EmuTvMode));
        Assert.False(model.GetBool(SettingKeys.EmuAutoTvMode));
        Assert.Equal(0, model.GetInt(SettingKeys.EmuCore));

        Assert.False(model.GetBool(SettingKeys.ConvSameBgPalette));
        Assert.False(model.GetBool(SettingKeys.ConvSameSpPalette));
        Assert.Equal(0, model.GetInt(SettingKeys.ConvSpriteTolerance));
        Assert.Equal(0, model.GetInt(SettingKeys.ConvScanDirection));
        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));

        Assert.True(model.GetBool(SettingKeys.AudioBgMusic));
        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(0)));
        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(1)));
        Assert.False(model.GetBool(SettingKeys.InputEmulatedMenuButton));
        Assert.Equal(0, model.GetInt(SettingKeys.UiSortOption));

        Assert.Equal(0, model.GetInt(SettingKeys.VrRuntime));
        Assert.Equal(1f, model.GetFloat(SettingKeys.VrSceneRatio));
        Assert.True(model.GetBool(SettingKeys.VrMirroring));
        Assert.Equal(0, model.GetInt(SettingKeys.VrControlMethod));
        Assert.Equal(0, model.GetInt(SettingKeys.EditorSelectionMode));
        Assert.Equal(0, model.GetInt(SettingKeys.MovieMode));
        Assert.Equal(1f, model.GetFloat(SettingKeys.SceneRootScale));
        Assert.Equal(1, model.GetInt(SettingKeys.AppEngineMode));
    }

    /// <summary>
    /// REQ-PERS-003 and §5.1: the validation rules and the six vectors of the table.
    /// </summary>
    [Fact]
    public void REQ_PERS_003_the_validation_vectors_of_paragraph_5_1()
    {
        SettingsModel model = new();

        Check(SettingKeys.VideoFrameRate, 60, 60);
        Check(SettingKeys.VideoFrameRate, 90, 90);
        Check(SettingKeys.VideoFrameRate, 120, 120);
        Check(SettingKeys.VideoFrameRate, 75, 90);
        Check(SettingKeys.VideoFrameRate, 0, 90);

        model.SetFloat(SettingKeys.EmuGameSpeed, -9f);
        Assert.Equal(-8f, model.GetFloat(SettingKeys.EmuGameSpeed));
        model.SetFloat(SettingKeys.EmuGameSpeed, 3.4f);
        Assert.Equal(3.4f, model.GetFloat(SettingKeys.EmuGameSpeed), 4);
        model.SetFloat(SettingKeys.EmuGameSpeed, 8f);
        Assert.Equal(8f, model.GetFloat(SettingKeys.EmuGameSpeed));
        model.SetFloat(SettingKeys.EmuGameSpeed, 12f);
        Assert.Equal(8f, model.GetFloat(SettingKeys.EmuGameSpeed));

        Check(SettingKeys.ConvSpriteTolerance, -3, 3);
        Check(SettingKeys.SceneSkybox, 7, -1);
        Check(SettingKeys.VideoResScale, 5, 0);

        // emu.core: the remainder of the division by the number of cores, which is 1 in v1.
        Assert.Equal(1, model.CoreCount);
        Check(SettingKeys.EmuCore, 3, 0);

        // A non-zero light direction is required; a zero vector takes the default.
        model.SetVector3(SettingKeys.LightDirection, new SettingVector3(0f, 0f, 0f));
        Assert.Equal(new SettingVector3(1f, -1f, 1f), model.GetVector3(SettingKeys.LightDirection));

        // The clip margins are clamped to their domain (REQ-PERS-003, §4.1.5).
        model.SetFloat(SettingKeys.ClipBottom, 999f);
        Assert.Equal(248f, model.GetFloat(SettingKeys.ClipBottom));
        model.SetFloat(SettingKeys.ClipLeft, -5f);
        Assert.Equal(0f, model.GetFloat(SettingKeys.ClipLeft));

        void Check(string key, int written, int effective)
        {
            model.SetInt(key, written);
            Assert.Equal(effective, model.GetInt(key));
        }
    }

    /// <summary>
    /// REQ-PERS-004: an enumerated setting accepts the numeric value of its domain, which is
    /// what a script writes when it uses the number instead of the named constant.
    /// </summary>
    [Fact]
    public void REQ_PERS_004_an_enumeration_accepts_its_numeric_value()
    {
        SettingsModel model = new();

        model.SetInt(SettingKeys.LayerAligned(2), 2);
        Assert.Equal(2, model.GetInt(SettingKeys.LayerAligned(2)));
        Assert.Equal(-1f, model.GetFloat(SettingKeys.LayerCoef(2)));

        model.SetInt(SettingKeys.LayerAligned(2), 0);
        Assert.Equal(1f, model.GetFloat(SettingKeys.LayerCoef(2)));

        // Every value of every enumerated domain round-trips through its number, apart from the
        // two the specification freezes on purpose (REQ-PERS-011, REQ-PERS-016).
        foreach (SettingDescriptor descriptor in SettingsCatalogue.All)
        {
            if (descriptor.Key is SettingKeys.VrRuntime or SettingKeys.MovieMode)
            {
                continue;
            }

            if (descriptor.Kind != SettingKind.Enum || descriptor.Domain.Options is not { } options)
            {
                continue;
            }

            foreach (SettingOption option in options)
            {
                model.Set(descriptor.Key, SettingValue.OfEnum((int)option.Value));
                Assert.Equal((int)option.Value, model.GetInt(descriptor.Key));
            }
        }
    }

    /// <summary>
    /// REQ-PERS-005 and §5.4: loading a different game replaces every P value, keeps every U
    /// value, forces <c>BgColor</c> true and rebuilds the six pairs of shadow flags.
    /// </summary>
    [Fact]
    public void REQ_PERS_005_the_profile_load_follows_the_vector_of_paragraph_5_4()
    {
        SettingsModel model = new();

        // State before, from §5.4.
        model.SetFloat(SettingKeys.LightIntensity, 1.2f);
        model.Set(SettingKeys.SceneSkybox, SettingValue.OfEnum(1));
        model.SetFloat(SettingKeys.CameraFov, 80f);
        model.SetBool(SettingKeys.ConvEmptyTile, true, SettingLayer.Profile);
        model.Set(SettingKeys.EmuTvMode, SettingValue.OfEnum(1), SettingLayer.Profile);
        model.Set(SettingKeys.VideoGraphicQuality, SettingValue.OfEnum(2), SettingLayer.Profile);
        model.SetColor(SettingKeys.SceneCustomBgColor, new SettingColor(1f, 0f, 0f, 1f));

        // Profile A, with every shadow flag on and BgColor off.
        List<KeyValuePair<string, SettingValue>> profile =
        [
            new(SettingKeys.LightIntensity, SettingValue.OfFloat(1.0f)),
            new(SettingKeys.LightDirection, SettingValue.OfVector3(new SettingVector3(-1f, -1f, 1f))),
            new(SettingKeys.ConvEmptyTile, SettingValue.OfBool(false)),
            new(SettingKeys.EmuTvMode, SettingValue.OfEnum(0)),
            new(SettingKeys.VideoGraphicQuality, SettingValue.OfEnum(0)),
            new(SettingKeys.SceneUseNesBgColor, SettingValue.OfBool(false)),
            new(SettingKeys.SceneSkybox, SettingValue.OfEnum(0)),
            new(SettingKeys.CameraFov, SettingValue.OfFloat(60f)),
        ];

        for (int layer = 0; layer < 6; layer++)
        {
            profile.Add(new KeyValuePair<string, SettingValue>(
                SettingKeys.LayerCastShadow(layer),
                SettingValue.OfBool(true)));
            profile.Add(new KeyValuePair<string, SettingValue>(
                SettingKeys.LayerRecvShadow(layer),
                SettingValue.OfBool(true)));
        }

        model.ApplyProfile(profile);

        // U values survive: intensity, skybox, FOV and the custom background colour.
        Assert.Equal(1.2f, model.GetFloat(SettingKeys.LightIntensity));
        Assert.Equal(1, model.GetInt(SettingKeys.SceneSkybox));
        Assert.Equal(80f, model.GetFloat(SettingKeys.CameraFov));
        Assert.Equal(new SettingColor(1f, 0f, 0f, 1f), model.GetColor(SettingKeys.SceneCustomBgColor));

        // P values are the profile's.
        Assert.Equal(new SettingVector3(-1f, -1f, 1f), model.GetVector3(SettingKeys.LightDirection));
        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));
        Assert.Equal(0, model.GetInt(SettingKeys.EmuTvMode));
        Assert.Equal(0, model.GetInt(SettingKeys.VideoGraphicQuality));

        // Point 3: BgColor is forced true whatever the profile says.
        Assert.True(model.GetBool(SettingKeys.SceneUseNesBgColor));

        // Point 4: the shadow flags are rebuilt, the profile's values ignored.
        for (int layer = 0; layer < 6; layer++)
        {
            Assert.Equal(layer == 2, model.GetBool(SettingKeys.LayerCastShadow(layer)));
            Assert.Equal(layer >= 2, model.GetBool(SettingKeys.LayerRecvShadow(layer)));
        }
    }

    /// <summary>
    /// REQ-PERS-005 point 1: a P setting absent from the profile takes its default again rather
    /// than keeping what the previous game left behind.
    /// </summary>
    [Fact]
    public void REQ_PERS_005_a_setting_absent_from_the_profile_takes_its_default()
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.LayerOffset(3), 99f, SettingLayer.Profile);

        model.ApplyProfile([]);
        Assert.Equal(12f, model.GetFloat(SettingKeys.LayerOffset(3)));
    }

    /// <summary>
    /// REQ-PERS-006: relaunching the same game must not re-apply the profile's P values — the
    /// model only does it when <c>ApplyProfile</c> is called, which the loop reserves for a
    /// different game.
    /// </summary>
    [Fact]
    public void REQ_PERS_006_the_same_game_keeps_what_the_scripts_changed()
    {
        SettingsModel model = new();
        model.ApplyProfile([new(SettingKeys.ConvEmptyTile, SettingValue.OfBool(false))]);

        // A script changes a P setting during the game.
        model.SetBool(SettingKeys.ConvEmptyTile, true, SettingLayer.Session);
        Assert.True(model.GetBool(SettingKeys.ConvEmptyTile));

        // End of the game: the session layer is dropped, but a P setting is not a preference, so
        // its live value survives for the next launch of the same game.
        model.EndOfGame();
        Assert.True(model.GetBool(SettingKeys.ConvEmptyTile));

        // Loading a different game applies the profile again.
        model.ApplyProfile([new(SettingKeys.ConvEmptyTile, SettingValue.OfBool(false))]);
        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));
    }

    /// <summary>
    /// REQ-PERS-009: a change that really changes the effective value raises exactly one event
    /// carrying the key; rewriting the same value raises nothing.
    /// </summary>
    [Fact]
    public void REQ_PERS_009_only_an_effective_change_is_notified()
    {
        SettingsModel model = new();
        List<SettingChange> changes = [];
        model.Changed += changes.Add;

        model.SetFloat(SettingKeys.CameraFov, 80f);
        Assert.Single(changes);
        Assert.Equal(SettingKeys.CameraFov, changes[0].Key);
        Assert.Equal(80f, changes[0].Value.AsFloat);
        Assert.Equal(SettingEffect.Immediate, changes[0].Effect);

        changes.Clear();
        model.SetFloat(SettingKeys.CameraFov, 80f);
        Assert.Empty(changes);
    }

    /// <summary>
    /// REQ-PERS-010 (DEC-09-Q3): a script's write on a user preference is a session value — the
    /// preference comes back at the end of the game and is never persisted.
    /// </summary>
    [Fact]
    public void REQ_PERS_010_a_script_write_on_a_preference_is_a_session_value()
    {
        SettingsModel model = new();
        SettingColor blue = new(0f, 0f, 1f, 1f);
        model.SetColor(SettingKeys.SceneCustomBgColor, blue);

        model.SetColor(SettingKeys.SceneCustomBgColor, new SettingColor(0.2f, 0.2f, 0.2f, 1f), SettingLayer.Session);
        Assert.True(model.HasSessionValue(SettingKeys.SceneCustomBgColor));
        Assert.NotEqual(blue, model.GetColor(SettingKeys.SceneCustomBgColor));

        // The persisted preference never saw the script's value.
        Assert.Contains(
            model.UserPreferences(),
            pair => pair.Key == SettingKeys.SceneCustomBgColor && pair.Value.AsColor == blue);

        model.EndOfGame();
        Assert.Equal(blue, model.GetColor(SettingKeys.SceneCustomBgColor));
        Assert.False(model.HasSessionValue(SettingKeys.SceneCustomBgColor));
    }

    /// <summary>
    /// REQ-PERS-011: on desktop <c>VrRuntime</c> always reads 0 and a write is ignored without an
    /// error; <c>SceneRatio</c> always reads 1 while the stored value is kept.
    /// </summary>
    [Fact]
    public void REQ_PERS_011_the_virtual_reality_settings_are_frozen_on_desktop()
    {
        SettingsModel model = new();

        Assert.False(model.SetInt(SettingKeys.VrRuntime, 1));
        Assert.Equal(0, model.GetInt(SettingKeys.VrRuntime));

        Assert.True(model.SetFloat(SettingKeys.VrSceneRatio, 5f));
        Assert.Equal(1f, model.GetFloat(SettingKeys.VrSceneRatio));
        Assert.Equal(5f, model.GetStored(SettingKeys.VrSceneRatio).AsFloat);

        // The scene scale, which is derived from it, stays 1 on desktop.
        Assert.Equal(1f, model.GetFloat(SettingKeys.SceneSceneScale));

        // The mirroring and the control method are stored and handed back.
        model.SetBool(SettingKeys.VrMirroring, false);
        Assert.False(model.GetBool(SettingKeys.VrMirroring));
        model.SetInt(SettingKeys.VrControlMethod, 2);
        Assert.Equal(2, model.GetInt(SettingKeys.VrControlMethod));
    }

    /// <summary>
    /// REQ-PERS-012: at the end of a game <c>emu.game_speed</c> goes back to 0 and
    /// <c>camera.smooth_time</c> to 0.4, and the input ports go back to Controller at every load
    /// of a different game.
    /// </summary>
    [Fact]
    public void REQ_PERS_012_the_end_of_a_game_resets_the_speed_and_the_smoothing()
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.EmuGameSpeed, 4f);
        model.SetFloat(SettingKeys.CameraSmoothTime, 0.7f);
        model.SetInt(SettingKeys.InputPortType(1), 1);

        model.EndOfGame();
        Assert.Equal(0f, model.GetFloat(SettingKeys.EmuGameSpeed));
        Assert.Equal(0.4f, model.GetFloat(SettingKeys.CameraSmoothTime));

        model.SetInt(SettingKeys.InputPortType(1), 1);
        model.ResetInputPorts();
        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(0)));
        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(1)));

        model.SetInt(SettingKeys.InputPortType(1), 1);
        model.ApplyProfile([]);
        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(1)));
    }

    /// <summary>REQ-PERS-013: the eight derived values.</summary>
    [Fact]
    public void REQ_PERS_013_the_eight_derived_values()
    {
        SettingsModel model = new() { DisplayRefreshRate = 144f };

        // 1 and 2: the target frame rate and its period.
        model.SetInt(SettingKeys.VideoFrameRate, 120);
        model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(0));
        Assert.Equal(120f, model.GetFloat(SettingKeys.VideoTargetFramerate));
        Assert.Equal(1f / 120f, model.GetFloat(SettingKeys.VideoTargetTimePerFrame), 6);

        model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(1));
        Assert.Equal(144f, model.GetFloat(SettingKeys.VideoTargetFramerate));
        model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(2));
        Assert.Equal(72f, model.GetFloat(SettingKeys.VideoTargetFramerate));

        // 3: the scene scale, 1 × 1 on desktop.
        Assert.Equal(1f, model.GetFloat(SettingKeys.SceneSceneScale));

        // 4: the skybox switch.
        Assert.False(model.GetBool(SettingKeys.SceneSkyboxEnable));
        model.Set(SettingKeys.SceneSkybox, SettingValue.OfEnum(1));
        Assert.True(model.GetBool(SettingKeys.SceneSkyboxEnable));
        model.Set(SettingKeys.SceneSkybox, SettingValue.OfEnum(0));
        Assert.False(model.GetBool(SettingKeys.SceneSkyboxEnable));

        // 5: the Zapper switch.
        Assert.False(model.GetBool(SettingKeys.InputZapperEnable));
        model.SetInt(SettingKeys.InputPortType(1), 1);
        Assert.True(model.GetBool(SettingKeys.InputZapperEnable));

        // 6: the resolution scale factor.
        float[] factors = [1f, 2f / 3f, 0.5f, 1f / 3f, 0.25f];
        for (int index = 0; index < factors.Length; index++)
        {
            model.Set(SettingKeys.VideoResScale, SettingValue.OfEnum(index));
            Assert.Equal(factors[index], model.GetFloat(SettingKeys.VideoResScaleFactor), 5);
        }

        // 7: the region timing block the model carries for the core.
        object block = new();
        model.TvData = block;
        Assert.Same(block, model.Get(SettingKeys.EmuTvData).AsObject);

        // 8: the alignment coefficients.
        Assert.Equal(0f, model.GetFloat(SettingKeys.LayerCoef(1)));
        model.SetInt(SettingKeys.LayerAligned(1), 0);
        Assert.Equal(1f, model.GetFloat(SettingKeys.LayerCoef(1)));
        model.SetInt(SettingKeys.LayerAligned(1), 2);
        Assert.Equal(-1f, model.GetFloat(SettingKeys.LayerCoef(1)));
    }

    /// <summary>
    /// REQ-PERS-013: writing <c>SceneScale</c> writes <c>SceneRatio</c>, and every other derived
    /// member refuses a write.
    /// </summary>
    [Fact]
    public void REQ_PERS_013_a_derived_member_refuses_a_write_except_the_scene_scale()
    {
        SettingsModel model = new();

        Assert.True(model.SetFloat(SettingKeys.SceneSceneScale, 3f));
        Assert.Equal(3f, model.GetStored(SettingKeys.VrSceneRatio).AsFloat);

        foreach (SettingDescriptor descriptor in SettingsCatalogue.OfScope(SettingScope.Derived))
        {
            if (descriptor.Key == SettingKeys.SceneSceneScale)
            {
                continue;
            }

            Assert.False(
                model.Set(descriptor.Key, SettingValue.OfFloat(7f)),
                $"{descriptor.Key} accepted a write");
        }
    }

    /// <summary>
    /// REQ-PERS-014 and §4.3: the notification identifiers and the three numeric constants are
    /// exposed read-only with the documented values. The list is read out of the document.
    /// </summary>
    [PersFact]
    public void REQ_PERS_014_the_constants_of_paragraph_4_3_are_exposed()
    {
        IReadOnlyList<(string Name, string Value)> constants = PersTables.Constants();
        Assert.True(constants.Count >= 45, $"only {constants.Count} constants were parsed");

        using LuaRun run = new LuaProfile().Script("probe", "-- no hook").Load();

        List<string> wrong = [];
        foreach ((string name, string value) in constants)
        {
            if (double.TryParse(value, System.Globalization.NumberStyles.Float,
                System.Globalization.CultureInfo.InvariantCulture, out double number))
            {
                double actual = run.Num($"Setting.{name}");
                if (Math.Abs(actual - number) > 1e-6)
                {
                    wrong.Add($"{name}: document {value}, runtime {actual}");
                }

                continue;
            }

            string text = run.Text($"Setting.{name}");
            if (!string.Equals(text, value, StringComparison.Ordinal))
            {
                wrong.Add($"{name}: document '{value}', runtime '{text}'");
            }
        }

        Assert.Empty(wrong);

        // The three numeric constants, named by the requirement.
        Assert.Equal(0.4, run.Num("Setting.DefaultSmoothTime"), 6);
        Assert.Equal(0.005, run.Num("Setting.VRScale"), 6);
        Assert.Equal(1.0, run.Num("Setting.NonVRScale"), 6);

        // They are read-only.
        Assert.NotNull(run.Error("Setting.FovId = \"x\""));
    }

    /// <summary>
    /// REQ-PERS-016: the editor and movie settings are accepted and handed back, and
    /// <c>movie.mode</c> stays at 0.
    /// </summary>
    [Fact]
    public void REQ_PERS_016_the_editor_and_movie_settings_are_inert()
    {
        SettingsModel model = new();

        model.SetInt(SettingKeys.EditorSelectionMode, 1);
        Assert.Equal(1, model.GetInt(SettingKeys.EditorSelectionMode));

        object patterns = new string[] { "Config1", "Config2" };
        model.Set(SettingKeys.EditorSavedPatterns, SettingValue.OfOpaque(patterns), SettingLayer.Profile);
        Assert.Same(patterns, model.Get(SettingKeys.EditorSavedPatterns).AsObject);

        Assert.False(model.SetInt(SettingKeys.MovieMode, 1));
        Assert.Equal(0, model.GetInt(SettingKeys.MovieMode));
        Assert.False(model.SetInt(SettingKeys.MovieMode, 2));
        Assert.Equal(0, model.GetInt(SettingKeys.MovieMode));
    }

    /// <summary>
    /// REQ-PERS-017: the three settings without an observable effect are stored and exposed, and
    /// the emulated menu button is deliberately left without effect (DEC-R2-D7).
    /// </summary>
    [Fact]
    public void REQ_PERS_017_the_three_inert_settings_are_stored_and_exposed()
    {
        SettingsModel model = new();

        model.SetBool(SettingKeys.SceneMapEnable, true, SettingLayer.Profile);
        Assert.True(model.GetBool(SettingKeys.SceneMapEnable));

        model.Set(SettingKeys.VideoAntiAlias, SettingValue.OfEnum(2));
        Assert.Equal(2, model.GetInt(SettingKeys.VideoAntiAlias));

        model.SetBool(SettingKeys.InputEmulatedMenuButton, true);
        Assert.True(model.GetBool(SettingKeys.InputEmulatedMenuButton));

        // All three are in the model, and the two user ones are persisted.
        Assert.True(SettingsCatalogue.Get(SettingKeys.VideoAntiAlias).IsPersisted);
        Assert.True(SettingsCatalogue.Get(SettingKeys.InputEmulatedMenuButton).IsPersisted);
        Assert.Equal(SettingEffect.None, SettingsCatalogue.Get(SettingKeys.SceneMapEnable).Effect);
    }

    /// <summary>
    /// REQ-PERS-080 and AMD-016 (Q-034): every key matches the grammar, and the set of keys is
    /// compared with the document as a non-regression list.
    /// </summary>
    [PersFact]
    public void REQ_PERS_080_every_key_matches_the_grammar()
    {
        foreach (SettingDescriptor descriptor in SettingsCatalogue.All)
        {
            Assert.True(
                SettingDescriptor.IsValidKey(descriptor.Key),
                $"{descriptor.Key} is not a stable key");
            Assert.Matches(@"^[a-z0-9_\[\]]+(\.[a-z0-9_\[\]]+)*$", descriptor.Key);
        }

        // No published key has disappeared.
        HashSet<string> model = [.. SettingsCatalogue.Keys];
        foreach (PersSettingRow row in PersTables.Settings())
        {
            Assert.Contains(row.Key, model);
        }

        // No key is declared twice.
        Assert.Equal(SettingsCatalogue.All.Count, model.Count);
    }

    /// <summary>
    /// REQ-PERS-081: every user preference exposed to libretro offers a finite ordered list, free
    /// of duplicates, that contains its default value.
    /// </summary>
    [Fact]
    public void REQ_PERS_081_every_exposed_setting_enumerates_its_domain()
    {
        Assert.NotEmpty(Zax.Settings.LibretroCoreOptions.All);

        foreach (LibretroCoreOption option in Zax.Settings.LibretroCoreOptions.All)
        {
            Assert.NotEmpty(option.Values);
            Assert.Equal(option.Values.Count, option.Values.Distinct(StringComparer.Ordinal).Count());
            Assert.Contains(option.DefaultValue, option.Values);
        }

        // audio.volume: 0 to 100 % in steps of 5, which is 21 values.
        LibretroCoreOption? volume = Zax.Settings.LibretroCoreOptions.Find("zaxnes_audio_volume");
        Assert.NotNull(volume);
        Assert.Equal(21, volume!.Value.Values.Count);
    }

    /// <summary>
    /// REQ-PERS-083 and §4.6: the core option key is <c>zaxnes_</c> plus the stable key with
    /// <c>.</c>, <c>[</c> and <c>]</c> replaced by <c>_</c>, and the extract of §4.6 is exact.
    /// </summary>
    [Fact]
    public void REQ_PERS_083_the_core_option_table_is_the_extract_of_paragraph_4_6()
    {
        Assert.Equal("zaxnes_camera_fov", SettingDescriptor.BuildLibretroKey("camera.fov"));
        Assert.Equal("zaxnes_layer_0__offset", SettingDescriptor.BuildLibretroKey("layer[0].offset"));

        LibretroCoreOption fov = Zax.Settings.LibretroCoreOptions.Find("zaxnes_camera_fov")!.Value;
        Assert.Equal<IEnumerable<string>>(
            ["45", "50", "55", "60", "65", "70", "75", "80", "85", "90", "95"],
            fov.Values);
        Assert.Equal("70", fov.DefaultValue);
        Assert.Equal("Camera", fov.Category);

        LibretroCoreOption skybox = Zax.Settings.LibretroCoreOptions.Find("zaxnes_scene_skybox")!.Value;
        Assert.Equal<IEnumerable<string>>(["Default", "Color", "Gradient", "Fantasy"], skybox.Values);
        Assert.Equal("Default", skybox.DefaultValue);

        // AMD-031 removed the TV box, so there is no core option for it any more.
        Assert.Null(Zax.Settings.LibretroCoreOptions.Find("zaxnes_scene_tv_box"));

        LibretroCoreOption mesh = Zax.Settings.LibretroCoreOptions.Find("zaxnes_render_mesh_algo")!.Value;
        Assert.Equal<IEnumerable<string>>(["GREEDY", "MARCHING", "GREEDY_MULTICOLOR"], mesh.Values);

        LibretroCoreOption shadow = Zax.Settings.LibretroCoreOptions.Find("zaxnes_render_shadow")!.Value;
        Assert.Equal<IEnumerable<string>>(["enabled", "disabled"], shadow.Values);
        Assert.Equal("enabled", shadow.DefaultValue);

        LibretroCoreOption auto = Zax.Settings.LibretroCoreOptions.Find("zaxnes_emu_auto_tv_mode")!.Value;
        Assert.Equal("disabled", auto.DefaultValue);

        LibretroCoreOption light = Zax.Settings.LibretroCoreOptions.Find("zaxnes_light_intensity")!.Value;
        Assert.Equal("0.50", light.Values[0]);
        Assert.Equal("1.50", light.Values[^1]);
        Assert.Equal("1.00", light.DefaultValue);

        LibretroCoreOption speed = Zax.Settings.LibretroCoreOptions.Find("zaxnes_emu_game_speed")!.Value;
        Assert.Equal(17, speed.Values.Count);
        Assert.Equal("0", speed.DefaultValue);
    }

    /// <summary>
    /// REQ-PERS-084: every setting carries the moment of effect of the column « Effet », with the
    /// three the criterion names.
    /// </summary>
    [Fact]
    public void REQ_PERS_084_the_moment_of_effect_is_exposed()
    {
        Assert.Equal(SettingEffect.Immediate, SettingsCatalogue.Get(SettingKeys.CameraFov).Effect);
        Assert.Equal(SettingEffect.NextRomLoad, SettingsCatalogue.Get(SettingKeys.EmuAutoTvMode).Effect);
        Assert.Equal(SettingEffect.Restart, SettingsCatalogue.Get(SettingKeys.UiWindowRect).Effect);
    }

    /// <summary>
    /// REQ-PERS-085 and §4.6: no P or S setting is exposed as a core option, the host-owned keys
    /// are not either, and a change from the host takes the same path as one from the UI.
    /// </summary>
    [Fact]
    public void REQ_PERS_085_only_user_preferences_are_core_options()
    {
        foreach (LibretroCoreOption option in Zax.Settings.LibretroCoreOptions.All)
        {
            Assert.Equal(SettingScope.User, SettingsCatalogue.Get(option.SettingKey).Scope);
        }

        string[] notExposed =
        [
            SettingKeys.VideoVSync, SettingKeys.VideoFrameRate, SettingKeys.VideoSkipFrame,
            SettingKeys.VideoRenderMethod, SettingKeys.VideoResScale, SettingKeys.AudioBgMusic,
            SettingKeys.InputEmulatedMenuButton, SettingKeys.EmuCore, SettingKeys.UiSortOption,
            SettingKeys.UiFullscreen, SettingKeys.VideoGraphicQuality, SettingKeys.EmuTvMode,
        ];

        foreach (string key in notExposed)
        {
            Assert.Null(Zax.Settings.LibretroCoreOptions.Find(SettingDescriptor.BuildLibretroKey(key)));
        }

        // A host change goes through the model: it is validated and notified.
        SettingsModel model = new();
        List<SettingChange> changes = [];
        model.Changed += changes.Add;

        Assert.True(Zax.Settings.LibretroCoreOptions.TryApply(model, "zaxnes_camera_fov", "85"));
        Assert.Equal(85f, model.GetFloat(SettingKeys.CameraFov));
        Assert.Contains(changes, change => change.Key == SettingKeys.CameraFov);

        Assert.False(Zax.Settings.LibretroCoreOptions.TryApply(model, "zaxnes_camera_fov", "999"));
        Assert.False(Zax.Settings.LibretroCoreOptions.TryApply(model, "zaxnes_unknown", "1"));
    }

    /// <summary>
    /// REQ-PERS-082: the model is a library with no dependency on the UI, the window or the file
    /// system, and two instances are independent.
    /// </summary>
    [Fact]
    public void REQ_PERS_082_two_models_are_independent_and_touch_no_disk()
    {
        SettingsModel first = new();
        SettingsModel second = new();

        first.SetFloat(SettingKeys.CameraFov, 95f);
        Assert.Equal(95f, first.GetFloat(SettingKeys.CameraFov));
        Assert.Equal(70f, second.GetFloat(SettingKeys.CameraFov));

        // The assembly references no windowing, graphics or file-system package.
        string[] forbidden = ["Silk.NET", "ImGui", "OpenGL", "OpenAL"];
        foreach (var reference in typeof(SettingsModel).Assembly.GetReferencedAssemblies())
        {
            foreach (string name in forbidden)
            {
                Assert.DoesNotContain(name, reference.Name ?? string.Empty, StringComparison.OrdinalIgnoreCase);
            }
        }

        // Reading and writing go through an abstract blob store, not through System.IO.
        Assert.NotNull(typeof(ISettingsStorage).GetMethod("Read"));
        Assert.NotNull(typeof(ISettingsStorage).GetMethod("Write"));
    }
}
