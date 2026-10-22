using Zax.Platform;
using Zax.Render;
using Zax.Settings;

namespace Zax.App.Persistence;

/// <summary>
/// The command line as an override of the persisted settings (SPEC-09 REQ-PERS-020, Q-066).
/// </summary>
/// <remarks>
/// <para>
/// A command-line option changes a setting for one run <b>only when it was really given</b>.
/// Before FIX-1 every one of these options carried the model's default and was copied into the
/// settings at start-up, so each launch quietly reset the field of view, the shadows, the
/// skybox, the TV box, the render skip, the frame rate and the vertical synchronisation the user
/// had chosen in the options window.
/// </para>
/// <para>
/// The override goes to <see cref="SettingLayer.CommandLine"/>: it masks the preference the file
/// holds without replacing it — so the file still carries the user's own choice at the next
/// launch — and a change made from the interface takes over and is persisted as usual.
/// </para>
/// </remarks>
internal static class CommandLineSettings
{
    /// <summary>
    /// The settings a command-line option can override, in the order of the usage text. The
    /// test of Q-066 walks this list, so an option added here cannot be forgotten.
    /// </summary>
    public static IReadOnlyList<string> OverridableKeys { get; } =
    [
        SettingKeys.CameraFov,
        SettingKeys.RenderShadow,
        SettingKeys.SceneSkybox,
        SettingKeys.SceneTvBox,
        SettingKeys.VideoSkipFrame,
        SettingKeys.VideoFrameRate,
        SettingKeys.VideoVSync,
    ];

    /// <summary>
    /// Puts the options <paramref name="options"/> really carries in force on
    /// <paramref name="settings"/>, for this run.
    /// </summary>
    public static void Apply(AppOptions options, AppSettings settings)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(settings);

        SettingsModel model = settings.Model;

        if (options.Fov is { } fov)
        {
            model.SetFloat(SettingKeys.CameraFov, fov, SettingLayer.CommandLine);
        }

        if (options.Shadows is { } shadows)
        {
            model.SetBool(SettingKeys.RenderShadow, shadows, SettingLayer.CommandLine);
        }

        if (options.Skybox is { } skybox)
        {
            model.Set(SettingKeys.SceneSkybox, SettingValue.OfEnum(skybox), SettingLayer.CommandLine);
        }

        if (options.TvBox is { } tvBox)
        {
            // AMD-025: a stored or requested TV 2 or TV 3 is the one model.
            model.Set(
                SettingKeys.SceneTvBox,
                SettingValue.OfEnum(TvBoxSetting.Normalise(tvBox)),
                SettingLayer.CommandLine);
        }

        if (options.SkipRender is { } skipRender)
        {
            model.SetBool(SettingKeys.VideoSkipFrame, skipRender, SettingLayer.CommandLine);
        }

        if (options.FrameRate is { } frameRate)
        {
            model.SetInt(SettingKeys.VideoFrameRate, frameRate, SettingLayer.CommandLine);
        }

        if (options.VSync is { } vsync)
        {
            model.Set(
                SettingKeys.VideoVSync,
                SettingValue.OfEnum(Math.Clamp(vsync, 0, DisplayPacing.MaximumVSync)),
                SettingLayer.CommandLine);
        }
    }
}
