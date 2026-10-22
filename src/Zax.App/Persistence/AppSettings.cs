using Zax.Nes;
using Zax.Render;
using Zax.Settings;

namespace Zax.App.Persistence;

/// <summary>
/// The typed view the host uses onto the settings model of SPEC-09 (REQ-PERS-001).
/// </summary>
/// <remarks>
/// <para>
/// There are no settings of its own here any more: every property reads and writes
/// <see cref="Model"/>, the one <see cref="SettingsModel"/> of the emulator instance, under the
/// stable keys of §4.1. The loop, the audio and the renderer keep their readable call sites
/// (<c>Settings.Volume</c>, <c>Settings.Region</c>) while the values, the domains, the layering
/// and the notifications all live in the model.
/// </para>
/// <para>
/// A write made here is a change by the <b>user</b> — the menus, the options window or a
/// libretro core option — so it goes to the preference layer, not to the session layer the
/// scripts write to (REQ-PERS-010).
/// </para>
/// </remarks>
public sealed class AppSettings
{
    /// <summary>Creates a view over a model of its own.</summary>
    public AppSettings()
        : this(new SettingsModel())
    {
    }

    /// <summary>Creates a view over <paramref name="model"/>.</summary>
    public AppSettings(SettingsModel model)
    {
        Model = model ?? throw new ArgumentNullException(nameof(model));
        Model.Changed += change => Changed?.Invoke(change.Key);
    }

    /// <summary>Default of <c>emu.game_speed</c>.</summary>
    public const int SpeedSettingDefault = 0;

    /// <summary>The three render-rate caps <c>video.frame_rate</c> accepts (REQ-PERS-003 rule 1).</summary>
    public static ReadOnlySpan<int> FrameRateDomain => [60, 90, 120];

    /// <summary>The settings model this view reads and writes.</summary>
    public SettingsModel Model { get; }

    /// <summary>Raised with the stable key of a setting that really changed (REQ-PERS-009).</summary>
    public event Action<string>? Changed;

    /// <summary><c>emu.game_speed</c>: integer −8…+8, default 0 (REQ-LOOP-015).</summary>
    public int GameSpeed
    {
        get => Model.GetInt(SettingKeys.EmuGameSpeed);
        set => Model.SetFloat(SettingKeys.EmuGameSpeed, value);
    }

    /// <summary><c>emu.tv_mode</c>: the active region, profile-scoped (DEC-09-Q1).</summary>
    public TvRegionSetting TvMode
    {
        get => (TvRegionSetting)Model.GetInt(SettingKeys.EmuTvMode);
        set => Model.Set(SettingKeys.EmuTvMode, SettingValue.OfEnum((int)value));
    }

    /// <summary><c>emu.auto_tv_mode</c>: take the region from the ROM header (REQ-LOOP-017).</summary>
    public bool AutoTvMode
    {
        get => Model.GetBool(SettingKeys.EmuAutoTvMode);
        set => Model.SetBool(SettingKeys.EmuAutoTvMode, value);
    }

    /// <summary><c>audio.volume</c>: <c>V</c> in [0 ; 1], default 1 (REQ-AUD-006).</summary>
    public float Volume
    {
        get => Model.GetFloat(SettingKeys.AudioVolume);
        set => Model.SetFloat(SettingKeys.AudioVolume, value);
    }

    /// <summary><c>audio.bg_music</c>: <c>M</c>, default true (REQ-AUD-021).</summary>
    public bool MenuMusic
    {
        get => Model.GetBool(SettingKeys.AudioBgMusic);
        set => Model.SetBool(SettingKeys.AudioBgMusic, value);
    }

    /// <summary>
    /// The mute toggle AMD-010 REQ-UI-200 adds to the Audio menu. SPEC-11 REQ-AUD-006 says
    /// there is no separate mute setting in the original, so this is a session-scoped switch
    /// that forces the effective gain to 0 and leaves <see cref="Volume"/> alone.
    /// </summary>
    public bool Mute
    {
        get => Model.GetBool(SettingKeys.AudioMute);
        set => Model.SetBool(SettingKeys.AudioMute, value);
    }

    /// <summary>The gain the mixer applies: 0 when muted, else <see cref="Volume"/>.</summary>
    public float EffectiveVolume => Mute ? 0f : Volume;

    /// <summary><c>video.skip_frame</c>: render skip, default true (REQ-LOOP-014).</summary>
    public bool SkipFrame
    {
        get => Model.GetBool(SettingKeys.VideoSkipFrame);
        set => Model.SetBool(SettingKeys.VideoSkipFrame, value);
    }

    /// <summary><c>video.frame_rate</c>: 60, 90 or 120; anything else becomes 90.</summary>
    public int FrameRate
    {
        get => Model.GetInt(SettingKeys.VideoFrameRate);
        set => Model.SetInt(SettingKeys.VideoFrameRate, value);
    }

    /// <summary><c>video.vsync</c>: 0 off, 1 every refresh, 2 every other refresh.</summary>
    public int VSync
    {
        get => Model.GetInt(SettingKeys.VideoVSync);
        set => Model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(Math.Clamp(value, 0, 2)));
    }

    /// <summary><c>render.shadow</c>: the global shadow switch, default true.</summary>
    public bool Shadow
    {
        get => Model.GetBool(SettingKeys.RenderShadow);
        set => Model.SetBool(SettingKeys.RenderShadow, value);
    }

    /// <summary><c>scene.skybox</c>: −1 Default, 0 Color, 1 Gradient, 2 Fantasy.</summary>
    public int Skybox
    {
        get => Model.GetInt(SettingKeys.SceneSkybox);
        set => Model.Set(SettingKeys.SceneSkybox, SettingValue.OfEnum(value));
    }

    /// <summary>
    /// <c>scene.tv_box</c>: 0 None, 1 the television. AMD-025 brings a stored 2 or 3 — the
    /// TV 2 and TV 3 of REQ-SCN-054 — back to 1, on the way in as on the way out, so a
    /// settings file written before the amendment keeps working.
    /// </summary>
    public int TvBox
    {
        get => TvBoxSetting.Normalise(Model.GetInt(SettingKeys.SceneTvBox));
        set => Model.Set(SettingKeys.SceneTvBox, SettingValue.OfEnum(TvBoxSetting.Normalise(value)));
    }

    /// <summary><c>camera.fov</c>: vertical field of view in degrees, default 70.</summary>
    public float Fov
    {
        get => Model.GetFloat(SettingKeys.CameraFov);
        set => Model.SetFloat(SettingKeys.CameraFov, value);
    }

    /// <summary><c>light.intensity</c>: user preference kept across profile loads.</summary>
    public float LightIntensity
    {
        get => Model.GetFloat(SettingKeys.LightIntensity);
        set => Model.SetFloat(SettingKeys.LightIntensity, value);
    }

    /// <summary><c>ui.fullscreen</c> (REQ-PERS-067).</summary>
    public bool Fullscreen
    {
        get => Model.GetBool(SettingKeys.UiFullscreen);
        set => Model.SetBool(SettingKeys.UiFullscreen, value);
    }

    /// <summary>The FPS counter option of DEC-10-Q7, off by default.</summary>
    public bool ShowFps
    {
        get => Model.GetBool(SettingKeys.UiShowFps);
        set => Model.SetBool(SettingKeys.UiShowFps, value);
    }

    /// <summary><c>ui.sort_option</c>: 0 A-Z, 1 Recent (REQ-PERS-033).</summary>
    public int SortOption
    {
        get => Model.GetInt(SettingKeys.UiSortOption);
        set => Model.Set(SettingKeys.UiSortOption, SettingValue.OfEnum(Math.Clamp(value, 0, 1)));
    }

    /// <summary><c>ui.last_browse_dir</c> (REQ-PERS-066).</summary>
    public string LastBrowseDirectory
    {
        get => Model.GetString(SettingKeys.UiLastBrowseDir);
        set => Model.SetString(SettingKeys.UiLastBrowseDir, value);
    }

    /// <summary>The speed factor <c>k</c> of the current setting.</summary>
    public double SpeedFactor => Loop.SpeedSetting.Factor(GameSpeed);

    /// <summary>The core region the current <see cref="TvMode"/> designates.</summary>
    public NesRegion Region => TvMode switch
    {
        TvRegionSetting.Pal => NesRegion.Pal,
        TvRegionSetting.Dendy => NesRegion.Dendy,

        // MULTI is not offered by the UI; SPEC-02 treats it as NTSC timing.
        _ => NesRegion.Ntsc,
    };

    /// <summary>
    /// The automatic resets of SPEC-09 REQ-PERS-012 at the end of a game: the speed setting goes
    /// back to 0, the camera smoothing to 0.4 and every value a script put on a preference is
    /// dropped (REQ-PERS-010).
    /// </summary>
    public void EndOfGameResets() => Model.EndOfGame();

    /// <summary>Applies the region the core detected in a ROM header (REQ-LOOP-017).</summary>
    public void ApplyDetectedRegion(NesRegion region) => TvMode = region switch
    {
        NesRegion.Pal => TvRegionSetting.Pal,
        NesRegion.Dendy => TvRegionSetting.Dendy,
        _ => TvRegionSetting.Ntsc,
    };

    /// <summary>Brings <c>video.frame_rate</c> into its domain (REQ-PERS-003 rule 1).</summary>
    public static int ValidateFrameRate(int value)
        => value is 60 or 90 or 120 ? value : 90;
}

/// <summary>The <c>emu.tv_mode</c> domain of SPEC-09 §4.1.7.</summary>
public enum TvRegionSetting
{
    /// <summary>NTSC, 60 emulated frames per second.</summary>
    Ntsc = 0,

    /// <summary>PAL, 50 emulated frames per second.</summary>
    Pal = 1,

    /// <summary>Multi-region; not offered by the UI.</summary>
    Multi = 2,

    /// <summary>Dendy, 50 emulated frames per second.</summary>
    Dendy = 3,
}
