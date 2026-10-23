//! The load jobs of SPEC-07 §3.3: launching a game, loading a save state, rebuilding the
//! meshes. A job advances by one stage per displayed frame, so the loading indicator is seen
//! and no emulated frame runs before it clears (REQ-LOOP-021 step 6).

use std::path::Path;

use zax_engine3d::settings::RenderAlgorithm;
use zax_engine3d::state_codec;
use zax_nes::{NesCore, NesRegion};
use zax_profiles::enums::TvRegionMode;
use zax_settings::catalogue::SettingId;

use super::{EmulationHost, GameView};
use crate::audio::audio_mixer::AudioSampleSource;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::game_catalog::{GameCatalog, ResolvedGame};
use crate::emulation::rom_file::{self, RomData, INVALID_ROM_MESSAGE};
use crate::game_session::{GameSession, SessionRequest};
use crate::persistence::app_settings::{AppSettings, AppSettingsMut, TvRegionSetting};
use crate::persistence::save_slots::is_valid_slot_id;
use crate::persistence::save_state_file::{
    self, SaveStateContent, CORRUPTED_MESSAGE, GAME_MISSING_MESSAGE,
};

/// What a load job does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    /// Launch a game.
    Launch,
    /// Load a save state.
    LoadSlot,
    /// Rebuild the meshes with another algorithm.
    Regenerate,
}

/// A load in progress.
pub(super) struct LoadJob {
    kind: LoadKind,
    game: Option<ResolvedGame>,
    rom: Option<RomData>,
    slot_id: String,
    state: Option<SaveStateContent>,
    algorithm: RenderAlgorithm,
    stage: u32,
    reload_profile: bool,
    different_game: bool,
}

impl LoadJob {
    fn new(kind: LoadKind) -> Self {
        Self {
            kind,
            game: None,
            rom: None,
            slot_id: String::new(),
            state: None,
            algorithm: RenderAlgorithm::Greedy,
            stage: 0,
            reload_profile: false,
            different_game: false,
        }
    }
}

/// How a stage ended.
enum Step {
    /// The job goes on with its next stage (or waits for the meshes).
    Continue,
    /// The job is complete.
    Finished,
    /// The job failed, with the notification to show.
    Failed(String),
}

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Launches `game_id`: checks the ROM, then starts the job (REQ-LOOP-018 to REQ-LOOP-023).
    pub(super) fn launch_game(&mut self, game_id: &str) {
        // REQ-LOOP-020: a selection is ignored while the loading indicator is up.
        if self.loading {
            return;
        }
        let Some(game) = self.games.resolve(game_id) else {
            self.notifications.show_error(GAME_MISSING_MESSAGE);
            return;
        };
        let Some(rom_path) = self.games.rom_path(game_id).filter(|path| !path.is_empty()) else {
            // REQ-LOOP-020 case 1: an entry with no ROM opens the file browser. Choosing a file
            // is the interface's job; programmatically, the open ROM command carries the path.
            self.notifications.show_error(INVALID_ROM_MESSAGE);
            return;
        };
        let rom = match rom_file::read(Path::new(&rom_path)) {
            Ok(rom) => rom,
            Err(message) => {
                self.notifications.show_error(&message);
                return;
            }
        };
        // REQ-LOOP-018: the content is checked by a throw-away core, so a bad file never
        // disturbs the game in progress.
        let mut probe = NesCore::new();
        let validation = probe.load_rom(&rom.image, &rom.name);
        probe.close_rom();
        if !validation.success {
            self.notifications.show_error(INVALID_ROM_MESSAGE);
            self.log(&format!(
                "{rom_path}: {}",
                validation.error.unwrap_or_default()
            ));
            return;
        }
        // AMD-010 REQ-UI-200: the ROM joins the ten recent ones once it is known to be valid.
        if let Some(store) = &mut self.settings_store {
            store.games_mut().note_rom_opened(&rom_path);
            store.schedule_write(self.elapsed);
        }
        let mut job = LoadJob::new(LoadKind::Launch);
        job.game = Some(game);
        job.rom = Some(rom);
        self.begin_job(job);
    }

    /// Opens a ROM file: with the profile chosen for it, the library entry that owns it, or
    /// else the profile chooser of the interface (REQ-UI-201, Q-036).
    pub(super) fn open_rom(&mut self, rom_path: &str) {
        if self.loading || rom_path.is_empty() {
            return;
        }
        // A profile that has since been removed falls back to the chooser rather than to an
        // error, which is also what REQ-UI-201 describes for a file never started.
        let chosen = self
            .settings_store
            .as_ref()
            .and_then(|store| store.games().profile_for(rom_path).map(str::to_owned))
            .filter(|target| self.games.exists(target));
        let target = chosen.or_else(|| self.games.game_for_rom(rom_path));
        match target {
            Some(game_id) => self.launch_rom_with_profile(rom_path, &game_id),
            None => self.request_ui(Command::ChooseProfile, &CommandArgs::of_text(rom_path)),
        }
    }

    /// Launches `rom_path` on the library entry `game_id`.
    pub fn launch_rom_with_profile(&mut self, rom_path: &str, game_id: &str) {
        if self.loading || rom_path.is_empty() || !self.games.exists(game_id) {
            return;
        }
        // REQ-PERS-032: the ROM path of the entry, and the per-file choice of Q-036.
        let full = std::path::absolute(rom_path).map_or_else(
            |_| rom_path.to_owned(),
            |path| path.to_string_lossy().into_owned(),
        );
        self.games.set_rom_path(game_id, &full);
        if let Some(store) = &mut self.settings_store {
            store.games_mut().set_profile_for(&full, game_id);
            store.schedule_write(self.elapsed);
        }
        self.launch_game(game_id);
    }

    /// Loads a slot: checks the game and the file, then starts the job (REQ-LOOP-044 to 046).
    pub(super) fn load_slot(&mut self, slot_id: &str) {
        // REQ-LOOP-044: ignored while the loading indicator is already up.
        if self.loading || !is_valid_slot_id(slot_id) {
            return;
        }
        let Some(game_id) = self
            .slots
            .find(slot_id)
            .filter(|slot| slot.is_occupied())
            .map(|slot| slot.game_id.clone())
        else {
            return;
        };
        if let Some(mixer) = &mut self.mixer {
            mixer.play_sound(crate::audio::UiSound::Confirm);
        }
        // Step 2: the game the slot names must still exist.
        let Some(game) = self.games.resolve(&game_id) else {
            self.fail_job(GAME_MISSING_MESSAGE);
            return;
        };
        let state = std::fs::read(self.paths.slot_state_file(slot_id))
            .ok()
            .and_then(|bytes| save_state_file::try_read(&bytes));
        let Some(state) = state else {
            // REQ-LOOP-046: an unreadable or inconsistent savestate clears the current game.
            self.fail_job(CORRUPTED_MESSAGE);
            return;
        };
        let mut job = LoadJob::new(LoadKind::LoadSlot);
        job.game = Some(game);
        slot_id.clone_into(&mut job.slot_id);
        job.state = Some(state);
        self.begin_job(job);
    }

    pub(super) fn regenerate_meshes(&mut self, algorithm: RenderAlgorithm) {
        if self.loading || self.session.is_none() {
            return;
        }
        let mut job = LoadJob::new(LoadKind::Regenerate);
        job.algorithm = algorithm;
        self.begin_job(job);
    }

    fn begin_job(&mut self, job: LoadJob) {
        self.job = Some(job);
        self.loading = true;
    }

    /// Advances the load job by one stage.
    pub(super) fn advance_load(&mut self) {
        let Some(mut job) = self.job.take() else {
            return;
        };
        let step = match job.kind {
            LoadKind::Launch => self.advance_launch(&mut job),
            LoadKind::LoadSlot => self.advance_slot_load(&mut job),
            LoadKind::Regenerate => self.advance_regenerate(&mut job),
        };
        match step {
            Step::Continue => self.job = Some(job),
            Step::Finished => {
                self.loading = false;
            }
            Step::Failed(message) => {
                self.fail_job(&message);
            }
        }
    }

    fn reset_port_types_on_game_change(&mut self, job: &LoadJob) {
        if !job.different_game {
            return;
        }
        if let Some(session) = &mut self.session {
            session.reset_port_types();
            self.zapper.reset();
        }
    }

    /// Makes the session of the job's game, unless the previous one is kept.
    fn ensure_session(&mut self, game: &ResolvedGame, reload_profile: bool) -> Result<(), String> {
        if self.session.is_some() {
            return Ok(());
        }
        let request = SessionRequest {
            profile: &game.profile,
            title: &game.title,
            scene: &mut self.scene,
            log: (self.script_log)(),
            different_game: reload_profile,
            // Q-065: the same empty profile plays flat as "2D Mode" and in 3D as "3D by
            // default (no profile)"; the entry decides, not the pattern count.
            two_dimensional_mode: game.is_default_three_dimensional.then_some(false),
        };
        match GameSession::create(request, &mut self.settings) {
            Ok(session) => {
                if let Some(mixer) = &mut self.mixer {
                    let reader = session.core().audio_reader();
                    let source: AudioSampleSource = Box::new(move |destination| {
                        // A mono read cannot be refused; an empty buffer reads as silence.
                        reader.read_samples(destination, false).unwrap_or(0)
                    });
                    mixer.set_emulation_source(Some(source));
                }
                session.set_smooth_gc(self.settings.get(SettingId::EmuLuaSmoothGc).as_bool());
                self.session = Some(session);
                Ok(())
            }
            Err(error) => {
                self.log(&format!("load failed: {error}"));
                Err(INVALID_ROM_MESSAGE.to_owned())
            }
        }
    }

    fn advance_launch(&mut self, job: &mut LoadJob) -> Step {
        let Some(game) = job.game.clone() else {
            return Step::Failed(INVALID_ROM_MESSAGE.to_owned());
        };
        match job.stage {
            0 => {
                // Step 1: the launch date feeds the "Recent" sort (REQ-PERS-033).
                self.games
                    .mark_played(&game.id, (self.now)().unix_timestamp());
                // Step 3: the running game is stopped, End hook and 3D scene emptied, and the
                // profile data is unloaded only when the game differs or is "2D Mode"
                // (REQ-LOOP-021, REQ-PERS-006).
                job.different_game = self.current_game_id() != Some(game.id.as_str());
                job.reload_profile =
                    self.session.is_none() || job.different_game || game.is_generic();
                self.stop_current_game(false, job.reload_profile);
                job.stage = 1;
                Step::Continue
            }
            1 => {
                if let Err(message) = self.ensure_session(&game, job.reload_profile) {
                    return Step::Failed(message);
                }
                // Step 4: the ROM goes to the core, which cold-starts; the region may change.
                let Some(rom) = job.rom.as_ref() else {
                    return Step::Failed(INVALID_ROM_MESSAGE.to_owned());
                };
                let result = match self.session.as_mut() {
                    Some(session) => session.load_rom(&rom.image, &rom.name),
                    None => return Step::Failed(INVALID_ROM_MESSAGE.to_owned()),
                };
                if !result.success {
                    return Step::Failed(INVALID_ROM_MESSAGE.to_owned());
                }
                self.apply_region_on_load(result.region, job.reload_profile, &game);
                self.attach_battery(true);
                job.stage = 2;
                Step::Continue
            }
            2 => {
                // Step 5: the profile, then the Start hook. Relaunching the same game only runs
                // the hook, which is why it loads visibly faster (REQ-LOOP-021, notes).
                if job.reload_profile {
                    self.apply_profile_scene();
                }
                self.reset_port_types_on_game_change(job);
                if let Some(session) = self.session.as_mut() {
                    session.start(&mut self.settings, &mut self.scene);
                }
                self.current_game = Some(game);
                // Nothing per-game is remembered about the camera (REQ-PERS-031): a launch always
                // starts from the default view.
                {
                    let scene = &mut self.scene;
                    scene.camera.reset_view();
                    scene.camera.snap_to_setpoints();
                }
                // Step 6: the state becomes Paused while the meshes finish.
                self.set_state(EmulationState::Paused);
                job.stage = 3;
                Step::Continue
            }
            3 => {
                if self.meshes_ready() {
                    job.stage = 4;
                }
                Step::Continue
            }
            _ => self.finish_launch(job),
        }
    }

    fn advance_slot_load(&mut self, job: &mut LoadJob) -> Step {
        let (Some(game), Some(state)) = (job.game.clone(), job.state.take()) else {
            return Step::Failed(CORRUPTED_MESSAGE.to_owned());
        };
        match job.stage {
            0 => {
                // Step 3: same stop as a launch.
                job.different_game = self.current_game_id() != Some(game.id.as_str());
                job.reload_profile =
                    self.session.is_none() || job.different_game || game.is_generic();
                self.stop_current_game(false, job.reload_profile);
                job.state = Some(state);
                job.stage = 1;
                Step::Continue
            }
            1 => {
                // Step 4: a profile load creates a new Lua state (REQ-LUA-014).
                if let Err(message) = self.ensure_session(&game, job.reload_profile) {
                    return Step::Failed(message);
                }
                if job.reload_profile {
                    self.apply_profile_scene();
                }
                job.state = Some(state);
                job.stage = 2;
                Step::Continue
            }
            2 => self.restore_slot(job, &game, &state),
            3 => {
                job.state = Some(state);
                if self.meshes_ready() {
                    job.stage = 4;
                }
                Step::Continue
            }
            _ => self.finish_launch(job),
        }
    }

    /// Step 5 of a slot load, in the order of DEC-R2-T6: core, learned conversion state and
    /// camera, then `PData`, then Start, then Load.
    fn restore_slot(
        &mut self,
        job: &mut LoadJob,
        game: &ResolvedGame,
        state: &SaveStateContent,
    ) -> Step {
        let Some(session) = self.session.as_mut() else {
            return Step::Failed(CORRUPTED_MESSAGE.to_owned());
        };
        if let Err(error) = session.restore_core(&state.core) {
            self.log(&format!("savestate refused: {error}"));
            return Step::Failed(CORRUPTED_MESSAGE.to_owned());
        }
        if let Err(error) = state_codec::read(session.engine_mut(), &state.conversion) {
            self.log(&format!("savestate sections refused: {error}"));
            return Step::Failed(CORRUPTED_MESSAGE.to_owned());
        }
        let Some(camera) = save_state_file::read_camera(&state.camera) else {
            self.log("savestate sections refused: the camera section has the wrong size.");
            return Step::Failed(CORRUPTED_MESSAGE.to_owned());
        };
        self.scene.camera.restore(&camera);
        // REQ-INP-060: the core block carries the port types, but they are not part of what is
        // persisted across a game change; they go back to Controller before the Start hook that
        // the script restore runs.
        self.reset_port_types_on_game_change(job);
        if let Some(session) = self.session.as_mut() {
            if let Err(error) =
                session.complete_script_restore(&mut self.settings, &mut self.scene, &state.lua)
            {
                self.log(&format!("savestate sections refused: {error}"));
                return Step::Failed(CORRUPTED_MESSAGE.to_owned());
            }
        }
        self.current_game = Some(game.clone());
        self.rollback.clear();
        // DEC-09-Q4: the ROM fingerprint is only logged, never a reason to refuse.
        if !state.header.rom_sha1.is_empty() {
            if let Some(current) = self.current_rom_fingerprint() {
                if current != state.header.rom_sha1 {
                    self.log(&format!(
                        "savestate of slot {} was taken with another ROM image.",
                        job.slot_id
                    ));
                }
            }
        }
        self.attach_battery(false);
        self.set_state(EmulationState::Paused);
        job.state = Some(state.clone());
        job.stage = 3;
        Step::Continue
    }

    fn advance_regenerate(&mut self, job: &mut LoadJob) -> Step {
        match job.stage {
            0 => {
                if let Some(session) = self.session.as_mut() {
                    session.set_render_algorithm(
                        &mut self.settings,
                        &mut self.scene,
                        job.algorithm,
                    );
                }
                job.stage = 1;
                Step::Continue
            }
            1 => {
                if self.meshes_ready() {
                    job.stage = 2;
                }
                Step::Continue
            }
            _ => {
                // REQ-LOOP-024: the clock is reset and the state, Running or Paused, does not
                // change, nor does the core.
                self.pacer.reset();
                Step::Finished
            }
        }
    }

    fn meshes_ready(&mut self) -> bool {
        self.load_progress = self
            .session
            .as_ref()
            .map_or(1.0, |session| session.engine().load_progress());
        self.load_progress >= 1.0
    }

    fn finish_launch(&mut self, job: &LoadJob) -> Step {
        let Some(session) = self.session.as_mut() else {
            return Step::Failed(INVALID_ROM_MESSAGE.to_owned());
        };
        if job.kind == LoadKind::Launch {
            // Step 7: the core is reset, which is the cold start of DEC-02-Q3; the
            // battery-backed RAM survives it.
            session.core_mut().reset();
            session.reset_frame_counter();
        }
        // REQ-AUD-010: no sound of the previous session is ever replayed.
        session
            .core_mut()
            .bus_mut()
            .apu_mut()
            .drop_pending_samples();
        self.rollback.clear();
        self.fast_forward.clear();
        self.apply_speed();
        self.pacer.reset();
        self.load_progress = 1.0;
        self.set_state(EmulationState::Running);
        Step::Finished
    }

    /// Q-031: the region of the ROM when the automatic mode is on, the profile's otherwise.
    fn apply_region_on_load(
        &mut self,
        detected: NesRegion,
        profile_reloaded: bool,
        game: &ResolvedGame,
    ) {
        if self.settings.auto_tv_mode() {
            self.settings.apply_detected_region(detected);
        } else if profile_reloaded {
            if let Some(settings) = game.profile.settings.as_ref() {
                let mode = match settings.tv_mode {
                    TvRegionMode::Pal => TvRegionSetting::Pal,
                    TvRegionMode::Dendy => TvRegionSetting::Dendy,
                    TvRegionMode::Multi => TvRegionSetting::Multi,
                    _ => TvRegionSetting::Ntsc,
                };
                self.settings.set_tv_mode(mode);
            }
        }
        if let Some(session) = &mut self.session {
            session.core_mut().set_region(self.settings.region());
        }
        self.dispatch_setting_changes();
        self.apply_speed();
    }

    /// Resets the light and the backdrop and pushes the profile's settings into the scene
    /// (REQ-SCN-009, SPEC-09 REQ-PERS-005).
    fn apply_profile_scene(&mut self) {
        let scene = &mut self.scene;
        scene.light.reset();
        scene.backdrop.reset();
        crate::scene_script_bridge::push_settings(&self.settings, scene);
    }

    pub(super) fn current_rom_fingerprint(&self) -> Option<String> {
        let session = self.session.as_ref()?;
        let core = session.core();
        let cartridge = core.cartridge()?;
        Some(save_state_file::rom_fingerprint(
            cartridge.rom().prg_rom(),
            cartridge.rom().chr_rom(),
        ))
    }

    fn attach_battery(&mut self, import_saved: bool) {
        let Some(fingerprint) = self.current_rom_fingerprint() else {
            return;
        };
        let saved = self.battery.attach(&self.paths, &fingerprint);
        // A savestate carries its own cartridge RAM, exactly as a console would, so it must not
        // be overwritten by the file on disk (REQ-PERS-060, notes).
        if let (true, Some(saved), Some(session)) = (import_saved, saved, self.session.as_mut()) {
            session.core_mut().import_battery_ram(&saved);
        }
    }

    /// REQ-LOOP-023 and REQ-LOOP-046: an error notification, the current game cleared, the
    /// loading indicator down and the state back to Stopped.
    pub(super) fn fail_job(&mut self, message: &str) {
        self.job = None;
        self.loading = false;
        self.load_progress = 0.0;
        self.stop_current_game(true, true);
        self.notifications.show_error(message);
    }
}
