//! The load jobs of SPEC-07 §3.3: launching a game, loading a save state, rebuilding the
//! meshes. A job advances by one stage per displayed frame, so the loading indicator is seen
//! and no emulated frame runs before it clears (REQ-LOOP-021 step 6).

use std::path::Path;

use zax_core_api::{EngineKind, NesRegion};
use zax_engine3d::settings::RenderAlgorithm;
use zax_engine3d::state_codec;
use zax_nes::NesCore;
use zax_profiles::enums::TvRegionMode;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

use super::games::{ConfirmationRequest, ConfirmedAction, NO_LABEL};
use super::{EmulationHost, GameView};
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::fceux_validated;
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

/// Real seconds a load may make no progress before AMD-051 REQ-LOOP-073 gives up on it.
pub const LOADING_TIMEOUT_SECONDS: f64 = 10.0;

/// The notification of AMD-051 REQ-LOOP-073, a divergence **(D)**: the original program has no
/// message for a load that stops advancing. Its French translation is in `ui/strings`
/// (AMD-035, AMD-036).
pub const LOADING_TIMED_OUT_MESSAGE: &str = "Loading timed out.";

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
    /// Real seconds since the job last moved: a stage completed, or the loading progress of
    /// REQ-E3D-600 changed (AMD-051 REQ-LOOP-073).
    still_seconds: f64,
    /// The progress the last tick saw, so that a mass build that is advancing is never
    /// interrupted however long it takes (REQ-ACC-031 excludes it from every budget).
    seen_progress: f32,
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
            still_seconds: 0.0,
            seen_progress: -1.0,
        }
    }
}

/// How a save state stands against the built-in hack as set (Rust-only, owner request).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotPatchStatus {
    /// It loads as it is (also any state of a ROM no built-in patch is made for).
    Fits,
    /// Made with other components of the built-in hack (its widescreen, its aids) than those set
    /// now: it loads with those set now or once they are set as it had them, as the player
    /// chooses (orange in the menus).
    OtherComponents,
    /// Made with another version of the hack: it cannot load (red in the menus).
    OtherVersion,
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
        // The ROM is studied as it starts: the list of the ROMs shows whether a hack fits it.
        self.certify_rom(Path::new(&rom_path), &rom.image);
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
        if !self.loading && self.choose_profile_for_rom(rom_path, game_id) {
            self.launch_game(game_id);
        }
    }

    /// Records `game_id` as the profile of the ROM at `rom_path`, without starting it (the
    /// console's profile list, owner request); false when the ROM or the profile is unknown.
    pub fn choose_profile_for_rom(&mut self, rom_path: &str, game_id: &str) -> bool {
        if rom_path.is_empty() || !self.games.exists(game_id) {
            return false;
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
        true
    }

    /// Loads a slot: checks the game and the file, then starts the job (REQ-LOOP-044 to 046).
    pub(super) fn load_slot(&mut self, slot_id: &str) {
        self.load_slot_checked(slot_id, true);
    }

    /// [`EmulationHost::load_slot`]; with `check`, a state made with other components of the
    /// built-in hack than those set now asks first how to load it.
    pub(super) fn load_slot_checked(&mut self, slot_id: &str, check: bool) {
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
        let state = std::fs::read(self.slot_state_file(slot_id))
            .ok()
            .and_then(|bytes| save_state_file::try_read(&bytes));
        let Some(state) = state else {
            // REQ-LOOP-046: an unreadable or inconsistent savestate clears the current game.
            self.fail_job(CORRUPTED_MESSAGE);
            return;
        };
        match self.state_patch_status(&state) {
            // The hack's code of another version would run on this memory: refused.
            SlotPatchStatus::OtherVersion => {
                self.notifications
                    .show_error(save_state_file::OTHER_HACK_VERSION_MESSAGE);
                return;
            }
            // Made with other components: loaded with those set now (the first answer), or once
            // they are set as the state had them.
            SlotPatchStatus::OtherComponents if check => {
                self.confirm(ConfirmationRequest {
                    title: game.title.clone(),
                    message: save_state_file::OTHER_COMPONENTS_QUESTION.to_owned(),
                    confirm_label: save_state_file::LOAD_AS_SET_LABEL.to_owned(),
                    cancel_label: NO_LABEL.to_owned(),
                    action: ConfirmedAction::LoadSlotAsSet(slot_id.to_owned()),
                    alternative: Some((
                        save_state_file::LOAD_ALIGNED_LABEL.to_owned(),
                        ConfirmedAction::LoadSlotAligned(slot_id.to_owned()),
                    )),
                });
                return;
            }
            SlotPatchStatus::Fits | SlotPatchStatus::OtherComponents => {}
        }
        let mut job = LoadJob::new(LoadKind::LoadSlot);
        job.game = Some(game);
        slot_id.clone_into(&mut job.slot_id);
        job.state = Some(state);
        self.begin_job(job);
    }

    /// Loads the resume point (AMD-053 REQ-LOOP-076), as a slot load loads a slot: the same job,
    /// the same stages, the same failure (REQ-LOOP-046). Its state is not in [`EmulationHost::slots`],
    /// which holds the slots of the game being played, so the game and the file come from the
    /// resume store instead (Q-102).
    pub(super) fn load_resume_point(&mut self) {
        if self.loading {
            return;
        }
        let Some(state) = crate::persistence::resume_point::read(&self.paths) else {
            self.fail_job(CORRUPTED_MESSAGE);
            return;
        };
        if let Some(mixer) = &mut self.mixer {
            mixer.play_sound(crate::audio::UiSound::Confirm);
        }
        let Some(game) = self.games.resolve(&state.header.game_id) else {
            self.fail_job(GAME_MISSING_MESSAGE);
            return;
        };
        // A state made with another version of the built-in hack's code would run on this memory:
        // refused, as for any slot.
        if self.state_patch_status(&state) == SlotPatchStatus::OtherVersion {
            self.notifications
                .show_error(save_state_file::OTHER_HACK_VERSION_MESSAGE);
            return;
        }
        let mut job = LoadJob::new(LoadKind::LoadSlot);
        job.game = Some(game);
        crate::persistence::resume_point::SLOT_ID.clone_into(&mut job.slot_id);
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

    /// Advances the load job by one stage. `real_seconds` is the real time since the last
    /// displayed frame, which is what the watchdog of AMD-051 REQ-LOOP-073 counts.
    pub(super) fn advance_load(&mut self, real_seconds: f64) {
        let Some(mut job) = self.job.take() else {
            return;
        };
        let stage_before = (job.kind, job.stage);
        let step = match job.kind {
            LoadKind::Launch => self.advance_launch(&mut job),
            LoadKind::LoadSlot => self.advance_slot_load(&mut job),
            LoadKind::Regenerate => self.advance_regenerate(&mut job),
        };
        match step {
            Step::Continue => {
                if self.load_is_still(&mut job, stage_before, real_seconds) {
                    // REQ-LOOP-073 point 2: as REQ-LOOP-023 does for a launch that fails — the
                    // game cleared, the loading indicator down, the state back to Stopped.
                    // Q-105: `fail_job` shows the message at the **error** severity, which is
                    // the gravity the amendment's own line of SPEC-07 §4 gives it; the first
                    // point of the requirement says "normale" instead, and the two readings do
                    // not behave alike (REQ-UI-081 drops a normal notification behind an error).
                    self.fail_job(LOADING_TIMED_OUT_MESSAGE);
                    return;
                }
                self.job = Some(job);
            }
            Step::Finished => {
                self.loading = false;
                // `--movie` (EDITOR-1): the movie given on the command line plays once the game
                // it is for has started.
                if job.kind == LoadKind::Launch {
                    if let Some(path) = self.movie_after_launch.take() {
                        self.play_movie(&path);
                    }
                }
            }
            Step::Failed(message) => {
                self.fail_job(&message);
            }
        }
    }

    /// Whether the job has made no progress at all for [`LOADING_TIMEOUT_SECONDS`]
    /// (AMD-051 REQ-LOOP-073).
    ///
    /// One rule covers both branches of the requirement. A stage that completed, or a loading
    /// progress of REQ-E3D-600 that moved, resets the clock; nothing else does. So:
    ///
    /// - a step of REQ-LOOP-021 or REQ-LOOP-044 that exposes **no** measure of progress has
    ///   10 s to finish, since neither its stage nor the progress changes while it runs;
    /// - the wait for the meshes (step 6), which does expose one, is only interrupted when that
    ///   measure stops moving — never for being long. The total duration of a mass build is not
    ///   bounded by anything (REQ-ACC-031 excludes it from its budget, and REQ-E3D-600 builds
    ///   every object of every pattern of a profile), which is what point 51-N1 of
    ///   `R2-review-AMD-050-053-v1.1` is about.
    fn load_is_still(
        &mut self,
        job: &mut LoadJob,
        stage_before: (LoadKind, u32),
        real_seconds: f64,
    ) -> bool {
        let moved = stage_before != (job.kind, job.stage)
            || (self.load_progress - job.seen_progress).abs() > f32::EPSILON;
        job.seen_progress = self.load_progress;
        if moved {
            job.still_seconds = 0.0;
            return false;
        }

        job.still_seconds += real_seconds;
        job.still_seconds >= LOADING_TIMEOUT_SECONDS
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
    pub(super) fn ensure_session(
        &mut self,
        game: &ResolvedGame,
        reload_profile: bool,
        rom_image: Option<&[u8]>,
    ) -> Result<(), String> {
        if self.session.is_some() {
            return Ok(());
        }
        // AMD-062 REQ-PROF-078 and REQ-PROF-082: the tiles a "3dh" profile names are looked for
        // in the cache and then in the ROM **before** its patterns are built, so that the first
        // converted frame already has them. Nothing happens for a 3dn profile.
        let game = &self.prepare_hash_tiles(game, rom_image);
        // Read before the mutable borrow of `self.scene` below.
        // Lot FCEUX-48 step 4: a `.fm2` playback forces the engine that can replay it for this
        // session, over the game's own choice, which is left stored as it was.
        let engine = self.movie_engine.unwrap_or_else(|| self.chosen_engine());
        let request = SessionRequest {
            profile: &game.profile,
            title: &game.title,
            scene: &mut self.scene,
            log: (self.script_log)(),
            different_game: reload_profile,
            // Q-065: the same empty profile plays flat as "2D Mode" and in 3D as "3D by
            // default (no profile)"; the entry decides, not the pattern count.
            two_dimensional_mode: game.is_default_three_dimensional.then_some(false),
            console_interface: self.console_interface,
            tiles: game.tiles.clone(),
            // `apply_game_options` has just run, so the model carries this game's own choice.
            engine,
        };
        match GameSession::create(request, &mut self.settings) {
            Ok(session) => {
                session.set_smooth_gc(self.settings.get(SettingId::EmuLuaSmoothGc).as_bool());
                self.session = Some(session);
                // Whichever output is open takes the new core's samples: the audio thread's
                // console slot (Q-153), or the mixer's source on the classic output of AUDIO-2.
                self.attach_console();
                Ok(())
            }
            Err(error) => {
                self.log(&format!("load failed: {error}"));
                Err(INVALID_ROM_MESSAGE.to_owned())
            }
        }
    }

    /// Gives a "3dh" profile what the cache of REQ-PROF-082 kept and what the ROM image holds
    /// (REQ-PROF-078), before its patterns are built, and returns the game as the resolution
    /// left it. A 3dn profile, and a profile with nothing waiting, come back unchanged and cost
    /// nothing.
    ///
    /// The cache is keyed by the identifier of the library entry, which for a profile file is
    /// the `Id` its header carries — the envelope's for a "3dh" (REQ-PROF-073) — and the
    /// identifier derived from the content when it has none (AMD-037).
    fn prepare_hash_tiles(
        &mut self,
        game: &ResolvedGame,
        rom_image: Option<&[u8]>,
    ) -> ResolvedGame {
        if !game.has_waiting_tiles() {
            return game.clone();
        }

        let kept = self.tile_cache.read(&self.paths, &game.id);
        let Some(counts) = self
            .games
            .prepare_hash_tiles(&game.id, &kept, rom_image.unwrap_or(&[]))
        else {
            return game.clone();
        };

        self.log(&format!(
            "{}: {} of {} profile tiles resolved.",
            game.title, counts.resolved, counts.total
        ));
        self.games.resolve(&game.id).unwrap_or_else(|| game.clone())
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
                job.reload_profile = self.session.is_none()
                    || job.different_game
                    || game.is_generic()
                    // Lot FCEUX-23 step 5: a changed engine is a new machine, so the session
                    // cannot be kept. The choice takes effect on load and never mid-game.
                    || self.session.as_ref().is_some_and(|session| {
                        session.core().kind() != self.engine_for_game(&game.id)
                    });
                self.stop_current_game(false, job.reload_profile);
                job.stage = 1;
                Step::Continue
            }
            1 => {
                self.apply_game_options(&game.id);
                let rom_image = job.rom.as_ref().map(|rom| rom.image.clone());
                if let Err(message) =
                    self.ensure_session(&game, job.reload_profile, rom_image.as_deref())
                {
                    return Step::Failed(message);
                }
                // Step 4: the ROM goes to the core, which cold-starts; the region may change.
                let Some(rom) = job.rom.as_ref() else {
                    return Step::Failed(INVALID_ROM_MESSAGE.to_owned());
                };
                let builtin_patches = self.builtin_patches_wanted();
                let advanced = self.advanced_patches_wanted();
                let forced = self.forced_patch();
                let cartridge_db = self.settings.get(SettingId::EmuCartridgeDb).as_bool();
                let oam_corruption = self.settings.get(SettingId::EmuOamCorruption).as_bool();
                let bus_conflicts = self.settings.get(SettingId::EmuPpuBusConflicts).as_bool();
                let power_on_ram = match self.settings.get(SettingId::EmuPowerOnRam).as_int() {
                    1 => zax_nes::PowerOnRam::Pattern,
                    2 => zax_nes::PowerOnRam::Ff,
                    3 => zax_nes::PowerOnRam::Random(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(1, |elapsed| {
                                u64::try_from(elapsed.as_nanos() & u128::from(u64::MAX))
                                    .unwrap_or(1)
                            }),
                    ),
                    _ => zax_nes::PowerOnRam::Zero,
                };
                let result = match self.session.as_mut() {
                    Some(session) => {
                        session.core_mut().set_cartridge_db(cartridge_db);
                        session.core_mut().set_oam_corruption(oam_corruption);
                        session.core_mut().set_bus_conflicts(bus_conflicts);
                        session.core_mut().set_power_on_ram(power_on_ram);
                        session.core_mut().set_builtin_patches(builtin_patches);
                        session.core_mut().set_advanced_patches(advanced);
                        session.core_mut().set_forced_patch(forced);
                        session.load_rom(&rom.image, &rom.name)
                    }
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
                    // AMD-063 REQ-LUA-119: the forced rotation is a session value that goes back
                    // to none at a launch and at a change of profile, both of which come here.
                    scene.camera.set_rot_override(None);
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
                self.apply_game_options(&game.id);
                // A savestate carries no ROM, so a "3dh" profile keeps what the cache gave it
                // and resolves the rest while the game runs (AMD-062 REQ-PROF-080).
                if let Err(message) = self.ensure_session(&game, job.reload_profile, None) {
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
        let builtin_patches = self.builtin_patches_wanted();
        let advanced = self.advanced_patches_wanted();
        let forced = self.forced_patch();
        let Some(session) = self.session.as_mut() else {
            return Step::Failed(CORRUPTED_MESSAGE.to_owned());
        };
        // Lot FCEUX-23 step 5: a state written by the other engine is refused **here**, before
        // the engine is asked to read it, and with a message that says what to do. The two
        // engines' blocks have nothing in common; each would also refuse the other's itself (its
        // first four bytes name its engine), but "corrupted" is not what happened and not what a
        // user can act on.
        let wrote_it = session.core().state_core_id();
        if !state.header.core_id.is_empty() && state.header.core_id != wrote_it {
            self.log(&format!(
                "savestate refused: it was written by {} and the engine in play is {wrote_it}",
                state.header.core_id
            ));
            return Step::Failed(save_state_file::OTHER_ENGINE_MESSAGE.to_owned());
        }
        // The state carries the original ROM: the hack applies to it as the setting says now.
        session.core_mut().set_builtin_patches(builtin_patches);
        session.core_mut().set_advanced_patches(advanced);
        session.core_mut().set_forced_patch(forced);
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
        self.frame_steps.clear();
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
        self.load_progress = match self.load_progress_stall {
            // AMD-051 REQ-LOOP-073, acceptance criterion: "le harnais bloque l'étape 6". One
            // `Option` test, on the path a load takes once per displayed frame — never on a
            // frame path.
            Some(held) => held,
            None => self
                .session
                .as_ref()
                .map_or(1.0, |session| session.engine().load_progress()),
        };
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
        // REQ-AUD-010: no sound of the previous session is ever replayed. `v0.4.2` emptied the
        // ring here, and the classic output of AUDIO-2 has nothing else that would — its mixer
        // keeps no gate and no ramp. On the default output `resynchronise_audio` below empties it
        // too, so this is simply the first of two idempotent calls there (AUDIO-2).
        session.core_mut().drop_pending_samples();
        self.rollback.clear();
        self.frame_steps.clear();
        self.fast_forward.clear();
        self.apply_speed();
        self.pacer.reset();
        self.start_achievements(job.state.as_ref());
        self.load_progress = 1.0;
        self.set_state(EmulationState::Running);
        // Lot FCEUX-23 step 5: the visible marker. A user who reports "the 3D is wrong" has to be
        // able to say which engine drew it (§6 of `docs/review/FCEUX-ENGINE.md` is why that
        // matters), so a game that is **not** on Zaxnes's own core says so at every load; the
        // performance overlay names the engine from then on, and the game's options carry the row
        // itself. Zaxnes is the default and says nothing, as it always did.
        // Lot FCEUX-48 step 2 adds the second half of the sentence: whether lot FCEUX-6's sweep
        // validated this ROM for 3D on that engine. It is a marker and not a gate (§7), so a game
        // nobody checked still plays — it is just told so, once, where the user will read it.
        if let Some(engine) = self
            .running_engine()
            .filter(|kind| *kind != EngineKind::Zaxnes)
        {
            let message = match self.fceux_validation() {
                fceux_validated::Status::Validated(rom) => format!(
                    "Playing on the {engine} engine; this ROM is validated for 3D ({}).",
                    rom.game
                ),
                fceux_validated::Status::NotValidated => format!(
                    "Playing on the {engine} engine; this ROM is not validated for 3D on it."
                ),
                fceux_validated::Status::Unknown => format!("Playing on the {engine} engine."),
            };
            self.log(&message);
            self.notifications.show_normal(&message);
        }
        // REQ-AUD-010: no sound of the previous session is ever replayed. `set_state` has
        // already done this when the game was not running, but a slot loaded while it plays
        // leaves the state alone, so it is asked for here too (Q-153 §4). The call empties the
        // core's ring and makes the stream prefill and ramp in rather than cut.
        self.resynchronise_audio();
        Step::Finished
    }

    /// Q-031: the region of the ROM when the automatic mode is on, the profile's otherwise.
    pub(super) fn apply_region_on_load(
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
    pub(super) fn apply_profile_scene(&mut self) {
        let scene = &mut self.scene;
        scene.light.reset();
        scene.backdrop.reset();
        crate::scene_script_bridge::push_settings(&self.settings, scene);
    }

    /// How `state` stands against the built-in hack as the settings have it.
    #[must_use]
    pub fn state_patch_status(&self, state: &SaveStateContent) -> SlotPatchStatus {
        // Only a ROM a built-in patch is made for has components to apply.
        let Some(patch) = NesCore::state_builtin_patch(&state.core, 0) else {
            return SlotPatchStatus::Fits;
        };
        let saved = state_components(state);
        // The hack's code of another version would run on this memory.
        if !saved.is_empty() {
            let current = format!("{:08X}", patch.version());
            if state
                .header
                .hack_version
                .as_ref()
                .is_some_and(|version| !version.eq_ignore_ascii_case(&current))
            {
                return SlotPatchStatus::OtherVersion;
            }
        }
        let mut now = self.components_set(patch);
        let mut saved = saved;
        // A file older than the components knows of the widescreen only.
        if state.header.hack_components.is_none() {
            now.retain(|component| component == zax_nes::hack_pack::WIDESCREEN);
        }
        now.sort();
        saved.sort();
        // A pack whose states carry across (its states section): a state made at the game's wait
        // for the next frame goes on with the hack as it is set, no question asked, in the
        // directions the pack allows (made with the widescreen and loaded without it, or the
        // other way round; only aids differing, either way allowed will do).
        let states = patch.states();
        let wide = zax_nes::hack_pack::WIDESCREEN;
        let carries = state.header.at_idle == Some(true)
            && match (
                saved.iter().any(|component| component == wide),
                now.iter().any(|component| component == wide),
            ) {
                (true, false) => states.to_off,
                (false, true) => states.to_on,
                _ => states.to_off || states.to_on,
            };
        if now == saved || carries {
            SlotPatchStatus::Fits
        } else {
            SlotPatchStatus::OtherComponents
        }
    }

    /// The components of `patch`'s hack pack a game of it gets as set now: its widescreen while
    /// the patch applies, and each aid whose option is on.
    pub(super) fn components_set(
        &self,
        patch: &'static zax_nes::builtin_patches::BuiltinPatch,
    ) -> Vec<String> {
        let mut components = Vec::new();
        if self.patch_applies(patch) {
            components.push(zax_nes::hack_pack::WIDESCREEN.to_owned());
        }
        components.extend(
            self.patch_options(patch)
                .into_iter()
                .filter(|(option, on)| *on && patch.is_aid(option.key))
                .map(|(option, _)| option.key.to_owned()),
        );
        components
    }

    /// [`EmulationHost::state_patch_status`] of the state of slot `slot_id`; remembered per slot,
    /// save time and setting, so that a menu can ask every frame.
    pub fn slot_patch_status(&self, slot_id: &str) -> SlotPatchStatus {
        let Some(record) = self
            .slots
            .find(slot_id)
            .filter(|record| record.is_occupied())
        else {
            return SlotPatchStatus::Fits;
        };
        let enabled = self.builtin_patches_wanted();
        let key = (slot_id.to_owned(), record.saved_at, enabled);
        if let Some(&status) = self.slot_caches.patch.borrow().get(&key) {
            return status;
        }
        let status = std::fs::read(self.slot_state_file(slot_id))
            .ok()
            .and_then(|bytes| save_state_file::try_read(&bytes))
            .map_or(SlotPatchStatus::Fits, |state| {
                self.state_patch_status(&state)
            });
        self.slot_caches.patch.borrow_mut().insert(key, status);
        status
    }

    /// The engine that wrote the state of `slot_id`, read from the core name its header records;
    /// `None` for an empty slot, an unreadable file or a name this build does not know. A state
    /// from before engines were recorded has an empty name, which is Zaxnes's.
    #[must_use]
    pub fn slot_engine(&self, slot_id: &str) -> Option<zax_core_api::EngineKind> {
        let record = self
            .slots
            .find(slot_id)
            .filter(|record| record.is_occupied())?;
        let key = (slot_id.to_owned(), record.saved_at);
        if let Some(&engine) = self.slot_caches.engine.borrow().get(&key) {
            return engine;
        }
        let engine = std::fs::read(self.slot_state_file(slot_id))
            .ok()
            .and_then(|bytes| save_state_file::try_read(&bytes))
            .and_then(|state| engine_of_core_id(&state.header.core_id));
        self.slot_caches.engine.borrow_mut().insert(key, engine);
        engine
    }

    /// The engine the game in play runs on, `None` without a game.
    #[must_use]
    pub fn engine_in_play(&self) -> Option<zax_core_api::EngineKind> {
        self.session.as_ref().map(|session| session.core().kind())
    }

    /// Sets the components of the built-in hack as the state of `slot_id` had them, then loads it:
    /// the second answer to the question [`EmulationHost::load_slot`] asks for a state made with
    /// other components.
    pub(super) fn load_slot_aligned(&mut self, slot_id: &str) {
        let state = std::fs::read(self.slot_state_file(slot_id))
            .ok()
            .and_then(|bytes| save_state_file::try_read(&bytes));
        let Some((state, patch)) = state.and_then(|state| {
            let patch = NesCore::state_builtin_patch(&state.core, 0)?;
            Some((state, patch))
        }) else {
            self.load_slot_checked(slot_id, false);
            return;
        };
        let saved = state_components(&state);
        let widescreen = saved
            .iter()
            .any(|component| component == zax_nes::hack_pack::WIDESCREEN);
        // The patch follows the widescreen of the game (owner request); an advanced patch is its
        // own mode of the widescreen, which stays on for it.
        if widescreen {
            self.settings
                .set(SettingId::EmuBuiltinPatches, SettingValue::Bool(true));
        }
        if patch.advanced {
            if widescreen {
                self.set_game_widescreen(true);
            }
            self.set_game_advanced(widescreen);
        } else {
            self.set_game_widescreen(widescreen);
        }
        // The aids, when the file tells them.
        if state.header.hack_components.is_some() {
            for option in patch.options {
                if patch.is_aid(option.key) {
                    let on = saved.iter().any(|component| component == option.key);
                    self.set_patch_option(patch, option.key, on);
                }
            }
        }
        self.dispatch_setting_changes();
        self.load_slot_checked(slot_id, false);
    }

    pub(super) fn current_rom_fingerprint(&self) -> Option<String> {
        let session = self.session.as_ref()?;
        let core = session.core();
        // A ROM patched in memory by a built-in patch is identified by the player's ROM, so
        // that its battery RAM survives a new version of the patch.
        if let Some(original) = core.original_image() {
            let rom = zax_nes::rom::RomImage::try_load(original, "").ok()?;
            return Some(save_state_file::rom_fingerprint(
                rom.prg_rom(),
                rom.chr_rom(),
            ));
        }
        let cartridge = core.cartridge()?;
        Some(save_state_file::rom_fingerprint(
            cartridge.rom().prg_rom(),
            cartridge.rom().chr_rom(),
        ))
    }

    pub(super) fn attach_battery(&mut self, import_saved: bool) {
        let Some(fingerprint) = self.current_rom_fingerprint() else {
            return;
        };
        // Owner request: the file is named after the ROM; one named after its fingerprint, as
        // they were before, is renamed.
        let name = self.current_game_id().map_or_else(
            || fingerprint.clone(),
            |game_id| self.rom_file_name(game_id),
        );
        let file = self.paths.battery_file(&name);
        let before = self.paths.battery_file(&fingerprint);
        if !file.exists() && before.is_file() && std::fs::rename(&before, &file).is_err() {
            self.log("the battery RAM file could not be renamed after the ROM.");
        }
        let saved = self.battery.attach(&self.paths, &name);
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

/// The components of the built-in hack a state was made with: those its header tells, or for a
/// file older than the field the widescreen when the hack ran.
fn state_components(state: &SaveStateContent) -> Vec<String> {
    state.header.hack_components.clone().unwrap_or_else(|| {
        let hack = state
            .header
            .is_hack
            .unwrap_or_else(|| NesCore::state_uses_widescreen(&state.core, 0).unwrap_or(false));
        if hack {
            vec![zax_nes::hack_pack::WIDESCREEN.to_owned()]
        } else {
            Vec::new()
        }
    })
}

/// The engine a savestate header's core name names: empty is Zaxnes (states written before the
/// name was recorded), otherwise the engine whose [`zax_core_api::state_core_name`] it is.
fn engine_of_core_id(core_id: &str) -> Option<zax_core_api::EngineKind> {
    if core_id.is_empty() {
        return Some(zax_core_api::EngineKind::Zaxnes);
    }
    zax_core_api::EngineKind::ALL
        .into_iter()
        .find(|&engine| zax_core_api::state_core_name(engine) == core_id)
}
