using Zax.Engine3D;
using Zax.Profiles;
using Zax.Settings;

namespace Zax.Scripting;

/// <summary>
/// The settings model as scripts see it: the <c>Setting</c> object of SPEC-05 REQ-LUA-089 with
/// the members, aliases, domains and defaults of SPEC-09 (REQ-PERS-007 to REQ-PERS-017, tables
/// §4.1 and §4.2).
/// </summary>
/// <remarks>
/// <para>
/// This holds no value of its own: it is a typed view over the one
/// <see cref="Settings.SettingsModel"/> of the emulator instance, so a write from Lua, a change
/// from the options window, a profile load and a libretro core option all meet in the same place
/// and all go through the same validation and the same notification (REQ-PERS-007).
/// </para>
/// <para>
/// Every write made through this object is a <b>script</b> write. On a user preference the model
/// puts it in the session layer, which the game's end discards (REQ-PERS-010, DEC-09-Q3); on a
/// profile setting it changes the live scene value, which relaunching the same game keeps
/// (REQ-PERS-006).
/// </para>
/// <para>
/// The conversion pipeline keeps its own slice of these values in
/// <see cref="ConversionSettings"/>, which it reads every frame. This class mirrors the model
/// into that slice on every change, so the engine sees a script's write at the next frame and a
/// profile load reaches it by the same single path.
/// </para>
/// </remarks>
public sealed class ScriptSettings : IDisposable
{
    private readonly SettingsModel _model;
    private readonly ConversionSettings? _conversion;
    private bool _disposed;

    /// <summary>Creates the view over <paramref name="model"/>.</summary>
    /// <param name="model">The settings of the emulator instance.</param>
    /// <param name="conversion">
    /// The pipeline slice to mirror into, or null in a headless use with no engine.
    /// </param>
    /// <param name="light">
    /// The light object; by default one backed by the model, which is what SPEC-09 §4.1.3
    /// requires since the three light values are settings.
    /// </param>
    public ScriptSettings(SettingsModel model, ConversionSettings? conversion = null, ScriptLight? light = null)
    {
        ArgumentNullException.ThrowIfNull(model);
        _model = model;
        _conversion = conversion;
        Light = light ?? new ModelScriptLight(model);

        _model.Changed += OnModelChanged;
        if (_conversion is not null)
        {
            MirrorAll();
        }
    }

    /// <summary>
    /// Creates a view with a model of its own, for a headless use that has no host: the engine
    /// slice still receives every change.
    /// </summary>
    public ScriptSettings(ConversionSettings conversion, ScriptLight? light = null)
        : this(new SettingsModel(), conversion, light)
    {
        ArgumentNullException.ThrowIfNull(conversion);
    }

    /// <summary>The settings of the emulator instance this view reads and writes.</summary>
    public SettingsModel Model => _model;

    /// <summary>The engine settings this view mirrors into, or null.</summary>
    public ConversionSettings? Conversion => _conversion;

    /// <summary>The directional light, which <c>Setting.Light</c> and <c>Light</c> designate.</summary>
    public ScriptLight Light { get; }

    /// <summary>
    /// Raised with the stable key of SPEC-09 §4.1 after a value really changed (REQ-PERS-009),
    /// so that the host can apply the effect it owns.
    /// </summary>
    public Action<string>? Changed { get; set; }

    /// <summary>
    /// What a view reset asks of the host (SPEC-06 REQ-SCN-012): a change of field of view or of
    /// scene ratio triggers it (REQ-PERS-011, REQ-LUA-095).
    /// </summary>
    public Action? ViewReset { get; set; }

    /// <summary>
    /// What <c>Setting:Save()</c> asks of the host: write the settings file now
    /// (REQ-PERS-015). Null in a headless use, where the call does nothing.
    /// </summary>
    public Action? SaveRequested { get; set; }

    /// <summary>
    /// What <c>Setting:Load()</c> asks of the host: re-read the settings file and apply only
    /// the user preferences it holds (REQ-PERS-015).
    /// </summary>
    public Action? LoadRequested { get; set; }

    #region video and rendering (§4.1.1)

    /// <summary>Graphic quality; stored, no effect on desktop.</summary>
    public int GraphicQuality
    {
        get => Int(SettingKeys.VideoGraphicQuality);
        set => Write(SettingKeys.VideoGraphicQuality, value);
    }

    /// <summary>Vertical sync: 0 off, 1 every refresh, 2 every other refresh.</summary>
    public int VSync
    {
        get => Int(SettingKeys.VideoVSync);
        set => Write(SettingKeys.VideoVSync, value);
    }

    /// <summary>Render rate cap; 60, 90 or 120, anything else becomes 90 (REQ-PERS-003).</summary>
    public int FrameRate
    {
        get => Int(SettingKeys.VideoFrameRate);
        set => Write(SettingKeys.VideoFrameRate, value);
    }

    /// <summary>Whether rendering is skipped when no new frame was emulated.</summary>
    public bool SkipFrame
    {
        get => Flag(SettingKeys.VideoSkipFrame);
        set => Write(SettingKeys.VideoSkipFrame, value);
    }

    /// <summary>Render method: 0 direct, 1 render texture.</summary>
    public int RenderMethod
    {
        get => Int(SettingKeys.VideoRenderMethod);
        set => Write(SettingKeys.VideoRenderMethod, value);
    }

    /// <summary>Resolution scale index, 0 to 4; anything else becomes 0 (REQ-PERS-003).</summary>
    public int ResScaleIndex
    {
        get => Int(SettingKeys.VideoResScale);
        set => Write(SettingKeys.VideoResScale, value);
    }

    /// <summary>Anti-aliasing: 0 none, 1 FXAA-like, 2 SMAA-like (REQ-PERS-017).</summary>
    public int AntiAlias
    {
        get => Int(SettingKeys.VideoAntiAlias);
        set => Write(SettingKeys.VideoAntiAlias, value);
    }

    /// <summary>Global mesh algorithm (REQ-E3D-340); a user setting, never taken from a profile.</summary>
    public int RenderMode
    {
        get => Int(SettingKeys.RenderMeshAlgo);
        set => Write(SettingKeys.RenderMeshAlgo, value);
    }

    /// <summary>The global shadow switch (REQ-SCN-030).</summary>
    public bool Shadow
    {
        get => Flag(SettingKeys.RenderShadow);
        set => Write(SettingKeys.RenderShadow, value);
    }

    #endregion

    #region camera (§4.1.2)

    /// <summary>Vertical field of view in degrees; a script's write is bounded to [10, 150].</summary>
    public float Fov
    {
        get => Real(SettingKeys.CameraFov);
        set => Write(SettingKeys.CameraFov, value);
    }

    /// <summary>The top-down camera profile setting (REQ-SCN-009); alias <c>TopDownCam</c>.</summary>
    public bool LocalCam
    {
        get => Flag(SettingKeys.CameraLocalCam);
        set => Write(SettingKeys.CameraLocalCam, value);
    }

    /// <summary>Camera smoothing time in seconds; back to 0.4 at the end of a game.</summary>
    public float SmoothTime
    {
        get => Real(SettingKeys.CameraSmoothTime);
        set => Write(SettingKeys.CameraSmoothTime, value);
    }

    #endregion

    #region background, skybox, TV (§4.1.4)

    /// <summary>Skybox: −1 default, 0 colour, 1 gradient, 2 fantasy; otherwise −1.</summary>
    public int Skybox
    {
        get => Int(SettingKeys.SceneSkybox);
        set => Write(SettingKeys.SceneSkybox, value);
    }

    /// <summary>The custom background colour (REQ-SCN-046).</summary>
    public LuaColor CustomBgColor
    {
        get => ToLua(_model.GetColor(SettingKeys.SceneCustomBgColor));
        set => _model.SetColor(SettingKeys.SceneCustomBgColor, FromLua(value), SettingLayer.Session);
    }

    /// <summary>Whether the scene background is the NES frame colour (REQ-E3D-254).</summary>
    public bool BgColor
    {
        get => Flag(SettingKeys.SceneUseNesBgColor);
        set => Write(SettingKeys.SceneUseNesBgColor, value);
    }

    /// <summary>Decorative TV box: 0 none, 1 to 3.</summary>
    public int TVBox
    {
        get => Int(SettingKeys.SceneTvBox);
        set => Write(SettingKeys.SceneTvBox, value);
    }

    /// <summary>Kept for data compatibility; no observable effect (REQ-PERS-017).</summary>
    public bool MapEnable
    {
        get => Flag(SettingKeys.SceneMapEnable);
        set => Write(SettingKeys.SceneMapEnable, value);
    }

    #endregion

    #region clipping (§4.1.5) and layers (§4.1.6)

    /// <summary>The clip margin of Lua index <paramref name="index"/>: 0 left, 1 bottom, 2 right, 3 top.</summary>
    public float Clipping(int index) => Real(SettingKeys.Clipping[Bound(index, SettingKeys.ClippingCount)]);

    /// <summary>Writes the clip margin of Lua index <paramref name="index"/>.</summary>
    public void SetClipping(int index, float value)
        => Write(SettingKeys.Clipping[Bound(index, SettingKeys.ClippingCount)], value);

    /// <summary>The four margins as a vector, which <c>Clipping.target</c> exposes.</summary>
    public Vec4F ClippingVector => new(Clipping(0), Clipping(1), Clipping(2), Clipping(3));

    /// <summary>Writes the four margins at once.</summary>
    public void SetClippingVector(Vec4F value)
    {
        SetClipping(0, value.X);
        SetClipping(1, value.Y);
        SetClipping(2, value.Z);
        SetClipping(3, value.W);
    }

    /// <summary>Layer given to patterns that name none; <c>Layer.DefLayer</c>.</summary>
    public int DefaultLayer
    {
        get => Int(SettingKeys.LayerDefaultLayer);
        set => Write(SettingKeys.LayerDefaultLayer, value);
    }

    /// <summary>Kept for data compatibility; no effect; <c>Layer.Auto</c>.</summary>
    public bool AutoLayer
    {
        get => Flag(SettingKeys.LayerAuto);
        set => Write(SettingKeys.LayerAuto, value);
    }

    /// <summary>The five members of layer <paramref name="index"/>; <c>Layer[i]</c>.</summary>
    public LayerView Layer(int index) => new(this, Bound(index, SettingKeys.LayerCount));

    /// <summary>One layer as Lua sees it (§4.2, type <c>Calque</c>).</summary>
    /// <param name="Settings">The owning view.</param>
    /// <param name="Index">Layer index, 0 to 5.</param>
    public readonly record struct LayerView(ScriptSettings Settings, int Index)
    {
        /// <summary>Depth offset in scene units.</summary>
        public float Offset
        {
            get => Settings.Real(SettingKeys.LayerOffset(Index));
            set => Settings.Write(SettingKeys.LayerOffset(Index), value);
        }

        /// <summary>Depth alignment: 0 FRONT, 1 CENTER, 2 BACK.</summary>
        public int Aligned
        {
            get => Settings.Int(SettingKeys.LayerAligned(Index));
            set => Settings.Write(SettingKeys.LayerAligned(Index), value);
        }

        /// <summary>The derived alignment coefficient, read only (REQ-PERS-013 point 8).</summary>
        public float Coef => Settings.Real(SettingKeys.LayerCoef(Index));

        /// <summary>Whether the layer casts shadows.</summary>
        public bool CastShadow
        {
            get => Settings.Flag(SettingKeys.LayerCastShadow(Index));
            set => Settings.Write(SettingKeys.LayerCastShadow(Index), value);
        }

        /// <summary>Whether the layer receives shadows.</summary>
        public bool RecvShadow
        {
            get => Settings.Flag(SettingKeys.LayerRecvShadow(Index));
            set => Settings.Write(SettingKeys.LayerRecvShadow(Index), value);
        }
    }

    #endregion

    #region emulation (§4.1.7)

    /// <summary>Emulation speed index, bounded to [−8, 8]; back to 0 at the end of a game.</summary>
    public float GameSpeed
    {
        get => Real(SettingKeys.EmuGameSpeed);
        set => Write(SettingKeys.EmuGameSpeed, value);
    }

    /// <summary>TV region: 0 NTSC, 1 PAL, 2 MULTI, 3 Dendy.</summary>
    public int TVMode
    {
        get => Int(SettingKeys.EmuTvMode);
        set => Write(SettingKeys.EmuTvMode, value);
    }

    /// <summary>Whether the ROM header picks the region (REQ-LOOP-017).</summary>
    public bool AutoTVMode
    {
        get => Flag(SettingKeys.EmuAutoTvMode);
        set => Write(SettingKeys.EmuAutoTvMode, value);
    }

    /// <summary>Core index; always 0 with a single core (REQ-PERS-003 point 2).</summary>
    public int Core
    {
        get => Int(SettingKeys.EmuCore);
        set => Write(SettingKeys.EmuCore, value);
    }

    #endregion

    #region conversion (§4.1.8)

    /// <summary>Whether empty background tiles take part in the conversion (REQ-E3D-004).</summary>
    public bool EmptyTile
    {
        get => Flag(SettingKeys.ConvEmptyTile);
        set => Write(SettingKeys.ConvEmptyTile, value);
    }

    /// <summary>Background recognition ignores the palette attribute; alias <c>SamePalette</c>.</summary>
    public bool SameBgPalette
    {
        get => Flag(SettingKeys.ConvSameBgPalette);
        set => Write(SettingKeys.ConvSameBgPalette, value);
    }

    /// <summary>The same for sprites.</summary>
    public bool SameSpPalette
    {
        get => Flag(SettingKeys.ConvSameSpPalette);
        set => Write(SettingKeys.ConvSameSpPalette, value);
    }

    /// <summary>Sprite grouping tolerance in pixels; a negative value becomes its absolute value.</summary>
    public int SpriteTolerance
    {
        get => Int(SettingKeys.ConvSpriteTolerance);
        set => Write(SettingKeys.ConvSpriteTolerance, value);
    }

    /// <summary>Background scan direction: 0 by rows, 1 by columns.</summary>
    public int ScanDirection
    {
        get => Int(SettingKeys.ConvScanDirection);
        set => Write(SettingKeys.ConvScanDirection, value);
    }

    #endregion

    #region audio, inputs, interface (§4.1.9)

    /// <summary>Master volume, bounded to [0, 1].</summary>
    public float Volume
    {
        get => Real(SettingKeys.AudioVolume);
        set => Write(SettingKeys.AudioVolume, value);
    }

    /// <summary>Whether the menu music plays.</summary>
    public bool BGMusic
    {
        get => Flag(SettingKeys.AudioBgMusic);
        set => Write(SettingKeys.AudioBgMusic, value);
    }

    /// <summary>Kept for data compatibility; no effect (DEC-08-6, DEC-R2-D7).</summary>
    public bool EmulatedMenuButton
    {
        get => Flag(SettingKeys.InputEmulatedMenuButton);
        set => Write(SettingKeys.InputEmulatedMenuButton, value);
    }

    /// <summary>Library sort option: 0 A-Z, 1 recent.</summary>
    public int SortOption
    {
        get => Int(SettingKeys.UiSortOption);
        set => Write(SettingKeys.UiSortOption, value);
    }

    #endregion

    #region VR, editor, film, internals (§4.1.10)

    /// <summary>Always 0 on desktop; a write is ignored without error (REQ-PERS-011).</summary>
    public int VrRuntime
    {
        get => Int(SettingKeys.VrRuntime);
        set => Write(SettingKeys.VrRuntime, value);
    }

    /// <summary>
    /// Reads 1 while <see cref="VrRuntime"/> is 0. The written value is remembered, has no
    /// geometric effect, and a change triggers the view reset of REQ-SCN-002.
    /// </summary>
    public float SceneRatio
    {
        get => Real(SettingKeys.VrSceneRatio);
        set => Write(SettingKeys.VrSceneRatio, value);
    }

    /// <summary>The value last written to <see cref="SceneRatio"/>, for the host.</summary>
    public float StoredSceneRatio => _model.GetStored(SettingKeys.VrSceneRatio).AsFloat;

    /// <summary>Kept and restored, without effect.</summary>
    public bool Mirroring
    {
        get => Flag(SettingKeys.VrMirroring);
        set => Write(SettingKeys.VrMirroring, value);
    }

    /// <summary>Kept and restored, without effect.</summary>
    public int ControlMethod
    {
        get => Int(SettingKeys.VrControlMethod);
        set => Write(SettingKeys.VrControlMethod, value);
    }

    /// <summary>Editor selection mode; no effect (REQ-PERS-016).</summary>
    public int SelectionMode
    {
        get => Int(SettingKeys.EditorSelectionMode);
        set => Write(SettingKeys.EditorSelectionMode, value);
    }

    /// <summary>Film mode; frozen at 0 (REQ-PERS-016).</summary>
    public int MovieMode
    {
        get => Int(SettingKeys.MovieMode);
        set => Write(SettingKeys.MovieMode, value);
    }

    /// <summary>Scene root scale; 1 on desktop.</summary>
    public float RootScale
    {
        get => Real(SettingKeys.SceneRootScale);
        set => Write(SettingKeys.SceneRootScale, value);
    }

    /// <summary>Scene scale, <c>RootScale × SceneRatio</c>; 1 on desktop (REQ-PERS-013 point 3).</summary>
    public float SceneScale
    {
        get => Real(SettingKeys.SceneSceneScale);
        set => Write(SettingKeys.SceneSceneScale, value);
    }

    #endregion

    #region derived (REQ-PERS-013)

    /// <summary>Refresh rate of the display, which <see cref="TargetFramerate"/> may use.</summary>
    public float DisplayRefreshRate
    {
        get => _model.DisplayRefreshRate;
        set => _model.DisplayRefreshRate = value;
    }

    /// <summary>Target frame rate (REQ-PERS-013 point 1).</summary>
    public float TargetFramerate => Real(SettingKeys.VideoTargetFramerate);

    /// <summary>Target time per frame in seconds (REQ-PERS-013 point 2).</summary>
    public float TargetTimePerFrame => Real(SettingKeys.VideoTargetTimePerFrame);

    /// <summary>Whether a skybox is drawn, <c>Skybox &gt; 0</c> (REQ-PERS-013 point 4).</summary>
    public bool SkyboxEnable => Flag(SettingKeys.SceneSkyboxEnable);

    /// <summary>The resolution scale factor of <see cref="ResScaleIndex"/>.</summary>
    public float ResScale => Real(SettingKeys.VideoResScaleFactor);

    /// <summary>
    /// The engine mode of SPEC-09 §4.1.10: always <c>1</c> (PROFILE), the ROM mode being the
    /// editor's. AMD-020 (Q-052) makes the three members of the row required in Lua, read only.
    /// </summary>
    // Q-052
    public int EngineMode => Int(SettingKeys.AppEngineMode);

    /// <summary>Whether the engine runs on a profile, which it always does (Q-052).</summary>
    public bool IsProfileMode => EngineMode == ProfileEngineMode;

    /// <summary>Whether the engine runs on a bare ROM, which it never does (Q-052).</summary>
    public bool IsRomMode => EngineMode == RomEngineMode;

    /// <summary>Value of <c>app.engine_mode</c> that means ROM (§4.1.10).</summary>
    public const int RomEngineMode = 0;

    /// <summary>Value of <c>app.engine_mode</c> that means PROFILE (§4.1.10).</summary>
    public const int ProfileEngineMode = 1;

    #endregion

    /// <summary>The characteristic smoothing time of SPEC-09 §4.3.</summary>
    public const float DefaultSmoothTime = 0.4f;

    /// <summary>The VR scene scale of SPEC-09 §4.3.</summary>
    public const float VrScale = 0.005f;

    /// <summary>The desktop scene scale of SPEC-09 §4.3.</summary>
    public const float NonVrScale = 1f;

    /// <summary>The end-of-game resets of REQ-PERS-012, applied to the whole model.</summary>
    public void EndGame() => _model.EndOfGame();

    /// <summary>
    /// The port types, which <c>Setting:SetInputType</c> records in the model so that
    /// <c>Setting.ZapperEnable</c> and the model agree (SPEC-08 REQ-INP-060, REQ-PERS-013
    /// point 5). The input layer stays the authority on what the ports actually do.
    /// </summary>
    public void SetPortTypes(int first, int second)
    {
        _model.Set(SettingKeys.InputPortType(0), SettingValue.OfEnum(first), SettingLayer.Session);
        _model.Set(SettingKeys.InputPortType(1), SettingValue.OfEnum(second), SettingLayer.Session);
    }

    /// <summary>Pushes every mirrored value into the engine slice, which a game start needs.</summary>
    public void MirrorAll()
    {
        if (_conversion is null)
        {
            return;
        }

        foreach (string key in MirroredKeys())
        {
            Mirror(key);
        }
    }

    /// <summary>Stops listening to the model.</summary>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _model.Changed -= OnModelChanged;
    }

    /// <summary>A <see cref="LuaColor"/> from a model colour.</summary>
    internal static LuaColor ToLua(SettingColor colour) => new(colour.R, colour.G, colour.B, colour.A);

    /// <summary>A model colour from a <see cref="LuaColor"/>.</summary>
    internal static SettingColor FromLua(LuaColor colour) => new(colour.R, colour.G, colour.B, colour.A);

    private static int Bound(int index, int count) => Math.Clamp(index, 0, count - 1);

    private static IEnumerable<string> MirroredKeys()
    {
        yield return SettingKeys.ConvEmptyTile;
        yield return SettingKeys.ConvScanDirection;
        yield return SettingKeys.ConvSameBgPalette;
        yield return SettingKeys.ConvSameSpPalette;
        yield return SettingKeys.ConvSpriteTolerance;
        yield return SettingKeys.RenderMeshAlgo;
        yield return SettingKeys.RenderShadow;
        yield return SettingKeys.SceneUseNesBgColor;
        yield return SettingKeys.SceneCustomBgColor;
        yield return SettingKeys.LayerDefaultLayer;
        yield return SettingKeys.LayerAuto;

        foreach (string key in SettingKeys.Clipping)
        {
            yield return key;
        }

        for (int i = 0; i < SettingKeys.LayerCount; i++)
        {
            yield return SettingKeys.LayerOffset(i);
            yield return SettingKeys.LayerAligned(i);
            yield return SettingKeys.LayerCastShadow(i);
            yield return SettingKeys.LayerRecvShadow(i);
        }
    }

    private bool Flag(string key) => _model.GetBool(key);

    private int Int(string key) => _model.GetInt(key);

    private float Real(string key) => _model.GetFloat(key);

    private void Write(string key, bool value) => _model.SetBool(key, value, SettingLayer.Session);

    private void Write(string key, int value)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        _model.Set(
            key,
            descriptor.Kind == SettingKind.Enum ? SettingValue.OfEnum(value) : SettingValue.OfInt(value),
            SettingLayer.Session);
    }

    private void Write(string key, float value) => _model.SetFloat(key, value, SettingLayer.Session);

    private void OnModelChanged(SettingChange change)
    {
        Mirror(change.Key);
        Changed?.Invoke(change.Key);

        // A change of field of view or of the kept scene ratio resets the view (REQ-SCN-002,
        // REQ-SCN-012, REQ-PERS-011).
        if (change.Key is SettingKeys.CameraFov or SettingKeys.VrSceneRatio)
        {
            ViewReset?.Invoke();
        }
    }

    /// <summary>
    /// Copies one setting into the slice the conversion pipeline reads. This is the only place
    /// the engine's working copy is written, so a Lua write and a profile load reach it the same
    /// way.
    /// </summary>
    private void Mirror(string key)
    {
        if (_conversion is not { } conversion)
        {
            return;
        }

        switch (key)
        {
            case SettingKeys.ConvEmptyTile:
                conversion.EmptyTile = Flag(key);
                return;
            case SettingKeys.ConvScanDirection:
                conversion.ScanDirection = Int(key);
                return;
            case SettingKeys.ConvSameBgPalette:
                conversion.SameBackgroundPalette = Flag(key);
                return;
            case SettingKeys.ConvSameSpPalette:
                conversion.SameSpritePalette = Flag(key);
                return;
            case SettingKeys.ConvSpriteTolerance:
                conversion.SpriteTolerance = Int(key);
                return;
            case SettingKeys.RenderMeshAlgo:
                conversion.RenderAlgorithm = (RenderAlgorithm)Int(key);
                return;
            case SettingKeys.RenderShadow:
                conversion.Shadow = Flag(key);
                return;
            case SettingKeys.SceneUseNesBgColor:
                conversion.UseNesBackgroundColor = Flag(key);
                return;
            case SettingKeys.SceneCustomBgColor:
                conversion.CustomBackgroundColor = ToLua(_model.GetColor(key)).ToRgb24();
                return;
            case SettingKeys.LayerDefaultLayer:
                conversion.DefaultLayer = (ZLayer)Int(key);
                return;
            case SettingKeys.LayerAuto:
                conversion.AutoLayer = Flag(key);
                return;
            default:
                MirrorIndexed(conversion, key);
                return;
        }
    }

    private void MirrorIndexed(ConversionSettings conversion, string key)
    {
        for (int i = 0; i < SettingKeys.ClippingCount; i++)
        {
            if (!string.Equals(SettingKeys.Clipping[i], key, StringComparison.Ordinal))
            {
                continue;
            }

            conversion.Clipping = ClippingVector;
            return;
        }

        if (!key.StartsWith("layer[", StringComparison.Ordinal))
        {
            return;
        }

        int index = key[6] - '0';
        if ((uint)index >= SettingKeys.LayerCount)
        {
            return;
        }

        LayerSetting target = conversion.Layer(index);
        LayerView source = Layer(index);
        target.Offset = source.Offset;
        target.Alignment = (ZAlignment)source.Aligned;
        target.CastShadow = source.CastShadow;
        target.ReceiveShadow = source.RecvShadow;
    }
}

/// <summary>
/// The directional light backed by the settings model: its direction and colour are
/// profile-scoped and its intensity is a user preference (SPEC-09 §4.1.3, REQ-PERS-005 point 2).
/// </summary>
public sealed class ModelScriptLight(SettingsModel model) : ScriptLight
{
    private readonly SettingsModel _model = model ?? throw new ArgumentNullException(nameof(model));

    /// <inheritdoc/>
    public override Vec3 Direction
    {
        get
        {
            SettingVector3 value = _model.GetVector3(SettingKeys.LightDirection);
            return new Vec3(value.X, value.Y, value.Z);
        }

        set => _model.SetVector3(
            SettingKeys.LightDirection,
            new SettingVector3(value.X, value.Y, value.Z),
            SettingLayer.Session);
    }

    /// <inheritdoc/>
    public override LuaColor Color
    {
        get => ScriptSettings.ToLua(_model.GetColor(SettingKeys.LightColor));
        set => _model.SetColor(SettingKeys.LightColor, ScriptSettings.FromLua(value), SettingLayer.Session);
    }

    /// <inheritdoc/>
    public override float Intensity
    {
        get => _model.GetFloat(SettingKeys.LightIntensity);
        set => _model.SetFloat(SettingKeys.LightIntensity, value, SettingLayer.Session);
    }
}
