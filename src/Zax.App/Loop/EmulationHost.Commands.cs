using Zax.App.Audio;
using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Render;

namespace Zax.App.Loop;

/// <summary>
/// Registration of the commands of AMD-010 REQ-UI-200 in the model of REQ-UI-205, with the
/// default shortcuts of REQ-UI-204.
/// </summary>
/// <remarks>
/// Every entry of the menu bar has its command here. The four that only open a window — the
/// options, the key configuration, the data folder and the about block — plus full screen raise
/// <see cref="EmulationHost.UiRequested"/>, because what they open belongs to the UI lot; their
/// name, availability and invocation are already part of the model, which is what REQ-UI-205
/// requires.
/// </remarks>
internal sealed partial class EmulationHost
{
    /// <summary>
    /// Reads the label of <paramref name="command"/> in the interface language (AMD-035): the
    /// commands are registered once at start-up and the language can change at any time, so the
    /// label is a lookup, not a stored text.
    /// </summary>
    private static Func<string> LabelOf(string command) => () => UiStrings.Get(UiKeys.Command(command));

    /// <summary>Reads a shortcut text whose wording is translated (AMD-035).</summary>
    private static Func<string> TextOf(string key) => () => UiStrings.Get(key);

    private void RegisterCommands()
    {
        RegisterFileCommands();
        RegisterStateCommands();
        RegisterEmulationCommands();
        RegisterVideoCommands();
        RegisterAudioCommands();
        RegisterToolCommands();
    }

    private void RegisterFileCommands()
    {
        Commands.Register(
            ZaxCommands.OpenRom,
            LabelOf(ZaxCommands.OpenRom),
            OpenRomOrAskForOne,
            static () => "Ctrl+O",
            _ => !IsLoading);

        Commands.Register(
            ZaxCommands.LaunchGame,
            LabelOf(ZaxCommands.LaunchGame),
            args => LaunchGame(args.Text ?? string.Empty),
            canExecute: args => !IsLoading && !string.IsNullOrEmpty(args.Text));

        Commands.Register(
            ZaxCommands.OpenRecentRom,
            LabelOf(ZaxCommands.OpenRecentRom),
            args => UiRequested?.Invoke(ZaxCommands.OpenRecentRom, args));

        Commands.Register(
            ZaxCommands.ChooseProfile,
            LabelOf(ZaxCommands.ChooseProfile),
            args => UiRequested?.Invoke(ZaxCommands.ChooseProfile, args),
            canExecute: _ => !IsLoading);

        Commands.Register(
            ZaxCommands.ImportProfile,
            LabelOf(ZaxCommands.ImportProfile),
            args => UiRequested?.Invoke(ZaxCommands.ImportProfile, args),
            canExecute: _ => !IsLoading);

        // AMD-030 §2: « Fichier → Importer depuis 3dSen… ». The folder picker and the import
        // itself belong to the interface, which owns the library.
        Commands.Register(
            ZaxCommands.ImportThreeDSen,
            LabelOf(ZaxCommands.ImportThreeDSen),
            args => UiRequested?.Invoke(ZaxCommands.ImportThreeDSen, args),
            canExecute: _ => !IsLoading);

        // SPEC-09 REQ-PERS-042 and SPEC-10 REQ-UI-037, which AMD-010 §3 keeps mandatory: the
        // deletion of a user game, behind the confirmation of REQ-UI-110.
        Commands.Register(
            ZaxCommands.DeleteGame,
            LabelOf(ZaxCommands.DeleteGame),
            args => RequestDeleteGame(args.Text ?? CurrentGameId ?? string.Empty),
            canExecute: args => !IsLoading
                && _games.CanDelete(args.Text ?? CurrentGameId ?? string.Empty));

        // REQ-PERS-036, P3 after AMD-020 (Q-050): the two sources a classic menu can offer.
        Commands.Register(
            ZaxCommands.CreateGameArt,
            LabelOf(ZaxCommands.CreateGameArt),
            _ => CreateGameArt(),
            canExecute: _ => !IsLoading && _currentGame is not null);

        Commands.Register(
            ZaxCommands.ImportCover,
            LabelOf(ZaxCommands.ImportCover),
            args =>
            {
                if (string.IsNullOrEmpty(args.Text))
                {
                    UiRequested?.Invoke(ZaxCommands.ImportCover, args);
                    return;
                }

                ImportCover(args.Text);
            },
            canExecute: _ => !IsLoading && _currentGame is not null);

        Commands.Register(
            ZaxCommands.Quit,
            LabelOf(ZaxCommands.Quit),
            _ => QuitRequested = true,
            static () => "Ctrl+Q");
    }

    private void RegisterStateCommands()
    {
        Commands.Register(
            ZaxCommands.SaveSlot,
            LabelOf(ZaxCommands.SaveSlot),
            args => SaveToSlot(SaveSlotIndex.NumberedSlotId(args.Number)),
            TextOf(UiKeys.ShortcutSaveSlots),
            args => CanSave(args.Number));

        Commands.Register(
            ZaxCommands.LoadSlot,
            LabelOf(ZaxCommands.LoadSlot),
            args => LoadSlot(SaveSlotIndex.NumberedSlotId(args.Number)),
            TextOf(UiKeys.ShortcutLoadSlots),
            args => !IsLoading
                && SaveSlotIndex.IsValidSlotId(SaveSlotIndex.NumberedSlotId(args.Number))
                && Slots.Find(SaveSlotIndex.NumberedSlotId(args.Number)) is { IsOccupied: true });

        Commands.Register(
            ZaxCommands.QuickSave,
            LabelOf(ZaxCommands.QuickSave),
            _ => QuickSave(),
            TextOf(UiKeys.ShortcutQuickSave),
            _ => _currentGame is not null && State != EmulationState.Stopped && !IsLoading);

        Commands.Register(
            ZaxCommands.QuickLoad,
            LabelOf(ZaxCommands.QuickLoad),
            _ => QuickLoad(),
            TextOf(UiKeys.ShortcutQuickLoad),
            _ => !IsLoading && Slots.Find(SaveSlotIndex.QuickSlotId) is { IsOccupied: true });

        // REQ-UI-052: the deletion asks for the confirmation of REQ-UI-110 first. A text
        // argument names the slot directly, which is how the quick slot is reached.
        Commands.Register(
            ZaxCommands.DeleteSlot,
            LabelOf(ZaxCommands.DeleteSlot),
            args => RequestDeleteSlot(args.Text ?? SaveSlotIndex.NumberedSlotId(args.Number)),
            canExecute: args => !IsLoading
                && Slots.Find(args.Text ?? SaveSlotIndex.NumberedSlotId(args.Number)) is { IsOccupied: true });
    }

    private void RegisterEmulationCommands()
    {
        Commands.Register(
            ZaxCommands.PauseResume,
            LabelOf(ZaxCommands.PauseResume),
            _ => TogglePause(),
            static () => "P",
            _ => State != EmulationState.Stopped && !IsLoading,
            () => State == EmulationState.Paused);

        Commands.Register(
            ZaxCommands.Pause,
            LabelOf(ZaxCommands.Pause),
            _ => Pause(),
            canExecute: _ => State == EmulationState.Running && !IsLoading);

        Commands.Register(
            ZaxCommands.Resume,
            LabelOf(ZaxCommands.Resume),
            _ => Resume(),
            canExecute: _ => State == EmulationState.Paused && !IsLoading);

        Commands.Register(
            ZaxCommands.Reset,
            LabelOf(ZaxCommands.Reset),
            _ => ResetGame(),
            static () => "Ctrl+R",
            _ => _currentGame is not null && !IsLoading);

        Commands.Register(
            ZaxCommands.Stop,
            LabelOf(ZaxCommands.Stop),
            _ => StopCurrentGame(clearCurrentGame: true),
            canExecute: _ => State != EmulationState.Stopped && !IsLoading);

        Commands.Register(
            ZaxCommands.FastForward,
            LabelOf(ZaxCommands.FastForward),
            _ => RequestFastForward(),
            TextOf(UiKeys.ShortcutFastForward),
            _ => State == EmulationState.Running,
            () => FastForward.IsActive);

        Commands.Register(
            ZaxCommands.Rollback,
            LabelOf(ZaxCommands.Rollback),
            _ => PerformRollback(),
            TextOf(UiKeys.ShortcutRollback),
            _ => State != EmulationState.Stopped && !IsLoading);

        Commands.Register(
            ZaxCommands.SetSpeed,
            LabelOf(ZaxCommands.SetSpeed),
            args => Settings.GameSpeed = args.Number);

        Commands.Register(
            ZaxCommands.SpeedUp,
            LabelOf(ZaxCommands.SpeedUp),
            _ => Settings.GameSpeed = SpeedSetting.Clamp(Settings.GameSpeed + 1),
            canExecute: _ => Settings.GameSpeed < SpeedSetting.Maximum);

        Commands.Register(
            ZaxCommands.SpeedDown,
            LabelOf(ZaxCommands.SpeedDown),
            _ => Settings.GameSpeed = SpeedSetting.Clamp(Settings.GameSpeed - 1),
            canExecute: _ => Settings.GameSpeed > SpeedSetting.Minimum);

        Commands.Register(
            ZaxCommands.SetRegion,
            LabelOf(ZaxCommands.SetRegion),
            args => Settings.TvMode = (TvRegionSetting)args.Number);

        Commands.Register(
            ZaxCommands.ToggleAutoRegion,
            LabelOf(ZaxCommands.ToggleAutoRegion),
            _ => Settings.AutoTvMode = !Settings.AutoTvMode,
            isChecked: () => Settings.AutoTvMode);

        Commands.Register(
            ZaxCommands.SetPortType,
            LabelOf(ZaxCommands.SetPortType),
            args => _session?.Core.SetPortType(args.Number, (ControllerPortType)args.Number2),
            canExecute: args => _session is not null && args.Number is 0 or 1);
    }

    private void RegisterVideoCommands()
    {
        // AMD-017 (Q-037): a hot toggle of the rendering. No relaunch, no state lost — so the
        // loop owns it outright instead of asking the interface to start the ROM on another
        // library entry, which is what it used to do.
        Commands.Register(
            ZaxCommands.ToggleTwoDimensional,
            LabelOf(ZaxCommands.ToggleTwoDimensional),
            _ => TwoDimensionalView = !TwoDimensionalView,
            static () => "F11",
            _ => _currentGame is not null && !IsLoading,
            () => ShowingTwoDimensional);

        Commands.Register(
            ZaxCommands.ResetView,
            LabelOf(ZaxCommands.ResetView),
            _ => _scene.Camera.ResetView(),
            TextOf(UiKeys.ShortcutLongPressMenu));

        // SPEC-06 REQ-SCN-006: the two camera modes, with their own setpoints. The menu entry
        // "Mode caméra" of REQ-UI-200 switches between them; REQ-SCN-009's local camera is the
        // third item of the same submenu.
        Commands.Register(
            ZaxCommands.SetCameraMode,
            LabelOf(ZaxCommands.SetCameraMode),
            args => _scene.Camera.SetMode(args.Number == 1 ? CameraMode.Fpv : CameraMode.Tpv),
            canExecute: _ => !IsLoading);

        Commands.Register(
            ZaxCommands.ToggleLocalCamera,
            LabelOf(ZaxCommands.ToggleLocalCamera),
            _ => Settings.Model.SetBool(
                Zax.Settings.SettingKeys.CameraLocalCam,
                !Settings.Model.GetBool(Zax.Settings.SettingKeys.CameraLocalCam)),
            isChecked: () => Settings.Model.GetBool(Zax.Settings.SettingKeys.CameraLocalCam));

        Commands.Register(
            ZaxCommands.SetSkybox,
            LabelOf(ZaxCommands.SetSkybox),
            args => Settings.Skybox = args.Number);

        Commands.Register(
            ZaxCommands.ToggleShadows,
            LabelOf(ZaxCommands.ToggleShadows),
            _ => Settings.Shadow = !Settings.Shadow,
            isChecked: () => Settings.Shadow);

        Commands.Register(
            ZaxCommands.ToggleFullscreen,
            LabelOf(ZaxCommands.ToggleFullscreen),
            _ =>
            {
                Settings.Fullscreen = !Settings.Fullscreen;
                UiRequested?.Invoke(ZaxCommands.ToggleFullscreen, default);
            },
            TextOf(UiKeys.ShortcutAltEnter),
            isChecked: () => Settings.Fullscreen);

        Commands.Register(
            ZaxCommands.ToggleFpsCounter,
            LabelOf(ZaxCommands.ToggleFpsCounter),
            _ => Settings.ShowFps = !Settings.ShowFps,
            isChecked: () => Settings.ShowFps);
    }

    private void RegisterAudioCommands()
    {
        Commands.Register(
            ZaxCommands.SetVolume,
            LabelOf(ZaxCommands.SetVolume),
            args => Settings.Volume = Math.Clamp(args.Number, 0, 100) / 100f);

        Commands.Register(
            ZaxCommands.VolumeUp,
            LabelOf(ZaxCommands.VolumeUp),
            _ => Settings.Volume = MathF.Min(1f, Settings.Volume + 0.05f));

        Commands.Register(
            ZaxCommands.VolumeDown,
            LabelOf(ZaxCommands.VolumeDown),
            _ => Settings.Volume = MathF.Max(0f, Settings.Volume - 0.05f));

        Commands.Register(
            ZaxCommands.ToggleMute,
            LabelOf(ZaxCommands.ToggleMute),
            _ => Settings.Mute = !Settings.Mute,
            isChecked: () => Settings.Mute);

        Commands.Register(
            ZaxCommands.ToggleMenuMusic,
            LabelOf(ZaxCommands.ToggleMenuMusic),
            _ => Settings.MenuMusic = !Settings.MenuMusic,
            isChecked: () => Settings.MenuMusic);
    }

    private void RegisterToolCommands()
    {
        Commands.Register(
            ZaxCommands.ShowOptions,
            LabelOf(ZaxCommands.ShowOptions),
            args => UiRequested?.Invoke(ZaxCommands.ShowOptions, args),
            static () => "Ctrl+,");

        Commands.Register(
            ZaxCommands.ShowKeyConfig,
            LabelOf(ZaxCommands.ShowKeyConfig),
            args => UiRequested?.Invoke(ZaxCommands.ShowKeyConfig, args));

        Commands.Register(
            ZaxCommands.ShowDataFolder,
            LabelOf(ZaxCommands.ShowDataFolder),
            args => UiRequested?.Invoke(ZaxCommands.ShowDataFolder, args));

        Commands.Register(
            ZaxCommands.ShowAbout,
            LabelOf(ZaxCommands.ShowAbout),
            args => UiRequested?.Invoke(ZaxCommands.ShowAbout, args));

        // REQ-UI-004 cases 2 and 3: what a short press on Menu becomes while a modal window is
        // up. The loop knows only that one is open (ModalWindowOpen); which window it is, and
        // whether Escape closes it, belongs to the frontend.
        Commands.Register(
            ZaxCommands.CancelModal,
            LabelOf(ZaxCommands.CancelModal),
            args => UiRequested?.Invoke(ZaxCommands.CancelModal, args),
            canExecute: _ => ModalWindowOpen);
    }

    /// <summary>
    /// "Ouvrir une ROM…" (AMD-010 REQ-UI-201). With a path it opens that file; without one it
    /// asks the interface for a file dialog, so the single command backs both the menu entry and
    /// the programmatic open the tests and the command line use.
    /// </summary>
    private void OpenRomOrAskForOne(CommandArgs args)
    {
        if (string.IsNullOrEmpty(args.Text))
        {
            UiRequested?.Invoke(ZaxCommands.OpenRom, args);
            return;
        }

        OpenRom(args.Text);
    }

    private bool CanSave(int slotNumber)
        => !IsLoading
            && State != EmulationState.Stopped
            && _currentGame is not null
            && SaveSlotIndex.IsValidSlotId(SaveSlotIndex.NumberedSlotId(slotNumber));

    /// <summary>
    /// The Menu short press of REQ-LOOP-005: it toggles between Running and Paused and is
    /// ignored while the loading indicator is up.
    /// </summary>
    private void TogglePause()
    {
        switch (State)
        {
            case EmulationState.Running:
                Pause();
                break;

            case EmulationState.Paused:
                Resume();
                break;

            default:
                // REQ-LOOP-004: Pause outside Running and Play outside Paused do nothing.
                break;
        }
    }

    private void Pause()
    {
        if (State != EmulationState.Running || IsLoading)
        {
            return;
        }

        SetState(EmulationState.Paused);
    }

    private void Resume()
    {
        if (State != EmulationState.Paused || IsLoading)
        {
            return;
        }

        SetState(EmulationState.Running);

        // REQ-LOOP-013: no catch-up of the time spent paused.
        Pacer.Reset();
    }

    /// <summary>
    /// The reset of REQ-LOOP-050: a full relaunch of the current game — the ROM is read again,
    /// the core cold-starts, End then Start run, the history is emptied — without reloading the
    /// 3D profile.
    /// </summary>
    private void ResetGame()
    {
        if (_currentGame is { } game && !IsLoading)
        {
            LaunchGame(game.Id);
        }
    }

    private void RequestFastForward()
    {
        if (State != EmulationState.Running)
        {
            return;
        }

        FastForward.Request();
        ApplySpeed();

        // REQ-LOOP-016: the notification is refreshed for as long as the key is held.
        Notifications.Show("Fast Forward", NotificationCenter.ShortSeconds);
    }

    /// <summary>
    /// Starts a mesh regeneration after an algorithm change (REQ-LOOP-024). The setting itself
    /// belongs to the persistence model; this is the loop's side of it.
    /// </summary>
    public void ChangeMeshAlgorithm(RenderAlgorithm algorithm) => RegenerateMeshes(algorithm);

    /// <summary>
    /// The explicit suspension of REQ-LOOP-053: losing the window focus must <b>not</b> pause,
    /// but a platform suspend does, and the resume goes back to Running only if it was Running.
    /// </summary>
    public void Suspend()
    {
        if (State == EmulationState.Running)
        {
            _wasRunningBeforeSuspend = true;
            Pause();
        }
    }

    /// <summary>Resumes after a <see cref="Suspend"/>.</summary>
    public void ResumeFromSuspend()
    {
        if (_wasRunningBeforeSuspend)
        {
            _wasRunningBeforeSuspend = false;
            Resume();
        }
    }

    private bool _wasRunningBeforeSuspend;
}
