using Zax.Engine3D;
using Zax.App.Persistence;
using Zax.Nes;
using Zax.Nes.Rom;
using Zax.Profiles;
using Zax.Render;

namespace Zax.App.Loop;

/// <summary>
/// The launch flow of SPEC-07 §3.3, the slot load of §3.5 and the mesh regeneration of
/// REQ-LOOP-024, as a staged job driven one step per rendered frame.
/// </summary>
/// <remarks>
/// Staging is what makes the loading indicator observable and what makes REQ-LOOP-007 real: a
/// command sent while the indicator is up waits for the end of the job instead of racing it.
/// Step 6 of REQ-LOOP-021 — wait until the mesh generation reaches 100 % — is a stage of its
/// own that the job stays in until <see cref="ConversionEngine.LoadProgress"/> reaches 1.
/// </remarks>
internal sealed partial class EmulationHost
{
    private enum LoadKind
    {
        /// <summary>Launching a game, which a reset is a particular case of (REQ-LOOP-050).</summary>
        Launch,

        /// <summary>Reading a slot (REQ-LOOP-044).</summary>
        LoadSlot,

        /// <summary>Rebuilding the meshes after an algorithm change (REQ-LOOP-024).</summary>
        Regenerate,
    }

    private sealed class LoadJob
    {
        public LoadKind Kind { get; init; }

        public ResolvedGame? Game { get; init; }

        public byte[]? RomImage { get; init; }

        public string RomName { get; init; } = string.Empty;

        public string? SlotId { get; init; }

        public SaveStateContent? State { get; init; }

        public RenderAlgorithm Algorithm { get; init; }

        public int Stage { get; set; }

        public bool ReloadProfile { get; set; }

        /// <summary>
        /// Whether the game being loaded differs from the one that was loaded before, which is
        /// what sends the port types back to Controller (REQ-INP-060, DEC-09-Q6).
        /// </summary>
        public bool DifferentGame { get; set; }
    }

    /// <summary>
    /// REQ-INP-060 (DEC-09-Q6, accepted divergence <b>D</b>): loading a game different from the
    /// previous one puts both port types back to <c>Controller</c>, <b>before</b> the profile's
    /// <c>Start</c> hook, so that a Zapper profile can set port 2 from that hook and a return to
    /// any other game is back on two controllers. Port types are never persisted.
    /// </summary>
    private void ResetPortTypesOnGameChange(LoadJob job)
    {
        if (!job.DifferentGame || _session is null)
        {
            return;
        }

        _session.ResetPortTypes();
        Zapper.Reset();
    }

    /// <summary>
    /// Starts a game (REQ-LOOP-020 case 3, REQ-LOOP-021). The ROM is read and validated
    /// <b>before</b> the running game is interrupted: on failure the current game carries on in
    /// the state it was in (AMD-002, REQ-LOOP-018).
    /// </summary>
    private void LaunchGame(string gameId)
    {
        // REQ-LOOP-020: a selection is ignored while the loading indicator is up.
        if (IsLoading)
        {
            return;
        }

        if (_games.Resolve(gameId) is not { } game)
        {
            Notifications.ShowError(SaveStateFile.GameMissingMessage);
            return;
        }

        if (_games.RomPath(gameId) is not { Length: > 0 } romPath)
        {
            // REQ-LOOP-020 case 1: an entry with no ROM opens the file browser. Choosing a file
            // is the UI's job (lot L9); programmatically, open the ROM command carries the path.
            Notifications.ShowError(RomFile.InvalidRomMessage);
            return;
        }

        RomReadResult read = RomFile.Read(romPath);
        if (!read.Success || read.Image is null)
        {
            Notifications.ShowError(read.ErrorMessage ?? RomFile.InvalidRomMessage);
            return;
        }

        // REQ-LOOP-018: the content is checked by a throw-away core, so a bad file never
        // disturbs the game in progress.
        NesCore probe = new();
        RomLoadResult validation = probe.LoadRom(read.Image, read.Name);
        probe.CloseRom();
        if (!validation.Success)
        {
            Notifications.ShowError(RomFile.InvalidRomMessage);
            _log($"{romPath}: {validation.Error}");
            return;
        }

        // AMD-010 REQ-UI-200: the ROM joins the ten recent ones once it is known to be valid.
        _options.RomOpened?.Invoke(romPath);

        _job = new LoadJob
        {
            Kind = LoadKind.Launch,
            Game = game,
            RomImage = read.Image,
            RomName = read.Name,
        };
    }

    /// <summary>
    /// Opens a ROM (AMD-010 REQ-UI-201). The 3D profile is (1) the one already remembered for
    /// that file in <c>games.json</c> (Q-036), failing that the association
    /// <c>library.json</c> already carries for it; with neither, the interface is asked for the
    /// profile chooser of step 2.
    /// </summary>
    private void OpenRom(string romPath)
    {
        if (IsLoading || string.IsNullOrEmpty(romPath))
        {
            return;
        }

        string? target = _options.ProfileForRom?.Invoke(romPath);
        if (target is not null && !_games.Exists(target))
        {
            // A profile that has since been removed falls back to the chooser rather than to an
            // error, which is also what REQ-UI-201 describes for a file never started.
            target = null;
        }

        target ??= _games.GameForRom(romPath);

        if (target is null)
        {
            UiRequested?.Invoke(ZaxCommands.ChooseProfile, CommandArgs.Of(romPath));
            return;
        }

        LaunchRomWithProfile(romPath, target);
    }

    /// <summary>
    /// Starts <paramref name="romPath"/> on the library entry <paramref name="gameId"/> and
    /// remembers the pair, which is what the profile chooser of REQ-UI-201 ends with: the second
    /// opening of the same file launches directly.
    /// </summary>
    public void LaunchRomWithProfile(string romPath, string gameId)
    {
        if (IsLoading || string.IsNullOrEmpty(romPath) || !_games.Exists(gameId))
        {
            return;
        }

        // REQ-PERS-032: the ROM path of the entry, and the per-file choice of Q-036.
        _games.SetRomPath(gameId, Path.GetFullPath(romPath));
        _options.RomProfileChosen?.Invoke(Path.GetFullPath(romPath), gameId);
        LaunchGame(gameId);
    }

    /// <summary>Starts a slot load (REQ-LOOP-044).</summary>
    private void LoadSlot(string slotId)
    {
        // REQ-LOOP-044: ignored while the loading indicator is already up.
        if (IsLoading || !SaveSlotIndex.IsValidSlotId(slotId))
        {
            return;
        }

        if (Slots.Find(slotId) is not { IsOccupied: true } slot)
        {
            return;
        }

        _mixer?.PlaySound(Audio.UiSound.Confirm);

        // Step 2: the game the slot names must still exist.
        if (_games.Resolve(slot.GameId) is not { } game)
        {
            FailJob(SaveStateFile.GameMissingMessage);
            return;
        }

        SaveStateContent? state;
        try
        {
            state = SaveStateFile.TryRead(File.ReadAllBytes(_paths.SlotStateFile(slotId)), out SaveStateContent? read)
                ? read
                : null;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            state = null;
        }

        if (state is null)
        {
            // REQ-LOOP-046: an unreadable or inconsistent savestate clears the current game.
            FailJob(SaveStateFile.CorruptedMessage);
            return;
        }

        _job = new LoadJob
        {
            Kind = LoadKind.LoadSlot,
            Game = game,
            SlotId = slotId,
            State = state,
        };
    }

    /// <summary>Starts a mesh regeneration (REQ-LOOP-024).</summary>
    private void RegenerateMeshes(RenderAlgorithm algorithm)
    {
        if (IsLoading || _session is null)
        {
            return;
        }

        _job = new LoadJob { Kind = LoadKind.Regenerate, Algorithm = algorithm };
    }

    private void AdvanceLoad()
    {
        if (_job is not { } job)
        {
            return;
        }

        try
        {
            switch (job.Kind)
            {
                case LoadKind.Launch:
                    AdvanceLaunch(job);
                    break;

                case LoadKind.LoadSlot:
                    AdvanceSlotLoad(job);
                    break;

                default:
                    AdvanceRegenerate(job);
                    break;
            }
        }
        catch (Exception exception) when (exception is InvalidOperationException or IOException or InvalidDataException)
        {
            _log($"load failed: {exception.Message}");
            FailJob(RomFile.InvalidRomMessage);
        }
    }

    private void AdvanceLaunch(LoadJob job)
    {
        ResolvedGame game = job.Game!;

        switch (job.Stage)
        {
            case 0:
                // Step 1: the launch date feeds the "Recent" sort (REQ-PERS-033).
                _games.MarkPlayed(game.Id, _now().ToUnixTimeSeconds());

                // Step 3: the running game is stopped — End hook, 3D scene emptied — and the
                // profile data is unloaded only when the game differs or is "2D Mode"
                // (REQ-LOOP-021, REQ-PERS-006).
                job.DifferentGame = !string.Equals(_currentGame?.Id, game.Id, StringComparison.Ordinal);
                job.ReloadProfile = _session is null || job.DifferentGame || game.IsGeneric;

                StopCurrentGame(clearCurrentGame: false, disposeSession: job.ReloadProfile);
                job.Stage = 1;
                break;

            case 1:
                _session ??= GameSession.Create(
                    game.Profile,
                    game.Title,
                    _scene,
                    _options.ScriptLog,
                    Settings.Model,
                    job.ReloadProfile,

                    // Q-065: the same empty profile plays flat as « 2D Mode » and in 3D as
                    // « 3D par défaut (sans profil) »; the entry decides, not the pattern count.
                    game.IsDefaultThreeDimensional ? false : null);
                _session.AttachZapper(Zapper);
                _session.ConnectSettingsFile(
                    _options.SaveSettingsRequested,
                    _options.ReloadSettingsRequested);

                // Step 4: the ROM goes to the core, which cold-starts; the region may change.
                RomLoadResult result = _session.LoadRom(job.RomImage!, job.RomName);
                if (!result.Success)
                {
                    FailJob(RomFile.InvalidRomMessage);
                    return;
                }

                ApplyRegionOnLoad(result.Region, job.ReloadProfile, game);
                AttachBattery();
                job.Stage = 2;
                break;

            case 2:
                // Step 5: the profile, then the Start hook. Relaunching the same game only runs
                // the hook, which is why it loads visibly faster (REQ-LOOP-021, notes).
                if (job.ReloadProfile)
                {
                    _session!.ApplyProfileScene(_scene, game.Profile.Settings);
                }

                ResetPortTypesOnGameChange(job);
                _session!.Start();
                _currentGame = game;

                // Nothing per-game is remembered about the camera (REQ-PERS-031): a launch
                // always starts from the default view.
                _scene.Camera.ResetView();
                _scene.Camera.SnapToSetpoints();

                // Step 6: the state becomes Paused while the meshes finish.
                SetState(EmulationState.Paused);
                job.Stage = 3;
                break;

            case 3:
                if (!MeshesReady())
                {
                    return;
                }

                job.Stage = 4;
                break;

            default:
                FinishLaunch();
                break;
        }
    }

    private void AdvanceSlotLoad(LoadJob job)
    {
        ResolvedGame game = job.Game!;
        SaveStateContent state = job.State!;

        switch (job.Stage)
        {
            case 0:
                // Step 3: same stop as a launch.
                job.DifferentGame = !string.Equals(_currentGame?.Id, game.Id, StringComparison.Ordinal);
                job.ReloadProfile = _session is null || job.DifferentGame || game.IsGeneric;

                StopCurrentGame(clearCurrentGame: false, disposeSession: job.ReloadProfile);
                job.Stage = 1;
                break;

            case 1:
                // Step 4: a profile load creates a new Lua state (REQ-LUA-014).
                _session ??= GameSession.Create(
                    game.Profile,
                    game.Title,
                    _scene,
                    _options.ScriptLog,
                    Settings.Model,
                    job.ReloadProfile,

                    // Q-065: the same empty profile plays flat as « 2D Mode » and in 3D as
                    // « 3D par défaut (sans profil) »; the entry decides, not the pattern count.
                    game.IsDefaultThreeDimensional ? false : null);
                _session.AttachZapper(Zapper);
                _session.ConnectSettingsFile(
                    _options.SaveSettingsRequested,
                    _options.ReloadSettingsRequested);
                if (job.ReloadProfile)
                {
                    _session.ApplyProfileScene(_scene, game.Profile.Settings);
                }

                job.Stage = 2;
                break;

            case 2:
                // Step 5, in the order of DEC-R2-T6: core, learned conversion state and camera,
                // then PData, then Start, then Load.
                if (!_session!.RestoreCore(state.Core, out string? error))
                {
                    _log($"savestate refused: {error}");
                    FailJob(SaveStateFile.CorruptedMessage);
                    return;
                }

                try
                {
                    EngineStateCodec.Read(_session.Engine, state.Conversion);
                    _scene.Camera.Restore(SaveStateFile.ReadCamera(state.Camera));
                }
                catch (Exception exception) when (exception is ArgumentException or InvalidDataException or IndexOutOfRangeException)
                {
                    _log($"savestate sections refused: {exception.Message}");
                    FailJob(SaveStateFile.CorruptedMessage);
                    return;
                }

                // REQ-INP-060: the core block carries the port types, but they are not part of
                // what is persisted across a game change; they go back to Controller before the
                // Start hook that CompleteScriptRestore runs.
                ResetPortTypesOnGameChange(job);

                _session.CompleteScriptRestore(state.Lua);
                _currentGame = game;
                Rollback.Clear();

                // DEC-09-Q4: the ROM fingerprint is only logged, never a reason to refuse.
                if (state.Header.RomSha1.Length > 0 && CurrentRomFingerprint() is { } current
                    && !string.Equals(current, state.Header.RomSha1, StringComparison.Ordinal))
                {
                    _log($"savestate of slot {job.SlotId} was taken with another ROM image.");
                }

                AttachBattery(importSaved: false);
                SetState(EmulationState.Paused);
                job.Stage = 3;
                break;

            case 3:
                if (!MeshesReady())
                {
                    return;
                }

                job.Stage = 4;
                break;

            default:
                FinishLaunch();
                break;
        }
    }

    private void AdvanceRegenerate(LoadJob job)
    {
        switch (job.Stage)
        {
            case 0:
                _session!.Engine.SetRenderAlgorithm(job.Algorithm);
                job.Stage = 1;
                break;

            case 1:
                if (!MeshesReady())
                {
                    return;
                }

                job.Stage = 2;
                break;

            default:
                // REQ-LOOP-024: the clock is reset and the state — Running or Paused — does not
                // change, nor does the core.
                Pacer.Reset();
                _job = null;
                break;
        }
    }

    private bool MeshesReady()
    {
        LoadProgress = _session?.Engine.LoadProgress ?? 1f;
        return LoadProgress >= 1f;
    }

    private void FinishLaunch()
    {
        GameSession session = _session!;

        if (_job?.Kind == LoadKind.Launch)
        {
            // Step 7: the core is reset, which is the cold start of DEC-02-Q3 — the
            // battery-backed RAM survives it.
            session.Core.Reset();
            session.ResetFrameCounter();
        }

        // REQ-AUD-010: no sound of the previous session is ever replayed.
        session.Core.Apu.DropPendingSamples();

        Rollback.Clear();
        FastForward.Clear();
        ApplySpeed();
        Pacer.Reset();

        _job = null;
        LoadProgress = 1f;
        SetState(EmulationState.Running);
    }

    /// <summary>
    /// REQ-LOOP-017 and DEC-03-Q01: the region detected in the ROM header is applied first, then
    /// the profile's own <c>TVMode</c> when a different game is loaded — the profile normally
    /// has the last word, because <c>emu.tv_mode</c> is a profile-scoped setting
    /// (SPEC-09 §4.1.7, DEC-09-Q1).
    /// </summary>
    /// <remarks>
    /// With automatic region on, the profile's value is <b>not</b> applied: otherwise the
    /// acceptance criterion of REQ-LOOP-017 — a PAL-marked ROM gives a 50 Hz cadence — could
    /// never hold, since every profile carries a <c>TVMode</c>. Automatic region is off by
    /// default, so the faithful order of DEC-03-Q01 is what a normal session sees. See
    /// <c>spec/changes/Q-031.md</c>.
    /// </remarks>
    // Q-031
    private void ApplyRegionOnLoad(NesRegion detected, bool profileReloaded, ResolvedGame game)
    {
        if (Settings.AutoTvMode)
        {
            Settings.ApplyDetectedRegion(detected);
            _session!.Core.Region = Settings.Region;
            ApplySpeed();
            return;
        }

        if (profileReloaded && game.Profile.Settings is { } settings)
        {
            Settings.TvMode = settings.TvMode switch
            {
                TvRegionMode.Pal => TvRegionSetting.Pal,
                TvRegionMode.Dendy => TvRegionSetting.Dendy,
                TvRegionMode.Multi => TvRegionSetting.Multi,
                _ => TvRegionSetting.Ntsc,
            };
        }

        _session!.Core.Region = Settings.Region;
        ApplySpeed();
    }

    private string? CurrentRomFingerprint()
    {
        if (_session?.Core.Cartridge is not { } cartridge)
        {
            return null;
        }

        return SaveStateFile.RomFingerprint(cartridge.Rom.PrgRom, cartridge.Rom.ChrRom);
    }

    private void AttachBattery(bool importSaved = true)
    {
        if (CurrentRomFingerprint() is not { } fingerprint)
        {
            return;
        }

        byte[]? saved = _battery.Attach(fingerprint);

        // A savestate carries its own cartridge RAM, exactly as a console would, so it must not
        // be overwritten by the file on disk (REQ-PERS-060, notes).
        if (importSaved && saved is not null)
        {
            _session!.Core.ImportBatteryRam(saved);
        }
    }

    /// <summary>
    /// The stop of REQ-LOOP-051: End hook, ROM closed, 3D scene emptied. Only removing the
    /// current game from the library — or an explicit Stop — clears the current game; the
    /// internal stop of a launch does not.
    /// </summary>
    private void StopCurrentGame(bool clearCurrentGame, bool disposeSession = true)
    {
        if (_session is { } session)
        {
            // REQ-PERS-060: the cartridge RAM is written when the game stops.
            _battery.Flush(session.Core.ExportBatteryRam());

            session.End();
            session.Engine.ClearFrames();

            if (disposeSession)
            {
                session.Dispose();
                _session = null;
                _battery.Detach();
            }
        }

        if (clearCurrentGame)
        {
            _currentGame = null;
            _pauseThumbnail = null;
            Rollback.Clear();
            FastForward.Clear();

            // REQ-PERS-012: the speed setting goes back to 0 at the end of a game.
            Settings.EndOfGameResets();

            // REQ-SCN-013: the end of a game is a view-reset trigger, and it must in addition go
            // back to TPV and put the smoothing time back to 0.4 s. EndGame does all three.
            _scene.Camera.EndGame();
            SetState(EmulationState.Stopped);
        }
    }

    private void FailJob(string message)
    {
        // REQ-LOOP-023 and REQ-LOOP-046: an error notification, the current game cleared, the
        // loading indicator down and the state back to Stopped.
        _job = null;
        LoadProgress = 0f;
        StopCurrentGame(clearCurrentGame: true);
        Notifications.ShowError(message);
    }
}
