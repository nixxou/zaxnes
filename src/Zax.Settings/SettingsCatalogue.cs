namespace Zax.Settings;

/// <summary>
/// The complete settings catalogue of SPEC-09 §4.1.1 to §4.1.10 (REQ-PERS-001), enumerable by
/// program (REQ-ARCH-021).
/// </summary>
/// <remarks>
/// <para>
/// One <see cref="SettingDescriptor"/> per setting, in the order of the specification's tables,
/// with the stable key, the shape, the default, the domain, the scope, the moment of effect, the
/// Lua path and aliases, the field number of the profile message and the libretro
/// discretisation. The table is immutable and shared: it is metadata, not state, so
/// REQ-PERS-082's "no global state" applies to <see cref="SettingsModel"/>, which holds the
/// values, not to this.
/// </para>
/// <para>
/// Where the specification gives a UI range and a wider script domain, the <b>domain</b> is the
/// one a script may reach and the UI range only feeds the libretro discretisation. The clearest
/// case is <c>light.intensity</c>: the slider covers 0.5 to 1.5 but SPEC-03 REQ-PROF-041's
/// acceptance criterion keeps a user value of 2 across a profile load, so the domain is
/// unbounded.
/// </para>
/// </remarks>
public static class SettingsCatalogue
{
    /// <summary>The number of layers, mirrored from <see cref="SettingKeys.LayerCount"/>.</summary>
    private const int Layers = SettingKeys.LayerCount;

    // Declared before Descriptors on purpose: static initialisers run in declaration order, so
    // anything Build() reads has to come first or it would still be null.

    /// <summary>
    /// The eight background presets of DEC-09-Q5, in the order the decision lists them, as
    /// packed RGB. The default — black — is one of them, which REQ-PERS-081 requires.
    /// </summary>
    private static readonly SettingOption[] BackgroundPresets =
    [
        // Packed RGB, widened to the option's double. No numeric suffix: a 'd' after a hex
        // literal is read as another hex digit, not as a double.
        new SettingOption(0x000000, "Black"),
        new SettingOption(0xFFFFFF, "White"),
        new SettingOption(0x808080, "Gray"),

        // NES colour $21 of the palette of SPEC-04 (DEC-04B-Q1).
        new SettingOption(0x0078FC, "NES Sky Blue"),
        new SettingOption(0x000080, "Dark Blue"),
        new SettingOption(0x008000, "Green"),
        new SettingOption(0xFF0000, "Red"),
        new SettingOption(0x800080, "Purple"),
    ];

    /// <summary>The alignment domain: 0 FRONT, 1 CENTER, 2 BACK (SPEC-05 constants).</summary>
    private static readonly SettingDomain AlignmentDomain = SettingDomain.Choice(
        new SettingOption(0d, "FRONT"),
        new SettingOption(1d, "CENTER"),
        new SettingOption(2d, "BACK"));

    /// <summary>The layer domain: 0 UNIDENTIFIED, 1 UI, 2 to 5 L1 to L4 (SPEC-05 TLayer).</summary>
    private static readonly SettingDomain LayerDomain = SettingDomain.Choice(
        new SettingOption(0d, "UNIDENTIFIED"),
        new SettingOption(1d, "UI"),
        new SettingOption(2d, "L1"),
        new SettingOption(3d, "L2"),
        new SettingOption(4d, "L3"),
        new SettingOption(5d, "L4"));

    private static readonly SettingDescriptor[] Descriptors = Build();

    private static readonly Dictionary<string, SettingDescriptor> ByKeyIndex = BuildKeyIndex(Descriptors);

    private static readonly Dictionary<string, SettingDescriptor> ByLuaIndex = BuildLuaIndex(Descriptors);

    /// <summary>Every setting of §4.1, in the order of the specification's tables.</summary>
    public static IReadOnlyList<SettingDescriptor> All => Descriptors;

    /// <summary>Every stable key.</summary>
    public static IEnumerable<string> Keys
    {
        get
        {
            foreach (SettingDescriptor descriptor in Descriptors)
            {
                yield return descriptor.Key;
            }
        }
    }

    /// <summary>
    /// The keys a former version of the program persisted and this one no longer knows
    /// (AMD-031 §3).
    /// </summary>
    /// <remarks>
    /// They are not settings: they have no descriptor, no default and no effect. They are
    /// named only so that <c>settings.json</c> can drop them instead of keeping them as the
    /// unknown keys of REQ-PERS-023 — a retired key is read without error and never written
    /// back, where an unknown one survives every rewrite.
    /// </remarks>
    public static IReadOnlySet<string> RetiredKeys { get; } = new HashSet<string>(StringComparer.Ordinal)
    {
        // AMD-031: the decorative TV box is gone from the product.
        "scene.tv_box",
    };

    /// <summary>Whether <paramref name="key"/> is one of the retired keys of AMD-031 §3.</summary>
    public static bool IsRetired(string? key) => key is not null && RetiredKeys.Contains(key);

    /// <summary>The descriptor of <paramref name="key"/>, or null when there is none.</summary>
    public static SettingDescriptor? Find(string key)
        => key is not null && ByKeyIndex.TryGetValue(key, out SettingDescriptor? found) ? found : null;

    /// <summary>The descriptor of <paramref name="key"/>.</summary>
    /// <exception cref="KeyNotFoundException">No setting carries that key.</exception>
    public static SettingDescriptor Get(string key)
        => Find(key) ?? throw new KeyNotFoundException($"'{key}' is not a setting of SPEC-09 §4.1");

    /// <summary>
    /// The descriptor the Lua path <paramref name="path"/> designates, aliases included
    /// (REQ-PERS-008). The path is written as the column "Lua" writes it, with
    /// <c>Setting.</c> left off.
    /// </summary>
    public static SettingDescriptor? FindByLuaPath(string path)
        => path is not null && ByLuaIndex.TryGetValue(path, out SettingDescriptor? found) ? found : null;

    /// <summary>The settings of <paramref name="scope"/>.</summary>
    public static IEnumerable<SettingDescriptor> OfScope(SettingScope scope)
    {
        foreach (SettingDescriptor descriptor in Descriptors)
        {
            if (descriptor.Scope == scope)
            {
                yield return descriptor;
            }
        }
    }

    /// <summary>The settings of <paramref name="theme"/>, for the options window.</summary>
    public static IEnumerable<SettingDescriptor> OfTheme(SettingTheme theme)
    {
        foreach (SettingDescriptor descriptor in Descriptors)
        {
            if (descriptor.Theme == theme)
            {
                yield return descriptor;
            }
        }
    }

    private static Dictionary<string, SettingDescriptor> BuildKeyIndex(SettingDescriptor[] descriptors)
    {
        Dictionary<string, SettingDescriptor> index = new(descriptors.Length, StringComparer.Ordinal);
        foreach (SettingDescriptor descriptor in descriptors)
        {
            index.Add(descriptor.Key, descriptor);
        }

        return index;
    }

    private static Dictionary<string, SettingDescriptor> BuildLuaIndex(SettingDescriptor[] descriptors)
    {
        Dictionary<string, SettingDescriptor> index = new(descriptors.Length, StringComparer.Ordinal);
        foreach (SettingDescriptor descriptor in descriptors)
        {
            if (descriptor.LuaPath is { } path)
            {
                index.Add(path, descriptor);
            }

            foreach (string alias in descriptor.LuaAliases)
            {
                index.Add(alias, descriptor);
            }
        }

        return index;
    }

    private static SettingDescriptor[] Build()
    {
        List<SettingDescriptor> list = [];
        AddVideo(list);
        AddCamera(list);
        AddLight(list);
        AddScene(list);
        AddClipping(list);
        AddLayers(list);
        AddEmulation(list);
        AddConversion(list);
        AddAudioInputInterface(list);
        AddVrEditorMovieDerived(list);
        AddReimplementationExtras(list);
        return [.. list];
    }

    #region §4.1.1 Video and rendering

    private static void AddVideo(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.VideoGraphicQuality,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Video)
        {
            // Profile-scoped, faithfully (DEC-09-Q1); hidden on desktop (SPEC-10 REQ-UI-062) and
            // withdrawn from the core options by SPEC-09 v1.1.
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "Low"),
                new SettingOption(1d, "Medium"),
                new SettingOption(2d, "High")),
            LuaPath = "GraphicQuality",
            ProfileField = "7",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoVSync,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Video)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "Off"),
                new SettingOption(1d, "Every refresh"),
                new SettingOption(2d, "Every other refresh")),
            LuaPath = "VSync",
            ProfileField = "9",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoFrameRate,
            SettingKind.Int,
            SettingValue.OfInt(90),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Video)
        {
            // Rule 1 of REQ-PERS-003: anything other than 60, 90 or 120 becomes 90.
            Domain = SettingDomain.Choice(
                new SettingOption(60d, "60fps"),
                new SettingOption(90d, "90fps"),
                new SettingOption(120d, "120fps")),
            LuaPath = "FrameRate",
            ProfileField = "10",
            UiLabel = "Render Rate",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoSkipFrame,
            SettingKind.Bool,
            SettingValue.OfBool(true),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Video)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "SkipFrame",
            ProfileField = "30",
            UiLabel = "Fast Render",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoRenderMethod,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Video)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "Direct"),
                new SettingOption(1d, "Render Tex")),
            LuaPath = "RenderMethod",
            ProfileField = "28",
            UiLabel = "Direct / Render Tex",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoResScale,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Video)
        {
            // §4.1.1 shows a discretisation, §4.6 lists the key among the ones libretro does not
            // get because the host owns the resolution. §4.6 wins; see Q-035.
            // Q-035
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "Native"),
                new SettingOption(1d, "2/3"),
                new SettingOption(2d, "1/2"),
                new SettingOption(3d, "1/3"),
                new SettingOption(4d, "1/4")),
            LuaPath = "ResScaleIndex",
            ProfileField = "29",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoAntiAlias,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Video)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "None"),
                new SettingOption(1d, "FFAA"),
                new SettingOption(2d, "SMAA")),
            LuaPath = "AntiAlias",
            ProfileField = "32",
            UiLabel = "Anti Alias",
            LibretroValues =
            [
                new SettingOption(0d, "None"),
                new SettingOption(1d, "FFAA"),
                new SettingOption(2d, "SMAA"),
            ],
        });

        list.Add(new SettingDescriptor(
            SettingKeys.RenderMeshAlgo,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Render)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "GREEDY"),
                new SettingOption(1d, "MARCHING"),
                new SettingOption(2d, "GREEDY_MULTICOLOR")),
            LuaPath = "RenderMode",
            ProfileField = "13",
            UiLabel = "Render",
            LibretroValues =
            [
                new SettingOption(0d, "GREEDY"),
                new SettingOption(1d, "MARCHING"),
                new SettingOption(2d, "GREEDY_MULTICOLOR"),
            ],
        });

        list.Add(new SettingDescriptor(
            SettingKeys.RenderShadow,
            SettingKind.Bool,
            SettingValue.OfBool(true),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Render)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "Shadow",
            ProfileField = "17",
            UiLabel = "Shadow",
            LibretroValues = [new SettingOption(1d, "enabled"), new SettingOption(0d, "disabled")],
        });

        list.Add(new SettingDescriptor(
            SettingKeys.UiFullscreen,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Interface)
        {
            Domain = SettingDomain.Switch,
            UiLabel = "Full Screen",
        });
    }

    #endregion

    #region §4.1.2 Camera

    private static void AddCamera(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.CameraFov,
            SettingKind.Float,
            SettingValue.OfFloat(70f),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Camera)
        {
            // The UI slider covers 45 to 95 in steps of 5; a script is bounded to [10, 150],
            // which the original did not bound at all.
            Domain = SettingDomain.Range(10d, 150d),
            LuaPath = "Fov",
            ProfileField = "38",
            UiLabel = "FOV",
            LibretroValues = SettingDescriptor.Steps(45d, 95d, 5d, "0"),
        });

        list.Add(new SettingDescriptor(
            SettingKeys.CameraLocalCam,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Camera)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "LocalCam",
            LuaAliases = ["TopDownCam"],
            ProfileField = "25",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.CameraSmoothTime,
            SettingKind.Float,
            SettingValue.OfFloat(0.4f),
            SettingScope.Session,
            SettingEffect.Immediate,
            SettingTheme.Camera)
        {
            // Domain "> 0"; no repair rule is given, so a value outside it takes the default
            // (REQ-PERS-003). Back to 0.4 at the end of a game (REQ-PERS-012).
            Domain = SettingDomain.GreaterThan(0d),
            LuaPath = "SmoothTime",
        });
    }

    #endregion

    #region §4.1.3 Light

    private static void AddLight(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.LightDirection,
            SettingKind.Vector3,
            SettingValue.OfVector3(new SettingVector3(1f, -1f, 1f)),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Light)
        {
            Domain = SettingDomain.NonZeroVector,
            LuaPath = "Light.Direction",
            ProfileField = "1.1",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.LightColor,
            SettingKind.Color,
            SettingValue.OfColor(SettingColor.White),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Light)
        {
            LuaPath = "Light.Color",
            ProfileField = "1.2",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.LightIntensity,
            SettingKind.Float,
            SettingValue.OfFloat(1f),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Light)
        {
            // Unbounded on purpose: the slider covers 0.5 to 1.5 but SPEC-03 REQ-PROF-041 keeps
            // a user intensity of 2 across a profile load.
            Domain = SettingDomain.Unbounded,
            LuaPath = "Light.Intensity",
            ProfileField = "1.3",
            UiLabel = "Light",
            LibretroValues = SettingDescriptor.Steps(0.5d, 1.5d, 0.05d, "0.00"),
        });
    }

    #endregion

    #region §4.1.4 Background and skybox

    private static void AddScene(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.SceneSkybox,
            SettingKind.Enum,
            SettingValue.OfEnum(-1),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Scene)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(-1d, "Default"),
                new SettingOption(0d, "Color"),
                new SettingOption(1d, "Gradient"),
                new SettingOption(2d, "Fantasy")),
            LuaPath = "Skybox",
            ProfileField = "21",
            UiLabel = "Skybox",
            LibretroValues =
            [
                new SettingOption(-1d, "Default"),
                new SettingOption(0d, "Color"),
                new SettingOption(1d, "Gradient"),
                new SettingOption(2d, "Fantasy"),
            ],
        });

        list.Add(new SettingDescriptor(
            SettingKeys.SceneCustomBgColor,
            SettingKind.Color,
            SettingValue.OfColor(SettingColor.Black),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Scene)
        {
            LuaPath = "CustomBgColor",
            ProfileField = "3",
            UiLabel = "Background colour",
            LibretroValues = BackgroundPresets,
        });

        list.Add(new SettingDescriptor(
            SettingKeys.SceneUseNesBgColor,
            SettingKind.Bool,
            SettingValue.OfBool(true),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Scene)
        {
            // Forced true by every profile load (REQ-PERS-005 point 3).
            Domain = SettingDomain.Switch,
            LuaPath = "BgColor",
            ProfileField = "2",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.SceneMapEnable,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Profile,
            SettingEffect.None,
            SettingTheme.Scene)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "MapEnable",
            ProfileField = "5",
        });
    }

    /// <summary>The background presets of DEC-09-Q5, as packed RGB with their labels.</summary>
    public static IReadOnlyList<SettingOption> CustomBackgroundPresets => BackgroundPresets;

    #endregion

    #region §4.1.5 Clipping

    private static void AddClipping(List<SettingDescriptor> list)
    {
        // Index 0 left, 1 bottom, 2 right, 3 top (DEC-R2-T1, DEC-09-Q2). The non-integer
        // defaults are wanted: they avoid equalities at the edges.
        (string Key, float Default, double Maximum, string Lua, string Field)[] margins =
        [
            (SettingKeys.ClipLeft, 0.1f, 256d, "Clipping[0]", "4.1"),
            (SettingKeys.ClipBottom, 8.1f, 248d, "Clipping[1]", "4.2"),
            (SettingKeys.ClipRight, 8.1f, 256d, "Clipping[2]", "4.3"),
            (SettingKeys.ClipTop, 0.1f, 248d, "Clipping[3]", "4.4"),
        ];

        foreach ((string key, float value, double maximum, string lua, string field) in margins)
        {
            list.Add(new SettingDescriptor(
                key,
                SettingKind.Float,
                SettingValue.OfFloat(value),
                SettingScope.Profile,
                SettingEffect.Immediate,
                SettingTheme.Clipping)
            {
                Domain = SettingDomain.Range(0d, maximum),
                LuaPath = lua,
                ProfileField = field,
            });
        }
    }

    #endregion

    #region §4.1.6 Layers

    /// <summary>Default depth offset of each layer (§4.1.6).</summary>
    private static ReadOnlySpan<float> LayerOffsetDefaults => [0f, 0f, 0f, 12f, 24f, 36f];

    /// <summary>Default alignment of each layer: FRONT, CENTER, then FRONT (§4.1.6).</summary>
    private static ReadOnlySpan<int> LayerAlignmentDefaults => [0, 1, 0, 0, 0, 0];

    /// <summary>
    /// The stable shadow state after any profile load: layer 2 alone casts, layers 2 to 5
    /// receive (REQ-PERS-005 point 4, REQ-PROF-042). The first-launch state of the original is
    /// not reproduced (DEC-04B-Q5).
    /// </summary>
    public static bool DefaultCastShadow(int layer) => layer == 2;

    /// <inheritdoc cref="DefaultCastShadow"/>
    public static bool DefaultRecvShadow(int layer) => layer >= 2;

    /// <summary>
    /// The derived alignment coefficient: 1 for FRONT, 0 for CENTER, −1 for BACK
    /// (REQ-PERS-013 point 8).
    /// </summary>
    public static float AlignmentCoefficient(int alignment) => alignment switch
    {
        0 => 1f,
        2 => -1f,
        _ => 0f,
    };

    private static void AddLayers(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.LayerDefaultLayer,
            SettingKind.Enum,
            SettingValue.OfEnum(2),
            SettingScope.Profile,

            // The column reads "jeu / imm."; the weaker of the two is recorded, so a frontend
            // never promises an effect sooner than the specification does.
            SettingEffect.NextGameLoad,
            SettingTheme.Layers)
        {
            Domain = LayerDomain,
            LuaPath = "Layer.DefLayer",
            ProfileField = "6.2",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.LayerAuto,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Profile,
            SettingEffect.None,
            SettingTheme.Layers)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "Layer.Auto",
            ProfileField = "6.3",
        });

        for (int i = 0; i < Layers; i++)
        {
            int layer = i;

            list.Add(new SettingDescriptor(
                SettingKeys.LayerOffset(layer),
                SettingKind.Float,
                SettingValue.OfFloat(LayerOffsetDefaults[layer]),
                SettingScope.Profile,
                SettingEffect.Immediate,
                SettingTheme.Layers)
            {
                Domain = SettingDomain.Unbounded,
                LuaPath = $"Layer[{layer}].Offset",
                ProfileField = "6.1.1",
            });

            list.Add(new SettingDescriptor(
                SettingKeys.LayerAligned(layer),
                SettingKind.Enum,
                SettingValue.OfEnum(LayerAlignmentDefaults[layer]),
                SettingScope.Profile,
                SettingEffect.Immediate,
                SettingTheme.Layers)
            {
                Domain = AlignmentDomain,
                LuaPath = $"Layer[{layer}].Aligned",
                ProfileField = "6.1.2",
            });

            list.Add(new SettingDescriptor(
                SettingKeys.LayerCoef(layer),
                SettingKind.Float,
                SettingValue.OfFloat(AlignmentCoefficient(LayerAlignmentDefaults[layer])),
                SettingScope.Derived,
                SettingEffect.None,
                SettingTheme.Layers)
            {
                LuaPath = $"Layer[{layer}].Coef",
                IsLuaWritable = false,
            });

            list.Add(new SettingDescriptor(
                SettingKeys.LayerCastShadow(layer),
                SettingKind.Bool,
                SettingValue.OfBool(DefaultCastShadow(layer)),
                SettingScope.Profile,
                SettingEffect.Immediate,
                SettingTheme.Layers)
            {
                Domain = SettingDomain.Switch,
                LuaPath = $"Layer[{layer}].CastShadow",
                ProfileField = "6.1.3",
            });

            list.Add(new SettingDescriptor(
                SettingKeys.LayerRecvShadow(layer),
                SettingKind.Bool,
                SettingValue.OfBool(DefaultRecvShadow(layer)),
                SettingScope.Profile,
                SettingEffect.Immediate,
                SettingTheme.Layers)
            {
                Domain = SettingDomain.Switch,
                LuaPath = $"Layer[{layer}].RecvShadow",
                ProfileField = "6.1.4",
            });
        }
    }

    #endregion

    #region §4.1.7 Emulation

    private static void AddEmulation(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.EmuGameSpeed,
            SettingKind.Float,
            SettingValue.OfFloat(0f),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Emulation)
        {
            // Rule 3 of REQ-PERS-003. Back to 0 at the end of a game (REQ-PERS-012).
            Domain = SettingDomain.Range(-8d, 8d),
            LuaPath = "GameSpeed",
            ProfileField = "11",
            UiLabel = "Speed",
            LibretroValues = SettingDescriptor.Steps(-8d, 8d, 1d, "0"),
        });

        list.Add(new SettingDescriptor(
            SettingKeys.EmuTvMode,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Emulation)
        {
            // Profile-scoped, faithfully (DEC-09-Q1); withdrawn from the core options by
            // SPEC-09 v1.1. MULTI is not offered by the UI.
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "NTSC"),
                new SettingOption(1d, "PAL"),
                new SettingOption(2d, "MULTI"),
                new SettingOption(3d, "DENDY")),
            LuaPath = "TVMode",
            ProfileField = "34",
            UiLabel = "TV Region",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.EmuAutoTvMode,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.User,
            SettingEffect.NextRomLoad,
            SettingTheme.Emulation)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "AutoTVMode",
            ProfileField = "33",
            LibretroValues = [new SettingOption(1d, "enabled"), new SettingOption(0d, "disabled")],
        });

        list.Add(new SettingDescriptor(
            SettingKeys.EmuCore,
            SettingKind.Int,
            SettingValue.OfInt(0),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Emulation)
        {
            // Rule 2 of REQ-PERS-003: the remainder of the division by the number of cores,
            // which is exactly 1 in v1, so the effective value is always 0.
            Domain = SettingDomain.CoreIndex,
            LuaPath = "Core",
            ProfileField = "22",
        });
    }

    #endregion

    #region §4.1.8 Conversion

    private static void AddConversion(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.ConvSameBgPalette,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Conversion)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "SameBgPalette",
            LuaAliases = ["SamePalette"],
            ProfileField = "18",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.ConvSameSpPalette,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Conversion)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "SameSpPalette",
            ProfileField = "37",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.ConvSpriteTolerance,
            SettingKind.Int,
            SettingValue.OfInt(0),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Conversion)
        {
            // Rule 4 of REQ-PERS-003: a negative value becomes its absolute value.
            Domain = SettingDomain.NonNegativeByAbsoluteValue,
            LuaPath = "SpriteTolerance",
            ProfileField = "19",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.ConvScanDirection,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Conversion)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "Rows"),
                new SettingOption(1d, "Columns")),
            LuaPath = "ScanDirection",
            ProfileField = "20",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.ConvEmptyTile,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Profile,
            SettingEffect.Immediate,
            SettingTheme.Conversion)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "EmptyTile",
            ProfileField = "24",
        });
    }

    #endregion

    #region §4.1.9 Audio, inputs, interface

    private static void AddAudioInputInterface(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.AudioVolume,
            SettingKind.Float,
            SettingValue.OfFloat(1f),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Audio)
        {
            Domain = SettingDomain.Range(0d, 1d),
            LuaPath = "Volume",
            ProfileField = "12",
            UiLabel = "Volume",

            // The UI slider is a percentage in steps of 1; libretro gets steps of 5.
            LibretroValues = PercentSteps(),
        });

        list.Add(new SettingDescriptor(
            SettingKeys.AudioBgMusic,
            SettingKind.Bool,
            SettingValue.OfBool(true),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Audio)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "BGMusic",
            ProfileField = "31",
            UiLabel = "BG Music",
        });

        for (int port = 0; port < SettingKeys.PortCount; port++)
        {
            list.Add(new SettingDescriptor(
                SettingKeys.InputPortType(port),
                SettingKind.Enum,
                SettingValue.OfEnum(0),
                SettingScope.Session,
                SettingEffect.Immediate,
                SettingTheme.Input)
            {
                // Lua reaches these through Setting:GetInputType and Setting:SetInputType, not
                // through a property, so there is no Lua path (SPEC-08 REQ-INP-060).
                Domain = SettingDomain.Choice(
                    new SettingOption(0d, "Gamepad"),
                    new SettingOption(1d, "Zapper")),
            });
        }

        list.Add(new SettingDescriptor(
            SettingKeys.InputZapperEnable,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Derived,
            SettingEffect.None,
            SettingTheme.Input)
        {
            LuaPath = "ZapperEnable",
            IsLuaWritable = false,
        });

        list.Add(new SettingDescriptor(
            SettingKeys.InputEmulatedMenuButton,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.User,

            // Stored and exposed with no effect at all (DEC-08-6, DEC-R2-D7).
            SettingEffect.None,
            SettingTheme.Input)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "EmulatedMenuButton",
            ProfileField = "27",
            UiLabel = "Menu = Select + Start",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.UiSortOption,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Interface)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "A-Z"),
                new SettingOption(1d, "Recent")),
            LuaPath = "SortOption",
            ProfileField = "36",
            UiLabel = "Sort by",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.UiLastBrowseDir,
            SettingKind.String,
            SettingValue.OfString(string.Empty),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Interface)
        {
            // The column gives the personal folder as the default. This library knows nothing
            // of the platform (REQ-PERS-082), so the default is empty and the host substitutes
            // its own documents folder when it reads an empty value (REQ-PERS-066).
        });

        list.Add(new SettingDescriptor(
            SettingKeys.UiWindowRect,
            SettingKind.Rect,
            SettingValue.OfRect(new SettingRect(-1, -1, 1280, 720)),
            SettingScope.User,
            SettingEffect.Restart,
            SettingTheme.Interface)
        {
            // The default size is the one of SPEC-10; −1 for the position means "let the
            // platform place the window" (REQ-PERS-067).
        });
    }

    /// <summary>The volume discretisation of §4.6: 0 to 100 % in steps of 5.</summary>
    private static IReadOnlyList<SettingOption> PercentSteps()
    {
        List<SettingOption> options = new(21);
        for (int percent = 0; percent <= 100; percent += 5)
        {
            options.Add(new SettingOption(percent / 100d, $"{percent}%"));
        }

        return options;
    }

    #endregion

    #region §4.1.10 VR, editor, movie, internal and derived

    private static void AddVrEditorMovieDerived(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.VrRuntime,
            SettingKind.Int,
            SettingValue.OfInt(0),
            SettingScope.User,
            SettingEffect.None,
            SettingTheme.Vr)
        {
            // Always 0 on reading, every write ignored without error (REQ-PERS-011).
            LuaPath = "VrRuntime",
            ProfileField = "8",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VrSceneRatio,
            SettingKind.Float,
            SettingValue.OfFloat(1f),
            SettingScope.User,
            SettingEffect.None,
            SettingTheme.Vr)
        {
            // The written value is kept; reading gives 1 while the runtime is 0 (REQ-PERS-011).
            LuaPath = "SceneRatio",
            ProfileField = "26",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VrMirroring,
            SettingKind.Bool,
            SettingValue.OfBool(true),
            SettingScope.User,
            SettingEffect.None,
            SettingTheme.Vr)
        {
            Domain = SettingDomain.Switch,
            LuaPath = "Mirroring",
            ProfileField = "23",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VrControlMethod,
            SettingKind.Int,
            SettingValue.OfInt(0),
            SettingScope.User,
            SettingEffect.None,
            SettingTheme.Vr)
        {
            LuaPath = "ControlMethod",
            ProfileField = "16",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.EditorSelectionMode,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.Session,
            SettingEffect.None,
            SettingTheme.Editor)
        {
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "Shape3D"),
                new SettingOption(1d, "Tile")),
            LuaPath = "SelectionMode",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.EditorSavedPatterns,
            SettingKind.Opaque,
            SettingValue.OfOpaque(null),
            SettingScope.Profile,
            SettingEffect.None,
            SettingTheme.Editor)
        {
            // Five records named Config1 to Config5 by default (SPEC-03 REQ-PROF-044); the
            // records themselves belong to the profile format, so this model only carries them.
            LuaPath = "SavedPattern3D",
            ProfileField = "14",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.EditorSavedAnimPatterns,
            SettingKind.Opaque,
            SettingValue.OfOpaque(null),
            SettingScope.Profile,
            SettingEffect.None,
            SettingTheme.Editor)
        {
            LuaPath = "SaveAnimPattern3D",
            ProfileField = "15",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.MovieMode,
            SettingKind.Enum,
            SettingValue.OfEnum(0),
            SettingScope.Session,
            SettingEffect.None,
            SettingTheme.Movie)
        {
            // Frozen at NONE; a write of anything else is ignored (REQ-PERS-016).
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "NONE"),
                new SettingOption(1d, "PLAYBACK"),
                new SettingOption(2d, "RECORDING")),
            LuaPath = "MovieMode",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.SceneRootScale,
            SettingKind.Float,
            SettingValue.OfFloat(1f),
            SettingScope.Session,
            SettingEffect.Immediate,
            SettingTheme.Internal)
        {
            LuaPath = "RootScale",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.SceneSceneScale,
            SettingKind.Float,
            SettingValue.OfFloat(1f),
            SettingScope.Derived,
            SettingEffect.Immediate,
            SettingTheme.Internal)
        {
            // Writing SceneScale writes SceneRatio (REQ-PERS-013 point 3), so this derived
            // member is the one exception that Lua may assign.
            LuaPath = "SceneScale",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.SceneSkyboxEnable,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Derived,
            SettingEffect.None,
            SettingTheme.Internal)
        {
            LuaPath = "SkyboxEnable",
            IsLuaWritable = false,
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoResScaleFactor,
            SettingKind.Float,
            SettingValue.OfFloat(1f),
            SettingScope.Derived,
            SettingEffect.None,
            SettingTheme.Internal)
        {
            LuaPath = "ResScale",
            IsLuaWritable = false,
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoTargetFramerate,
            SettingKind.Float,
            SettingValue.OfFloat(90f),
            SettingScope.Derived,
            SettingEffect.None,
            SettingTheme.Internal)
        {
            LuaPath = "TargetFramerate",
            IsLuaWritable = false,
        });

        list.Add(new SettingDescriptor(
            SettingKeys.VideoTargetTimePerFrame,
            SettingKind.Float,
            SettingValue.OfFloat(1f / 90f),
            SettingScope.Derived,
            SettingEffect.None,
            SettingTheme.Internal)
        {
            LuaPath = "TargetTimePerFrame",
            IsLuaWritable = false,
        });

        list.Add(new SettingDescriptor(
            SettingKeys.EmuTvData,
            SettingKind.Opaque,
            SettingValue.OfOpaque(null),
            SettingScope.Derived,
            SettingEffect.None,
            SettingTheme.Internal)
        {
            LuaPath = "TVData",
            IsLuaWritable = false,
        });

        list.Add(new SettingDescriptor(
            SettingKeys.AppEngineMode,
            SettingKind.Enum,
            SettingValue.OfEnum(1),
            SettingScope.Session,
            SettingEffect.None,
            SettingTheme.Internal)
        {
            // Always PROFILE: the ROM mode is the editor's (§4.1.10).
            Domain = SettingDomain.Choice(
                new SettingOption(0d, "ROM"),
                new SettingOption(1d, "PROFILE")),
            LuaPath = "EngineMode",
            IsLuaWritable = false,
        });
    }

    #endregion

    #region settings this reimplementation adds

    private static void AddReimplementationExtras(List<SettingDescriptor> list)
    {
        list.Add(new SettingDescriptor(
            SettingKeys.AudioMute,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.Session,
            SettingEffect.Immediate,
            SettingTheme.Audio)
        {
            Domain = SettingDomain.Switch,
            UiLabel = "Mute",
        });

        list.Add(new SettingDescriptor(
            SettingKeys.UiShowFps,
            SettingKind.Bool,
            SettingValue.OfBool(false),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Interface)
        {
            Domain = SettingDomain.Switch,
            UiLabel = "Show FPS",
        });

        // AMD-035 §1: the interface language, « en » by default, with no detection of the
        // system's language. The two texts are the whole domain; anything else reads as « en »
        // (SettingDescriptor.CoerceText).
        list.Add(new SettingDescriptor(
            SettingKeys.UiLanguage,
            SettingKind.String,
            SettingValue.OfString(LanguageEnglish),
            SettingScope.User,
            SettingEffect.Immediate,
            SettingTheme.Interface)
        {
            TextOptions = [LanguageEnglish, LanguageFrench],
            UiLabel = "Language",
        });
    }

    /// <summary>The <c>ui.language</c> text for English, which is also its default (AMD-035).</summary>
    public const string LanguageEnglish = "en";

    /// <summary>The <c>ui.language</c> text for French (AMD-035).</summary>
    public const string LanguageFrench = "fr";

    #endregion
}
