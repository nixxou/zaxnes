using KeraLua;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Settings;

namespace Zax.Scripting;

/// <summary>
/// The global <c>Setting</c> and the objects it carries — <c>Layer</c>, one layer and
/// <c>Clipping</c> — with the members, aliases and rights of SPEC-09 §4.2 (REQ-LUA-089).
/// </summary>
internal static partial class LuaApi
{
    private static void RegisterSettings(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.SettingApi, "SettingAPI");
        ScriptSettings settings = runtime.Settings;

        type.Property(
            "Light",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.Light, settings.Light);
                return 1;
            },
            result: HostTypeId.Light);
        type.Property(
            "Layer",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.LayerConfig, runtime.TableOf(HostTypeId.LayerConfig));
                return 1;
            },
            result: HostTypeId.LayerConfig);
        type.Property(
            "Clipping",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.Clipping, runtime.TableOf(HostTypeId.Clipping));
                return 1;
            },
            result: HostTypeId.Clipping);

        Flag("EmptyTile", () => settings.EmptyTile, v => settings.EmptyTile = v);
        Flag("BgColor", () => settings.BgColor, v => settings.BgColor = v);
        Flag("SameBgPalette", () => settings.SameBgPalette, v => settings.SameBgPalette = v);
        type.Alias("SamePalette", "SameBgPalette");
        Flag("SameSpPalette", () => settings.SameSpPalette, v => settings.SameSpPalette = v);
        Flag("LocalCam", () => settings.LocalCam, v => settings.LocalCam = v);
        type.Alias("TopDownCam", "LocalCam");
        Flag("Shadow", () => settings.Shadow, v => settings.Shadow = v);
        Flag("MapEnable", () => settings.MapEnable, v => settings.MapEnable = v);
        Flag("Mirroring", () => settings.Mirroring, v => settings.Mirroring = v);
        Flag("SkipFrame", () => settings.SkipFrame, v => settings.SkipFrame = v);
        Flag("BGMusic", () => settings.BGMusic, v => settings.BGMusic = v);
        Flag("AutoTVMode", () => settings.AutoTVMode, v => settings.AutoTVMode = v);
        Flag("EmulatedMenuButton", () => settings.EmulatedMenuButton, v => settings.EmulatedMenuButton = v);

        Real("GameSpeed", () => settings.GameSpeed, v => settings.GameSpeed = v);
        Real("SmoothTime", () => settings.SmoothTime, v => settings.SmoothTime = v);
        Real("Fov", () => settings.Fov, v => settings.Fov = v);
        Real("Volume", () => settings.Volume, v => settings.Volume = v);
        Real("SceneRatio", () => settings.SceneRatio, v => settings.SceneRatio = v);
        Real("SceneScale", () => settings.SceneScale, v => settings.SceneScale = v);
        Real("RootScale", () => settings.RootScale, v => settings.RootScale = v);

        Whole("VrRuntime", () => settings.VrRuntime, v => settings.VrRuntime = v);
        Whole("VSync", () => settings.VSync, v => settings.VSync = v);
        Whole("FrameRate", () => settings.FrameRate, v => settings.FrameRate = v);
        Whole("GraphicQuality", () => settings.GraphicQuality, v => settings.GraphicQuality = v);
        Whole("RenderMode", () => settings.RenderMode, v => settings.RenderMode = v);
        Whole("RenderMethod", () => settings.RenderMethod, v => settings.RenderMethod = v);
        Whole("ResScaleIndex", () => settings.ResScaleIndex, v => settings.ResScaleIndex = v);
        Whole("AntiAlias", () => settings.AntiAlias, v => settings.AntiAlias = v);
        Whole("SpriteTolerance", () => settings.SpriteTolerance, v => settings.SpriteTolerance = v);
        Whole("ScanDirection", () => settings.ScanDirection, v => settings.ScanDirection = v);
        Whole("Skybox", () => settings.Skybox, v => settings.Skybox = v);
        Whole("Core", () => settings.Core, v => settings.Core = v);
        Whole("TVMode", () => settings.TVMode, v => settings.TVMode = v);
        Whole("TVBox", () => settings.TVBox, v => settings.TVBox = v);
        Whole("SortOption", () => settings.SortOption, v => settings.SortOption = v);
        Whole("MovieMode", () => settings.MovieMode, v => settings.MovieMode = v);
        Whole("SelectionMode", () => settings.SelectionMode, v => settings.SelectionMode = v);
        Whole("ControlMethod", () => settings.ControlMethod, v => settings.ControlMethod = v);

        type.Property(
            "CustomBgColor",
            (b, l) =>
            {
                b.PushColor(l, settings.CustomBgColor);
                return 1;
            },
            (b, l) =>
            {
                if (!b.TryGetColor(l, 3, out LuaColor colour))
                {
                    b.InvalidArguments(l, "CustomBgColor");
                    return;
                }

                settings.CustomBgColor = colour;
            },
            HostTypeId.Color);

        // Derived, read only (REQ-PERS-013).
        type.Property("TargetFramerate", (_, l) => Number(l, settings.TargetFramerate));
        type.Property("TargetTimePerFrame", (_, l) => Number(l, settings.TargetTimePerFrame));
        type.Property("SkyboxEnable", (_, l) => Boolean(l, settings.SkyboxEnable));
        type.Property("ResScale", (_, l) => Number(l, settings.ResScale));
        type.Property("ZapperEnable", (_, l) => Boolean(
            l,
            runtime.Environment.Input.GetPortType(0) == 1 || runtime.Environment.Input.GetPortType(1) == 1));

        // The engine mode of SPEC-09 §4.1.10, read only. AMD-020 (Q-052) settles the
        // contradiction between SPEC-09, which publishes the three names, and SPEC-05 §4, which
        // listed none of them: the three are required.
        // Q-052
        type.Property("EngineMode", (_, l) => Number(l, settings.EngineMode));
        type.Property("IsProfileMode", (_, l) => Boolean(l, settings.IsProfileMode));
        type.Property("IsRomMode", (_, l) => Boolean(l, settings.IsRomMode));

        // Kept and handed back, with no model of their own in this version (REQ-PERS-016).
        Stored("SavedPattern3D");
        Stored("SaveAnimPattern3D");
        Stored("TVData");

        // The notification identifiers and the two scale constants of SPEC-09 §4.3
        // (REQ-PERS-014); no built-in profile reads them.
        foreach ((string name, string value) in SettingConstants)
        {
            type.Property(name, (_, l) => Text(l, value));
        }

        type.Property("DefaultSmoothTime", (_, l) => Number(l, ScriptSettings.DefaultSmoothTime));
        type.Property("VRScale", (_, l) => Number(l, ScriptSettings.VrScale));
        type.Property("NonVRScale", (_, l) => Number(l, ScriptSettings.NonVrScale));

        type.Function("GetInputType", (b, l) =>
        {
            Require(b, l, "GetInputType");
            int port = b.CheckInteger(l, 2, "GetInputType");
            return Number(l, runtime.Environment.Input.GetPortType(port));
        });

        type.Function("SetInputType", (b, l) =>
        {
            Require(b, l, "SetInputType");
            int first = b.CheckInteger(l, 2, "SetInputType");
            int second = b.CheckInteger(l, 3, "SetInputType");
            runtime.Environment.Input.SetPortTypes(first, second);
            settings.SetPortTypes(first, second);
            return 0;
        });

        type.Function("SetRuntime", (b, l) =>
        {
            Require(b, l, "SetRuntime");

            // No effect on desktop (REQ-PERS-015).
            return 0;
        });

        type.Function("Save", (b, l) =>
        {
            Require(b, l, "Save");

            // Writes the settings file at once; nothing happens when the host set no hook
            // (REQ-PERS-015, P3).
            settings.SaveRequested?.Invoke();
            return 0;
        });

        type.Function("Load", (b, l) =>
        {
            Require(b, l, "Load");

            // Re-reads the file and applies only the user preferences it holds (REQ-PERS-015).
            settings.LoadRequested?.Invoke();
            return 0;
        });

        RegisterLayers(binder, runtime, settings);
        return;

        void Flag(string name, Func<bool> read, Action<bool> write)
            => type.Property(name, (_, l) => Boolean(l, read()), (_, l) => write(AssignedBoolean(l)));

        void Real(string name, Func<float> read, Action<float> write)
            => type.Property(
                name,
                (_, l) => Number(l, read()),
                (b, l) => write((float)AssignedNumber(b, l, name)));

        void Whole(string name, Func<int> read, Action<int> write)
            => type.Property(
                name,
                (_, l) => Number(l, read()),
                (b, l) => write(AssignedInteger(b, l, name)));

        void Stored(string name)
            => type.Property(
                name,
                static (_, l) =>
                {
                    l.PushNil();
                    return 1;
                },
                static (_, _) => { });

        static void Require(LuaBinder binder, Lua state, string member)
        {
            if (binder.TypeIdAt(state, 1) != HostTypeId.SettingApi)
            {
                binder.InvalidArguments(state, member);
            }
        }
    }

    /// <summary>The notification identifiers of SPEC-09 §4.3.</summary>
    private static readonly (string Name, string Value)[] SettingConstants =
    [
        ("SelectionModeId", "SelectionModeId"),
        ("ControlMethodId", "ControlMethodId"),
        ("SceneScaleId", "SceneScaleId"),
        ("LightId", "LightId"),
        ("LightDirectionId", "Light.DirectionId"),
        ("LightColorId", "Light.ColorId"),
        ("LightIntensityId", "Light.IntensityId"),
        ("BgColorId", "BgColorId"),
        ("CustomBgColorId", "CustomBgColorId"),
        ("ClippingId", "Clipping"),
        ("ClippingElementId", "Clipping"),
        ("MapEnableId", "MapEnable"),
        ("LayerId", "Layer"),
        ("LayerAlignedId", "Layer.Aligned"),
        ("LayerOffsetId", "Layer.Offset"),
        ("LayerDefLayerId", "Layer.DefLayer"),
        ("LayerAutoId", "Layer.Auto"),
        ("LocalCamId", "LocalCamId"),
        ("GraphicQualityId", "GraphicQuality"),
        ("VrRuntimeId", "VrRuntimeId"),
        ("PreVrRuntimeId", "PreVrRuntimeId"),
        ("VSyncId", "VSyncId"),
        ("FrameRateId", "FrameRateId"),
        ("FovId", "FovId"),
        ("GameSpeedId", "GameSpeedId"),
        ("VolumeId", "VolumeId"),
        ("RenderModeId", "RenderModeId"),
        ("SkyboxId", "SkyboxId"),
        ("CoreId", "CoreId"),
        ("MirroringId", "MirroringId"),
        ("EmptyTileId", "EmptyTileId"),
        ("InputTypeId", "InputTypeId"),
        ("MovieModeId", "MovieModeId"),
        ("SceneRatioId", "SceneRatioId"),
        ("EmuMenuId", "EmuMenuId"),
        ("RenderMethodId", "RenderMethodId"),
        ("ResScaleId", "ResScaleId"),
        ("SkipFrameId", "SkipFrameId"),
        ("BgMusicId", "BgMusicId"),
        ("AntiAliasId", "AntiAliasId"),
        ("AutoTVModeId", "AutoTVModeId"),
        ("TVModeId", "TVModeId"),
        ("TVBoxId", "TVBoxId"),
        ("SortOptionId", "SortOptionId"),
        ("ShadowId", "ShadowId"),
        ("SameBgPaletteId", "SameBgPaletteId"),
        ("SameSpPaletteId", "SameSpPaletteId"),
        ("SpriteToleranceId", "SpriteToleranceId"),
        ("ScanDirectionId", "ScanDirectionId"),
        ("SavedPattern3DId", "SavePatternId"),
        ("SavedAnimPattern3DId", "SaveAnimPatternId"),
    ];

    /// <summary>
    /// <c>Setting.Layer</c> and <c>Setting.Clipping</c>. Both read and write the settings model
    /// through <paramref name="settings"/>, which is the living model REQ-PERS-007 demands: a
    /// write is visible to the very next read, from Lua, from the UI or from the renderer, and
    /// the model mirrors it into the conversion pipeline's own slice.
    /// </summary>
    private static void RegisterLayers(
        LuaBinder binder,
        ProfileScriptRuntime runtime,
        ScriptSettings settings)
    {
        // Setting.Layer, the global Layer (§4.2, type ConfigCalques).
        HostType config = binder.DeclareType(HostTypeId.LayerConfig, "ConfigCalques");
        config.Property(
            "DefLayer",
            (_, l) => Number(l, settings.DefaultLayer),
            (b, l) =>
            {
                int value = AssignedInteger(b, l, "DefLayer");
                if ((uint)value >= (uint)SettingKeys.LayerCount)
                {
                    b.Raise(l, $"layer {value} is out of range for DefLayer");
                    return;
                }

                settings.DefaultLayer = value;
            });
        config.Property(
            "Auto",
            (_, l) => Boolean(l, settings.AutoLayer),
            (_, l) => settings.AutoLayer = AssignedBoolean(l));

        // Layer[i] accepts 0 to 5; out of range raises (REQ-LUA-045, REQ-LUA-063).
        config.ElementType = HostTypeId.Layer;
        config.Indexer = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Layer");
            b.CheckBounds(l, index, SettingKeys.LayerCount, "Layer");
            b.PushDescriptor(l, HostTypeId.Layer, runtime.TableOf(HostTypeId.LayerConfig), index);
            return 1;
        };

        // One layer (§4.2, type Calque).
        HostType layer = binder.DeclareType(HostTypeId.Layer, "Calque");
        layer.Property(
            "Offset",
            (b, l) => Number(l, Of(b, l).Offset),
            (b, l) =>
            {
                float value = (float)AssignedNumber(b, l, "Offset");
                Of(b, l).Offset = value;
            });
        layer.Property(
            "Aligned",
            (b, l) => Number(l, Of(b, l).Aligned),
            (b, l) =>
            {
                int value = AssignedInteger(b, l, "Aligned");
                Of(b, l).Aligned = Math.Clamp(value, 0, 2);
            });
        layer.Property("Coef", (b, l) => Number(l, Of(b, l).Coef));
        layer.Property(
            "CastShadow",
            (b, l) => Boolean(l, Of(b, l).CastShadow),
            (b, l) =>
            {
                bool value = AssignedBoolean(l);
                Of(b, l).CastShadow = value;
            });
        layer.Property(
            "RecvShadow",
            (b, l) => Boolean(l, Of(b, l).RecvShadow),
            (b, l) =>
            {
                bool value = AssignedBoolean(l);
                Of(b, l).RecvShadow = value;
            });

        // Setting.Clipping, the global Clipping (§4.2, type Découpe). Index 0 is the left
        // margin, 1 the bottom, 2 the right, 3 the top (DEC-R2-T1).
        HostType clipping = binder.DeclareType(HostTypeId.Clipping, "Decoupe");
        Margin(clipping, "x", 0);
        Margin(clipping, "y", 1);
        Margin(clipping, "z", 2);
        Margin(clipping, "w", 3);
        clipping.Property(
            "target",
            (b, l) =>
            {
                Vec4F margins = settings.ClippingVector;
                b.PushColor(l, new LuaColor(margins.X, margins.Y, margins.Z, margins.W));
                return 1;
            },
            (b, l) =>
            {
                if (!b.TryGetColor(l, 3, out LuaColor value))
                {
                    b.InvalidArguments(l, "target");
                    return;
                }

                settings.SetClippingVector(new Vec4F(value.R, value.G, value.B, value.A));
            },
            HostTypeId.Color);
        clipping.Indexer = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Clipping");
            b.CheckBounds(l, index, SettingKeys.ClippingCount, "Clipping");
            return Number(l, settings.Clipping(index));
        };
        clipping.IndexSetter = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Clipping");
            b.CheckBounds(l, index, SettingKeys.ClippingCount, "Clipping");
            settings.SetClipping(index, (float)AssignedNumber(b, l, "Clipping"));
        };
        return;

        ScriptSettings.LayerView Of(LuaBinder innerBinder, Lua state)
            => settings.Layer(innerBinder.MemberAt(state, 1));

        void Margin(HostType host, string name, int index)
            => host.Property(
                name,
                (_, l) => Number(l, settings.Clipping(index)),
                (b, l) => settings.SetClipping(index, (float)AssignedNumber(b, l, name)));
    }
}
