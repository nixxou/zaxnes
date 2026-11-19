use crate::apu::{Apu2A03, AudioError, AudioReader};
use crate::cartridges::{
    has_fast_path, with_board_kind, with_fast_board_kind, AnyBoard, BoardKind, Cartridge,
};
use crate::cpu::{Cpu6502, CpuBus};
use crate::input::{ControllerPortType, InputPorts, InvalidPort, ZapperPosition};
use crate::nes_bus::{FidelityBus, KindBus};
use crate::ppu::{Ppu2C02, MAX_TILE_RECORD_STATE_SIZE, OAM_SIZE, PALETTE_SIZE, VRAM_SIZE};
use crate::rom::{InesHeader, RomImage, RomLoadResult};
use crate::state::{CoreState, InvalidState, StateLoadFailure, StateReader, StateWriter};
use crate::{
    FrameBatch, NametableMirroring, NesBus, NesRegion, RegionTiming, TileRecord, VideoInfo,
};

/// Receives one frame batch per emulated frame (REQ-CORE-030). The emulation behaves
/// identically with or without it (REQ-CORE-033).
pub type VideoCallback = Box<dyn FnMut(&FrameBatch<'_>) + Send>;

/// Supplies the Zapper's aimed position, queried once per frame for each Zapper port
/// (REQ-CORE-060).
pub type ZapperPositionSource = Box<dyn FnMut(usize) -> ZapperPosition + Send>;

// What a restore reports and why a save had nothing to write are on the `Engine` trait, so they
// live in `zax-core-api` since lot FCEUX-23 step 2; re-exported here at the paths they were at
// (`zax_nes::StateLoaded`, `zax_nes::NoRomLoaded`) through `lib.rs`.
pub use zax_core_api::{NoRomLoaded, StateLoaded};

/// One NES machine, seen from the host (REQ-CORE-001, REQ-CORE-002).
///
/// Everything an instance needs lives in the instance, so two cores in one process are
/// independent (REQ-ARCH-011, REQ-CORE-001). Nothing advances unless the host calls
/// [`NesCore::run_frame`]: the core owns no clock and no thread (REQ-ARCH-010).
#[allow(clippy::struct_excessive_bools)] // Independent switches of the machine and of the host.
pub struct NesCore {
    cpu: Cpu6502,
    bus: NesBus,
    region: NesRegion,
    /// FIDELITY-LEADS G5: the 2A03 netlist's power-on get / put parity and DMC timer.
    netlist_get_put: bool,
    battery_ram_imported: bool,
    frame_count: i64,
    video_callback: Option<VideoCallback>,
    zapper_position_source: Option<ZapperPositionSource>,
    /// The image the host handed over when a built-in patch was applied to it: savestates
    /// carry this one, so that restoring one applies the patch of the running build.
    original_image: Option<Vec<u8>>,
    builtin_patch: Option<&'static str>,
    /// Whether the built-in patches apply at all (on by default).
    builtin_patches_enabled: bool,
    /// Whether the cartridge database corrects iNES 1.0 headers at load (lot CARTDB).
    cartridge_db: bool,
    /// What the 2 KiB of work RAM hold at power-on (lot CARTDB step 2); FCEUX's and NesHawk's
    /// `00 00 00 00 FF FF FF FF` by default since the owner's decision of 2026-11-13.
    power_on_ram: crate::PowerOnRam,
    /// What the PPU's palette RAM, OAM and nametable RAM hold at power-on (lot FIDELITY step 36).
    power_on_ppu_ram: crate::PowerOnPpuRam,
    /// The console model's MMC3 revision (lot FIDPLUS-C), applied when a ROM is loaded.
    mmc3_revision: crate::Mmc3Revision,
    /// What a cartridge's battery-backed PRG-RAM holds at power-on when no save was imported
    /// (MAPPERS-N, owner 2026-11-15): seeded pseudo-random bytes by default, as a real SRAM's
    /// contents are undefined and Mesen2's `RamPowerOnState` defaults to `Random`
    /// (`SettingTypes.h:659`). All zero made *Famicom Jump II* take the empty RAM for a valid
    /// save and jump into it (FIDELITY-LEADS J14). The seed is fixed, so runs stay reproducible.
    power_on_battery_ram: crate::PowerOnRam,
    advanced_patches_enabled: bool,
    /// `emu.rainbow_network`: mapper 682's ESP may use the host's network (off by default).
    rainbow_network: bool,
    /// `emu.vs_dip_*`: the VS. System's DIP switches for the next load.
    vs_dip_switches: u8,
    /// The built-in patch applied to a ROM none is made for (Rust-only, owner request: forced).
    forced_patch: Option<&'static crate::builtin_patches::BuiltinPatch>,
    /// CRC-32 of the data (iNES header excluded) of the ROM loaded as the host gave it, which
    /// picks a margin decoder made for an unpatched ROM.
    rom_crc: Option<u32>,
    /// The hack pack made for the ROM loaded, with the patch it is, whether its widescreen runs
    /// (advanced mode) or not: its aids (an "Invincible" component) are turned on and off by
    /// their options either way.
    pack: Option<(
        &'static crate::builtin_patches::BuiltinPatch,
        crate::hack_pack::AppliedPack,
    )>,
    /// The options whose aid components are applied to the cartridge's PRG-ROM now.
    aids_on: u16,
    /// The options the host set last.
    host_options: u16,
    /// The experimental smooth scrolling (option `smooth_scroll`): the camera's fraction read
    /// after the last frame (the next picture's), the camera's pixels and fraction read last,
    /// whether the fraction left the camera (turned round 256 with the pixels still: its last
    /// coupled value kept until they move), and the shift of the picture just made.
    camera_fraction: Option<u8>,
    camera_read: Option<(i32, u8)>,
    camera_decoupled: bool,
    picture_shift: Option<(f32, i32, i32)>,
    /// With the option `smooth_scroll_120`, how far the camera moves from the picture just made
    /// to the next one, in pixels (the RAM is a frame ahead of the picture: it is known now).
    picture_motion: Option<f32>,
    /// The camera the picture just made shows, in pixels (for a debug log of the scrolling).
    picture_camera: Option<f64>,
    /// The playfield scroll the PPU drew in the frame run before (the run-ahead's real frame,
    /// or the frame before without run-ahead), for the option `smooth_scroll_ahead`.
    last_scroll: Option<i32>,
    /// Frames since the last camera report of the game's hack ([`crate::wide::CameraReport`]),
    /// while the smooth scrolling follows its reports.
    report_age: Option<u8>,
    /// Whether the smooth scrolling from the run-ahead is on (the host's option of each game).
    smooth_ahead: bool,
    /// Whether the game's own `smooth_scroll` aid is on, worked out when the options, the patch
    /// or the pack change ([`NesCore::refresh_patch_cache`]) and **never inside a frame**: the
    /// question costs a scan of `builtin_patches::patches()`, and `end_frame` asked it twice a
    /// frame until lot KIRBY-MERGE step 4.
    smooth_scroll_on: bool,
    /// Whether the last frame did any smooth scrolling, so that the frame after the option goes
    /// off still runs once to put the picture and the PPU's per-line record back.
    smooth_scroll_live: bool,
    /// Where the game waits for the next frame, from the patch that runs
    /// ([`crate::builtin_patches::BuiltinPatch::idle_loop`]): the overclock ends there. Cached
    /// with [`NesCore::smooth_scroll_on`] and for the same reason — finding it means scanning
    /// `builtin_patches::patches()`, and `end_frame` did that once a frame.
    idle_loop: Option<(u16, u16)>,
    /// Lot CORE-2: every frame through the path of every board, even for a specialised board
    /// (the equivalence test's reference). Not a setting, not in the state.
    generic_path: bool,
    /// Fidélité+ (`docs/design/HALF-CYCLE.md`): every frame through its own loop
    /// ([`NesCore::run_fidelity_plus_steps`]). Off by default; not in the state (step 4).
    fidelity_plus: bool,
    /// Runs the frames of the boards that have one through the fast core's loop (lot FAST-1,
    /// `docs/design/FAST-CORE.md`). Off by default; not in the state, like Fidélité+.
    fast_core: bool,
    /// The frame a sliced run (`NesCore::run_frame_slice`, lot FIDPLUS-B) has started and not
    /// finished: its first cycle, margin decoder and idle loop. Not part of the state: a state is
    /// taken between frames.
    slice: Option<SliceFrame>,
    /// The margin decoder of the ROM loaded, found by [`NesCore::find_margin_decoder`] when the
    /// ROM, the pack or `emu.builtin_patches` changes, and read by `end_frame` every frame.
    decoder: Option<&'static dyn crate::wide_decode::MarginDecoder>,
    /// The sprites that move with the camera (Kirby, the camera on him), which the shift must
    /// leave where they are: their record indices in the picture just made, and their screen
    /// places; the camera's pixels the last picture showed, and every sprite's place there.
    steady_sprites: Vec<i32>,
    steady_places: Vec<(i16, i16)>,
    shown_camera: Option<i32>,
    sprite_places: Vec<(i16, i16)>,
    /// The state of the real frame of a run-ahead sequence (AMD-041 REQ-LOOP-065 step 2), kept
    /// from one real frame to the next so that nothing is allocated per frame. Empty, and never
    /// touched, while run-ahead is off.
    real_frame: CoreState,
    /// Whether `real_frame` holds a state [`NesCore::restore_real_frame`] still owes a restore.
    real_frame_held: bool,
}

/// How a slice's instructions stopped.
enum SliceOutcome {
    /// The frame ended (vblank began).
    FrameDone,
    /// The slice's cycle budget is spent, the frame goes on.
    Budget,
    /// The board changed variant: go on through `with_board_kind!` again.
    KindChanged,
}

/// A frame a sliced run has started (lot FIDPLUS-B, `docs/design/REALTIME-PACING.md`).
#[derive(Clone, Copy)]
struct SliceFrame {
    start_cycle: i64,
    decoder: Option<&'static dyn crate::wide_decode::MarginDecoder>,
    idle_loop: Option<(u16, u16)>,
}

impl Default for NesCore {
    fn default() -> Self {
        Self::new()
    }
}

impl NesCore {
    /// Creates a machine with no ROM loaded, in NTSC.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu: Cpu6502::new(),
            bus: NesBus::new(Ppu2C02::new(), Apu2A03::new(), InputPorts::new()),
            region: NesRegion::Ntsc,
            netlist_get_put: false,
            battery_ram_imported: false,
            frame_count: 0,
            video_callback: None,
            zapper_position_source: None,
            original_image: None,
            builtin_patch: None,
            builtin_patches_enabled: true,
            cartridge_db: true,
            power_on_ram: crate::PowerOnRam::Pattern,
            power_on_ppu_ram: crate::PowerOnPpuRam::Zero,
            mmc3_revision: crate::Mmc3Revision::Cartridge,
            power_on_battery_ram: crate::PowerOnRam::Random(Self::BATTERY_RAM_SEED),
            advanced_patches_enabled: false,
            rainbow_network: false,
            vs_dip_switches: 0,
            forced_patch: None,
            rom_crc: None,
            pack: None,
            aids_on: 0,
            host_options: 0,
            camera_fraction: None,
            camera_read: None,
            camera_decoupled: false,
            picture_shift: None,
            picture_motion: None,
            picture_camera: None,
            last_scroll: None,
            report_age: None,
            smooth_ahead: false,
            smooth_scroll_on: false,
            smooth_scroll_live: false,
            idle_loop: None,
            generic_path: false,
            fidelity_plus: false,
            fast_core: false,
            slice: None,
            decoder: None,
            steady_sprites: Vec::new(),
            steady_places: Vec::new(),
            shown_camera: None,
            sprite_places: Vec::new(),
            real_frame: CoreState::default(),
            real_frame_held: false,
        }
    }

    /// Turns the built-in patches on or off for the next [`NesCore::load_rom`] and state
    /// restore; the ROM loaded keeps what it has.
    pub fn set_builtin_patches(&mut self, enabled: bool) {
        self.builtin_patches_enabled = enabled;
        self.refresh_patch_cache();
    }

    /// Turns the cartridge database on or off for the next [`NesCore::load_rom`] (lot CARTDB):
    /// on, an iNES 1.0 header the database knows is replaced by the NES 2.0 header the dump
    /// should have carried; off, every header is read as the file has it.
    pub fn set_cartridge_db(&mut self, enabled: bool) {
        self.cartridge_db = enabled;
    }

    /// What the work RAM holds at the next power-on (lot CARTDB step 2). A real console's RAM
    /// starts with whatever its cells settled to; a few games read it before writing it, and
    /// FCEUX and NesHawk fill it with `00 00 00 00 FF FF FF FF` for that reason (NesHawk's
    /// `NES.Core.cs` names *Cybernoid*, *Minna no Taabou* and *Huang Di*). That pattern is the
    /// default (owner, 2026-11-13); REQ-CORE-004's zeros are one choice among four.
    pub fn set_power_on_ram(&mut self, pattern: crate::PowerOnRam) {
        self.power_on_ram = pattern;
    }

    /// The fixed seed of the default battery-RAM power-on pattern.
    pub const BATTERY_RAM_SEED: u64 = 0x5A41_584E_4553_5241;

    /// Chooses what a battery-backed PRG-RAM without an imported save holds at power-on; takes
    /// effect at the next one. Movie tools set [`crate::PowerOnRam::Zero`], FCEUX's convention.
    pub fn set_power_on_battery_ram(&mut self, pattern: crate::PowerOnRam) {
        self.power_on_battery_ram = pattern;
    }

    /// Chooses what the PPU's palette RAM, OAM and nametable RAM hold at power-on; takes effect
    /// at the next one.
    pub fn set_power_on_ppu_ram(&mut self, pattern: crate::PowerOnPpuRam) {
        self.power_on_ppu_ram = pattern;
    }

    /// The console model's CPU (lot FIDPLUS-C): a letterless RP2A03, whose noise channel has no
    /// short mode and its own period table. At once.
    pub fn set_letterless_cpu(&mut self, letterless: bool) {
        self.bus.apu_mut().set_letterless_cpu(letterless);
    }

    /// The console model's MMC3 revision (lot FIDPLUS-C): read when the next ROM is loaded.
    pub fn set_mmc3_revision(&mut self, revision: crate::Mmc3Revision) {
        self.mmc3_revision = revision;
    }

    /// Whether the PPU corrupts a row of OAM when the rendering is switched off inside a line and
    /// comes back (lot FIDELITY step 9; `Ppu2C02::rendering_switched`). Off by default (owner, 2026-11-13) and a setting, because the
    /// hardware does it and `AccuracyCoin`'s `OAM Corruption` checks it, but the known model is
    /// incomplete — with it on, blargg's `sprite_overflow_tests/5.Emulator` fails its third test,
    /// which the console passes.
    pub fn set_oam_corruption(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_oam_corruption(enabled);
    }

    /// Whether the 2C02G/H's OAMADDR defect is emulated: with OAMADDR at 8 or more when rendering
    /// starts, eight bytes of OAM are copied over the first eight (nesdev, "PPU registers"). On by
    /// default; never on a PAL console, whose 2C07 does not have it.
    /// Whether the second `$2006` write loads `v` three dots later with the hybrid address
    /// (measured on the transistor netlists). On by default.
    pub fn set_address_load_delay(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_address_load_delay(enabled);
    }

    /// Whether a `$2006` load colliding with an increment of `v` ANDs with it (nesdev; the
    /// transistor netlists). On by default.
    pub fn set_glitch_2006(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_glitch_2006(enabled);
    }

    /// Whether switching rendering off mid-picture, or a `$2006` load leaving palette RAM, can
    /// corrupt the palettes in alignment 2 (`TriCNES`'s model, D5). On by default.
    pub fn set_palette_corruption(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_palette_corruption(enabled);
    }

    /// Whether resets reach the PPU's /RST pin, as on the front-loading NES: until the
    /// pre-render line after a reset the PPU ignores `$2000`, `$2001`, `$2005` and `$2006`, and
    /// the Reset button sends it back to its power-on position. Off by default: homebrew that
    /// writes those registers at once does not start with it (`Duelito`), and `TriCNES` leaves it
    /// off too, its author "not even 100% certain" of it.
    pub fn set_ppu_reset_line(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_reset_line(enabled);
    }

    /// Chooses the PPU revision: `true` for a 2C02 older than G (palette not readable through
    /// `$2007`, no OAMADDR write corruption), `false` for the 2C02G (default).
    pub fn set_ppu_pre_g_revision(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_pre_g_revision(enabled);
    }

    /// Whether a 2C02G's `$2003` writes copy an OAM row over the one they address (D4). Off by
    /// default: the console-checked OAM tests (blargg `oam_read`, `oam_stress`, `AccuracyCoin`'s
    /// `$2004` behaviour) pass on consoles that do not show it.
    pub fn set_oamaddr_write_corruption(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_oamaddr_write_corruption(enabled);
    }

    /// Whether the PPU's reset line still holds `$2000`, `$2001`, `$2005` and `$2006` (from a
    /// reset to the pre-render line).
    #[must_use]
    pub fn ppu_reset_window_shut(&self) -> bool {
        self.bus.ppu().reset_window_shut()
    }

    /// Shuts or opens that window by hand, for host tools that write the PPU's memories through
    /// its registers (`movie-sync --ppu-ram`) and must put it back as they found it.
    pub fn set_ppu_reset_window_shut(&mut self, shut: bool) {
        self.bus.ppu_mut().set_reset_window_shut(shut);
    }

    /// Whether a `$2000` write starting on dot 257 takes the nametable-X bit of the dot-257 copy
    /// from the CPU's open bus (nesdev; the transistor netlists). On by default.
    pub fn set_glitch_2000(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_glitch_2000(enabled);
    }

    /// Which of the four CPU/PPU clock alignments (0 to 3, `AccuracyCoin`'s numbering) the PPU
    /// follows: `$2007` read latch, `$2000` glitch, `$2001`/`$2006` delays and the OAM row
    /// corruption (`docs/review/FIDELITY-LEADS.md`, A4 and D1). 3 by default.
    pub fn set_cpu_ppu_alignment(&mut self, alignment: u8) {
        self.bus.ppu_mut().set_cpu_ppu_alignment(alignment);
    }

    pub fn set_oamaddr_bug(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_oamaddr_bug(enabled);
    }

    /// Whether a power-on puts the PPU at the top of the picture, as nesdev documents ("PPU power
    /// up state"), instead of the start of vblank. Off by default: it moves the timeline of every
    /// game against the frames (recordings, recipes, saved states made at power-on). Takes
    /// effect at the next power-on.
    pub fn set_power_on_top(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_power_on_top(enabled);
    }

    /// Imitates the analog audio path of one measured console: 0 for none (the output as it
    /// always was), `n` for `apu::console_profile::CONSOLE_AUDIO_PROFILES[n - 1]`. Takes effect
    /// at once, without a reset.
    /// Study `study/access-phase`: moves the CPU/PPU alignment by `dots` at power-on.
    pub fn set_phase_offset(&mut self, dots: i32) {
        self.bus.ppu_mut().set_phase_offset(dots);
    }

    pub fn set_console_audio_profile(&mut self, index: usize) {
        self.bus.apu_mut().set_console_profile(index);
    }

    /// Whether OAM decays when the rendering stays off: a row nothing accessed for 8 NTSC frames
    /// loses its bytes (nesdev, "PPU OAM"; `BeesNES`). Off by default: the decay stamps are not in
    /// the savestate, so a run-ahead or a rewind, which load a state every frame, never let OAM
    /// decay.
    pub fn set_oam_decay(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_oam_decay(enabled);
    }

    /// Whether a `$2007` read or a `$2006` write that collides with a fetch of the PPU disturbs
    /// that fetch, as on the console (lot FIDELITY step 16; `Ppu2C02::read_buffer_from_fetch`,
    /// `Ppu2C02::delay_address_load`). On by default: a PAL console video of *Super Mario Bros. 3*
    /// (Europe) shows the damage it predicts on its split line, same place and same rate
    /// (`docs/review/FIDELITY-LEADS.md`, I1); a setting because it is a defect of the hardware.
    pub fn set_bus_conflicts(&mut self, enabled: bool) {
        self.bus.ppu_mut().set_bus_conflicts(enabled);
    }

    /// Whether the Rainbow board's ESP (mapper 682) reaches the host's network: off by default,
    /// so that nothing leaves the emulator unless the player asks (lot MAPPERS-N). Applies to the
    /// cartridge loaded now and to the next ones.
    pub fn set_rainbow_network(&mut self, enabled: bool) {
        self.rainbow_network = enabled;
        self.apply_rainbow_network();
    }

    /// The end of a state load: [`Self::forget_real_frame`], and the VS PPU's palette remap put
    /// back over the palette RAM the state restored (lot MAPPERS-N; `Ppu2C02::remap_palette`).
    fn forget_real_frame_and_reshow_palette(&mut self) {
        self.forget_real_frame();
        self.bus.reapply_vs_palette();
    }

    /// Attaches a freshly built cartridge with the settings that live on the core.
    fn attach_cartridge(&mut self, cartridge: Cartridge) {
        self.bus.attach(Some(cartridge));
        self.apply_rainbow_network();
    }

    #[cfg(feature = "board-682")]
    fn apply_rainbow_network(&mut self) {
        let enabled = self.rainbow_network;
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.set_network_enabled(enabled);
        }
    }

    #[cfg(not(feature = "board-682"))]
    #[allow(clippy::unused_self)]
    const fn apply_rainbow_network(&mut self) {}

    /// Whether the built-in patches and the advanced ones are on for the next load, as
    /// [`NesCore::set_builtin_patches`] and [`NesCore::set_advanced_patches`] left them.
    #[must_use]
    pub const fn patch_switches(&self) -> (bool, bool) {
        (self.builtin_patches_enabled, self.advanced_patches_enabled)
    }

    /// Turns the advanced patches (`BuiltinPatch::advanced`) on or off for the next
    /// [`NesCore::load_rom`] and state restore: off, a ROM whose patch is advanced loads as it
    /// is, widened by its margin decoder alone.
    pub fn set_advanced_patches(&mut self, enabled: bool) {
        self.advanced_patches_enabled = enabled;
    }

    /// The built-in patch `image` gets as the patches are set.
    fn patch_image(&self, image: &[u8]) -> Option<crate::builtin_patches::PatchedRom> {
        if !self.builtin_patches_enabled {
            return None;
        }
        crate::builtin_patches::apply_with(image, self.forced_patch).filter(|patched| {
            self.advanced_patches_enabled
                || crate::builtin_patches::by_name(patched.name).is_none_or(|patch| !patch.advanced)
        })
    }

    /// The built-in patch the next [`NesCore::load_rom`] and state restore apply to a ROM no
    /// patch is made for, while the patches are on (Rust-only: a forced patch, whose checksums
    /// are not asked for); none by default.
    pub fn set_forced_patch(
        &mut self,
        patch: Option<&'static crate::builtin_patches::BuiltinPatch>,
    ) {
        self.forced_patch = patch;
    }

    /// Whether the state block at `offset` in `data` can be restored with the built-in patches as
    /// they are set: a block made by a ROM a built-in patch applies to must have been made with
    /// the patch (widescreen extension running) when the patches are on, and without it when they
    /// are off; any other block fits. `None` when `data` holds no readable block there.
    #[must_use]
    pub fn state_fits_builtin_patches(&self, data: &[u8], offset: usize) -> Option<bool> {
        let wide = Self::state_uses_widescreen(data, offset)?;
        let mut reader = StateReader::new(data, offset).ok()?;
        reader.read_u32().ok()?;
        reader.read_i32().ok()?;
        reader.read_string().ok()?;
        let has_rom_image = reader.read_bool().ok()?;
        reader.read_string().ok()?;
        let rom = reader.read_blob().ok()?;
        let patchable = has_rom_image
            && crate::builtin_patches::patch_for(&rom)
                .is_some_and(|patch| !patch.advanced || self.advanced_patches_enabled);
        Some(wide == (patchable && self.builtin_patches_enabled))
    }

    /// The built-in patch made for the ROM image the state block at `offset` in `data` carries,
    /// whether or not it ran: `None` for a block without an image or with an image no patch is
    /// made for.
    #[must_use]
    pub fn state_builtin_patch(
        data: &[u8],
        offset: usize,
    ) -> Option<&'static crate::builtin_patches::BuiltinPatch> {
        let mut reader = StateReader::new(data, offset).ok()?;
        reader.read_u32().ok()?;
        reader.read_i32().ok()?;
        reader.read_string().ok()?;
        if !reader.read_bool().ok()? {
            return None;
        }
        reader.read_string().ok()?;
        let rom = reader.read_blob().ok()?;
        crate::builtin_patches::patch_for(&rom)
    }

    /// Whether the state block at `offset` in `data` was made by a ROM running the widescreen
    /// extension (format 5 or 6: `Some(true)`) or by one without it (4: `Some(false)`); `None`
    /// when `data` holds no block of this core there. A ROM patched by a built-in patch needs
    /// states of the first kind, the unpatched ROM states of the second.
    #[must_use]
    pub fn state_uses_widescreen(data: &[u8], offset: usize) -> Option<bool> {
        let header = data.get(offset..offset.checked_add(8)?)?;
        if u32::from_le_bytes([header[0], header[1], header[2], header[3]]) != Self::STATE_MAGIC {
            return None;
        }
        // MF-UNL-9 added the two version-8/9 arms with the bump that made 8 and 9 the *previous*
        // pair: without them a state written by the build before this lot would answer `None`
        // here and the host would stop recognising it, which is the one way bumping the format
        // could have broken a file it still loads.
        match i32::from_le_bytes([header[4], header[5], header[6], header[7]]) {
            Self::STATE_VERSION
            | Self::STATE_VERSION_8
            | Self::FIDPLUS_STATE_VERSION
            | Self::EXTENDED_STATE_VERSION => Some(false),
            Self::WIDE_STATE_VERSION
            | Self::FIDPLUS_WIDE_STATE_VERSION
            | Self::EXTENDED_WIDE_STATE_VERSION
            | Self::WIDE_STATE_VERSION_9
            | Self::WIDE_STATE_VERSION_6
            | Self::WIDE_STATE_VERSION_5 => Some(true),
            _ => None,
        }
    }

    /// The image the host loaded, before the built-in patch applied to it; `None` when no
    /// built-in patch applied (the cartridge's image is then the host's). Hosts identify the
    /// game by this one (battery RAM, savestate headers), so a new version of a patch is the
    /// same game.
    #[must_use]
    pub fn original_image(&self) -> Option<&[u8]> {
        self.original_image.as_deref()
    }

    /// The built-in patch applied to the ROM loaded, if any (see [`crate::builtin_patches`]).
    #[must_use]
    pub const fn builtin_patch(&self) -> Option<&'static str> {
        self.builtin_patch
    }

    /// Name of the core, recorded at the head of savestate files (REQ-CORE-120). There is
    /// exactly one core.
    pub const CORE_NAME: &'static str =
        zax_core_api::state_core_name(zax_core_api::EngineKind::Zaxnes);

    /// The name the core carried under the « Sen » working name. Still accepted when reading
    /// a state block, never written: AMD-029 §2 keeps states written before the rename
    /// loadable (Q-070).
    pub const LEGACY_CORE_NAME: &'static str = "Sen";

    /// Message a savestate written by a different core is refused with (REQ-CORE-120). Defined
    /// as `zax_core_api`'s since lot FCEUX-23 step 3, because FCEUX's engine returns the same
    /// one for a block of this core's.
    pub const FOREIGN_CORE_MESSAGE: &'static str = zax_core_api::FOREIGN_CORE_MESSAGE;

    /// Magic that opens a core savestate block, "SEN0". Kept as it is across the rename of
    /// AMD-029: it is a format datum, and changing it would refuse every block written
    /// before (Q-070). It is `zax_core_api::state_magic(EngineKind::Zaxnes)` since lot
    /// FCEUX-23 step 3, where the first four bytes of a block became how the application tells
    /// which engine wrote it.
    pub const STATE_MAGIC: u32 = zax_core_api::state_magic(zax_core_api::EngineKind::Zaxnes);

    /// Version of the savestate format. Bumped whenever the block layout changes; L1c added
    /// the APU channel, mixer and resampler state, L1d the mapper bank windows, latches and
    /// IRQ counters, RU-3a the MMC5's sound registers, and MF-UNL-9 the flash chip's changed
    /// sectors (REQ-CORE-090, AMD-054 REQ-CORE-075).
    pub const STATE_VERSION: i32 = 10;

    /// The earlier block, without the MMC5's sound registers; still read (REQ-CORE-091).
    pub const STATE_VERSION_4: i32 = 4;

    /// The earlier block, without the flash chip's changed sectors; still read. It is the one
    /// every state written between RU-3a and MF-UNL-9 carries, and the only board that lost
    /// anything by not having the bytes is mapper 111, which did not persist its flash at all
    /// before this lot.
    pub const STATE_VERSION_8: i32 = 8;

    /// Version of a block that also carries the widescreen extension registers: written only
    /// once a ROM unlocked the extension, so every other block keeps [`Self::STATE_VERSION`].
    pub const WIDE_STATE_VERSION: i32 = 11;

    /// The earlier widescreen block, without the flash chip's changed sectors; still read.
    pub const WIDE_STATE_VERSION_9: i32 = 9;

    /// [`Self::STATE_VERSION`] followed by the PPU's pending Fidélité+ edges
    /// (`Ppu2C02::save_edge_state`): written only when a state is taken with edges pending, so
    /// every other block keeps the version earlier builds read.
    pub const FIDPLUS_STATE_VERSION: i32 = 12;

    /// [`Self::STATE_VERSION`] followed by a byte of flags and the optional blocks it names: bit 0
    /// the PPU's pending Fidélité+ edges, bit 1 the console audio profile's filter memory (lot
    /// FIDPLUS-D1b). Written only when one of them is present, so every other block keeps the
    /// version earlier builds read; 12 and 13 stay readable.
    pub const EXTENDED_STATE_VERSION: i32 = 14;

    /// [`Self::WIDE_STATE_VERSION`] followed by the flags and blocks of
    /// [`Self::EXTENDED_STATE_VERSION`].
    pub const EXTENDED_WIDE_STATE_VERSION: i32 = 15;

    /// [`Self::WIDE_STATE_VERSION`] followed by the PPU's pending Fidélité+ edges.
    pub const FIDPLUS_WIDE_STATE_VERSION: i32 = 13;

    /// The earlier widescreen block, without the MMC5's sound registers; still read.
    pub const WIDE_STATE_VERSION_7: i32 = 7;

    /// The earlier widescreen block, without the execution zone register; still read.
    pub const WIDE_STATE_VERSION_6: i32 = 6;

    /// The earlier widescreen block, without the PRG window registers; still read.
    pub const WIDE_STATE_VERSION_5: i32 = 5;

    /// Bytes the widescreen extension's own block occupies at most, which the in-memory state
    /// of REQ-CORE-121 always carries: its two 64-byte tables, its 3 584-byte RAM, their length
    /// prefixes and its dozen registers.
    const MAX_WIDE_STATE_SIZE: usize = 64 + 64 + crate::wide::RAM_SIZE + 64;

    /// Whether `version` is one of a block with the widescreen extension's registers.
    const fn is_wide_state_version(version: i32) -> bool {
        matches!(
            version,
            Self::WIDE_STATE_VERSION
                | Self::FIDPLUS_WIDE_STATE_VERSION
                | Self::EXTENDED_WIDE_STATE_VERSION
                | Self::WIDE_STATE_VERSION_9
                | Self::WIDE_STATE_VERSION_7
                | Self::WIDE_STATE_VERSION_6
                | Self::WIDE_STATE_VERSION_5
        )
    }

    /// Whether `version` is one this build reads at all.
    const fn is_known_state_version(version: i32) -> bool {
        version == Self::STATE_VERSION
            || version == Self::FIDPLUS_STATE_VERSION
            || version == Self::EXTENDED_STATE_VERSION
            || version == Self::STATE_VERSION_8
            || version == Self::STATE_VERSION_4
            || Self::is_wide_state_version(version)
    }

    /// Whether a block of `version` carries the MMC5's sound registers, which RU-3a added at
    /// the end of that board's own registers (AMD-054 REQ-CORE-075). An older block loads
    /// with the board's sound left silent, which it also is after a reset.
    ///
    /// **The threshold is a number and no longer [`Self::STATE_VERSION`]**: the two were the same
    /// until MF-UNL-9 bumped the format, and a comparison against the current version would have
    /// quietly stopped reading the registers out of every version-8 and version-9 block the
    /// moment it moved. The plain and the widescreen numbering interleave — 4, then 5-7, then 8,
    /// 9, 10, 11 — so one `>=` covers both as long as it names the version the registers
    /// appeared in.
    pub(crate) const fn state_version_has_mmc5_audio(version: i32) -> bool {
        version >= Self::STATE_VERSION_8
    }

    /// Whether a block of `version` carries the flash chip's changed sectors, which MF-UNL-9
    /// added at the end of the cartridge's own section (REQ-CORE-090, REQ-CORE-095, **Q-169**). An
    /// older block loads with the chip holding what the dump holds, which is what a fresh power-on
    /// gives; only mapper 111 has a flash chip, and before this lot it persisted nothing at all.
    pub(crate) const fn state_version_has_flash(version: i32) -> bool {
        version >= Self::STATE_VERSION
    }

    /// Tile grid dimensions: always (33, 31, 128) (REQ-CORE-031).
    pub const VIDEO_INFO: VideoInfo = VideoInfo::STANDARD;

    /// Mapper numbers this build accepts (REQ-CORE-100, REQ-CORE-102).
    pub const SUPPORTED_MAPPERS: &'static [i32] = Cartridge::SUPPORTED_MAPPERS;

    /// Registers the receiver of the frame batches, or unregisters it with `None`
    /// (REQ-CORE-030, REQ-CORE-033).
    pub fn set_video_callback(&mut self, callback: Option<VideoCallback>) {
        self.video_callback = callback;
    }

    /// Registers the source of the Zapper's aimed position (REQ-CORE-060).
    pub fn set_zapper_position_source(&mut self, source: Option<ZapperPositionSource>) {
        self.zapper_position_source = source;
    }

    /// Loads a ROM image (REQ-CORE-010) and powers the machine on. On failure the instance
    /// is left stopped (REQ-CORE-011, Q-002). The detected region is returned, not applied
    /// (REQ-CORE-013).
    pub fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult {
        // A ROM a built-in patch was made for is patched in memory (the host's copy is left
        // alone); the original is kept for the savestates.
        let patched = self.patch_image(image);
        let loaded = patched
            .as_ref()
            .map_or(image, |patched| patched.image.as_slice());
        #[allow(unused_mut)] // The VS. System's board correction is the one writer.
        let mut rom = match RomImage::try_load(loaded, name) {
            Ok(rom) => rom,
            Err(error) => {
                self.close_rom();
                return RomLoadResult::fail(error.failure, error.message);
            }
        };

        let crc = crate::builtin_patches::data_crc(image);

        // The VS. System of the image, settled once, before the power-on: an arcade board, its
        // PPU model and its stick wiring are detected from the header and from the CRC of the
        // data, never from anything the machine does later (lot MF-VS step 1).
        //
        // **Detection first, then the board correction, and the order is forced** (lot
        // MF-VS-fix step 2): mapper 99 and mapper 151 are themselves VS signals, so an image
        // whose header carries no console byte would stop being a VS image the moment the table
        // rewrote that number. The correction then happens before `Cartridge::create`, which is
        // the whole point — the board is chosen from the corrected header.
        #[cfg(feature = "board-vs-system")]
        let note = {
            let mut vs = crate::vs::VsSystem::detect(rom.header(), crc);
            // Lot MAPPERS-N: the host's DIP switches (`emu.vs_dip_*`), set before the power-on.
            if let Some(vs) = vs.as_mut() {
                vs.set_dip_switches(self.vs_dip_switches);
            }
            let note = vs.as_ref().map(crate::vs::VsSystem::load_note).map(|note| {
                match rom.correct_vs_board(crc) {
                    Some(correction) => format!("{note}; {correction}"),
                    None => note,
                }
            });
            self.bus.set_vs_system(vs);
            note
        };
        #[cfg(not(feature = "board-vs-system"))]
        let note: Option<String> = None;

        // Lot CARTDB: what an iNES 1.0 header cannot say (the chip revision, the submapper, the
        // RAM, the battery, the region), from the cartridge database, before the board is built
        // from the header. A NES 2.0 header is never touched.
        let note = if self.cartridge_db {
            match crate::rom::cartdb::corrected_header(rom.header(), crc) {
                Some(correction) => {
                    rom.take_database_header(correction.header);
                    let fixed = format!("cartridge database: {}", correction.changes.join("; "));
                    Some(note.map_or(fixed.clone(), |note| format!("{note}; {fixed}")))
                }
                None => note,
            }
        } else {
            note
        };

        let mut cartridge = match Cartridge::create(rom) {
            Ok(cartridge) => cartridge,
            Err(error) => {
                self.close_rom();
                return RomLoadResult::fail(error.failure, error.message);
            }
        };
        match self.mmc3_revision {
            crate::Mmc3Revision::Cartridge => {}
            crate::Mmc3Revision::A => {
                cartridge.set_mmc3_irq_revision_a(true);
            }
            crate::Mmc3Revision::B => {
                cartridge.set_mmc3_irq_revision_a(false);
            }
        }

        // The header's region, unless the dump is known to be of another one.
        let region =
            crate::rom::region_exception(crc).unwrap_or_else(|| cartridge.rom().announced_region());
        self.battery_ram_imported = false;
        self.builtin_patch = patched.as_ref().map(|patched| patched.name);
        self.rom_crc = Some(crc);
        // The patch's options start as they are by default; a host sets the player's choice.
        let options = self
            .builtin_patch
            .and_then(crate::builtin_patches::by_name)
            .or_else(|| crate::builtin_patches::patch_for(image))
            .map_or(0, crate::builtin_patches::BuiltinPatch::default_options);
        self.set_hack_options(options);
        self.original_image = patched.map(|_| image.to_vec());

        self.attach_cartridge(cartridge);
        self.install_pack(Some(image));
        self.power_on();
        match note {
            Some(note) => RomLoadResult::ok_with_note(region, note),
            None => RomLoadResult::ok(region),
        }
    }

    /// Stops the emulation (REQ-CORE-005).
    pub fn close_rom(&mut self) {
        self.battery_ram_imported = false;
        self.original_image = None;
        self.builtin_patch = None;
        self.rom_crc = None;
        self.pack = None;
        self.refresh_patch_cache();
        #[cfg(feature = "board-vs-system")]
        self.bus.set_vs_system(None);
        self.bus.attach(None);
        self.bus.apu_mut().discard_samples();
        // A stopped machine makes no sound, whatever a run-ahead sequence had asked for.
        self.bus.apu_mut().set_output_enabled(true);
        self.forget_real_frame(); // REQ-LOOP-066, as in `power_on`.
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.bus.cartridge().is_some()
    }

    /// The VS. System of the image loaded, or `None` for a NES image (lot MF-VS).
    ///
    /// What it carries was chosen at load and does not change; what a host may *drive* through
    /// [`Self::vs_system_mut`] is the cabinet — the coin slots, the service button and the eight
    /// DIP switches.
    #[cfg(feature = "board-vs-system")]
    #[must_use]
    pub fn vs_system(&self) -> Option<&crate::vs::VsSystem> {
        self.bus.input().vs()
    }

    /// The VS. System of the image loaded, to drive its cabinet.
    #[cfg(feature = "board-vs-system")]
    pub fn vs_system_mut(&mut self) -> Option<&mut crate::vs::VsSystem> {
        self.bus.input_mut().vs_mut()
    }

    /// Whether the image loaded is a VS. System; always `false` in a build without
    /// `board-vs-system`, so that a host needs no `cfg` (lot MAPPERS-N).
    #[must_use]
    pub fn is_vs_system(&self) -> bool {
        #[cfg(feature = "board-vs-system")]
        return self.vs_system().is_some();
        #[cfg(not(feature = "board-vs-system"))]
        false
    }

    /// Drops a coin into slot `slot` (1 or 2) of the VS. System loaded; nothing on a NES image.
    #[allow(unused_variables)]
    pub fn vs_insert_coin(&mut self, slot: usize) {
        #[cfg(feature = "board-vs-system")]
        if let Some(vs) = self.vs_system_mut() {
            vs.insert_coin(slot);
        }
    }

    /// Presses the VS. System's service button for a few frames (a free credit).
    pub fn vs_press_service(&mut self) {
        #[cfg(feature = "board-vs-system")]
        if let Some(vs) = self.vs_system_mut() {
            vs.press_service();
        }
    }

    /// The eight DIP switches of the next VS. System loaded (and of the one loaded now), bit `n`
    /// being switch `n + 1`; a cabinet reads most of them at power-on.
    #[allow(unused_variables)]
    pub fn set_vs_dip_switches(&mut self, switches: u8) {
        self.vs_dip_switches = switches;
        #[cfg(feature = "board-vs-system")]
        if let Some(vs) = self.vs_system_mut() {
            vs.set_dip_switches(switches);
        }
    }

    /// Name the ROM was loaded under (REQ-CORE-014).
    #[must_use]
    pub fn rom_name(&self) -> Option<&str> {
        self.bus.cartridge().map(|cartridge| cartridge.rom().name())
    }

    /// The CPU / APU get-put parity and DMC timer a power-on starts with (FIDELITY-LEADS G5):
    /// `false` the ones Zaxnes has always started with, `true` the 2A03 netlist's. A console
    /// takes one or the other at power-on; the reset button keeps them. Takes effect at the next
    /// power-on.
    pub fn set_netlist_get_put(&mut self, netlist: bool) {
        self.netlist_get_put = netlist;
    }

    /// Resets the machine, which is equivalent to a power cycle (REQ-CORE-004, DEC-02-Q3).
    pub fn reset(&mut self) {
        if self.bus.cartridge().is_some() {
            self.power_on();
        }
    }

    /// The **soft** reset of AMD-044 REQ-CORE-122, which imitates a press on the console's
    /// RESET button: the internal RAM, the save RAM and the CHR RAM are **kept**, the CPU keeps
    /// A, X, Y and every flag but `I` while S drops by 3 and PC comes from the reset vector,
    /// and the PPU, the APU and the controller ports go to their documented *after reset*
    /// state. The frame counter restarts from 0 (REQ-CORE-006 amended).
    ///
    /// **The mapper is left alone**, which is the one way this differs from REQ-CORE-122 as
    /// written ("le PPU, l'APU *et le mapper* suivent leur état documenté *after reset*"): the
    /// mapper half of that clause has no documented state behind it, because the cartridge
    /// connector has no `/RESET` pin for a mapper to see. The amendment the clause needs is
    /// written up in `docs/review/AMD-REQ-CORE-122-mapper-reset.md`; the code implements the
    /// hardware, on the owner's decision of 2026-11-10.
    ///
    /// The hard reset of [`Self::reset`] is unchanged: it stays a power cycle (DEC-02-Q3),
    /// which is why `cpu_reset/ram_after_reset` fails under it on purpose (REQ-CORE-004) and
    /// passes under this one.
    pub fn soft_reset(&mut self) {
        if self.bus.cartridge().is_none() {
            return;
        }

        // The order of `power_on`, minus everything that clears memory: the PPU, the APU and
        // the mapper are put back first, so that the CPU's own seven reset cycles — real bus
        // accesses — already run against the after-reset machine.
        self.bus.ppu_mut().soft_reset();
        self.bus.apu_mut().soft_reset();

        // The latch and the shift registers of the ports go to zero. This is a choice of the
        // reimplementation and has no effect on the buttons the player is holding, which are
        // read again on the next frame (AMD-044 §2).
        self.bus.input_mut().reset();

        // **The mapper is left alone** (lot EVAL-SYNC-3 step 4, owner's decision of 2026-11-10).
        // The NES cartridge connector has no `/RESET` pin — nesdev's "Cartridge connector" lists
        // every pin of both edges and there is none — so a mapper cannot see the reset button:
        // its bank registers, its IRQ unit and its A12 watcher hold whatever the game last wrote
        // into them, across as many resets as the player presses. The boards that *do* react are
        // the reset-detecting multicarts, which notice the absence of M2 rather than a wire, and
        // they say so by overriding `Mapper::soft_reset`; `Cartridge::soft_reset` is the call
        // that reaches them and that refreshes the bus's fast windows afterwards.
        //
        // Until this lot `Cartridge::reset` was called here, which put every board back to
        // power-on. See `docs/review/AMD-REQ-CORE-122-mapper-reset.md` for the amendment this
        // asks of REQ-CORE-122, whose current wording covers the mapper.
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.soft_reset();
        }

        self.cpu.soft_reset(&mut self.bus);
        self.frame_count = 0;
        self.forget_real_frame(); // REQ-LOOP-066, as in `power_on`.
    }

    /// Battery-backed PRG-RAM, for a host that wants to persist it (REQ-CORE-095). Empty
    /// when no ROM is loaded or the ROM has no battery.
    #[must_use]
    pub fn export_battery_ram(&self) -> &[u8] {
        self.bus.cartridge().map_or(&[], Cartridge::battery_ram)
    }

    /// Loads battery-backed PRG-RAM (REQ-CORE-095). Once imported, a reset no longer clears
    /// it. Returns `false` when no ROM is loaded or it has no battery-backed PRG-RAM.
    pub fn import_battery_ram(&mut self, data: &[u8]) -> bool {
        let Some(cartridge) = self.bus.cartridge_mut() else {
            return false;
        };

        let target = cartridge.battery_ram_mut();
        if target.is_empty() {
            return false;
        }

        let count = data.len().min(target.len());
        target[..count].copy_from_slice(&data[..count]);
        self.battery_ram_imported = true;
        true
    }

    /// The flash chip's changed sectors, for a host to persist beside the battery file
    /// (REQ-CORE-095, MF-UNL-9 step 2). **Empty** when no ROM is loaded, when the board's PRG-ROM
    /// is mask ROM, or when the game has not written the chip — so a host can write a file only
    /// when this is non-empty and never create one for the hundreds of boards that cannot save
    /// this way.
    ///
    /// This is the second of the two save media of this build and the only one that is not RAM.
    /// The GTROM (mapper 111) is the one board with it, and nesdev is categorical that it has no
    /// other: "Both of these mappers lack PRG RAM for game saves but the configuration of both
    /// boards make it possible to erase and rewrite sections of the chip using NES software." The
    /// file is the **changed sectors** and not an image of the chip, which is what
    /// `docs/review/UNL-census.md` §7.2 asks for: one flashed 4 KiB sector of a 512 KiB chip is
    /// about 4 KiB on disk.
    ///
    /// **Q-169**: REQ-CORE-095 describes the battery-backed PRG-RAM and says nothing about a
    /// board whose save medium is its own ROM, which is why this is a second pair of methods and
    /// not a widening of the first. It is deliberately **not** gated on the header's battery bit:
    /// that bit does not describe this board, and a GTROM dump that leaves it clear can still
    /// save.
    ///
    /// Unlike [`Self::export_battery_ram`] it returns an owned buffer, because the changed sectors
    /// are not contiguous in any memory this core holds.
    #[must_use]
    pub fn export_flash_save(&self) -> Vec<u8> {
        self.bus
            .cartridge()
            .map_or_else(Vec::new, crate::cartridges::Cartridge::export_flash_save)
    }

    /// Loads a file written by [`Self::export_flash_save`] (REQ-CORE-095). Returns `false` when
    /// no ROM is loaded, the board has no flash chip, or `data` is not a file this build reads —
    /// and in every one of those cases the chip is left exactly as it was.
    ///
    /// No counterpart of `battery_ram_imported` is needed: the flash lives in the PRG-ROM image,
    /// which neither a reset nor `Cartridge::clear_volatile_ram` touches, so an imported save
    /// survives a power cycle the way the real chip does.
    pub fn import_flash_save(&mut self, data: &[u8]) -> bool {
        self.bus
            .cartridge_mut()
            .is_some_and(|cartridge| cartridge.import_flash_save(data))
    }

    fn power_on(&mut self) {
        // Q-003: the region goes in first, so the PPU's power-on position is the start of
        // vblank of the region in force (REQ-CORE-015, REQ-CORE-020 notes).
        self.apply_region();

        self.bus.reset();
        // The bus's power-on zeroed the work RAM; any other pattern goes over it.
        if self.power_on_ram != crate::PowerOnRam::Zero {
            self.power_on_ram.fill(self.bus.ram_mut());
        }
        self.bus.ppu_mut().reset();
        if let Some((palette, oam, vram)) = self.power_on_ppu_ram.contents() {
            self.bus
                .ppu_mut()
                .fill_power_on_memories(&palette, &oam, &vram);
        }
        // Lot MAPPERS-N: the VS PPU's palette remap lives on the bus now (`Ppu2C02::remap_palette`).
        self.bus.reapply_vs_palette();
        self.boot_patch_window(self.builtin_patch);
        self.boot_pack_zone();
        self.bus.apu_mut().reset();
        let clear_prg_ram = !self.battery_ram_imported;
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.reset();
            cartridge.clear_volatile_ram(clear_prg_ram);
            // Plain SRAM only: on 16, 157, 159 and 164 the battery slice is a serial EEPROM's
            // array and on 19 it carries the Namco 163's internal RAM, which keep their
            // all-zero power-on (FIDELITY-LEADS J14).
            if clear_prg_ram
                && self.power_on_battery_ram != crate::PowerOnRam::Zero
                && !matches!(cartridge.board_number(), 16 | 19 | 157 | 159 | 164)
            {
                self.power_on_battery_ram.fill(cartridge.battery_ram_mut());
            }
        }

        self.cpu.reset(&mut self.bus);
        if self.netlist_get_put {
            // FIDELITY-LEADS G5: the 2A03 netlist's power-on, the other get / put parity and the
            // DMC output timer at 431 with 8 bits left, as the CPU differential aligns them at
            // the first opcode.
            self.bus.diagnostic_flip_get_put();
            self.bus.apu_mut().diagnostic_set_dmc_phase(431, 8);
        }
        self.run_boot_program();
        self.frame_count = 0;
        // REQ-LOOP-066: a reset or a change of game starts the next real frame afresh, with the
        // same N; the state of the machine that is gone is never put back.
        self.forget_real_frame();
    }

    /// Lot MAPPERS-P: a copier loader's boot (puNES `extcl_cpu_init_pc_561`), at power-on only.
    /// The data goes to CPU RAM (`$0000-$1FFF`, mirrored every 2 KiB) or to the work RAM (its
    /// `address & $1FFF`); with an `init` address the CPU starts at `$0700` on `JSR init`,
    /// `JMP ($FFFC)`, so the game's own reset vector follows.
    fn run_boot_program(&mut self) {
        let Some(boot) = self
            .bus
            .cartridge()
            .and_then(crate::cartridges::Cartridge::boot_program)
        else {
            return;
        };
        let start = usize::from(boot.address & 0x1FFF);
        if boot.address < 0x2000 {
            let ram = self.bus.ram_mut();
            for (index, byte) in boot.data.iter().enumerate() {
                ram[(start + index) % ram.len()] = *byte;
            }
        } else if let Some(cartridge) = self.bus.cartridge_mut() {
            let ram = cartridge.memory_mut().prg_ram_mut();
            if !ram.is_empty() {
                let len = ram.len();
                for (index, byte) in boot.data.iter().enumerate() {
                    ram[(start + index) % len] = *byte;
                }
            }
        }
        if boot.init != 0 {
            let [init_low, init_high] = boot.init.to_le_bytes();
            let stub = [0x20, init_low, init_high, 0x6C, 0xFC, 0xFF];
            self.bus.ram_mut()[0x700..0x706].copy_from_slice(&stub);
            self.cpu.pc = 0x0700;
        }
    }

    /// TV region in force (REQ-CORE-016).
    #[must_use]
    pub const fn region(&self) -> NesRegion {
        self.region
    }

    /// Changes the region; takes effect on the next frame, without a reset (REQ-CORE-016).
    pub fn set_region(&mut self, region: NesRegion) {
        self.region = region;
        self.apply_region();
    }

    /// Timings of the region in force (REQ-CORE-015).
    #[must_use]
    pub const fn timing(&self) -> RegionTiming {
        RegionTiming::for_region(self.region)
    }

    fn apply_region(&mut self) {
        let timing = RegionTiming::for_region(self.region);
        self.bus.ppu_mut().set_timing(timing);
        self.bus.apu_mut().set_timing(timing);
    }

    /// Output sample rate in Hz (REQ-CORE-070).
    #[must_use]
    pub const fn sample_rate(&self) -> i32 {
        self.bus.apu().sample_rate()
    }

    /// Sets the output sample rate (REQ-CORE-070).
    ///
    /// # Errors
    /// [`AudioError::SampleRateOutOfRange`] outside 22 050 to 96 000 Hz.
    pub fn set_sample_rate(&mut self, value: i32) -> Result<(), AudioError> {
        self.bus.apu_mut().set_sample_rate(value)
    }

    /// Speed factor: it only retimes the audio, never the emulation (REQ-CORE-072).
    #[must_use]
    pub const fn speed(&self) -> f64 {
        self.bus.apu().speed()
    }

    /// Lot FIDPLUS-B (`docs/design/REALTIME-PACING.md`): the sound follows the console's own frame
    /// rate, for a host that chains frames at [`Self::frame_rate`]`(true)`.
    pub fn set_exact_rate(&mut self, exact: bool) {
        self.bus.apu_mut().set_exact_rate(exact);
    }

    /// The frame rate a host chains frames at for this core's region: the console's own with the
    /// exact rate (lot FIDPLUS-B), else the nominal one (DEC-R2-T3).
    #[must_use]
    pub fn frame_rate(&self, exact: bool) -> f64 {
        let timing = self.bus.ppu().timing();
        if exact {
            timing.exact_frames_per_second()
        } else {
            f64::from(timing.nominal_frames_per_second)
        }
    }

    /// Sets the speed factor (REQ-CORE-072).
    ///
    /// # Errors
    /// [`AudioError::SpeedNotPositive`] for zero or less.
    pub fn set_speed(&mut self, value: f64) -> Result<(), AudioError> {
        self.bus.apu_mut().set_speed(value)
    }

    /// Fine adjustment of the audio resampling ratio, at most ±0.5 % (REQ-CORE-070).
    #[must_use]
    pub const fn audio_ratio_adjustment(&self) -> f64 {
        self.bus.apu().ratio_adjustment()
    }

    /// Sets the fine ratio adjustment (REQ-CORE-070).
    ///
    /// # Errors
    /// [`AudioError::RatioAdjustmentOutOfRange`] beyond ±0.5 %.
    pub fn set_audio_ratio_adjustment(&mut self, value: f64) -> Result<(), AudioError> {
        self.bus.apu_mut().set_ratio_adjustment(value)
    }

    /// Fills `destination` with audio, entirely or not at all (REQ-CORE-071); see
    /// [`Apu2A03::read_samples`].
    ///
    /// # Errors
    /// [`AudioError::OddStereoBuffer`] when `stereo` and the length is odd.
    pub fn read_audio(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError> {
        self.bus.apu().read_samples(destination, stereo)
    }

    /// A handle the host's audio thread can read samples from (REQ-CORE-071).
    #[must_use]
    pub fn audio_reader(&self) -> AudioReader {
        self.bus.apu().audio_reader()
    }

    /// The APU.
    #[must_use]
    pub const fn apu(&self) -> &Apu2A03 {
        self.bus.apu()
    }

    /// Runs one frame with `input_word` (REQ-CORE-020, REQ-CORE-022): port 1 in bits 0 to
    /// 7, port 2 in bits 8 to 15; for a controller the order is A, B, Select, Start, Up,
    /// Down, Left, Right (SPEC-02 §4.4).
    ///
    /// The frame runs from where the previous one stopped to the start of the next vblank;
    /// the video callback is then invoked once, synchronously, even when the batch is empty
    /// (REQ-CORE-021). Returns the CPU cycles the frame took, 0 when no ROM is loaded
    /// (REQ-CORE-005).
    pub fn run_frame(&mut self, input_word: u16) -> i64 {
        if self.bus.cartridge().is_none() {
            return 0;
        }
        if self.slice.is_some() {
            // Lot FIDPLUS-B: a frame begun in slices (the host has just stopped slicing) is
            // finished, rather than a second frame begun on top of it.
            return self.finish_sliced_frame(input_word);
        }

        self.bus.input_mut().begin_frame(input_word);
        // Mapper 188's microphone has two buttons of its own (lot MAPPERS-N, `BandaiKaraoke`).
        #[cfg(feature = "board-188")]
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.set_input_word(input_word);
        }

        // The aimed position is sampled once, at the start of the frame, and stays fixed
        // for its whole duration (REQ-CORE-060).
        if let Some(source) = self.zapper_position_source.as_mut() {
            self.bus.input_mut().aim_zappers(source);
        }

        let start_cycle = self.bus.cycle_count();

        // A game with a margin decoder gets the host's margin whatever its ROM asks for.
        let decoder = self.margin_decoder();
        self.bus.ppu_mut().wide_mut().set_decoded(decoder.is_some());

        // The overclock ends once the game waits for the next frame, when its patch says where
        // (the patch applied, or the one whose hack pack gives the ROM its aids: an aid may ask
        // for the overclock).
        //
        // **Read, not looked up, and `None` unless a stall is possible** (lot KIRBY-MERGE step
        // 4). Two separate costs were paid here, and the second was the lot's whole regression
        // on the five commercial games.
        //
        // *The lookup.* Finding the patch means `builtin_patches::by_name`, which scans
        // `PATCHES` — a `const` array, so each call materialises all of it. A patch's idle loop
        // cannot change inside a frame, so it is worked out with the rest of the patch's cache
        // ([`NesCore::refresh_patch_cache`]).
        //
        // *The test it arms.* The loop below asks `(first..=last).contains(&self.cpu.pc)` on
        // **every CPU instruction**, which is about 5 Ir each. `dev` answered `None` here for an
        // unpatched game and the test vanished; the branch added `.or_else(|| self.pack …)`, and
        // a hack pack is found by **signature** on every one of the five commercial games
        // whether or not its patch is applied — so the test came back on for all of them. Line
        // by line (callgrind, `core/src/ops/range.rs` charged to `run_frame`), that was
        // **3.1 M Ir over 60 frames on `metroid`**: +24 M over 300, +0.52 % on the ROM, and the
        // same on `smb1`, `smb3`, `mm2` and `z2`.
        //
        // The filter keeps the branch's intent whole — an aid that asks for the overclock still
        // gets its wait ended — and costs one test a frame. A ROM that has not unlocked the
        // register window cannot have written `$5006`, so it cannot be stalling, so the test
        // has nothing to find.
        let idle_loop = self.idle_loop.filter(|_| self.bus.ppu().wide().unlocked());

        self.bus.ppu_mut().clear_frame_boundary();
        if self.fidelity_plus || self.bus.ppu().edges_pending() {
            // Fidélité+: its own loop (one test a frame on the standard path). The PPU is told
            // every frame: a load builds it afresh. The PPU and the APU are batched as in the
            // standard frame: every register access still catches the PPU up first, and the
            // edges count dots, not catch-ups. A Fidélité+ state loaded with the option off runs
            // this loop too while its edges are pending, posting none.
            self.bus.ppu_mut().set_fidelity_plus(self.fidelity_plus);
            self.bus.defer_ppu();
            loop {
                let done = match self.bus.cartridge().filter(|_| !self.generic_path) {
                    Some(cartridge) => with_board_kind!(cartridge.board(), K => {
                        self.run_fidelity_plus_steps::<K>(idle_loop)
                    }),
                    None => self.run_fidelity_plus_steps::<AnyBoard>(idle_loop),
                };
                if done {
                    break;
                }
            }
            self.bus.end_deferred_ppu();
        } else if !(self.fast_core && self.run_fast_frame(idle_loop)) {
            self.run_standard_frame(idle_loop);
        }
        if let Some(decoder) = decoder {
            self.decode_margins(decoder);
        }
        self.finish_frame(decoder, start_cycle)
    }

    /// Runs the frame in slices (lot FIDPLUS-B step 2, `docs/design/REALTIME-PACING.md`): from
    /// where the last slice stopped, or from a new frame, up to the first CPU instruction boundary
    /// at or after cycle `until` (the bus's cycle count), or to the frame's end. `input_word` is
    /// the pad's state now: a new frame starts with it, a frame in progress takes it at the next
    /// `$4016` strobe. Returns the frame's cycles when it ended in this slice (the video callback
    /// has fired, as after [`Self::run_frame`]), `None` when it goes on. A machine run only by
    /// whole frames and one run by slices of any size are the same machine.
    pub fn run_frame_slice(&mut self, input_word: u16, until: i64) -> Option<i64> {
        if self.bus.cartridge().is_none() {
            return Some(0);
        }
        let frame = if let Some(frame) = self.slice {
            self.bus.input_mut().set_live_word(input_word);
            frame
        } else {
            self.bus.input_mut().begin_frame(input_word);
            if let Some(source) = self.zapper_position_source.as_mut() {
                self.bus.input_mut().aim_zappers(source);
            }
            let start_cycle = self.bus.cycle_count();
            let decoder = self.margin_decoder();
            self.bus.ppu_mut().wide_mut().set_decoded(decoder.is_some());
            let idle_loop = self.idle_loop.filter(|_| self.bus.ppu().wide().unlocked());
            self.bus.ppu_mut().clear_frame_boundary();
            let frame = SliceFrame {
                start_cycle,
                decoder,
                idle_loop,
            };
            self.slice = Some(frame);
            frame
        };

        let fidelity = self.fidelity_plus || self.bus.ppu().edges_pending();
        if fidelity {
            self.bus.ppu_mut().set_fidelity_plus(self.fidelity_plus);
        }
        self.bus.defer_ppu();
        // Lot REWRITE-UI: the fast core runs real-time slices too (the default host loop), its
        // counting held to the slice's end.
        let fast = self.fast_core
            && !fidelity
            && !self.generic_path
            && !self.bus.is_vs_image()
            && self
                .bus
                .cartridge()
                .is_some_and(|cartridge| has_fast_path(cartridge.board()));
        if fast {
            self.bus.set_fast_limit(until);
        }
        let done = loop {
            let outcome = match self.bus.cartridge().filter(|_| !self.generic_path) {
                Some(cartridge) if fast => with_fast_board_kind!(cartridge.board(), K => {
                    self.run_fast_frame_steps::<K>(frame.idle_loop, until)
                }, self.run_slice_steps::<AnyBoard>(frame.idle_loop, until, fidelity)),
                Some(cartridge) => with_board_kind!(cartridge.board(), K => {
                    self.run_slice_steps::<K>(frame.idle_loop, until, fidelity)
                }),
                None => self.run_slice_steps::<AnyBoard>(frame.idle_loop, until, fidelity),
            };
            match outcome {
                SliceOutcome::FrameDone => break true,
                SliceOutcome::Budget => break false,
                SliceOutcome::KindChanged => {}
            }
        };
        self.bus.end_deferred_ppu();
        self.bus.set_fast_limit(i64::MAX);
        if !done {
            return None;
        }
        self.slice = None;
        if let Some(decoder) = frame.decoder {
            self.decode_margins(decoder);
        }
        Some(self.finish_frame(frame.decoder, frame.start_cycle))
    }

    /// The rest of a frame begun in slices, in one go; its cycles. Out of line: the whole-frame
    /// path only tests that a slice is in progress.
    #[inline(never)]
    fn finish_sliced_frame(&mut self, input_word: u16) -> i64 {
        loop {
            if let Some(cycles) = self.run_frame_slice(input_word, i64::MAX) {
                return cycles;
            }
        }
    }

    /// Whether a sliced run has a frame in progress ([`Self::run_frame_slice`]).
    #[must_use]
    pub const fn frame_in_progress(&self) -> bool {
        self.slice.is_some()
    }

    /// The instructions of a slice on the board kind `K`: as [`Self::run_frame_steps`] (or the
    /// Fidélité+ loop's), and a stop at the first instruction boundary at or after cycle `until`.
    /// A loop of its own, so that the whole-frame loops carry no budget test.
    fn run_slice_steps<K: BoardKind>(
        &mut self,
        idle_loop: Option<(u16, u16)>,
        until: i64,
        fidelity: bool,
    ) -> SliceOutcome {
        loop {
            if fidelity {
                self.cpu.step(&mut FidelityBus::<K>::new(&mut self.bus));
            } else {
                self.cpu.step(&mut KindBus::<K>::new(&mut self.bus));
            }
            if let Some((first, last)) = idle_loop {
                if (first..=last).contains(&self.cpu.pc) && self.bus.ppu().wide().stalling() {
                    self.bus.ppu_mut().wide_mut().end_stall();
                }
            }
            if self.bus.ppu().vblank_just_started() {
                return SliceOutcome::FrameDone;
            }
            if K::SWITCHES
                && !self
                    .bus
                    .cartridge()
                    .is_some_and(|cartridge| K::matches(cartridge.board()))
            {
                return SliceOutcome::KindChanged;
            }
            if self.bus.cycle_count() >= until {
                return SliceOutcome::Budget;
            }
        }
    }

    /// The standard frame: the PPU and the APU batched, the board's path (lot CORE-2).
    fn run_standard_frame(&mut self, idle_loop: Option<(u16, u16)>) {
        self.bus.defer_ppu();
        // Lot MAPPERS-P: mapper 532's overlay reads CPU RAM as it stood at the pre-render line.
        #[cfg(feature = "board-532")]
        let ersan2_ram = if self.ersan2_active() {
            Some(self.run_to_pre_render())
        } else {
            None
        };
        // The frame's path (lot CORE-2, `docs/design/CORE-TIERS.md`): the board is looked at once
        // here, and the frame runs through the code compiled for it, or for every board when it
        // is not in the table. `run_frame` checked that a cartridge is loaded.
        loop {
            // Lot MAPPERS-N: a VS. System image always takes the generic path, the one kind that
            // carries the bus's VS quirks (`BoardKind::RUNS_VS`).
            let generic = self.generic_path || self.bus.is_vs_image();
            let done = match self.bus.cartridge().filter(|_| !generic) {
                Some(cartridge) => {
                    with_board_kind!(cartridge.board(), K => self.run_frame_steps::<K>(idle_loop))
                }
                None => self.run_frame_steps::<AnyBoard>(idle_loop),
            };
            if done {
                break;
            }
        }
        self.bus.end_deferred_ppu();
        #[cfg(feature = "board-532")]
        if let Some(ram) = ersan2_ram {
            self.paint_ersan2(&ram);
        }
    }

    /// What follows a frame's instructions on every path: smooth scrolling, the frame count, the
    /// video callback. Returns the cycles the frame took.
    fn finish_frame(
        &mut self,
        decoder: Option<&'static dyn crate::wide_decode::MarginDecoder>,
        start_cycle: i64,
    ) -> i64 {
        // Lot FDS: the drive's automatic disk changes, on every frame path (standard,
        // Fidelity+, sliced). Nothing for a cartridge without a drive.
        #[cfg(feature = "board-fds")]
        self.fds_end_frame();
        // **One `or` of three booleans per frame for every game that wants none of this**, and
        // the whole of the smooth scrolling behind it (lot KIRBY-MERGE step 4). The branch asked
        // the question per frame instead, and the question was expensive: `hack_option_on`
        // reaches `builtin_patches::by_name`, which scans `PATCHES` — a `const` array of 64
        // entries, re-materialised at every call site — so a ROM with a built-in patch paid it
        // twice a frame. Measured on the PR-1 set, it cost `metroid` **+27.6 M instructions over
        // 300 frames** inside `run_frame` alone (+0.584 % on the ROM), `smb1` +26.0 M, `z2`
        // +24.2 M, and about +6 M even on a ROM with no patch at all, where only the rest of the
        // two functions ran. The answer cannot change inside a frame, so it is worked out when
        // the options, the patch or the pack change ([`NesCore::refresh_smooth_scroll`]).
        if self.smooth_scroll_wanted() {
            self.smooth_scroll_frame(decoder);
        }

        self.frame_count += 1;

        if let Some(callback) = self.video_callback.as_mut() {
            callback(&Self::batch_of(self.bus.ppu()));
        }

        self.bus.cycle_count() - start_cycle
    }

    /// Lot FDS: the drive's once-a-frame work — the BIOS's disk search resolved from what it
    /// looks for (the ten bytes `$00`/`$01` point at, Mesen `Fds::ReadRam`), and the delayed and
    /// automatic disk changes. Nothing for a cartridge without a drive.
    #[cfg(feature = "board-fds")]
    #[cold]
    #[inline(never)]
    fn fds_end_frame(&mut self) {
        let Some(cartridge) = self.bus.cartridge() else {
            return;
        };
        if cartridge.disk_sides() == 0 {
            return;
        }
        let header = cartridge.disk_wanted().then(|| {
            let ram = self.bus.ram();
            let pointer = u16::from_le_bytes([ram[0], ram[1]]);
            std::array::from_fn(|index| {
                let address = pointer.wrapping_add(u16::try_from(index).unwrap_or(0));
                if address == 0xE445 {
                    0
                } else {
                    self.read_memory(address)
                }
            })
        });
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.disk_end_frame(header);
        }
    }

    /// Lot FDS: the disk sides of the loaded image's drive, 0 without one.
    #[must_use]
    pub fn disk_sides(&self) -> usize {
        self.bus
            .cartridge()
            .map_or(0, crate::cartridges::Cartridge::disk_sides)
    }

    /// The side in the drive (0 for disk 1 side A, 1 for side B, …), `None` when it is empty.
    #[must_use]
    pub fn disk_side(&self) -> Option<usize> {
        self.bus
            .cartridge()
            .and_then(crate::cartridges::Cartridge::disk_side)
    }

    /// Puts `side` in the drive; a disk in it comes out first and the new one goes in about two
    /// seconds later, as a player would do it.
    pub fn insert_disk(&mut self, side: usize) {
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.insert_disk(side);
        }
    }

    /// Takes the disk out of the drive.
    pub fn eject_disk(&mut self) {
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.eject_disk();
        }
    }

    /// Turns the other side of the disk in the drive over (side A to B, B to A).
    pub fn switch_disk_side(&mut self) {
        if let Some(side) = self.disk_side() {
            let sides = self.disk_sides();
            self.insert_disk((side ^ 1) % sides.max(1));
        }
    }

    /// Puts the next disk in (side A of the following disk, back to the first after the last).
    pub fn insert_next_disk(&mut self) {
        let sides = self.disk_sides();
        if sides > 0 {
            let side = self.disk_side().unwrap_or(sides - 1);
            self.insert_disk(((side & !1) + 2) % sides);
        }
    }

    /// Turns the drive's automatic disk changes on or off (on by default).
    pub fn set_disk_auto_insert(&mut self, enabled: bool) {
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.set_disk_auto_insert(enabled);
        }
    }

    /// Lot MAPPERS-P: whether the loaded image is mapper 532 with a font (*China Ersan 2*).
    #[cfg(feature = "board-532")]
    fn ersan2_active(&self) -> bool {
        self.bus.cartridge().is_some_and(|cartridge| {
            cartridge.mapper_number() == 532 && !Self::misc_rom(cartridge).is_empty()
        })
    }

    /// The NES 2.0 misc ROM of the loaded image (the bytes past its announced ROMs).
    #[cfg(feature = "board-532")]
    fn misc_rom(cartridge: &crate::cartridges::Cartridge) -> &[u8] {
        let rom = cartridge.rom();
        rom.raw()
            .get(rom.header().announced_size()..)
            .unwrap_or_default()
    }

    /// Steps the CPU, the PPU in step, until the PPU reaches the pre-render line (puNES copies
    /// RAM there, `ppu.c`), and returns CPU RAM `$00-$FF`. Only a 532 image runs this.
    #[cfg(feature = "board-532")]
    #[cold]
    #[inline(never)]
    fn run_to_pre_render(&mut self) -> Box<[u8; 256]> {
        loop {
            self.bus.sync_ppu();
            let ppu = self.bus.ppu();
            if ppu.scanline() == ppu.pre_render_scanline() || ppu.vblank_just_started() {
                break;
            }
            self.cpu
                .step(&mut KindBus::<crate::cartridges::AnyBoard>::new(
                    &mut self.bus,
                ));
        }
        let mut ram = Box::new([0u8; 256]);
        ram.copy_from_slice(&self.bus.ram()[..256]);
        ram
    }

    /// Paints mapper 532's glyphs on the finished picture ([`crate::ersan2_overlay`]).
    #[cfg(feature = "board-532")]
    #[cold]
    #[inline(never)]
    fn paint_ersan2(&mut self, ram: &[u8; 256]) {
        let mut pixels = self.bus.ppu_mut().take_pixels();
        if let Some(cartridge) = self.bus.cartridge() {
            let ppu = self.bus.ppu();
            let (palette, palette_raw) = ppu.palette_colors();
            let read = |address: u16| {
                let address = address & 0x3FFF;
                if address < 0x2000 {
                    cartridge.ppu_peek(address, crate::cartridges::ChrFetch::Background)
                } else if address < 0x3F00 {
                    ppu.read_nametable_byte(0x2000 | (address & 0x0FFF), Some(cartridge))
                } else {
                    palette_raw[usize::from(address & 0x1F)]
                }
            };
            crate::ersan2_overlay::apply(
                ram,
                Self::misc_rom(cartridge),
                &read,
                &palette,
                &mut pixels,
            );
        }
        self.bus.ppu_mut().restore_pixels(pixels);
    }

    /// Runs every frame through the path of every board, specialised board or not (lot CORE-2):
    /// the reference [`with_board_kind!`]'s paths are checked against. Off by default.
    #[doc(hidden)]
    pub const fn set_generic_path(&mut self, generic: bool) {
        self.generic_path = generic;
    }

    /// Runs every frame through the Fidélité+ loop (`docs/design/HALF-CYCLE.md`), off by
    /// default. Takes effect at the next frame.
    pub fn set_fidelity_plus(&mut self, enabled: bool) {
        self.fidelity_plus = enabled;
        if !enabled {
            self.bus.ppu_mut().set_fidelity_plus(false);
        }
    }

    /// Runs the frames through the fast core (lot FAST-1, `docs/design/FAST-CORE.md`) when the
    /// board has a fast path, off by default. Takes effect at the next frame; the state is the
    /// same for both cores. Fidélité+ wins when both are on.
    pub const fn set_fast_core(&mut self, enabled: bool) {
        self.fast_core = enabled;
    }

    /// Lot FAST-1: the next frames are not shown (a host's catch-up), so the fast core may leave
    /// their pixels unpainted; everything else runs as usual. The host turns it off before the
    /// frame it shows. No effect on the standard core.
    pub fn set_frame_hidden(&mut self, hidden: bool) {
        self.bus.ppu_mut().set_skip_pixels(hidden);
    }

    /// Whether the fast core runs the frames of the boards that have a fast path.
    #[must_use]
    pub const fn fast_core(&self) -> bool {
        self.fast_core
    }

    /// One frame through the fast core, when the board has a fast path: `false` (nothing run)
    /// otherwise, and the standard loop takes the frame.
    fn run_fast_frame(&mut self, idle_loop: Option<(u16, u16)>) -> bool {
        if self.generic_path || self.bus.is_vs_image() {
            return false;
        }
        if !self
            .bus
            .cartridge()
            .is_some_and(|cartridge| has_fast_path(cartridge.board()))
        {
            return false;
        }
        self.bus.defer_ppu();
        loop {
            let done = match self.bus.cartridge() {
                Some(cartridge) => with_fast_board_kind!(cartridge.board(), K => {
                    !matches!(
                        self.run_fast_frame_steps::<K>(idle_loop, i64::MAX),
                        SliceOutcome::Budget
                    )
                }, self.run_frame_steps::<AnyBoard>(idle_loop)),
                None => true,
            };
            if done {
                break;
            }
        }
        self.bus.end_deferred_ppu();
        true
    }

    /// [`NesCore::run_frame_steps`] for the fast core: the CPU steps a [`FastBus`], whose counted
    /// cycles are handed to the machine before the frame's loop looks at it.
    ///
    /// `until`: the cycle a real-time slice ends on (`i64::MAX` for a whole frame); the steps
    /// stop at the first instruction that reaches it, as [`NesCore::run_slice_steps`] does, and
    /// no idle loop is counted past it.
    fn run_fast_frame_steps<K: BoardKind>(
        &mut self,
        idle_loop: Option<(u16, u16)>,
        until: i64,
    ) -> SliceOutcome {
        self.bus.flush_fast_as::<K>();
        // The program counters of the last three instructions: a loop of up to three
        // instructions shows as the counter coming back to one of them.
        let mut last_pcs = [u16::MAX; 3];
        loop {
            let pc = self.cpu.pc;
            if last_pcs.contains(&pc) && self.cpu.quiet_boundary() {
                match self.idle_loop(pc) {
                    Some(IdleLoop::Steady(period)) => {
                        self.bus.skip_idle_cycles(period, i64::MAX);
                    }
                    Some(IdleLoop::Countdown {
                        period,
                        index_y,
                        turns,
                    }) => {
                        let done = self.bus.skip_idle_cycles(period, turns);
                        self.cpu.count_down(index_y, done);
                    }
                    Some(IdleLoop::StatusPoll(period)) => self.bus.skip_status_poll::<K>(period),
                    Some(IdleLoop::ZeroHitPoll {
                        period,
                        waits_for_hit,
                    }) => self.bus.skip_zero_hit_poll::<K>(period, 4, waits_for_hit),
                    None => {}
                }
            }
            last_pcs = [pc, last_pcs[0], last_pcs[1]];
            self.cpu.step(&mut crate::nes_bus::FastBus::<K>::new(&mut self.bus));
            if let Some((first, last)) = idle_loop {
                if (first..=last).contains(&self.cpu.pc) && self.bus.ppu().wide().stalling() {
                    self.bus.flush_fast_as::<K>();
                    self.bus.ppu_mut().wide_mut().end_stall();
                    self.bus.flush_fast_as::<K>();
                }
            }
            if self.bus.ppu().vblank_just_started() {
                self.bus.flush_fast_as::<K>();
                return SliceOutcome::FrameDone;
            }
            if self.bus.cycles_with_pending() >= until {
                self.bus.flush_fast_as::<K>();
                return SliceOutcome::Budget;
            }
        }
    }

    /// The idle loop at `pc`, when it is one the fast core may repeat by counting (lot FAST-1).
    /// All of it in plain PRG-ROM, so that a turn reads nothing but RAM, PRG-ROM or `$2002`:
    ///
    /// * `JMP` to itself, or a load of a RAM byte and a conditional branch back to it that the
    ///   byte makes taken ([`IdleLoop::Steady`]): a turn leaves the machine as it found it;
    /// * `DEX`/`DEY` and `BNE` back to it ([`IdleLoop::Countdown`]): a turn counts the register
    ///   down, and the turns before the one that falls through can be counted at once;
    /// * `LDA $2002` or `BIT $2002` and `BPL` back to it ([`IdleLoop::StatusPoll`]): until the
    ///   vblank flag rises a turn only repeats what the turn before did to the PPU.
    ///
    /// `pc` may stand on the branch of a two-instruction steady loop: the loop is then read from
    /// its start, one turn being the same length from either of its instructions.
    fn idle_loop(&self, pc: u16) -> Option<IdleLoop> {
        let byte = |address: u16| self.bus.rom_byte(address);
        if let Some(found) = self.zero_hit_loop(pc) {
            return Some(found);
        }
        let first = byte(pc)?;
        let start = if matches!(first, 0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xB0 | 0xD0 | 0xF0) {
            #[allow(clippy::cast_possible_wrap)] // The signed branch offset.
            let offset = i16::from(byte(pc.wrapping_add(1))? as i8);
            let target = pc.wrapping_add(2).wrapping_add_signed(offset);
            if target != pc.wrapping_sub(2) && target != pc.wrapping_sub(3) {
                return None;
            }
            target
        } else {
            pc
        };
        let at = |offset: u16| byte(start.wrapping_add(offset));
        let opcode = at(0)?;
        if opcode == 0x4C {
            let target = u16::from_le_bytes([at(1)?, at(2)?]);
            return (target == start).then_some(IdleLoop::Steady(3));
        }
        let (length, load) = match opcode {
            0xA5 => (2, 3),
            0xAD | 0x2C => (3, 4),
            0x88 | 0xCA => (1, 2),
            _ => return None,
        };
        let branch = start.wrapping_add(length);
        let condition = byte(branch)?;
        let after = branch.wrapping_add(2);
        #[allow(clippy::cast_possible_wrap)] // The signed branch offset.
        let offset = i16::from(byte(branch.wrapping_add(1))? as i8);
        let target = after.wrapping_add_signed(offset);
        if target != start {
            return None;
        }
        let period = load + 3 + i64::from((after & 0xFF00) != (target & 0xFF00));
        match opcode {
            0x88 | 0xCA => {
                if condition != 0xD0 || start != pc {
                    return None;
                }
                let index_y = opcode == 0x88;
                let value = if index_y { self.cpu.y } else { self.cpu.x };
                // The turns that branch back: the one that brings the register to 0 falls through.
                let turns = if value == 0 { 255 } else { i64::from(value) - 1 };
                (turns > 0).then_some(IdleLoop::Countdown {
                    period,
                    index_y,
                    turns,
                })
            }
            _ => {
                let address = if opcode == 0xA5 {
                    u16::from(at(1)?)
                } else {
                    u16::from_le_bytes([at(1)?, at(2)?])
                };
                if address == 0x2002 && opcode != 0xA5 {
                    return (condition == 0x10 && start == pc)
                        .then_some(IdleLoop::StatusPoll(period));
                }
                if opcode == 0x2C || address >= 0x2000 {
                    return None;
                }
                let value = self.bus.ram_byte(address);
                let taken = match condition {
                    0xF0 => value == 0,
                    0xD0 => value != 0,
                    0x10 => value & 0x80 == 0,
                    0x30 => value & 0x80 != 0,
                    _ => return None,
                };
                taken.then_some(IdleLoop::Steady(period))
            }
        }
    }

    /// A loop on the sprite 0 flag starting at `pc` (lot FAST-1): `LDA $2002`, `AND #$40` and
    /// `BEQ`/`BNE` back, or `BIT $2002` and `BVC`/`BVS` back, all in plain PRG-ROM. `BEQ` and
    /// `BVC` wait for the hit, `BNE` and `BVS` for the flag to clear.
    fn zero_hit_loop(&self, pc: u16) -> Option<IdleLoop> {
        let byte = |offset: u16| self.bus.rom_byte(pc.wrapping_add(offset));
        let (branch, waits_for_hit, load) = match (byte(0)?, byte(1)?, byte(2)?) {
            (0xAD, 0x02, 0x20) if byte(3)? == 0x29 && byte(4)? == 0x40 => match byte(5)? {
                0xF0 => (5, true, 6),
                0xD0 => (5, false, 6),
                _ => return None,
            },
            (0x2C, 0x02, 0x20) => match byte(3)? {
                0x50 => (3, true, 4),
                0x70 => (3, false, 4),
                _ => return None,
            },
            _ => return None,
        };
        let after = pc.wrapping_add(branch + 2);
        #[allow(clippy::cast_possible_wrap)] // The signed branch offset.
        let target = after.wrapping_add_signed(i16::from(byte(branch + 1)? as i8));
        if target != pc {
            return None;
        }
        let period = load + 3 + i64::from((after & 0xFF00) != (target & 0xFF00));
        Some(IdleLoop::ZeroHitPoll {
            period,
            waits_for_hit,
        })
    }

    /// Whether the Fidélité+ loop runs the frames.
    #[must_use]
    pub const fn fidelity_plus(&self) -> bool {
        self.fidelity_plus
    }

    /// The instructions of one Fidélité+ frame on the board kind `K`: the CPU steps a
    /// [`FidelityBus`], the PPU runs on every cycle. Same returns as
    /// [`NesCore::run_frame_steps`].
    fn run_fidelity_plus_steps<K: BoardKind>(&mut self, idle_loop: Option<(u16, u16)>) -> bool {
        loop {
            self.cpu.step(&mut FidelityBus::<K>::new(&mut self.bus));
            if let Some((first, last)) = idle_loop {
                if (first..=last).contains(&self.cpu.pc) && self.bus.ppu().wide().stalling() {
                    self.bus.ppu_mut().wide_mut().end_stall();
                }
            }
            if self.bus.ppu().vblank_just_started() {
                return true;
            }
            if K::SWITCHES
                && !self
                    .bus
                    .cartridge()
                    .is_some_and(|cartridge| K::matches(cartridge.board()))
            {
                return false;
            }
        }
    }

    /// The instructions of one frame, on the board kind `K` (lot CORE-2): the bus is lent to the
    /// CPU as a [`KindBus`] so that every cartridge call of the frame reaches `K`'s board.
    /// Returns `false` when the board changed variant (see [`BoardKind::SWITCHES`]) before the
    /// frame was over, for `run_frame` to go on with the right kind.
    fn run_frame_steps<K: BoardKind>(&mut self, idle_loop: Option<(u16, u16)>) -> bool {
        loop {
            self.cpu.step(&mut KindBus::<K>::new(&mut self.bus));
            if let Some((first, last)) = idle_loop {
                if (first..=last).contains(&self.cpu.pc) && self.bus.ppu().wide().stalling() {
                    self.bus.ppu_mut().wide_mut().end_stall();
                }
            }
            if self.bus.ppu().vblank_just_started() {
                return true;
            }
            if K::SWITCHES
                && !self
                    .bus
                    .cartridge()
                    .is_some_and(|cartridge| K::matches(cartridge.board()))
            {
                return false;
            }
        }
    }

    // ----------------------------------------------------------------- run-ahead

    /// Largest run-ahead the core accepts (AMD-041 REQ-LOOP-064).
    ///
    /// Rust-only (owner request): 10 instead of the 3 of AMD-041, to try a game's own lag to the
    /// absurd; every frame of it costs a whole emulated frame.
    pub const MAX_RUN_AHEAD: u8 = 10;

    /// Runs the frames one **real** frame of run-ahead is made of (AMD-041 REQ-LOOP-065) and
    /// leaves the core in the state of the **displayed** frame, the last one in advance.
    ///
    /// With `ahead` = 0 this is [`NesCore::run_frame`] and nothing else: the same call, the same
    /// frame, no state copied, no flag moved. That is the whole of the N = 0 path, so a host that
    /// never asks for run-ahead pays nothing for it.
    ///
    /// With `ahead` ≥ 1, and the input word `input_word` read once by the host (step (a) of
    /// REQ-LOOP-060) and used by every frame of the sequence:
    /// 1. the real frame runs, **without sound** and without being handed to the host;
    /// 2. its complete state is kept in memory (REQ-CORE-121);
    /// 3. `ahead − 1` more frames in advance run, still silent and unhanded;
    /// 4. one last frame in advance runs **with** its sound and its picture: the displayed frame,
    ///    whose video callback fires and whose batch and pixels the host then reads;
    /// 5. the real state goes back — but from [`NesCore::restore_real_frame`], which the host
    ///    calls once it has finished reading the displayed frame. REQ-LOOP-067 allows exactly
    ///    that ("retarde la restauration après (d)"), and it is what lets the conversion, the
    ///    2D picture and `Script:ReadMem` see the displayed frame with nothing copied.
    ///
    /// Returns the CPU cycles the **displayed** frame took, 0 when no ROM is loaded.
    ///
    /// # Panics
    /// Never: the capture can only fail with no ROM loaded, which is checked first.
    pub fn run_frame_ahead(&mut self, input_word: u16, ahead: u8) -> i64 {
        // A frame begun in slices (lot FIDPLUS-B) is finished first, without run-ahead.
        if ahead == 0 || self.slice.is_some() {
            return self.run_frame(input_word);
        }

        self.run_ahead_sequence(input_word, ahead)
    }

    /// Steps 1 to 4 of REQ-LOOP-065, out of line so that nothing of them reaches the N = 0 path.
    #[inline(never)]
    fn run_ahead_sequence(&mut self, input_word: u16, ahead: u8) -> i64 {
        if self.bus.cartridge().is_none() {
            return 0;
        }
        // A host that forgot its restore: the real state is put back first, so that it can never
        // be lost by a displayed frame the host dropped.
        self.restore_real_frame();

        // Steps 1 and 3: the real frame and the frames between, silent and unhanded (and, on the
        // fast core, unpainted: lot FAST-1).
        let callback = self.video_callback.take();
        self.bus.apu_mut().set_output_enabled(false);
        self.bus.ppu_mut().set_skip_pixels(true);

        self.run_frame(input_word);

        // Step 2: the complete state of the real frame, in memory. The buffer is lent out and
        // put back, which moves three words and allocates nothing.
        let mut state = std::mem::take(&mut self.real_frame);
        let captured = self.capture_core_state(&mut state).is_ok();
        self.real_frame = state;
        self.real_frame_held = captured;
        debug_assert!(captured, "a ROM is loaded, so the capture cannot fail");

        for _ in 1..ahead.min(Self::MAX_RUN_AHEAD) {
            self.run_frame(input_word);
        }

        // Step 4: the displayed frame, with its sound and its picture.
        self.bus.ppu_mut().set_skip_pixels(false);
        self.bus.apu_mut().set_output_enabled(true);
        self.video_callback = callback;
        self.run_frame(input_word)
    }

    /// Step 5 of REQ-LOOP-065: puts the state of the real frame back, the sound the displayed
    /// frame produced left untouched (REQ-CORE-121).
    ///
    /// Returns whether a state was held — false, and nothing done at all, when the frame before
    /// was an ordinary one. The host calls it once it has finished reading the displayed frame;
    /// with run-ahead off it is one `bool` test per real frame and nothing more.
    pub fn restore_real_frame(&mut self) -> bool {
        if !self.real_frame_held {
            return false;
        }

        self.real_frame_held = false;
        let state = std::mem::take(&mut self.real_frame);
        let restored = self.restore_core_state(&state).is_ok();
        self.real_frame = state;
        debug_assert!(restored, "the state was captured from this very core");
        restored
    }

    /// Whether a run-ahead sequence still owes its restore (the core is in the displayed frame's
    /// state, not the real one).
    #[must_use]
    pub const fn holds_a_real_frame(&self) -> bool {
        self.real_frame_held
    }

    /// Forgets the state of a real frame without restoring it: the machine the state belonged to
    /// is gone. Loading a savestate, resetting and changing game all go through here, and the
    /// next real frame then starts a sequence of its own (REQ-LOOP-066).
    fn forget_real_frame(&mut self) {
        self.real_frame_held = false;
        // A sliced frame in progress belongs to the machine that was replaced (lot FIDPLUS-B).
        self.slice = None;
    }

    /// Whether a frame has any smooth scrolling to do: the game's own aid, the host's option for
    /// every game, or — once, after either is turned off — a frame to put the state back.
    ///
    /// Three loads and two `or`s, and it is what every game that wants none of this pays per
    /// frame. See the note at the call site in [`NesCore::end_frame`] for what the per-frame
    /// question cost before.
    #[must_use]
    const fn smooth_scroll_wanted(&self) -> bool {
        self.smooth_scroll_on || self.smooth_ahead || self.smooth_scroll_live
    }

    /// The smooth scrolling of one frame, **out of line**: the margin decoder of a pack is found
    /// by a sixty-arm match on the patch's name, and neither that nor the two passes below has
    /// any business inside `run_frame` for a game that never reaches them.
    #[inline(never)]
    fn smooth_scroll_frame(&mut self, decoder: Option<&dyn crate::wide_decode::MarginDecoder>) {
        // The smooth scrolling is an aid: without the widescreen (its built-in patches off), the
        // decoder of the game's hack pack reads the camera and finds the playfield's bottom.
        let aid_decoder = decoder.or_else(|| {
            self.pack
                .as_ref()
                .and_then(|(patch, _)| crate::wide_decode::for_patch(patch.name))
        });
        if decoder.is_none() {
            if let Some(aid_decoder) = aid_decoder {
                self.find_playfield_end(aid_decoder);
            }
        }
        self.smooth_scroll(aid_decoder);
        self.smooth_scroll_ahead();
        // The frame that turns everything off has run; the next one need not come back here.
        self.smooth_scroll_live = self.smooth_scroll_on || self.smooth_ahead;
    }

    /// Works out everything the frame path reads of the patch that runs: its margin decoder,
    /// whether the game's `smooth_scroll` aid is on, and where the game waits for the next
    /// frame. None of the three can change inside a frame, and each costs a walk of a long
    /// match or a scan of `builtin_patches::patches()` to find; `end_frame` worked all three out
    /// once a frame until lot KIRBY-MERGE step 4, which is where the lot's whole regression on
    /// the five commercial games came from. This runs at every point an answer can move: a ROM
    /// loaded or closed, a pack installed, the options set, `emu.builtin_patches` turned.
    fn refresh_patch_cache(&mut self) {
        let on = self.hack_option_on("smooth_scroll");
        if on && !self.smooth_scroll_on {
            self.smooth_scroll_live = true;
        }
        self.smooth_scroll_on = on;
        self.idle_loop = self
            .options_patch_of_the_rom()
            .and_then(|patch| patch.idle_loop);
        self.decoder = self.find_margin_decoder();
    }

    /// The built-in patch whose properties the frame path reads: the one applied, or the one
    /// whose hack pack gives the ROM its aids.
    fn options_patch_of_the_rom(&self) -> Option<&'static crate::builtin_patches::BuiltinPatch> {
        self.builtin_patch
            .and_then(crate::builtin_patches::by_name)
            .or_else(|| self.pack.as_ref().map(|(patch, _)| *patch))
    }

    /// Has `decoder` fill the margins of the frame just run from the game's memory.
    /// The experimental smooth scrolling: with the option `smooth_scroll` of the patch running
    /// on, the picture just made is shown shifted left by the fraction of a pixel its camera was
    /// past its scroll (read after the frame before: the RAM is a frame ahead of the picture), down
    /// to the playfield's bottom ([`NesCore::picture_shift`]). Nothing read with the option off.
    fn smooth_scroll(&mut self, decoder: Option<&dyn crate::wide_decode::MarginDecoder>) {
        // An aid of the game's hack pack (in the Hacks), with or without its widescreen patch: the
        // camera its code reports, else the one a margin decoder reads. Worked out when the
        // options change, not here: see `refresh_smooth_scroll`.
        let on = self.smooth_scroll_on;
        self.picture_motion = None;
        let report = self.bus.ppu_mut().wide_mut().take_camera_report();
        self.bus
            .ppu_mut()
            .set_wants_playfield_end(on && decoder.is_some());
        let tracked = on || self.smooth_ahead;
        self.bus.ppu_mut().set_tracks_line_scroll(tracked);
        if !on {
            self.camera_fraction = None;
            self.camera_read = None;
            self.report_age = None;
            self.picture_shift = None;
            self.steady_sprites.clear();
            return;
        }
        // The camera where the picture just made shows it, before the next one is read.
        let shown = self
            .camera_read
            .zip(self.camera_fraction)
            .map(|((pixels, _), fraction)| f64::from(pixels) + f64::from(fraction) / 256.0);
        self.picture_camera = shown;
        // The band that scrolls: as the PPU drew it, its bottom the report's when it gives one;
        // else from the top down to the decoder's bottom.
        let band = self.bus.ppu().playfield_scroll();
        let start = band.map_or(0, |(_, start, _)| start);
        let end = report
            .and_then(|report| report.end)
            .or_else(|| band.map(|(_, _, end)| end))
            .or_else(|| self.bus.ppu().playfield_end());
        self.picture_shift = self.camera_fraction.zip(end).map(|(fraction, end)| {
            (
                f32::from(fraction) / 256.0,
                i32::from(start),
                i32::from(end),
            )
        });
        self.find_steady_sprites(self.camera_read.map(|(pixels, _)| pixels));
        let read = if let Some(report) = report {
            self.report_age = Some(0);
            report.x
        } else if let Some(age) = self.report_age.filter(|age| *age < 2) {
            // A frame the game ran late reports nothing: its camera did not move.
            self.report_age = Some(age + 1);
            self.camera_read
        } else {
            self.report_age = None;
            let (ram, _, cartridge) = self.bus.decoder_parts();
            decoder.zip(cartridge).and_then(|(decoder, cartridge)| {
                decoder.camera_position(&crate::wide_decode::DecoderMemory {
                    ram,
                    prg_ram: cartridge.memory().prg_ram(),
                    prg_rom: cartridge.rom().prg_rom(),
                    nametable: &|_| 0,
                    nametable_written: &|_| 0,
                })
            })
        };
        let Some((pixels, fraction)) = read else {
            self.camera_fraction = None;
            self.camera_read = None;
            return;
        };
        // A fraction that goes round 256 with the pixels still is no longer the camera's (Kirby
        // turning round, the camera at rest): its last value is kept until the pixels move, the
        // picture still rather than shaking.
        match self.camera_read {
            Some((last, _)) if last != pixels => self.camera_decoupled = false,
            Some((_, last)) if fraction.abs_diff(last) > 128 => self.camera_decoupled = true,
            None => self.camera_decoupled = false,
            _ => {}
        }
        self.camera_read = Some((pixels, fraction));
        if !self.camera_decoupled || self.camera_fraction.is_none() {
            self.camera_fraction = Some(fraction);
        }
        // Where the next picture will show it: the motion between the two, a jump (a door, a
        // room changed) left out.
        let next = self
            .camera_fraction
            .map(|fraction| f64::from(pixels) + f64::from(fraction) / 256.0);
        #[allow(clippy::cast_possible_truncation)] // A few pixels.
        {
            self.picture_motion = shown
                .zip(next)
                .map(|(shown, next)| (next - shown) as f32)
                .filter(|motion| motion.abs() < 4.0);
        }
    }

    /// With the option `smooth_scroll_120`, how far the camera moves from the picture of the frame
    /// just run to the next, in pixels: a host that shows the picture several times draws its
    /// playfield that much further along ([`NesCore::picture_shift`] plus the motion times the
    /// part of the frame gone by). `None` with the option off or the camera jumping.
    #[must_use]
    pub const fn picture_motion(&self) -> Option<f32> {
        self.picture_motion
    }

    /// The camera the picture of the frame just run shows, in pixels and fractions of one (with
    /// the smooth scrolling on), for a host's debug log.
    #[must_use]
    pub const fn picture_camera(&self) -> Option<f64> {
        self.picture_camera
    }

    /// The sprites of the picture just made that move with the camera: a sprite at the same
    /// place on the screen as in the picture before while the camera moved (or, the camera still,
    /// at a place found so before). The shift leaves them where they are.
    /// Turns the smooth scrolling from the run-ahead on or off (an option of the host for every
    /// game; the game's own smooth scrolling from its memory, when on, comes first).
    pub fn set_smooth_scroll_ahead(&mut self, on: bool) {
        if on && !self.smooth_ahead {
            self.smooth_scroll_live = true;
        }
        self.smooth_ahead = on;
    }

    /// Whether option `key` of the game's patch, or of its hack pack's aids, is on.
    fn hack_option_on(&self, key: &str) -> bool {
        self.options_patch_of_the_rom()
            .and_then(|patch| patch.options.iter().find(|option| option.key == key))
            .is_some_and(|option| self.host_options & (1 << (option.bit & 15)) != 0)
    }

    /// The experimental smooth scrolling from the run-ahead (option `smooth_scroll_ahead`), to
    /// compare with the one that reads the game's camera: nothing of the game known but the
    /// scroll the PPU drew. With run-ahead, the frame shown is one ahead of the real frame run
    /// just before it, whose scroll is known: the picture is shown shifted back to the real
    /// frame's scroll, then slides to its own over the frame (the motion times the display's
    /// phase), the run-ahead's frame of latency spent on it. Without run-ahead, the same a frame
    /// late.
    fn smooth_scroll_ahead(&mut self) {
        // The game's own camera, when its hack reads it, comes first (owner request).
        if !self.smooth_ahead || self.smooth_scroll_on {
            self.last_scroll = None;
            return;
        }
        let current = self.bus.ppu().playfield_scroll();
        let previous =
            std::mem::replace(&mut self.last_scroll, current.map(|(scroll, _, _)| scroll));
        let Some((scroll, start, end)) = current else {
            self.picture_shift = None;
            self.picture_motion = None;
            self.steady_sprites.clear();
            return;
        };
        // A few pixels a frame at most; more is a jump (a door, a room changed), not shown moving.
        let motion = previous
            .map(|previous| (scroll - previous + 256).rem_euclid(512) - 256)
            .filter(|motion| motion.abs() <= 8)
            .unwrap_or(0);
        #[allow(clippy::cast_precision_loss)] // A few pixels.
        let motion = motion as f32;
        self.picture_shift = Some((-motion, i32::from(start), i32::from(end)));
        self.picture_motion = Some(motion);
        self.find_steady_sprites(Some(scroll));
    }

    fn find_steady_sprites(&mut self, shown: Option<i32>) {
        let moved = shown.is_some() && self.shown_camera.is_some() && shown != self.shown_camera;
        self.shown_camera = shown;
        let band = self
            .picture_shift
            .map_or(0..0, |(_, start, end)| start..end);
        let all = std::mem::take(&mut self.sprite_places);
        let steady = std::mem::take(&mut self.steady_places);
        let reference = if moved { all } else { steady };
        self.steady_sprites.clear();
        for record in self.bus.ppu().tile_records() {
            if record.background {
                continue;
            }
            let place = (record.x, record.y);
            self.sprite_places.push(place);
            if band.contains(&i32::from(record.y)) && reference.contains(&place) {
                self.steady_sprites.push(record.index);
                self.steady_places.push(place);
            }
        }
    }

    /// The sprite slots of the picture just made that move with the camera (see
    /// [`NesCore::split_steady_sprites`]): the 3D's smooth scrolling leaves their tiles in place.
    #[must_use]
    pub fn steady_sprites(&self) -> &[i32] {
        if self.picture_shift.is_none() {
            return &[];
        }
        &self.steady_sprites
    }

    /// Moves the sprites that move with the camera out of `picture` (the wide picture of the
    /// frame just run) into `overlay`, transparent (`0xFF`) elsewhere, for the experimental
    /// smooth scrolling; `overlay` is left empty while the playfield is not shifted or no sprite
    /// moves with the camera.
    pub fn split_steady_sprites(&self, picture: &mut [u8], overlay: &mut Vec<u8>) {
        if self.picture_shift.is_none() || self.steady_sprites.is_empty() {
            overlay.clear();
            return;
        }
        self.bus
            .ppu()
            .split_steady_sprites(picture, overlay, &self.steady_sprites);
    }

    /// The experimental smooth scrolling's shift of the picture of the frame just run: the
    /// fraction of a pixel (0 to 1) to move the playfield left by, and the picture line (screen
    /// lines, the vertical margin excluded) it ends at; `None` while the option is off or the
    /// picture has no playfield the decoder knows.
    #[must_use]
    pub const fn picture_shift(&self) -> Option<(f32, i32, i32)> {
        self.picture_shift
    }

    /// The playfield's bottom alone, for the smooth scrolling without margins (nothing read
    /// unless it was turned on: [`crate::ppu::Ppu2C02::set_wants_playfield_end`]).
    fn find_playfield_end(&mut self, decoder: &dyn crate::wide_decode::MarginDecoder) {
        let (ram, ppu, cartridge) = self.bus.decoder_parts();
        let Some(cartridge) = cartridge else {
            return;
        };
        let memory = crate::wide_decode::DecoderMemory {
            ram,
            prg_ram: cartridge.memory().prg_ram(),
            prg_rom: cartridge.rom().prg_rom(),
            nametable: &|_| 0,
            nametable_written: &|_| 0,
        };
        ppu.find_playfield_end(decoder, &memory, Some(cartridge));
    }

    #[inline(never)]
    fn decode_margins(&mut self, decoder: &dyn crate::wide_decode::MarginDecoder) {
        let (ram, ppu, cartridge) = self.bus.decoder_parts();
        let Some(cartridge) = cartridge else {
            return;
        };
        let memory = crate::wide_decode::DecoderMemory {
            ram,
            prg_ram: cartridge.memory().prg_ram(),
            prg_rom: cartridge.rom().prg_rom(),
            // The PPU puts its own readers in.
            nametable: &|_| 0,
            nametable_written: &|_| 0,
        };
        ppu.decode_margins(decoder, &memory, Some(cartridge));
    }

    /// Unlocks the widescreen extension with its PRG window open when the built-in patch named
    /// `patch` asks for it (`BuiltinPatch::boot_window`).
    fn boot_patch_window(&mut self, patch: Option<&'static str>) {
        let known = patch.and_then(crate::builtin_patches::by_name);
        if let Some(((bank, page), keeps_prg_ram)) =
            known.and_then(|known| Some((known.boot_window?, known.boot_window_keeps_prg_ram)))
        {
            self.bus
                .ppu_mut()
                .wide_mut()
                .boot(bank, page, !keeps_prg_ram);
        }
        if let Some(known) = known.filter(|known| known.boot_zone) {
            self.bus.ppu_mut().wide_mut().boot_zone(known.zone_data);
        }
    }

    /// Runs the `after_other_way_state` of `patch` (the built-in patch made for the ROM of a
    /// state restored the other way), then the redraw recipe of its pack, on CPU RAM; a state of
    /// the hack restored without it (`to_off`) also loses the hack's return (its pack's unwind).
    fn put_right_other_way_state(
        &mut self,
        patch: Option<&'static crate::builtin_patches::BuiltinPatch>,
        to_off: bool,
    ) {
        let Some(known) = patch else {
            return;
        };
        if let Some(fix) = known.after_other_way_state {
            fix(self.bus.ram_mut());
        }
        let states = known.states();
        if to_off {
            states.unwind(self.bus.ram_mut(), &mut self.cpu.s);
        }
        states.redraw(self.bus.ram_mut());
    }

    /// The margin decoder of the game loaded: its built-in patch's, or one made for the ROM
    /// itself, or that of the patch whose hack pack the ROM has the signatures of (another
    /// version of the game, a ROM hack); none while the built-in patches are off.
    fn margin_decoder(&self) -> Option<&'static dyn crate::wide_decode::MarginDecoder> {
        self.decoder
    }

    /// Finds the margin decoder of the ROM loaded, for [`NesCore::refresh_patch_cache`].
    ///
    /// `wide_decode::for_patch` is a match on **string literals**, one arm per patch of every
    /// game that has a decoder, and `for_rom` is a match on CRCs; `end_frame` called this once a
    /// frame until lot KIRBY-MERGE step 4, so every ROM with a built-in patch walked that chain
    /// 60 times a second. The branch added seven arms to `for_patch` and two to `for_rom` for
    /// Kirby's builds, which made the walk longer for games that have nothing to do with Kirby:
    /// `metroid` **+24 M instructions over 300 frames** (+0.52 %), `smb1` +23.2 M, `z2` +19.9 M,
    /// `smb3` +20.5 M, `mm2` +19.9 M. The answer changes only when the ROM, the pack or
    /// `emu.builtin_patches` does.
    fn find_margin_decoder(&self) -> Option<&'static dyn crate::wide_decode::MarginDecoder> {
        if !self.builtin_patches_enabled {
            return None;
        }
        self.builtin_patch
            .and_then(crate::wide_decode::for_patch)
            .or_else(|| self.rom_crc.and_then(crate::wide_decode::for_rom))
            .or_else(|| {
                self.pack
                    .as_ref()
                    .and_then(|(patch, _)| crate::wide_decode::for_patch(patch.name))
            })
    }

    /// The batch of the frame just run (REQ-CORE-030).
    #[must_use]
    pub fn frame_batch(&self) -> FrameBatch<'_> {
        Self::batch_of(self.bus.ppu())
    }

    fn batch_of(ppu: &Ppu2C02) -> FrameBatch<'_> {
        FrameBatch {
            records: ppu.tile_records(),
            palette: ppu.snapshot_palette(),
            margin_palette: ppu.margin_palette(),
            mask_x: ppu.mask_x(),
            mask_y: ppu.mask_y(),
            margin: ppu.frame_margin(),
            visible_rows: ppu.visible_rows(),
            margin_y: ppu.frame_vertical_margin(),
        }
    }

    /// Sets the widescreen margin the host offers on each side, in pixels (0 = off). A ROM sees
    /// it only through the widescreen extension it has to unlock; nothing changes for the
    /// others. Rounded down to a multiple of 8 and capped at [`crate::wide::MAX_MARGIN`].
    pub fn set_widescreen_margin(&mut self, pixels: i32) {
        self.bus.ppu_mut().wide_mut().set_host_margin(pixels);
    }

    /// Sets whether the picture shows more than eight sprites on a line (the 2D picture; the
    /// tile records of the 3D view have every sprite whatever the choice).
    pub fn set_sprite_limit(&mut self, limit: crate::ppu::SpriteLimit) {
        self.bus.ppu_mut().set_sprite_limit(limit);
    }

    /// Sets the options of the built-in patch the player turned on, one bit each as the patch's
    /// `options` say; the ROM reads the low byte at `$500A` (bits 8-15: aids only).
    pub fn set_hack_options(&mut self, options: u16) {
        let [rom_bits, _] = options.to_le_bytes();
        self.bus.ppu_mut().wide_mut().set_host_options(rom_bits);
        self.host_options = options;
        self.sync_aids();
        // One of the bits just written may be the `smooth_scroll` aid's; the frame path reads
        // the answer and never works it out itself (lot KIRBY-MERGE step 4).
        self.refresh_patch_cache();
    }

    /// Applies to the cartridge's PRG-ROM the hooks of the pack's aids whose option is on, and
    /// puts the game's bytes back where it is off (see [`crate::hack_pack`]).
    fn sync_aids(&mut self) {
        let Some((patch, pack)) = &self.pack else {
            return;
        };
        let Some(cartridge) = self.bus.cartridge_mut() else {
            return;
        };
        let mut turned_off = false;
        for option in patch.options {
            let Some(component) = pack.component(option.key) else {
                continue;
            };
            let mask = 1 << (option.bit & 15);
            let on = self.host_options & mask != 0;
            if on != (self.aids_on & mask != 0) {
                component.apply(cartridge.rom_mut().prg_rom_mut(), on);
                self.aids_on ^= mask;
                turned_off |= !on;
            }
        }
        // An aid's code may have asked for the overclock, which nothing would clear once its
        // hooks are gone; a widescreen running sets it again every frame.
        if turned_off {
            self.bus.ppu_mut().wide_mut().clear_overclock();
        }
        self.boot_pack_zone();
    }

    /// Opens the extension's execution zone on the pack's code while its widescreen runs or one
    /// of its aids is on, with or without the widescreen: an aid's hooks may call the pack's code
    /// (a level selector, an invincibility that needs the hack's code). The zone shows the code
    /// to instruction fetches only, which no game reaches but through the pack's hooks.
    fn boot_pack_zone(&mut self) {
        let Some((patch, pack)) = &self.pack else {
            return;
        };
        let running = self.builtin_patch == Some(patch.name);
        if !patch.boot_zone || pack.code.is_empty() || !(running || self.aids_on != 0) {
            return;
        }
        let wide = self.bus.ppu_mut().wide_mut();
        if wide.code() != pack.code {
            wide.set_code(pack.code);
        }
        if !wide.unlocked() {
            wide.boot_zone(patch.zone_data);
        }
    }

    /// After a cartridge is attached for `image` (the host's, before any patch): the hack pack
    /// made for it, its code in the extension's window while its widescreen runs, its aids as
    /// their options say. Without an image (a state that has none), the pack stays as it was.
    fn install_pack(&mut self, image: Option<&[u8]>) {
        let Some(image) = image else {
            return;
        };
        self.pack = crate::builtin_patches::pack_for(image);
        self.aids_on = 0;
        let running = match (&self.pack, self.builtin_patch) {
            (Some((patch, pack)), Some(name)) if patch.name == name => Some((*patch, pack.code)),
            _ => None,
        };
        let wide = self.bus.ppu_mut().wide_mut();
        wide.set_code(running.map_or(&[], |(_, code)| code));
        // The window on the pack's code, whatever a state left it as (a state of the hack built
        // as a BPS patch had it on the bank that patch added).
        if let Some(((bank, page), keeps_prg_ram)) = running
            .and_then(|(patch, _)| Some((patch.boot_window?, patch.boot_window_keeps_prg_ram)))
        {
            wide.boot(bank, page, !keeps_prg_ram);
        }
        // And on its execution zone, for a pack whose code runs there (sync_aids opens it, its
        // widescreen running or one of its aids on).
        if let Some((patch, _)) = running.filter(|(patch, _)| patch.boot_zone) {
            wide.boot_zone(patch.zone_data);
        }
        self.sync_aids();
        // The pack just changed, so the patch whose options count may have too.
        self.refresh_patch_cache();
    }

    /// The built-in patch whose options apply to the ROM loaded: the one applied, or, without it,
    /// the one whose hack pack gives the ROM its aids (see [`Self::hack_options`]).
    #[must_use]
    pub fn options_patch(&self) -> Option<&'static str> {
        self.builtin_patch
            .or_else(|| self.pack.as_ref().map(|(patch, _)| patch.name))
    }

    /// The options of the built-in patch applied to the ROM loaded (none without one).
    #[must_use]
    pub fn hack_options(&self) -> Vec<crate::builtin_patches::HackOption> {
        if let Some(patch) = self.builtin_patch.and_then(crate::builtin_patches::by_name) {
            return patch.options.to_vec();
        }
        // Without the patch running, the options of a hack pack's aids (each turned on apart).
        self.pack.as_ref().map_or_else(Vec::new, |(patch, pack)| {
            patch
                .options
                .iter()
                .filter(|option| pack.component(option.key).is_some())
                .copied()
                .collect()
        })
    }

    /// Sets the margin the host offers above and below, in pixels (0 = off). Only a game with
    /// a margin decoder that reads rows past the screen fills it. Rounded down to a multiple of
    /// 8 and capped at [`crate::wide::MAX_VERTICAL_MARGIN`].
    pub fn set_widescreen_vertical_margin(&mut self, pixels: i32) {
        self.bus
            .ppu_mut()
            .wide_mut()
            .set_host_vertical_margin(pixels);
    }

    /// The widescreen margin the host offers on each side, in pixels.
    #[must_use]
    pub fn widescreen_margin(&self) -> i32 {
        self.bus.ppu().wide().host_margin()
    }

    /// The 256 x 240 image of the frame just run: the NES colour index of REQ-CORE-048 in bits
    /// 0-5 and the opacity bit of AMD-050 REQ-CORE-049 in bit 6
    /// (`zax_nes::ppu::PIXEL_COLOR`, `OPAQUE_PIXEL`).
    #[must_use]
    pub fn pixel_frame_with_mask(&self) -> &[u8] {
        self.bus.ppu().pixel_frame_with_mask()
    }

    /// The plain index image of REQ-CORE-048, 0 to 63 per pixel, copied into `out`.
    pub fn pixel_frame_indices_into(&self, out: &mut Vec<u8>) {
        self.bus.ppu().pixel_frame_indices_into(out);
    }

    /// The picture of the frame just run, widened by its widescreen margins (the sides, top and
    /// bottom composed from the tile records), into `out`: `256 + 2 * margin` NES colour indices
    /// a row, `240 + 2 * margin_y` rows; returns the number of rows. The plain
    /// [`NesCore::pixel_frame`] without a margin.
    pub fn wide_picture_into(&self, out: &mut Vec<u8>) -> usize {
        self.bus.ppu().wide_picture_into(out)
    }

    /// The colour subcarrier's phase at the start of the frame just drawn (0 to 11), for the
    /// composite decoder ([`crate::composite::RowDecoder`], lot FIDPLUS-D2b).
    #[must_use]
    pub fn video_phase(&self) -> usize {
        self.bus.ppu().video_phase()
    }

    /// The colour emphasis of each pixel of [`NesCore::wide_picture_into`]'s picture (PPUMASK
    /// bits 5-7 as 0 to 7), into `out`; `false` and empty `out` for a frame without emphasis
    /// (lot FIDPLUS-D2b).
    pub fn wide_emphasis_into(&self, out: &mut Vec<u8>) -> bool {
        self.bus.ppu().wide_emphasis_into(out)
    }

    /// Whether the margin decoder of the frame just run asks the screen's top and bottom 8 lines
    /// hidden with a vertical margin as without one, when the host hides them
    /// ([`crate::wide_decode::MarginDecoder::hides_overscan`]; false for a game without one).
    #[must_use]
    pub fn hides_overscan(&self) -> bool {
        self.bus.ppu().hides_overscan()
    }

    /// The tile records of the frame just run.
    #[must_use]
    pub fn tile_records(&self) -> &[TileRecord] {
        self.bus.ppu().tile_records()
    }

    /// Sets the frame counter (REQ-CORE-006).
    pub fn set_frame_count(&mut self, value: i64) {
        self.frame_count = value;
    }

    /// The device plugged into `port` (REQ-CORE-024).
    ///
    /// # Errors
    /// [`InvalidPort`] when `port` is not 0 or 1.
    pub fn port_type(&self, port: usize) -> Result<ControllerPortType, InvalidPort> {
        self.bus.input().port_type(port)
    }

    /// Plugs `kind` into `port`; allowed at any time between two frames (REQ-CORE-024).
    ///
    /// # Errors
    /// [`InvalidPort`] when `port` is not 0 or 1.
    pub fn set_port_type(
        &mut self,
        port: usize,
        kind: ControllerPortType,
    ) -> Result<(), InvalidPort> {
        self.bus.input_mut().set_port_type(port, kind)
    }

    /// Frames run since the last power-on.
    #[must_use]
    pub const fn frame_count(&self) -> i64 {
        self.frame_count
    }

    /// CPU cycles since the last power-on.
    #[must_use]
    pub fn cycle_count(&self) -> i64 {
        self.bus.cycle_count()
    }

    /// Reads CPU memory without side effects (REQ-CORE-080, REQ-CORE-081).
    #[must_use]
    pub fn read_memory(&self, address: u16) -> u8 {
        self.bus.peek(address)
    }

    /// Writes CPU memory as a CPU write would, without advancing time (REQ-CORE-082).
    pub fn write_memory(&mut self, address: u16, value: u8) {
        self.bus.poke(address, value);
    }

    /// The nametable layout the cartridge currently selects.
    #[must_use]
    pub fn mirroring(&self) -> NametableMirroring {
        Ppu2C02::mirroring_of(self.bus.cartridge())
    }

    /// Offset in VRAM of nametable address `address` under the current layout.
    #[must_use]
    pub fn map_nametable(&self, address: u16) -> usize {
        Ppu2C02::map_nametable(address, self.mirroring())
    }

    /// Reads a PPU register with its side effects, as the CPU would, without advancing
    /// time.
    ///
    /// Advancing nothing means two calls in a row land on the same PPU dot, which no pair of CPU
    /// reads can; `Ppu2C02::read` takes that for what it is and gives the second call the
    /// refilled `$2007` buffer rather than the one-cycle-apart stale byte of
    /// `READ_BUFFER_READY_DOTS`.
    pub fn ppu_read(&mut self, register: u16) -> u8 {
        let (ppu, cartridge) = self.bus.ppu_and_cartridge_mut();
        ppu.read(register, cartridge)
    }

    /// The CPU.
    #[must_use]
    pub const fn cpu(&self) -> &Cpu6502 {
        &self.cpu
    }

    /// The CPU and the bus, mutably, to step instructions by hand.
    pub fn cpu_and_bus_mut(&mut self) -> (&mut Cpu6502, &mut NesBus) {
        (&mut self.cpu, &mut self.bus)
    }

    /// The bus.
    #[must_use]
    pub const fn bus(&self) -> &NesBus {
        &self.bus
    }

    /// The bus, mutably.
    pub fn bus_mut(&mut self) -> &mut NesBus {
        &mut self.bus
    }

    /// The PPU.
    #[must_use]
    pub const fn ppu(&self) -> &Ppu2C02 {
        self.bus.ppu()
    }

    /// The cartridge, if a ROM is loaded.
    #[must_use]
    pub const fn cartridge(&self) -> Option<&Cartridge> {
        self.bus.cartridge()
    }

    /// The cartridge, mutably.
    pub fn cartridge_mut(&mut self) -> Option<&mut Cartridge> {
        self.bus.cartridge_mut()
    }

    /// The cartridge's CHR-RAM, read only, addressed linearly from 0 in the order of the
    /// cartridge's memory (AMD-062 REQ-CORE-123). Its length is the size the requirement asks
    /// for: zero for a cartridge with CHR-ROM, and zero when no ROM is loaded.
    ///
    /// Meant to be read between two frames. It is a borrow of the buffer the boards write
    /// through, so a host that reads it every frame and one that never reads it run the same
    /// emulation: nothing is latched and no bank is switched.
    #[must_use]
    pub fn chr_ram(&self) -> &[u8] {
        self.cartridge().map_or(&[], Cartridge::chr_ram)
    }

    // ---------------------------------------------------------------- savestates

    /// The savestate format version, as a host writes it in the metadata header of a
    /// savestate file (SPEC-09 REQ-PERS-056 point 2).
    #[must_use]
    pub const fn state_format_version() -> i32 {
        Self::STATE_VERSION
    }

    /// Returns a self-contained state block, ROM image and ROM name included
    /// (REQ-CORE-090). `include_rom_image` false is for the in-memory rollback history of
    /// SPEC-07, which trades self-containment for size; such a block must never be written
    /// to disk (REQ-CORE-093).
    ///
    /// # Errors
    /// [`NoRomLoaded`] when there is nothing to save.
    pub fn save_state(&self, include_rom_image: bool) -> Result<Vec<u8>, NoRomLoaded> {
        let rom_bytes = self
            .bus
            .cartridge()
            .map_or(0, |cartridge| cartridge.rom().raw().len());
        let capacity = if include_rom_image { rom_bytes } else { 0 } + 32 * 1024;
        let mut buffer = Vec::with_capacity(capacity);
        self.save_state_to(include_rom_image, &mut buffer)?;
        Ok(buffer)
    }

    /// The state block of [`NesCore::save_state`], taken once the game waits for the next frame
    /// (the `idle_loop` of its built-in patch, or of the patch whose hack pack gives it its
    /// aids): the CPU is then in the game's own wait, no code of the hack running nor on the
    /// stack, so the state restores with other components of the hack applied (an aid on or off).
    /// The machine runs on through the vertical blank to get there (the NMI's work done); a game
    /// that does not reach its wait before the picture starts again, or whose wait is not known,
    /// is saved where it is. Returns the block and whether it was taken at the wait.
    ///
    /// # Errors
    /// [`NoRomLoaded`] when there is nothing to save.
    pub fn save_state_at_idle(
        &mut self,
        include_rom_image: bool,
    ) -> Result<(Vec<u8>, bool), NoRomLoaded> {
        if self.bus.cartridge().is_none() {
            return Err(NoRomLoaded);
        }
        let idle_loop = self.idle_loop;
        let mut at_idle = false;
        if let Some((first, last)) = idle_loop {
            // A bound past any vertical blank, for a machine stalled by the overclock.
            for _ in 0..100_000 {
                if (first..=last).contains(&self.cpu.pc) {
                    at_idle = true;
                    break;
                }
                let ppu = self.bus.ppu();
                if ppu.scanline() < 240 || ppu.scanline() >= ppu.pre_render_scanline() {
                    break;
                }
                self.cpu.step(&mut self.bus);
                if (first..=last).contains(&self.cpu.pc) && self.bus.ppu().wide().stalling() {
                    self.bus.ppu_mut().wide_mut().end_stall();
                }
            }
        }
        Ok((self.save_state(include_rom_image)?, at_idle))
    }

    /// The components of the built-in hack pack applied to the ROM loaded: `widescreen` while
    /// the pack's widescreen runs (its patch applied), and the key of each aid whose option is
    /// on (see [`crate::hack_pack`]); none for a ROM without a pack.
    #[must_use]
    pub fn applied_components(&self) -> Vec<&'static str> {
        let Some((patch, pack)) = &self.pack else {
            return Vec::new();
        };
        let mut components = Vec::new();
        if self.builtin_patch == Some(patch.name)
            && pack.component(crate::hack_pack::WIDESCREEN).is_some()
        {
            components.push(crate::hack_pack::WIDESCREEN);
        }
        for option in patch.options {
            if pack.component(option.key).is_some() && self.aids_on & (1 << (option.bit & 15)) != 0
            {
                components.push(option.key);
            }
        }
        components
    }

    /// [`NesCore::save_state`] written into `buffer`, whose earlier content is replaced and
    /// whose allocation is kept (the rollback history recycles its oldest snapshot this way).
    /// The bytes are those of [`NesCore::save_state`]; on an error `buffer` is untouched.
    ///
    /// # Errors
    ///
    /// [`NoRomLoaded`] when no ROM is loaded.
    pub fn save_state_to(
        &self,
        include_rom_image: bool,
        buffer: &mut Vec<u8>,
    ) -> Result<(), NoRomLoaded> {
        let cartridge = self.bus.cartridge().ok_or(NoRomLoaded)?;
        let rom = cartridge.rom();

        let wide = self.bus.ppu().wide();
        let edges = self.bus.ppu().edges_pending();
        let filter = self.bus.apu().has_profile_filter();
        let mut writer = StateWriter::reusing(std::mem::take(buffer));
        writer.write_u32(Self::STATE_MAGIC);
        writer.write_i32(match (wide.unlocked(), edges || filter) {
            (true, false) => Self::WIDE_STATE_VERSION,
            (false, false) => Self::STATE_VERSION,
            (true, true) => Self::EXTENDED_WIDE_STATE_VERSION,
            (false, true) => Self::EXTENDED_STATE_VERSION,
        });
        writer.write_string(Self::CORE_NAME);
        writer.write_bool(include_rom_image);
        writer.write_string(rom.name());
        let image = self.original_image.as_deref().unwrap_or_else(|| rom.raw());
        writer.write_blob(if include_rom_image { image } else { &[] });
        writer.write_u8(self.region as u8);
        writer.write_i64(self.frame_count);

        self.bus.save_state(&mut writer);
        self.cpu.save_state(&mut writer);
        self.bus.ppu().save_state(&mut writer);
        self.bus.apu().save_state(&mut writer);
        self.bus.input().save_state(&mut writer);
        cartridge.save_state(&mut writer);
        if wide.unlocked() {
            wide.save_state(&mut writer);
        }
        if edges || filter {
            writer.write_u8(u8::from(edges) | (u8::from(filter) << 1));
            if edges {
                self.bus.ppu().save_edge_state(&mut writer);
            }
            if filter {
                self.bus.apu().save_profile_filter(&mut writer);
            }
        }

        *buffer = writer.into_vec();
        Ok(())
    }

    /// Upper bound on the size of a state block for the ROM currently loaded, announced
    /// before serialisation and valid for the whole session (REQ-ARCH-013).
    #[must_use]
    pub fn max_state_size(&self) -> usize {
        let (rom_bytes, name_bytes) = self.bus.cartridge().map_or((0, 0), |cartridge| {
            (
                cartridge.rom().raw().len(),
                cartridge.rom().name().len() * 4,
            )
        });
        let fixed_bytes = NesBus::RAM_SIZE
            + VRAM_SIZE
            + OAM_SIZE
            + PALETTE_SIZE
            + 64 * 1024 // PRG-RAM: 8 KiB usually, up to 64 KiB on MMC5.
            + InesHeader::DEFAULT_CHR_RAM_SIZE
            + 1024 // MMC5 ExRAM.
            + MAX_TILE_RECORD_STATE_SIZE
            + Apu2A03::MAX_STATE_SIZE
            + Apu2A03::PROFILE_FILTER_STATE_SIZE
            + 4096; // headers, registers, counters and the length prefixes

        rom_bytes + fixed_bytes + 256 + name_bytes
    }

    // ------------------------------------------------------- in-memory state

    /// Upper bound on the bytes a [`CoreState`] of the ROM currently loaded occupies
    /// (REQ-CORE-121, REQ-ARCH-013): [`NesCore::max_state_size`] without the ROM image and its
    /// name, plus the widescreen block, which an in-memory state always carries.
    #[must_use]
    pub fn max_core_state_size(&self) -> usize {
        let (rom_bytes, name_bytes) = self.bus.cartridge().map_or((0, 0), |cartridge| {
            (
                cartridge.rom().raw().len(),
                cartridge.rom().name().len() * 4,
            )
        });
        self.max_state_size() - rom_bytes - name_bytes + Self::MAX_WIDE_STATE_SIZE
    }

    /// Captures the complete state of the machine into `state`, the ROM image excluded
    /// (AMD-041 REQ-CORE-121): work RAM, the mapper, the CPU, the PPU, the APU, the input ports,
    /// the widescreen registers and the frame counter.
    ///
    /// The per-component code is the savestate's own, so the two can never drift apart; what is
    /// left out is the portable wrapping — magic number, format version, core name, ROM image
    /// and ROM name — and the half of the PPU state that *is* the frame just run (its tile
    /// records and edge bands, which [`crate::ppu::Ppu2C02::begin_rendered_frame`] clears at the
    /// start of every frame anyway). Nothing is allocated once `state` has grown to its size.
    ///
    /// # Errors
    /// [`NoRomLoaded`] when there is nothing to capture.
    pub fn capture_core_state(&self, state: &mut CoreState) -> Result<(), NoRomLoaded> {
        let cartridge = self.bus.cartridge().ok_or(NoRomLoaded)?;

        let mut writer = state.begin();
        writer.write_i64(self.frame_count);
        writer.write_u8(self.region as u8);

        self.bus.save_state(&mut writer);
        self.cpu.save_state(&mut writer);
        self.bus.ppu().save_machine_state(&mut writer);
        self.bus.apu().save_state(&mut writer);
        self.bus.input().save_state(&mut writer);
        cartridge.save_state(&mut writer);
        self.bus.ppu().wide().save_state(&mut writer);
        // Fidélité+: the edges pending (an empty block otherwise); run-ahead must stay exact.
        self.bus.ppu().save_edge_state(&mut writer);
        // A console profile's filters and expansion signal (lot FIDPLUS-D1e), likewise.
        let filter = self.bus.apu().has_profile_filter();
        writer.write_u8(u8::from(filter));
        if filter {
            self.bus.apu().save_profile_filter(&mut writer);
        }

        state.end(writer);
        Ok(())
    }

    /// Restores a state [`NesCore::capture_core_state`] produced on this very instance, **the
    /// audio already produced left alone** (AMD-041 REQ-CORE-121).
    ///
    /// Step 5 of REQ-LOOP-065 undoes the frames in advance, not the sound the displayed frame
    /// produced, which the player is about to hear: the samples the host has not read yet stay
    /// in the queue, unlike a savestate restore (REQ-CORE-074, Q-006). The PPU's picture and the
    /// tile records of the displayed frame are not in the block either, so they survive the
    /// restore and the host can still show and convert that frame (REQ-LOOP-067).
    ///
    /// # Errors
    /// [`InvalidState`] when no ROM is loaded, when `state` holds nothing, or when the block
    /// does not match the machine — which can only happen if it was captured from another core.
    pub fn restore_core_state(&mut self, state: &CoreState) -> Result<(), InvalidState> {
        if self.bus.cartridge().is_none() {
            return Err(InvalidState::corrupt(
                "No ROM is loaded: there is no state to restore.",
            ));
        }
        if state.is_empty() {
            return Err(InvalidState::corrupt(
                "The in-memory state holds nothing to restore.",
            ));
        }

        let mut reader = StateReader::new(state.as_slice(), 0)?;
        reader.set_format_version(Self::STATE_VERSION);

        let frame_count = reader.read_i64()?;
        let region_byte = reader.read_u8()?;
        let region = NesRegion::from_u8(region_byte).ok_or_else(|| {
            InvalidState::corrupt(format!("Corrupt core state: unknown region {region_byte}."))
        })?;

        self.bus.load_state(&mut reader)?;
        self.cpu.load_state(&mut reader)?;
        self.bus.ppu_mut().load_machine_state(&mut reader)?;
        self.bus.apu_mut().load_state_keeping_samples(&mut reader)?;
        self.bus.input_mut().load_state(&mut reader)?;
        // The cartridge is the one already attached: the ROM never left, so there is nothing to
        // rebuild. `Cartridge::load_state` refreshes the fast ROM windows itself.
        let last_nametable_read = self.bus.ppu().last_nametable_read();
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.load_state(&mut reader)?;
            cartridge.after_state_load(last_nametable_read);
        }
        self.bus
            .ppu_mut()
            .wide_mut()
            .load_state(&mut reader, true, true)?;
        self.bus.ppu_mut().load_edge_state(&mut reader)?;
        if reader.read_u8()? != 0 {
            self.bus.apu_mut().load_profile_filter(&mut reader)?;
        }

        self.frame_count = frame_count;
        if region != self.region {
            // Unreachable inside one real frame — the region only changes between two of them —
            // but a state is a state: it carries the region and putting it back costs nothing.
            self.region = region;
            self.apply_region();
        }

        Ok(())
    }

    /// Restores the state block that starts at `offset` in `data`, which may hold other
    /// data around it (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] for an invalid or truncated block, or one written by another core
    /// ([`StateLoadFailure::ForeignCore`]); the core is then left stopped (REQ-CORE-005).
    pub fn try_load_state(
        &mut self,
        data: &[u8],
        offset: usize,
    ) -> Result<StateLoaded, InvalidState> {
        let loaded = self.restore_state(data, offset);
        if loaded.is_err() {
            self.close_rom();
        }

        loaded
    }

    /// The widescreen block of a state and, after it, the Fidélité+ edges. When the widescreen
    /// block is skipped (a hack's state restored without the hack) the edges cannot be found and
    /// are dropped with it.
    fn restore_wide_and_edges(
        &mut self,
        reader: &mut StateReader,
        version: i32,
        unpatched_hack: bool,
    ) -> Result<(), InvalidState> {
        let wide = self.bus.ppu_mut().wide_mut();
        let wide_read = Self::is_wide_state_version(version) && !unpatched_hack;
        if wide_read {
            wide.load_state(
                reader,
                version != Self::WIDE_STATE_VERSION_5,
                version >= Self::WIDE_STATE_VERSION_7,
            )?;
        } else {
            wide.reset();
        }
        if version == Self::FIDPLUS_STATE_VERSION
            || (version == Self::FIDPLUS_WIDE_STATE_VERSION && wide_read)
        {
            self.bus.ppu_mut().load_edge_state(reader)?;
        }
        if version == Self::EXTENDED_STATE_VERSION
            || (version == Self::EXTENDED_WIDE_STATE_VERSION && wide_read)
        {
            let flags = reader.read_u8()?;
            if flags & 1 != 0 {
                self.bus.ppu_mut().load_edge_state(reader)?;
            }
            if flags & 2 != 0 {
                self.bus.apu_mut().load_profile_filter(reader)?;
            }
        }
        Ok(())
    }

    fn restore_state(&mut self, data: &[u8], offset: usize) -> Result<StateLoaded, InvalidState> {
        let mut reader = StateReader::new(data, offset)?;

        if reader.read_u32()? != Self::STATE_MAGIC {
            return Err(InvalidState::corrupt(
                "Corrupt savestate: wrong magic number.",
            ));
        }

        let version = reader.read_i32()?;
        if !Self::is_known_state_version(version) {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: format version {version} is not supported (expected {}).",
                Self::STATE_VERSION
            )));
        }
        // Objects whose own layout grew with a version of the format read it from here: the
        // MMC5's sound registers are written only from `STATE_VERSION` on.
        reader.set_format_version(version);

        let core_name = reader.read_string()?;
        if core_name != Self::CORE_NAME && core_name != Self::LEGACY_CORE_NAME {
            // REQ-CORE-120 fixes this message to the character; it is one of the few
            // literal strings of the spec (AMD-002, answer to Q-001).
            return Err(InvalidState {
                failure: StateLoadFailure::ForeignCore,
                message: Self::FOREIGN_CORE_MESSAGE.to_owned(),
            });
        }

        let has_rom_image = reader.read_bool()?;
        let stored_name = reader.read_string()?;
        let rom_bytes = reader.read_blob()?;
        let rom_patch = if has_rom_image {
            crate::builtin_patches::patch_for(&rom_bytes)
        } else {
            None
        };
        let patchable = rom_patch.is_some();
        let rom_crc = crate::builtin_patches::data_crc(&rom_bytes);
        // The image as the state gave it, for the hack pack made for it (installed once the
        // cartridge is attached).
        let pack_image = has_rom_image.then(|| rom_bytes.clone());
        let region_byte = reader.read_u8()?;
        let region = NesRegion::from_u8(region_byte).ok_or_else(|| {
            InvalidState::corrupt(format!("Corrupt savestate: unknown region {region_byte}."))
        })?;
        let frame_count = reader.read_i64()?;

        let mut patch = (self.original_image.take(), self.builtin_patch);
        let rom = if has_rom_image {
            // The block is self-contained: it restores even with no ROM loaded
            // (REQ-CORE-090). A ROM a built-in patch was made for gets the patch of this build.
            let patched = self.patch_image(&rom_bytes);
            let image = patched
                .as_ref()
                .map_or(rom_bytes.as_slice(), |p| p.image.as_slice());
            let rom = RomImage::try_load(image, &stored_name)
                .map_err(|error| InvalidState::corrupt(format!("Corrupt savestate: {error}")))?;
            patch = match patched {
                Some(patched) => (Some(rom_bytes), Some(patched.name)),
                None => (None, None),
            };
            rom
        } else {
            // Without an image the block restores onto the ROM already loaded. The old
            // cartridge is taken apart: the state replaces everything it held, and a failure
            // leaves the core stopped anyway.
            self.bus
                .take_cartridge()
                .map(Cartridge::into_rom)
                .ok_or_else(|| {
                    InvalidState::corrupt(
                        "Corrupt savestate: the block carries no ROM image and none is loaded.",
                    )
                })?
        };

        let mut cartridge = Cartridge::create(rom)
            .map_err(|error| InvalidState::corrupt(format!("Corrupt savestate: {error}")))?;

        // Everything is read into the live objects only once the header checks out, so that
        // a truncation leaves the core stopped rather than half-restored.
        self.bus.load_state(&mut reader)?;
        self.cpu.load_state(&mut reader)?;
        self.bus.ppu_mut().load_state(&mut reader)?;
        self.bus.apu_mut().load_state(&mut reader)?;
        self.bus.input_mut().load_state(&mut reader)?;
        cartridge.load_state(&mut reader)?;
        cartridge.after_state_load(self.bus.ppu().last_nametable_read());
        // A state of the hack restored without it: the original ROM never unlocks the
        // extension, which must not stay as the hack left it (four nametables, margins).
        let unpatched_hack = patch.1.is_none() && patchable;
        self.restore_wide_and_edges(&mut reader, version, unpatched_hack)?;

        // A state made the other way (the patch ran and does not now, or the reverse).
        let made_with_patch = Self::is_wide_state_version(version);
        if made_with_patch != patch.1.is_some() {
            self.put_right_other_way_state(rom_patch, made_with_patch);
        }
        // A patch that boots the extension needs its window, whatever the state left it as.
        if !self.bus.ppu().wide().unlocked() {
            self.boot_patch_window(patch.1);
        }

        self.battery_ram_imported = false;
        self.rom_crc = has_rom_image.then_some(rom_crc).or(self.rom_crc);
        (self.original_image, self.builtin_patch) = patch;
        self.attach_cartridge(cartridge);
        self.install_pack(pack_image.as_deref());
        self.region = region;
        self.apply_region();
        self.frame_count = frame_count;
        self.bus.apu_mut().drop_pending_samples(); // REQ-CORE-074, Q-006.
                                                   // REQ-LOOP-066: loading a state resumes at the next real frame with the same N. The
                                                   // state of a real frame of the machine that was replaced is dropped, not put back.
        self.forget_real_frame_and_reshow_palette();

        Ok(StateLoaded {
            consumed: reader.consumed(),
            rom_name: stored_name,
        })
    }
}

/// An idle loop the fast core repeats by counting (lot FAST-1, [`NesCore::idle_loop`]).
#[derive(Debug, Clone, Copy)]
enum IdleLoop {
    /// A turn of this many cycles leaves the machine as it found it.
    Steady(i64),
    /// `DEX`/`DEY` and `BNE`: `turns` more turns branch back.
    Countdown {
        period: i64,
        index_y: bool,
        turns: i64,
    },
    /// `LDA`/`BIT $2002` and `BPL`, waiting for the vblank flag.
    StatusPoll(i64),
    /// A `$2002` loop on the sprite 0 flag, waiting for the hit or for the flag to clear.
    ZeroHitPoll { period: i64, waits_for_hit: bool },
}
