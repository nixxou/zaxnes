//! The game in play, in safe Rust: the NES core of `zax-nes` driven one frame at a time, its
//! picture turned into pixels, its sound into 16-bit stereo, its states and memories handed out
//! as the libretro API asks for them. The C side ([`crate::ffi`]) only moves data in and out.

use zax_nes::builtin_patches::HackOption;
use zax_nes::ppu::SpriteLimit;
use zax_nes::{NesCore, NesRegion, RegionTiming};

/// The sample rate the core makes its sound at.
pub const SAMPLE_RATE: i32 = 48_000;
/// The widest picture: the NES's 256 pixels and the widest margins on each side.
pub const MAX_WIDTH: u32 = 256 + 2 * zax_nes::wide::MAX_MARGIN.unsigned_abs();
/// The tallest picture: 240 lines and the tallest margins above and below.
pub const MAX_HEIGHT: u32 = 240 + 2 * zax_nes::wide::MAX_VERTICAL_MARGIN.unsigned_abs();

/// The region the game runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegionChoice {
    /// The one the ROM's header says.
    #[default]
    Auto,
    /// A region forced.
    Forced(NesRegion),
}

/// What the core options of the frontend set.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// The widescreen hack of the game, when it has one: its patch runs and the picture widens.
    pub widescreen: bool,
    /// The shape of the widened picture, width over height (16:9 = 1.777...).
    pub ratio: f32,
    /// The advanced mode of a patch that has one.
    pub advanced: bool,
    /// The region.
    pub region: RegionChoice,
    /// The eight sprites a line.
    pub sprite_limit: SpriteLimit,
    /// The palette of the NES colours, by its index (`zax_engine3d::palette`).
    pub palette: u8,
    /// The options of the game's hack the player turned on or off, by key; the others keep
    /// their default.
    pub hacks: Vec<(String, bool)>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            widescreen: true,
            ratio: 16.0 / 9.0,
            advanced: true,
            region: RegionChoice::Auto,
            sprite_limit: SpriteLimit::Kept,
            palette: 0,
            hacks: Vec::new(),
        }
    }
}

/// What a change of options did that the frontend must be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Changed {
    /// The picture's size or shape changed.
    pub geometry: bool,
    /// The frame rate changed (another region).
    pub timing: bool,
    /// The game's hack options changed (the widescreen switched): declare them again.
    pub hack_options: bool,
}

/// The game in play.
pub struct Core {
    nes: NesCore,
    detected_region: NesRegion,
    options: Options,
    indices: Vec<u8>,
    frame: Vec<u32>,
    width: u32,
    height: u32,
    mono: Vec<f32>,
    audio: Vec<i16>,
    /// Whether a frame has run since the options last changed the picture: its size is then
    /// the picture's, which a game may make narrower than the options ask (its hack's widest).
    framed: bool,
    /// The size the frontend was last told.
    announced: (u32, u32),
}

impl Core {
    /// Loads `image` (an iNES file) named `name`, its hack as `options` say.
    ///
    /// # Errors
    ///
    /// The reason the ROM is refused.
    pub fn load(image: &[u8], name: &str, options: Options) -> Result<Self, String> {
        let mut nes = NesCore::new();
        nes.set_builtin_patches(options.widescreen);
        nes.set_advanced_patches(options.advanced);
        let result = nes.load_rom(image, name);
        if !result.success {
            return Err(result
                .error
                .unwrap_or_else(|| "the ROM is not supported".to_owned()));
        }
        nes.set_sample_rate(SAMPLE_RATE)
            .map_err(|error| format!("{error:?}"))?;
        // A battery RAM the frontend writes into stays when the game resets.
        let battery = nes.export_battery_ram().to_vec();
        if !battery.is_empty() {
            nes.import_battery_ram(&battery);
        }
        let mut core = Self {
            nes,
            detected_region: result.region,
            options: Options::default(),
            indices: Vec::new(),
            frame: Vec::new(),
            width: 256,
            height: 240,
            mono: Vec::new(),
            audio: Vec::new(),
            framed: false,
            announced: (256, 240),
        };
        core.options.widescreen = options.widescreen;
        core.options.advanced = options.advanced;
        core.set_options(options);
        core.announced = core.picture_size();
        Ok(core)
    }

    /// The options in force.
    #[must_use]
    pub const fn options(&self) -> &Options {
        &self.options
    }

    /// Puts `options` in force; says what the frontend must be told.
    pub fn set_options(&mut self, options: Options) -> Changed {
        let mut changed = Changed::default();
        let region = self.region_of(options.region);
        if region != self.nes.region() {
            self.nes.set_region(region);
            changed.timing = true;
        }
        if (options.widescreen, options.advanced) != (self.options.widescreen, self.options.advanced)
        {
            changed.hack_options = self.switch_patch(options.widescreen, options.advanced);
        }
        zax_engine3d::palette::select_nes_palette(options.palette);
        self.nes.set_sprite_limit(options.sprite_limit);
        let before = self.margin();
        self.options = options;
        let margin = self.margin();
        changed.geometry = margin != before;
        if changed.geometry || changed.hack_options {
            self.framed = false;
        }
        self.nes.set_widescreen_margin(i32::try_from(margin.0).unwrap_or(0));
        self.nes
            .set_widescreen_vertical_margin(i32::try_from(margin.1).unwrap_or(0));
        self.nes.set_hack_options(self.hack_bits());
        changed
    }

    fn region_of(&self, choice: RegionChoice) -> NesRegion {
        match choice {
            RegionChoice::Auto => self.detected_region,
            RegionChoice::Forced(region) => region,
        }
    }

    /// Turns the game's patch on or off while it plays, when its hack pack carries its states
    /// across (the `to_on` and `to_off` of its states section); otherwise the change waits for
    /// the next load. Returns whether the game now runs the other way.
    fn switch_patch(&mut self, on: bool, advanced: bool) -> bool {
        let Ok(state) = self.nes.save_state(true) else {
            return false;
        };
        let before = self.nes.patch_switches();
        self.nes.set_builtin_patches(on);
        self.nes.set_advanced_patches(advanced);
        if self.nes.state_fits_builtin_patches(&state, 0) != Some(false) {
            return false;
        }
        let to_off = NesCore::state_uses_widescreen(&state, 0) == Some(true);
        let carried = NesCore::state_builtin_patch(&state, 0).is_some_and(|patch| {
            let states = patch.states();
            if to_off {
                states.to_off
            } else {
                states.to_on
            }
        });
        if !carried {
            self.nes.set_builtin_patches(before.0);
            self.nes.set_advanced_patches(before.1);
            return false;
        }
        let region = self.nes.region();
        if self.nes.try_load_state(&state, 0).is_err() {
            return false;
        }
        self.nes.set_region(region);
        true
    }

    /// The margins of the picture, on each side and above and below: none without the hack,
    /// or for a game whose picture the hack does not widen.
    fn margin(&self) -> (u32, u32) {
        if !self.options.widescreen || !self.widens() {
            return (0, 0);
        }
        let ratio = self.options.ratio.clamp(4.0 / 3.0, 32.0 / 9.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Clamped.
        let tiles = ((240.0 * ratio - 256.0).max(0.0) / 2.0 / 8.0).ceil() as u32;
        ((tiles * 8).min(zax_nes::wide::MAX_MARGIN.unsigned_abs()), 0)
    }

    /// Whether the game's picture widens: a built-in patch runs, or a margin decoder is made
    /// for it.
    fn widens(&self) -> bool {
        self.nes.options_patch().is_some() || self.nes.original_image().is_some()
    }

    /// The options of the game's hack, each with whether it is on.
    #[must_use]
    pub fn hack_options(&self) -> Vec<(HackOption, bool)> {
        self.nes
            .hack_options()
            .into_iter()
            .map(|option| {
                let on = self
                    .options
                    .hacks
                    .iter()
                    .find(|(key, _)| key == option.key)
                    .map_or(option.default, |(_, on)| *on);
                (option, on)
            })
            .collect()
    }

    fn hack_bits(&self) -> u16 {
        self.hack_options()
            .iter()
            .filter(|(_, on)| *on)
            .fold(0, |bits, (option, _)| bits | (1 << (option.bit & 15)))
    }

    /// The name of the built-in patch whose options apply to the game: the one running, or the
    /// one whose hack pack gives the game its aids.
    #[must_use]
    pub fn patch_name(&self) -> Option<&'static str> {
        self.nes.options_patch()
    }

    /// Runs one frame with the two pads' buttons (A, B, Select, Start, Up, Down, Left, Right in
    /// bits 0 to 7 of each byte, port 1 in the low byte), then makes its picture and sound.
    pub fn run(&mut self, input: u16) {
        self.nes.run_frame(input);
        self.make_picture();
        self.make_sound();
    }

    fn make_picture(&mut self) {
        let rows = self.nes.wide_picture_into(&mut self.indices);
        let rows = u32::try_from(rows).unwrap_or(240).max(1);
        let pixels = u32::try_from(self.indices.len()).unwrap_or(0);
        self.height = rows;
        self.width = (pixels / rows).max(1);
        self.framed = true;
        let mut colours = [0_u32; 64];
        for (index, colour) in colours.iter_mut().enumerate() {
            let rgb = zax_engine3d::palette::nes_color(index);
            *colour = (u32::from(rgb.r) << 16) | (u32::from(rgb.g) << 8) | u32::from(rgb.b);
        }
        self.frame.clear();
        self.frame.extend(
            self.indices
                .iter()
                .map(|index| colours[usize::from(index & 0x3F)]),
        );
    }

    fn make_sound(&mut self) {
        let available = self.nes.apu().available_samples();
        self.mono.resize(available, 0.0);
        let read = self.nes.read_audio(&mut self.mono, false).unwrap_or(0);
        self.audio.clear();
        for sample in &self.mono[..read] {
            #[allow(clippy::cast_possible_truncation)] // Clamped to the i16 range.
            let value = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
            self.audio.push(value);
            self.audio.push(value);
        }
    }

    /// The last frame's picture, 0RGB a pixel, row after row, and its width and height.
    #[must_use]
    pub fn picture(&self) -> (&[u32], u32, u32) {
        (&self.frame, self.width, self.height)
    }

    /// The width and height of the picture: the last frame's, or before a frame has run with
    /// the options in force, the one they ask for.
    #[must_use]
    pub fn picture_size(&self) -> (u32, u32) {
        if self.framed {
            return (self.width, self.height);
        }
        let margin = self.margin();
        (256 + 2 * margin.0, 240 + 2 * margin.1)
    }

    /// Whether the picture's size differs from the one the frontend was last told; it counts as
    /// told from now on.
    pub fn take_new_size(&mut self) -> bool {
        let size = self.picture_size();
        let new = size != self.announced;
        self.announced = size;
        new
    }

    /// The width over the height of the picture as shown: the NES's 4:3 for 256 x 240, widened
    /// by the margins.
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.picture_size();
        #[allow(clippy::cast_precision_loss)] // Small sizes.
        let ratio = (4.0 / 3.0) * (width as f32 / 256.0) / (height as f32 / 240.0);
        ratio
    }

    /// The last frame's sound, 16-bit stereo interleaved.
    #[must_use]
    pub fn sound(&self) -> &[i16] {
        &self.audio
    }

    /// The frames a second of the region in force.
    #[must_use]
    pub fn frames_per_second(&self) -> f64 {
        let timing = RegionTiming::for_region(self.nes.region());
        let dots_per_second = f64::from(timing.cpu_clock_hz)
            * f64::from(timing.dots_per_cycle_numerator)
            / f64::from(timing.dots_per_cycle_denominator);
        let mut dots_per_frame = f64::from(RegionTiming::DOTS_PER_SCANLINE)
            * f64::from(timing.scanlines_per_frame);
        if timing.skips_dot_on_odd_frames {
            dots_per_frame -= 0.5;
        }
        dots_per_second / dots_per_frame
    }

    /// Whether the region in force is PAL's frame rate.
    #[must_use]
    pub fn is_pal(&self) -> bool {
        matches!(self.nes.region(), NesRegion::Pal | NesRegion::Dendy)
    }

    /// The size every state of this game fits in.
    #[must_use]
    pub fn state_size(&self) -> usize {
        // The image is not in the state (the frontend keeps the ROM); the bound with it covers
        // the state without it.
        self.nes.max_state_size()
    }

    /// Writes the state into `out`, which is [`Core::state_size`] long; false when it does not
    /// fit.
    pub fn save_state(&self, out: &mut [u8]) -> bool {
        let Ok(state) = self.nes.save_state(false) else {
            return false;
        };
        let Some(head) = out.get_mut(..state.len()) else {
            return false;
        };
        head.copy_from_slice(&state);
        out[state.len()..].fill(0);
        true
    }

    /// Restores a state [`Core::save_state`] wrote.
    pub fn load_state(&mut self, data: &[u8]) -> bool {
        let region = self.nes.region();
        let loaded = self.nes.try_load_state(data, 0).is_ok();
        self.nes.set_region(region);
        loaded
    }

    /// The console's 2 KiB of work RAM.
    pub fn system_ram(&mut self) -> &mut [u8] {
        self.nes.bus_mut().ram_mut()
    }

    /// The cartridge's battery-backed RAM (none for most games).
    pub fn save_ram(&mut self) -> &mut [u8] {
        match self.nes.cartridge_mut() {
            Some(cartridge) => cartridge.battery_ram_mut(),
            None => &mut [],
        }
    }

    /// The reset button.
    pub fn reset(&mut self) {
        self.nes.soft_reset();
    }
}
