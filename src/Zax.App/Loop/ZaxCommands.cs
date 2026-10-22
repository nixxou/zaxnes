namespace Zax.App.Loop;

/// <summary>
/// The stable names of the UI-independent commands of AMD-010 REQ-UI-205.
/// </summary>
/// <remarks>
/// <para>
/// These names are a contract: the menu bar, the keyboard shortcuts, the gamepad bindings and
/// the round-1 tests all drive the emulation through them (REQ-UI-200, REQ-UI-203,
/// REQ-UI-205). They are never renamed and never reused for another meaning, the same rule
/// SPEC-09 REQ-PERS-080 sets for setting keys.
/// </para>
/// <para>
/// The commands this lot implements are the File, Emulation and state-slot ones plus the
/// audio and video toggles that have a model behind them. The commands that only open a window
/// (Options, key configuration, About, the data folder) are registered by the shell and raise a
/// request the UI lot answers.
/// </para>
/// </remarks>
public static class ZaxCommands
{
    // ------------------------------------------------------------------ File

    /// <summary>Open a ROM; argument: its path in <see cref="CommandArgs.Text"/>.</summary>
    public const string OpenRom = "file.open_rom";

    /// <summary>Open a recent ROM; argument: its index in the recent list, 0-based.</summary>
    public const string OpenRecentRom = "file.open_recent_rom";

    /// <summary>Pick the 3D profile of the current game again (AMD-010 REQ-UI-201).</summary>
    public const string ChooseProfile = "file.choose_profile";

    /// <summary>Import a user profile; argument: the <c>.3dn</c> path.</summary>
    public const string ImportProfile = "file.import_profile";

    /// <summary>Launch a library entry; argument: its profile id in <see cref="CommandArgs.Text"/>.</summary>
    public const string LaunchGame = "file.launch_game";

    /// <summary>
    /// Delete a user game with its save states (SPEC-09 REQ-PERS-042, SPEC-10 REQ-UI-037);
    /// argument: its profile id in <see cref="CommandArgs.Text"/>, the current game by default.
    /// </summary>
    public const string DeleteGame = "file.delete_game";

    /// <summary>Cover art from the pause capture (« Create game art », REQ-PERS-036).</summary>
    public const string CreateGameArt = "file.create_game_art";

    /// <summary>Cover art from an image file; argument: its path (REQ-PERS-036).</summary>
    public const string ImportCover = "file.import_cover";

    /// <summary>Quit without confirmation and without an automatic save (REQ-LOOP-052).</summary>
    public const string Quit = "app.quit";

    // ------------------------------------------------------------- Save states

    /// <summary>Write the state of slot <c>n</c>; argument: <c>n</c> in <see cref="CommandArgs.Number"/>.</summary>
    public const string SaveSlot = "state.save_slot";

    /// <summary>Read the state of slot <c>n</c>.</summary>
    public const string LoadSlot = "state.load_slot";

    /// <summary>Write the quick slot (REQ-LOOP-043).</summary>
    public const string QuickSave = "state.quick_save";

    /// <summary>Read the quick slot (REQ-LOOP-045).</summary>
    public const string QuickLoad = "state.quick_load";

    /// <summary>Delete slot <c>n</c> (SPEC-09 REQ-PERS-054).</summary>
    public const string DeleteSlot = "state.delete_slot";

    // ------------------------------------------------------------- Emulation

    /// <summary>Toggle between Running and Paused (REQ-LOOP-005).</summary>
    public const string PauseResume = "emulation.pause_resume";

    /// <summary>Pause; no effect outside Running (REQ-LOOP-004).</summary>
    public const string Pause = "emulation.pause";

    /// <summary>Resume; no effect outside Paused.</summary>
    public const string Resume = "emulation.resume";

    /// <summary>Full relaunch of the current game, a cold start (REQ-LOOP-050).</summary>
    public const string Reset = "emulation.reset";

    /// <summary>Stop the current game and go back to Stopped (REQ-LOOP-051).</summary>
    public const string Stop = "emulation.stop";

    /// <summary>One fast-forward request: arms 30 emulated frames at ×1.5 (REQ-LOOP-016).</summary>
    public const string FastForward = "emulation.fast_forward";

    /// <summary>One rollback (REQ-LOOP-032).</summary>
    public const string Rollback = "emulation.rollback";

    /// <summary>Set the speed setting; argument: <c>s</c> in <see cref="CommandArgs.Number"/>.</summary>
    public const string SetSpeed = "emulation.set_speed";

    /// <summary>Speed setting + 1, bounded at +8.</summary>
    public const string SpeedUp = "emulation.speed_up";

    /// <summary>Speed setting − 1, bounded at −8.</summary>
    public const string SpeedDown = "emulation.speed_down";

    /// <summary>Set the region; argument: 0 NTSC, 1 PAL, 3 Dendy (REQ-LOOP-017).</summary>
    public const string SetRegion = "emulation.set_region";

    /// <summary>Toggle automatic region detection from the ROM header.</summary>
    public const string ToggleAutoRegion = "emulation.auto_region";

    /// <summary>Set a port type; arguments: port in <c>Number</c>, type in <c>Number2</c>.</summary>
    public const string SetPortType = "emulation.set_port_type";

    // ----------------------------------------------------------------- Video

    /// <summary>Toggle the 2D / 3D view (AMD-010 REQ-UI-200).</summary>
    public const string ToggleTwoDimensional = "video.toggle_2d";

    /// <summary>Reset the camera view (SPEC-06 REQ-SCN-013).</summary>
    public const string ResetView = "video.reset_view";

    /// <summary>Set the skybox; argument: −1 Default, 0 Color, 1 Gradient, 2 Fantasy.</summary>
    public const string SetSkybox = "video.set_skybox";

    /// <summary>Set the decorative TV box; argument: 0 to 3.</summary>
    public const string SetTvBox = "video.set_tv_box";

    /// <summary>Set the camera mode; argument: 0 TPV, 1 FPV (SPEC-06 REQ-SCN-006).</summary>
    public const string SetCameraMode = "video.set_camera_mode";

    /// <summary>Toggle the local — top-down — camera of REQ-SCN-009.</summary>
    public const string ToggleLocalCamera = "video.toggle_local_camera";

    /// <summary>Toggle the global shadow switch.</summary>
    public const string ToggleShadows = "video.toggle_shadows";

    /// <summary>Toggle full screen (AMD-010 REQ-UI-209).</summary>
    public const string ToggleFullscreen = "video.toggle_fullscreen";

    /// <summary>Toggle the FPS counter (DEC-10-Q7).</summary>
    public const string ToggleFpsCounter = "video.toggle_fps";

    // ----------------------------------------------------------------- Audio

    /// <summary>Set the general volume; argument: percent 0 to 100 in <c>Number</c>.</summary>
    public const string SetVolume = "audio.set_volume";

    /// <summary>Volume + 5 %.</summary>
    public const string VolumeUp = "audio.volume_up";

    /// <summary>Volume − 5 %.</summary>
    public const string VolumeDown = "audio.volume_down";

    /// <summary>Mute: keeps the volume and forces the effective gain to 0.</summary>
    public const string ToggleMute = "audio.toggle_mute";

    /// <summary>Toggle the menu music (<c>BG Music</c>, SPEC-11 REQ-AUD-021).</summary>
    public const string ToggleMenuMusic = "audio.toggle_menu_music";

    // ----------------------------------------------------------------- Tools

    /// <summary>Open the options window (AMD-010 REQ-UI-202).</summary>
    public const string ShowOptions = "tools.options";

    /// <summary>Open the key configuration window (REQ-UI-203).</summary>
    public const string ShowKeyConfig = "tools.key_config";

    /// <summary>Reveal the user data folder (SPEC-09 REQ-PERS-072).</summary>
    public const string ShowDataFolder = "tools.data_folder";

    /// <summary>Show the about block (DEC-10-Q3).</summary>
    public const string ShowAbout = "tools.about";

    // ---------------------------------------------------------------- Interface

    /// <summary>
    /// What a short press on Menu becomes while a modal window is up (SPEC-10 REQ-UI-004 cases
    /// 2 and 3): the frontend closes the file browser as with « Cancel », and a confirmation box
    /// swallows it (REQ-UI-110).
    /// </summary>
    public const string CancelModal = "ui.cancel_modal";
}
