use crate::apu::{Apu2A03, AudioError, AudioReader};
use crate::cartridges::Cartridge;
use crate::cpu::{Cpu6502, CpuBus};
use crate::input::{ControllerPortType, InputPorts, InvalidPort, ZapperPosition};
use crate::ppu::{Ppu2C02, MAX_TILE_RECORD_STATE_SIZE, OAM_SIZE, PALETTE_SIZE, VRAM_SIZE};
use crate::rom::{InesHeader, RomImage, RomLoadResult};
use crate::state::{InvalidState, StateLoadFailure, StateReader, StateWriter};
use crate::{
    FrameBatch, NametableMirroring, NesBus, NesRegion, RegionTiming, TileRecord, VideoInfo,
};

/// Receives one frame batch per emulated frame (REQ-CORE-030). The emulation behaves
/// identically with or without it (REQ-CORE-033).
pub type VideoCallback = Box<dyn FnMut(&FrameBatch<'_>) + Send>;

/// Supplies the Zapper's aimed position, queried once per frame for each Zapper port
/// (REQ-CORE-060).
pub type ZapperPositionSource = Box<dyn FnMut(usize) -> ZapperPosition + Send>;

/// What a successful "restore state" reports (REQ-CORE-091).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateLoaded {
    /// Bytes the block occupied, so the host can read on.
    pub consumed: usize,
    /// The ROM name stored in the block.
    pub rom_name: String,
}

/// Why a savestate could not be written: no ROM is loaded, so there is no state to save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoRomLoaded;

impl std::fmt::Display for NoRomLoaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("No ROM is loaded: there is no state to save.")
    }
}

impl std::error::Error for NoRomLoaded {}

/// One NES machine, seen from the host (REQ-CORE-001, REQ-CORE-002).
///
/// Everything an instance needs lives in the instance, so two cores in one process are
/// independent (REQ-ARCH-011, REQ-CORE-001). Nothing advances unless the host calls
/// [`NesCore::run_frame`]: the core owns no clock and no thread (REQ-ARCH-010).
pub struct NesCore {
    cpu: Cpu6502,
    bus: NesBus,
    region: NesRegion,
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
            battery_ram_imported: false,
            frame_count: 0,
            video_callback: None,
            zapper_position_source: None,
            original_image: None,
            builtin_patch: None,
            builtin_patches_enabled: true,
        }
    }

    /// Turns the built-in patches on or off for the next [`NesCore::load_rom`] and state
    /// restore; the ROM loaded keeps what it has.
    pub fn set_builtin_patches(&mut self, enabled: bool) {
        self.builtin_patches_enabled = enabled;
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
        let patchable = has_rom_image && crate::builtin_patches::matches(&rom);
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
        match i32::from_le_bytes([header[4], header[5], header[6], header[7]]) {
            Self::STATE_VERSION => Some(false),
            Self::WIDE_STATE_VERSION | Self::WIDE_STATE_VERSION_5 => Some(true),
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
    pub const CORE_NAME: &'static str = "Zaxnes";

    /// The name the core carried under the « Sen » working name. Still accepted when reading
    /// a state block, never written: AMD-029 §2 keeps states written before the rename
    /// loadable (Q-070).
    pub const LEGACY_CORE_NAME: &'static str = "Sen";

    /// Message a savestate written by a different core is refused with (REQ-CORE-120).
    pub const FOREIGN_CORE_MESSAGE: &'static str = "Save state used a different core.";

    /// Magic that opens a core savestate block, "SEN0". Kept as it is across the rename of
    /// AMD-029: it is a format datum, and changing it would refuse every block written
    /// before (Q-070).
    pub const STATE_MAGIC: u32 = 0x304E_4553;

    /// Version of the savestate format. Bumped whenever the block layout changes; L1c added
    /// the APU channel, mixer and resampler state and L1d the mapper bank windows, latches
    /// and IRQ counters (REQ-CORE-090).
    pub const STATE_VERSION: i32 = 4;

    /// Version of a block that also carries the widescreen extension registers: written only
    /// once a ROM unlocked the extension, so every other block keeps [`Self::STATE_VERSION`].
    pub const WIDE_STATE_VERSION: i32 = 6;

    /// The earlier widescreen block, without the PRG window registers; still read.
    pub const WIDE_STATE_VERSION_5: i32 = 5;

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
        let patched = if self.builtin_patches_enabled {
            crate::builtin_patches::apply(image)
        } else {
            None
        };
        let loaded = patched
            .as_ref()
            .map_or(image, |patched| patched.image.as_slice());
        let cartridge = match RomImage::try_load(loaded, name).and_then(Cartridge::create) {
            Ok(cartridge) => cartridge,
            Err(error) => {
                self.close_rom();
                return RomLoadResult::fail(error.failure, error.message);
            }
        };

        let region = cartridge.rom().announced_region();
        self.battery_ram_imported = false;
        self.builtin_patch = patched.as_ref().map(|patched| patched.name);
        self.original_image = patched.map(|_| image.to_vec());
        self.bus.attach(Some(cartridge));
        self.power_on();
        RomLoadResult::ok(region)
    }

    /// Stops the emulation (REQ-CORE-005).
    pub fn close_rom(&mut self) {
        self.battery_ram_imported = false;
        self.original_image = None;
        self.builtin_patch = None;
        self.bus.attach(None);
        self.bus.apu_mut().discard_samples();
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.bus.cartridge().is_some()
    }

    /// Name the ROM was loaded under (REQ-CORE-014).
    #[must_use]
    pub fn rom_name(&self) -> Option<&str> {
        self.bus.cartridge().map(|cartridge| cartridge.rom().name())
    }

    /// Resets the machine, which is equivalent to a power cycle (REQ-CORE-004, DEC-02-Q3).
    pub fn reset(&mut self) {
        if self.bus.cartridge().is_some() {
            self.power_on();
        }
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

    fn power_on(&mut self) {
        // Q-003: the region goes in first, so the PPU's power-on position is the start of
        // vblank of the region in force (REQ-CORE-015, REQ-CORE-020 notes).
        self.apply_region();

        self.bus.reset();
        self.bus.ppu_mut().reset();
        self.bus.apu_mut().reset();
        let clear_prg_ram = !self.battery_ram_imported;
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.reset();
            cartridge.clear_volatile_ram(clear_prg_ram);
        }

        self.cpu.reset(&mut self.bus);
        self.frame_count = 0;
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

        self.bus.input_mut().begin_frame(input_word);

        // The aimed position is sampled once, at the start of the frame, and stays fixed
        // for its whole duration (REQ-CORE-060).
        if let Some(source) = self.zapper_position_source.as_mut() {
            self.bus.input_mut().aim_zappers(source);
        }

        let start_cycle = self.bus.cycle_count();

        // A game with a margin decoder gets the host's margin whatever its ROM asks for.
        let decoder = self.builtin_patch.and_then(crate::wide_decode::for_patch);
        self.bus.ppu_mut().wide_mut().set_decoded(decoder.is_some());

        self.bus.ppu_mut().clear_frame_boundary();
        self.bus.defer_ppu();
        loop {
            self.cpu.step(&mut self.bus);
            if self.bus.ppu().vblank_just_started() {
                break;
            }
        }
        self.bus.end_deferred_ppu();
        if let Some(decoder) = decoder {
            self.decode_margins(decoder);
        }

        self.frame_count += 1;

        if let Some(callback) = self.video_callback.as_mut() {
            callback(&Self::batch_of(self.bus.ppu()));
        }

        self.bus.cycle_count() - start_cycle
    }

    /// Has `decoder` fill the margins of the frame just run from the game's memory.
    fn decode_margins(&mut self, decoder: &dyn crate::wide_decode::MarginDecoder) {
        let (ram, ppu, cartridge) = self.bus.decoder_parts();
        let Some(cartridge) = cartridge else {
            return;
        };
        let memory = crate::wide_decode::DecoderMemory {
            ram,
            prg_ram: cartridge.memory().prg_ram(),
            prg_rom: cartridge.rom().prg_rom(),
        };
        ppu.decode_margins(decoder, &memory, Some(cartridge));
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

    /// The 256 x 240 indexed image of the frame just run (REQ-CORE-048).
    #[must_use]
    pub fn pixel_frame(&self) -> &[u8] {
        self.bus.ppu().pixel_frame()
    }

    /// The picture of the frame just run, widened by its widescreen margin on each side (the
    /// sides composed from the tile records), into `out`: `256 + 2 * margin` NES colour indices
    /// a row, 240 rows. The plain [`NesCore::pixel_frame`] without a margin.
    pub fn wide_picture_into(&self, out: &mut Vec<u8>) {
        self.bus.ppu().wide_picture_into(out);
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
        let mut writer = StateWriter::reusing(std::mem::take(buffer));
        writer.write_u32(Self::STATE_MAGIC);
        writer.write_i32(if wide.unlocked() {
            Self::WIDE_STATE_VERSION
        } else {
            Self::STATE_VERSION
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
            + 4096; // headers, registers, counters and the length prefixes

        rom_bytes + fixed_bytes + 256 + name_bytes
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

    fn restore_state(&mut self, data: &[u8], offset: usize) -> Result<StateLoaded, InvalidState> {
        let mut reader = StateReader::new(data, offset)?;

        if reader.read_u32()? != Self::STATE_MAGIC {
            return Err(InvalidState::corrupt(
                "Corrupt savestate: wrong magic number.",
            ));
        }

        let version = reader.read_i32()?;
        if version != Self::STATE_VERSION
            && version != Self::WIDE_STATE_VERSION
            && version != Self::WIDE_STATE_VERSION_5
        {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: format version {version} is not supported (expected {}).",
                Self::STATE_VERSION
            )));
        }

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
        let patchable = has_rom_image && crate::builtin_patches::matches(&rom_bytes);
        let region_byte = reader.read_u8()?;
        let region = NesRegion::from_u8(region_byte).ok_or_else(|| {
            InvalidState::corrupt(format!("Corrupt savestate: unknown region {region_byte}."))
        })?;
        let frame_count = reader.read_i64()?;

        let mut patch = (self.original_image.take(), self.builtin_patch);
        let rom = if has_rom_image {
            // The block is self-contained: it restores even with no ROM loaded
            // (REQ-CORE-090). A ROM a built-in patch was made for gets the patch of this build.
            let patched = if self.builtin_patches_enabled {
                crate::builtin_patches::apply(&rom_bytes)
            } else {
                None
            };
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
        // A state of the hack restored without it: the original ROM never unlocks the
        // extension, which must not stay as the hack left it (four nametables, margins).
        let unpatched_hack = patch.1.is_none() && patchable;
        let wide = self.bus.ppu_mut().wide_mut();
        if (version == Self::WIDE_STATE_VERSION || version == Self::WIDE_STATE_VERSION_5)
            && !unpatched_hack
        {
            wide.load_state(&mut reader, version == Self::WIDE_STATE_VERSION)?;
        } else {
            wide.reset();
        }

        self.battery_ram_imported = false;
        (self.original_image, self.builtin_patch) = patch;
        self.bus.attach(Some(cartridge));
        self.region = region;
        self.apply_region();
        self.frame_count = frame_count;
        self.bus.apu_mut().drop_pending_samples(); // REQ-CORE-074, Q-006.

        Ok(StateLoaded {
            consumed: reader.consumed(),
            rom_name: stored_name,
        })
    }
}
