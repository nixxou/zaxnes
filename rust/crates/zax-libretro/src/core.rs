//! The game in play, in safe Rust: the NES core of `zax-nes` driven one frame at a time, its
//! picture turned into pixels, its sound into 16-bit stereo, its states and memories handed out
//! as the libretro API asks for them. The C side ([`crate::ffi`]) only moves data in and out.

use crate::overscan::Overscan;
use zax_nes::builtin_patches::HackOption;
use zax_nes::input::{ControllerPortType, ZapperPosition};
use zax_nes::ppu::SpriteLimit;
use zax_nes::{NesCore, NesRegion, RegionTiming};

/// The sample rate the core makes its sound at.
pub const SAMPLE_RATE: i32 = 48_000;
/// The bytes at the end of a state for the 120 Hz scrolling: whether the picture between two
/// frames is owed, and its shift.
const HALF_BYTES: usize = 5;

/// How many pixels the picture has for each NES pixel across with the 120 Hz scrolling on, so
/// that the playfield slides by quarters of a pixel.
pub const SMOOTH_SCALE: u32 = 4;
/// The widest picture: the NES's 256 pixels and the widest margins on each side, at the scale
/// of the 120 Hz scrolling.
pub const MAX_WIDTH: u32 = (256 + 2 * zax_nes::wide::MAX_MARGIN.unsigned_abs()) * SMOOTH_SCALE;
/// The widest the 3D is drawn at (the frontend's `MAX_3D`): a wider shape loses height.
pub const MAX_3D_WIDTH: u32 = 3840;
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
#[allow(clippy::struct_excessive_bools)] // One per core option.
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
    /// The 3D of a game that has a profile, drawn with the frontend's OpenGL (at the next load).
    pub three_d: bool,
    /// The size the 3D is drawn at (at the next load).
    pub resolution: (u32, u32),
    /// The shape of the 3D picture (its width follows from the resolution's height).
    pub three_d_aspect: ThreeDAspect,
    /// The render settings of the 3D (at once).
    pub render: crate::three_d::RenderOptions,
    /// The console's settings: core variant and hardware (most at the next load).
    pub machine: crate::machine::Machine,
    /// The picture's settings: television, composite signal, emphasis in 3D.
    pub picture: crate::picture::Picture,
    /// A crosshair where a Zapper is aimed (2D).
    pub zapper_crosshair: bool,
    /// The application's optimised Lua paths, `unsafe` code (at the next load).
    pub optimized_lua: bool,
    /// `video.widescreen_extra`: the share of the width, in percent, the margins grow by beyond
    /// the ratio, and the height as much, so that the 3D objects at the edges are whole.
    pub extra_percent: u16,
    /// `video.widescreen_strict_edges`: a band of the wider picture the game stopped drawing
    /// leaves its shapes unfinished in 3D.
    pub strict_edges: bool,
    /// The sides a television did not show, left out: top, bottom, left, right, in pixels. The
    /// left and right ones only without a widescreen margin.
    pub overscan: [u8; 4],
    /// The widescreen hack of another version of the game forced on a ROM it is not made for
    /// (at the next load).
    pub force_patch: bool,
    /// The 2D at twice the game's frame rate, for a display at 120 Hz: between two frames the
    /// playfield is drawn half way along the camera's motion (the application's experimental
    /// smooth scrolling: the camera a game's hack reports, its `smooth_scroll` aid).
    pub smooth_120: bool,
    /// With `smooth_120`, the motion of any game found by running a frame ahead
    /// (`emu.smooth_scroll_ahead`), when its hack does not report its camera.
    pub smooth_ahead: bool,
    /// With `smooth_120`, the 3D scene slides as well (`emu.smooth_scroll_3d`).
    pub smooth_3d: bool,
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
            three_d: true,
            resolution: (1280, 720),
            three_d_aspect: ThreeDAspect::Wide,
            render: crate::three_d::RenderOptions::default(),
            machine: crate::machine::Machine::default(),
            picture: crate::picture::Picture::default(),
            zapper_crosshair: true,
            optimized_lua: false,
            extra_percent: 0,
            strict_edges: false,
            overscan: [0; 4],
            force_patch: false,
            smooth_120: false,
            smooth_ahead: true,
            smooth_3d: false,
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

/// The shape of the 3D picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThreeDAspect {
    /// 16:9, as the core has always drawn it.
    #[default]
    Wide,
    /// 4:3, for bezels made for it and CRTs.
    FourThree,
    /// The widescreen ratio for a game it widens, 4:3 for the others.
    Widescreen,
}

/// The game in play.
pub struct Core {
    nes: NesCore,
    detected_region: NesRegion,
    options: Options,
    indices: Vec<u8>,
    /// The colour emphasis of every pixel of `indices` (PPUMASK bits 5-7 as 0-7), empty for a
    /// frame without any (lot FIDPLUS-D2b).
    emphasis: Vec<u8>,
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
    /// The 3D of the game, when it has a profile and the frontend draws it.
    three_d: Option<crate::three_d::ThreeD>,
    /// The sprites that move with the camera, not shifted with the playfield (120 Hz).
    overlay: Vec<u8>,
    /// With the 120 Hz scrolling, the second picture of the frame is due: its shift, and the
    /// sound the first left for it.
    half: Option<f32>,
    second_sound: Vec<i16>,
    /// The rows of the picture of the frame just run (120 Hz).
    rows: u32,
    /// The composite signal's decoder, and the picture it decodes (RGB).
    composite: zax_nes::composite::RowDecoder,
    rgb: Vec<u8>,
    /// Which ports have a Zapper plugged in.
    zappers: [bool; 2],
    /// Where each Zapper's pointer is on the 3D picture, 0 to 1 across and down.
    pointers: [Option<(f32, f32)>; 2],
    /// Where each Zapper is aimed, which the console asks once a frame.
    aim: std::sync::Arc<std::sync::Mutex<[ZapperPosition; 2]>>,
    /// The last 2D picture shown: its left and top in the full picture, its size, and the full
    /// picture's width and height (the aim of a Zapper is read on it).
    shown_area: (u32, u32, u32, u32, u32, u32),
}

impl Core {
    /// Loads `image` (an iNES file) named `name`, its hack as `options` say.
    ///
    /// # Errors
    ///
    /// The reason the ROM is refused.
    pub fn load(image: &[u8], name: &str, options: Options) -> Result<Self, String> {
        let mut nes = NesCore::new();
        options.machine.apply_before_load(&mut nes);
        nes.set_builtin_patches(options.widescreen);
        nes.set_advanced_patches(options.advanced);
        if options.force_patch {
            let stem = std::path::Path::new(name)
                .file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
            nes.set_forced_patch(zax_nes::builtin_patches::for_game_name(&stem));
        }
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
            emphasis: Vec::new(),
            frame: Vec::new(),
            width: 256,
            height: 240,
            mono: Vec::new(),
            audio: Vec::new(),
            framed: false,
            announced: (256, 240),
            three_d: None,
            overlay: Vec::new(),
            half: None,
            second_sound: Vec::new(),
            rows: 240,
            composite: zax_nes::composite::RowDecoder::default(),
            rgb: Vec::new(),
            zappers: [false; 2],
            pointers: [None; 2],
            aim: std::sync::Arc::new(std::sync::Mutex::new([ZapperPosition::OFF_SCREEN; 2])),
            shown_area: (0, 0, 256, 240, 256, 240),
        };
        let aim = std::sync::Arc::clone(&core.aim);
        core.nes
            .set_zapper_position_source(Some(Box::new(move |port| {
                aim.lock()
                    .ok()
                    .and_then(|aims| aims.get(port).copied())
                    .unwrap_or(ZapperPosition::OFF_SCREEN)
            })));
        core.options.widescreen = options.widescreen;
        core.options.advanced = options.advanced;
        core.set_options(options);
        core.announced = core.picture_size();
        Ok(core)
    }

    /// Plays the game in 3D with `profile` (its tiles resolved by `tiles` for a "3dh" one): its
    /// scripts start and the machine resets. `log` receives what the scripts print.
    ///
    /// # Errors
    ///
    /// Why the scripts or the profile were refused; the game then stays in 2D.
    pub fn start_three_d(
        &mut self,
        profile: &zax_profiles::profile::GameProfile,
        tiles: Option<zax_profiles::hash::tiles::TileResolver>,
        log: fn(&str),
    ) -> Result<(), String> {
        self.three_d = Some(crate::three_d::ThreeD::start(
            &mut self.nes,
            profile,
            tiles,
            &self.options.render,
            self.options.optimized_lua,
            log,
        )?);
        self.frame_three_d();
        Ok(())
    }

    /// Back to 2D (the frontend cannot draw the 3D).
    pub fn stop_three_d(&mut self) {
        self.three_d = None;
    }

    /// The 3D of the game, while it plays in 3D.
    pub fn three_d_mut(&mut self) -> Option<&mut crate::three_d::ThreeD> {
        self.three_d.as_mut()
    }

    /// Whether the game plays in 3D.
    #[must_use]
    pub const fn is_three_d(&self) -> bool {
        self.three_d.is_some()
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
        if options.smooth_120 != self.options.smooth_120
            || (self.three_d.is_some() && options.smooth_3d != self.options.smooth_3d)
        {
            changed.timing = true;
            changed.geometry = true;
            self.half = None;
        }
        self.nes
            .set_smooth_scroll_ahead(options.smooth_120 && options.smooth_ahead);
        if (options.widescreen, options.advanced)
            != (self.options.widescreen, self.options.advanced)
        {
            changed.hack_options = self.switch_patch(options.widescreen, options.advanced);
        }
        if options.machine != self.options.machine {
            options.machine.apply_live(&mut self.nes);
        }
        zax_engine3d::palette::select_nes_palette(options.palette);
        zax_engine3d::palette::set_tv_look(options.picture.tv_look());
        if let Some((decoder, _)) = options.picture.composite() {
            if self.composite.options() != &decoder {
                self.composite.set_options(decoder);
            }
        }
        self.nes.set_sprite_limit(options.sprite_limit);
        if options.render != self.options.render {
            if let Some(three_d) = self.three_d.as_mut() {
                three_d.set_render(&mut self.nes, &options.render);
            }
        }
        let before = (self.margin(), self.options.overscan);
        self.options = options;
        let margin = self.margin();
        changed.geometry |= (margin, self.options.overscan) != before;
        if changed.geometry || changed.hack_options {
            self.framed = false;
        }
        self.nes
            .set_widescreen_margin(i32::try_from(margin.0).unwrap_or(0));
        self.nes
            .set_widescreen_vertical_margin(i32::try_from(margin.1).unwrap_or(0));
        self.nes.set_hack_options(self.hack_bits());
        self.frame_three_d();
        changed
    }

    /// Tells the 3D the framing the options ask for.
    fn frame_three_d(&mut self) {
        let overscan = self.overscan();
        let strict = self.options.strict_edges;
        if let Some(three_d) = self.three_d.as_mut() {
            three_d.overscan = overscan;
            three_d.strict_edges = strict;
        }
    }

    /// The sides the options leave out, all four (`Overscan::in_force` drops the left and right
    /// ones with a margin).
    fn overscan(&self) -> Overscan {
        let [top, bottom, left, right] = self.options.overscan.map(i32::from);
        Overscan {
            top,
            bottom,
            left,
            right,
            resize: false,
        }
    }

    /// The part of a picture `width` by `height` (its margins around the screen's 256 by 240)
    /// that is shown: first column, first row, width and height.
    fn shown(&self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let margin = width.saturating_sub(256) / 2;
        let margin_y = height.saturating_sub(240) / 2;
        let cut = self.overscan().in_force(i32::try_from(margin).unwrap_or(0));
        let side = |pixels: i32| u32::try_from(pixels.clamp(0, 64)).unwrap_or(0);
        // As `Overscan::leave_out`: a side cut takes the vertical margin past it along.
        let top = if cut.top > 0 {
            margin_y + side(cut.top)
        } else {
            0
        };
        let bottom = if cut.bottom > 0 {
            margin_y + side(cut.bottom)
        } else {
            0
        };
        let (left, right) = (side(cut.left), side(cut.right));
        (
            left,
            top,
            width.saturating_sub(left + right).max(1),
            height.saturating_sub(top + bottom).max(1),
        )
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
        // As the application's `widescreen_margin`: whole tiles, the bonus on both axes.
        let bonus = 1.0 + f32::from(self.options.extra_percent) / 100.0;
        let whole_tiles = |pixels: f32, max: i32| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Clamped.
            let tiles = (pixels.max(0.0) / 8.0).ceil() as u32;
            (tiles * 8).min(max.unsigned_abs())
        };
        (
            whole_tiles(
                (240.0 * ratio * bonus - 256.0) / 2.0,
                zax_nes::wide::MAX_MARGIN,
            ),
            whole_tiles(
                240.0 * (bonus - 1.0) / 2.0,
                zax_nes::wide::MAX_VERTICAL_MARGIN,
            ),
        )
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
    ///
    /// With the 120 Hz scrolling on, it takes two calls a frame: the first runs it and shows its
    /// picture, the second shows the playfield half way to the next (in 3D, its tiles).
    pub fn run(&mut self, input: u16) {
        if self.smooth() {
            self.run_smooth(input);
            return;
        }
        if self.doubled() {
            self.run_smooth_three_d(input);
            return;
        }
        self.nes.run_frame(input);
        if let Some(three_d) = self.three_d.as_mut() {
            let emphasis = if self.options.picture.emphasis_in_three_d() {
                self.nes.wide_emphasis_into(&mut self.emphasis);
                crate::picture::common_emphasis(&self.emphasis)
            } else {
                0
            };
            zax_engine3d::palette::set_scene_emphasis(emphasis);
            three_d.convert(&mut self.nes);
            three_d.set_smooth_band(None);
        } else {
            self.make_picture();
        }
        self.make_sound();
    }

    /// Whether the 120 Hz scrolling is in force in 2D: asked for, and the game in 2D.
    fn smooth(&self) -> bool {
        self.options.smooth_120 && self.three_d.is_none()
    }

    /// Whether the 120 Hz scrolling is in force, in 2D or (asked for) in 3D: two calls a frame.
    fn doubled(&self) -> bool {
        self.options.smooth_120 && (self.three_d.is_none() || self.options.smooth_3d)
    }

    /// [`Core::run_smooth`] in 3D: the first call runs the frame and converts it, its tiles where
    /// the frame put them; the second slides the playfield's tiles half way to the next frame,
    /// as the application does between two frames.
    fn run_smooth_three_d(&mut self, input: u16) {
        if let Some(amount) = self.half.take() {
            if let Some(three_d) = self.three_d.as_mut() {
                three_d.slide(amount);
            }
            self.audio = std::mem::take(&mut self.second_sound);
            return;
        }
        self.nes
            .run_frame_ahead(input, u8::from(self.options.smooth_ahead));
        let shift = self.nes.picture_shift();
        let motion = self.nes.picture_motion().unwrap_or(0.0);
        let steady = self.nes.steady_sprites().to_vec();
        if let Some(three_d) = self.three_d.as_mut() {
            let emphasis = if self.options.picture.emphasis_in_three_d() {
                self.nes.wide_emphasis_into(&mut self.emphasis);
                crate::picture::common_emphasis(&self.emphasis)
            } else {
                0
            };
            zax_engine3d::palette::set_scene_emphasis(emphasis);
            three_d.convert(&mut self.nes);
            three_d.set_smooth_band(shift.map(|(first, start, end)| {
                zax_render::scene::ThreeDShift {
                    amount: first,
                    start,
                    end,
                    steady,
                }
            }));
        }
        self.make_sound();
        self.nes.restore_real_frame();
        self.half = shift.map(|(first, _, _)| first + motion / 2.0);
        // The frame's sound in two halves, whole stereo frames each.
        let middle = (self.audio.len() / 4) * 2;
        self.second_sound.clear();
        self.second_sound.extend_from_slice(&self.audio[middle..]);
        self.audio.truncate(middle);
    }

    fn run_smooth(&mut self, input: u16) {
        if let Some(shift) = self.half.take() {
            self.make_shifted_picture(shift);
            self.audio = std::mem::take(&mut self.second_sound);
            return;
        }
        self.nes
            .run_frame_ahead(input, u8::from(self.options.smooth_ahead));
        let rows = self.nes.wide_picture_into(&mut self.indices);
        self.nes.wide_emphasis_into(&mut self.emphasis);
        self.rows = u32::try_from(rows).unwrap_or(240).max(1);
        self.nes
            .split_steady_sprites(&mut self.indices, &mut self.overlay);
        let shift = self.nes.picture_shift();
        let motion = self.nes.picture_motion().unwrap_or(0.0);
        self.make_sound();
        self.nes.restore_real_frame();
        let first = shift.map_or(0.0, |(shift, _, _)| shift);
        self.make_shifted_picture(first);
        self.half = Some(first + motion / 2.0);
        // The frame's sound in two halves, whole stereo frames each.
        let middle = (self.audio.len() / 4) * 2;
        self.second_sound.clear();
        self.second_sound.extend_from_slice(&self.audio[middle..]);
        self.audio.truncate(middle);
    }

    /// The picture of the frame just run at [`SMOOTH_SCALE`] times its width, its playfield
    /// moved left by `shift` pixels (quarters of one), the sprites that move with the camera
    /// left where they are.
    fn make_shifted_picture(&mut self, shift: f32) {
        let pixels = u32::try_from(self.indices.len()).unwrap_or(0);
        let full_height = self.rows;
        let full_width = (pixels / full_height).max(1);
        let (x0, y0, width, height) = self.shown(full_width, full_height);
        let margin_y = full_height.saturating_sub(240) / 2;
        let band = self.nes.picture_shift().map_or(0..0, |(_, start, end)| {
            let start = u32::try_from(start).unwrap_or(0) + margin_y;
            let end = u32::try_from(end).unwrap_or(0) + margin_y;
            start..end
        });
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // A few pixels.
        let quarters = (shift * SMOOTH_SCALE as f32).round() as i64;
        let colours = Colours::of(&self.emphasis, self.indices.len());
        let scale = SMOOTH_SCALE as usize;
        let (full_width, x0, width) = (full_width as usize, x0 as usize, width as usize);
        self.width = u32::try_from(width * scale).unwrap_or(1);
        self.height = height;
        self.framed = true;
        self.frame.clear();
        for y in y0..y0 + height {
            let start = y as usize * full_width;
            let Some(row) = self.indices.get(start..start + full_width) else {
                break;
            };
            let overlay = self.overlay.get(start..start + full_width);
            let moved = band.contains(&y);
            for out in 0..width * scale {
                let place = i64::try_from(x0 * scale + out).unwrap_or(0);
                let source = if moved { place + quarters } else { place };
                let column = usize::try_from(source.max(0) / i64::from(SMOOTH_SCALE))
                    .unwrap_or(0)
                    .min(full_width - 1);
                let still = overlay
                    .and_then(|overlay| overlay.get(x0 + out / scale))
                    .copied()
                    .filter(|index| *index != 0xFF);
                let pixel = start
                    + if still.is_some() {
                        x0 + out / scale
                    } else {
                        column
                    };
                let index = still.unwrap_or(row[column]);
                self.frame.push(colours.get(index, &self.emphasis, pixel));
            }
        }
    }

    fn make_picture(&mut self) {
        let rows = self.nes.wide_picture_into(&mut self.indices);
        self.nes.wide_emphasis_into(&mut self.emphasis);
        let rows = u32::try_from(rows).unwrap_or(240).max(1);
        let pixels = u32::try_from(self.indices.len()).unwrap_or(0);
        let full_width = (pixels / rows).max(1);
        let (x, y, width, height) = self.shown(full_width, rows);
        self.height = height;
        self.width = width;
        self.framed = true;
        self.shown_area = (x, y, width, height, full_width, rows);
        self.frame.clear();
        let (x, width, full_width) = (x as usize, width as usize, full_width as usize);
        if let Some((_, frozen)) = self.options.picture.composite() {
            let phase = self.nes.video_phase();
            // Frozen: every frame at the same phase (the 2C07's does not crawl anyway).
            let phase = if frozen {
                phase & zax_nes::composite::PAL_VIDEO
            } else {
                phase
            };
            crate::picture::decode_composite(
                &mut self.composite,
                &self.indices,
                &self.emphasis,
                (full_width, rows as usize),
                phase,
                &mut self.rgb,
            );
            for row in (y as usize..).take(height as usize) {
                let end = (x + width).min(full_width);
                let start = (row * full_width + x) * 3;
                let stop = (row * full_width + end) * 3;
                if let Some(pixels) = self.rgb.get(start..stop) {
                    self.frame.extend(pixels.chunks_exact(3).map(|rgb| {
                        (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2])
                    }));
                }
            }
            self.draw_crosshairs();
            return;
        }
        let colours = Colours::of(&self.emphasis, self.indices.len());
        for (number, row) in self
            .indices
            .chunks(full_width)
            .enumerate()
            .skip(y as usize)
            .take(height as usize)
        {
            let start = number * full_width;
            let end = (x + width).min(row.len());
            self.frame.extend(
                row[x..end]
                    .iter()
                    .zip(x..end)
                    .map(|(&index, column)| colours.get(index, &self.emphasis, start + column)),
            );
        }
        self.draw_crosshairs();
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
        let (_, _, width, height) = self.shown(256 + 2 * margin.0, 240 + 2 * margin.1);
        if self.smooth() {
            (width * SMOOTH_SCALE, height)
        } else {
            (width, height)
        }
    }

    /// Whether the picture's size differs from the one the frontend was last told; it counts as
    /// told from now on.
    pub fn take_new_size(&mut self) -> bool {
        let size = self.picture_size();
        let new = size != self.announced;
        self.announced = size;
        new
    }

    /// The size the 3D is drawn at: the resolution's height, the width its shape gives.
    #[must_use]
    pub fn three_d_size(&self) -> (u32, u32) {
        let height = self.options.resolution.1.max(1);
        let ratio = match self.options.three_d_aspect {
            ThreeDAspect::Wide => 16.0 / 9.0,
            ThreeDAspect::Widescreen if self.margin().0 > 0 => {
                self.options.ratio.clamp(4.0 / 3.0, 32.0 / 9.0)
            }
            // 4:3, and the widescreen ratio of a game it does not widen.
            ThreeDAspect::FourThree | ThreeDAspect::Widescreen => 4.0 / 3.0,
        };
        // An even width, as video sizes are.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )] // Screen sizes.
        let width = ((height as f32 * ratio / 2.0).round() as u32 * 2).max(2);
        // Within the widest the frontend was told (3840): a wider shape loses height.
        if width > MAX_3D_WIDTH {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )] // Screen sizes.
            let height = ((MAX_3D_WIDTH as f32 / ratio / 2.0).round() as u32 * 2).max(2);
            return (MAX_3D_WIDTH, height);
        }
        (width, height)
    }

    /// The width over the height of the picture as shown: square pixels, as the application
    /// draws its 2D picture (`Scene::picture_aspect`), which is what the widescreen ratios are
    /// computed for (16:9 is 427 x 240, 432 in whole tiles).
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.picture_size();
        let width = if self.smooth() {
            width / SMOOTH_SCALE
        } else {
            width
        };
        #[allow(clippy::cast_precision_loss)] // Small sizes.
        let ratio = width as f32 / height as f32;
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
        let mut dots_per_frame =
            f64::from(RegionTiming::DOTS_PER_SCANLINE) * f64::from(timing.scanlines_per_frame);
        if timing.skips_dot_on_odd_frames {
            dots_per_frame -= 0.5;
        }
        let rate = dots_per_second / dots_per_frame;
        if self.doubled() {
            rate * 2.0
        } else {
            rate
        }
    }

    /// Plugs a Zapper into `port` (0 or 1), or a pad when `on` is false.
    pub fn set_zapper(&mut self, port: usize, on: bool) {
        let kind = if on {
            ControllerPortType::Zapper
        } else {
            ControllerPortType::Controller
        };
        if port < 2 && self.nes.set_port_type(port, kind).is_ok() {
            self.zappers[port] = on;
        }
    }

    /// Which ports have a Zapper plugged in.
    #[must_use]
    pub const fn zappers(&self) -> [bool; 2] {
        self.zappers
    }

    /// Aims the Zapper of `port` at `point` of the picture shown, 0 to 1 across and down, or
    /// away from the screen. In 3D the aim goes through the camera to the picture's plane, as
    /// in the application.
    pub fn aim_zapper(&mut self, port: usize, point: Option<(f32, f32)>) {
        let (left, top, width, height, full_width, rows) = self.shown_area;
        let size = self.three_d_size();
        if let Some(pointer) = self.pointers.get_mut(port) {
            *pointer = point;
        }
        let position = match point {
            Some(point) if self.three_d.is_some() => self
                .three_d
                .as_ref()
                .and_then(|three_d| three_d.aim(point, size))
                .unwrap_or(ZapperPosition::OFF_SCREEN),
            Some((x, y)) => {
                let margin = full_width.saturating_sub(256) / 2;
                let margin_y = rows.saturating_sub(240) / 2;
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_precision_loss,
                    clippy::cast_possible_wrap
                )] // Screen pixels.
                let to_pixel = |along: f32, size: u32, start: u32, origin: u32| {
                    (along.clamp(0.0, 1.0) * size as f32).floor() as i32 + start as i32
                        - origin as i32
                };
                ZapperPosition::new(
                    to_pixel(x, width, left, margin),
                    to_pixel(y, height, top, margin_y),
                )
            }
            _ => ZapperPosition::OFF_SCREEN,
        };
        if let Ok(mut aims) = self.aim.lock() {
            if let Some(aim) = aims.get_mut(port) {
                *aim = position;
            }
        }
    }

    /// Where to draw a crosshair on the 3D picture: each Zapper's pointer, 0 to 1 across and
    /// down; none without the option or outside 3D.
    #[must_use]
    pub fn three_d_crosshairs(&self) -> Vec<(f32, f32)> {
        if !self.options.zapper_crosshair || self.three_d.is_none() {
            return Vec::new();
        }
        (0..2)
            .filter(|&port| self.zappers[port])
            .filter_map(|port| self.pointers[port])
            .collect()
    }

    /// Draws a crosshair where each Zapper is aimed, on the 2D picture just made.
    fn draw_crosshairs(&mut self) {
        if !self.options.zapper_crosshair || self.zappers == [false; 2] {
            return;
        }
        let (left, top, width, height, full_width, rows) = self.shown_area;
        let margin = i32::try_from(full_width.saturating_sub(256) / 2).unwrap_or(0);
        let margin_y = i32::try_from(rows.saturating_sub(240) / 2).unwrap_or(0);
        let aims = self
            .aim
            .lock()
            .map_or([ZapperPosition::OFF_SCREEN; 2], |aims| *aims);
        let (w, h) = (
            i32::try_from(width).unwrap_or(0),
            i32::try_from(height).unwrap_or(0),
        );
        for (port, aim) in aims.iter().enumerate() {
            if !self.zappers[port] || !aim.is_on_screen() {
                continue;
            }
            let x = aim.x + margin - i32::try_from(left).unwrap_or(0);
            let y = aim.y + margin_y - i32::try_from(top).unwrap_or(0);
            // A white cross with a black edge, 4 pixels each way, the centre left clear.
            for (dx, dy) in (-5_i32..=5).flat_map(|d| [(d, 0), (0, d)]) {
                if dx.abs() + dy.abs() < 2 {
                    continue;
                }
                for (ex, ey, colour) in [
                    (1_i32, 0_i32, 0x0000_0000_u32),
                    (-1, 0, 0x0000_0000),
                    (0, 1, 0x0000_0000),
                    (0, -1, 0x0000_0000),
                    (0, 0, 0x00FF_FFFF),
                ] {
                    let edge = (ex, ey) != (0, 0);
                    if edge && (dx.abs() == 5 || dy.abs() == 5) {
                        continue;
                    }
                    let (px, py) = (x + dx + ex, y + dy + ey);
                    if px < 0 || py < 0 || px >= w || py >= h {
                        continue;
                    }
                    let index = usize::try_from(py * w + px).unwrap_or(usize::MAX);
                    if let Some(pixel) = self.frame.get_mut(index) {
                        if !edge || *pixel != 0x00FF_FFFF {
                            *pixel = colour;
                        }
                    }
                }
            }
        }
    }

    /// Whether the region in force is PAL's frame rate.
    #[must_use]
    pub fn is_pal(&self) -> bool {
        matches!(self.nes.region(), NesRegion::Pal | NesRegion::Dendy)
    }

    /// The size every state of this game fits in: the machine's (the image is not in it: the
    /// frontend keeps the ROM, and the bound with it covers the state without it), then in 3D
    /// the scripts' `PData` block, then [`HALF_BYTES`] for the picture between two frames the
    /// 120 Hz scrolling owes (so that the frontend's run-ahead and rewind keep its pace).
    #[must_use]
    pub fn state_size(&self) -> usize {
        let scripts = if self.three_d.is_some() {
            zax_scripting::persistent::BLOCK_SIZE
        } else {
            0
        };
        self.nes.max_state_size() + scripts + HALF_BYTES
    }

    /// Writes the state into `out`, which is [`Core::state_size`] long; false when it does not
    /// fit.
    pub fn save_state(&self, out: &mut [u8]) -> bool {
        let Ok(state) = self.nes.save_state(false) else {
            return false;
        };
        let machine = self.nes.max_state_size();
        if state.len() > machine || out.len() < machine {
            return false;
        }
        let (head, tail) = out.split_at_mut(machine);
        head[..state.len()].copy_from_slice(&state);
        head[state.len()..].fill(0);
        tail.fill(0);
        if let Some(block) = self
            .three_d
            .as_ref()
            .and_then(crate::three_d::ThreeD::persistent_data)
        {
            if let Some(room) = tail.get_mut(..block.len()) {
                room.copy_from_slice(&block);
            }
        }
        if let (Some(shift), Some(room)) = (self.half, tail.len().checked_sub(HALF_BYTES)) {
            tail[room] = 1;
            tail[room + 1..room + HALF_BYTES].copy_from_slice(&shift.to_le_bytes());
        }
        true
    }

    /// Restores a state [`Core::save_state`] wrote.
    pub fn load_state(&mut self, data: &[u8]) -> bool {
        self.half = data
            .len()
            .checked_sub(HALF_BYTES)
            .and_then(|at| data.get(at..))
            .filter(|half| half[0] == 1)
            .and_then(|half| half[1..].try_into().ok())
            .map(f32::from_le_bytes)
            .filter(|_| self.doubled());
        let region = self.nes.region();
        let loaded = self.nes.try_load_state(data, 0).is_ok();
        self.nes.set_region(region);
        let machine = self.nes.max_state_size();
        if let (true, Some(three_d), Some(block)) = (
            loaded,
            self.three_d.as_ref(),
            data.get(machine..machine + zax_scripting::persistent::BLOCK_SIZE),
        ) {
            three_d.restore_persistent_data(block);
        }
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
        self.half = None;
        self.nes.soft_reset();
    }
}

/// The colours of a picture: the palette's 64, or with a colour emphasis plane its 512 (lot
/// FIDPLUS-D2b), each pixel taking the emphasis it was drawn with, as the app's 2D picture.
struct Colours(Vec<u32>);

impl Colours {
    /// The table for a picture of `pixels` pixels with the emphasis plane `emphasis` (empty, or
    /// not of the picture's size, for none).
    fn of(emphasis: &[u8], pixels: usize) -> Self {
        if emphasis.is_empty() || emphasis.len() != pixels {
            return Self(palette_colours().to_vec());
        }
        Self(
            (0..512_usize)
                .map(|entry| {
                    #[allow(clippy::cast_possible_truncation)] // 0-7
                    let bits = (entry >> 6) as u8;
                    pack(zax_engine3d::palette::nes_color_emphasized(
                        entry & 0x3F,
                        bits,
                    ))
                })
                .collect(),
        )
    }

    /// Whether the table carries the emphasis.
    fn emphasized(&self) -> bool {
        self.0.len() == 512
    }

    /// The colour of NES colour `index` at picture pixel `pixel`.
    fn get(&self, index: u8, emphasis: &[u8], pixel: usize) -> u32 {
        let index = usize::from(index & 0x3F);
        let bits = if self.emphasized() {
            usize::from(emphasis.get(pixel).copied().unwrap_or(0) & 7)
        } else {
            0
        };
        self.0[(bits << 6) | index]
    }
}

/// An RGB colour as 0RGB.
fn pack(rgb: zax_engine3d::palette::Rgb24) -> u32 {
    (u32::from(rgb.r) << 16) | (u32::from(rgb.g) << 8) | u32::from(rgb.b)
}

/// The NES colours of the palette in force, 0RGB.
fn palette_colours() -> [u32; 64] {
    let mut colours = [0_u32; 64];
    for (index, colour) in colours.iter_mut().enumerate() {
        *colour = pack(zax_engine3d::palette::nes_color(index));
    }
    colours
}

#[cfg(test)]
mod tests {
    use super::Colours;

    #[test]
    fn a_picture_without_emphasis_takes_the_palette() {
        let colours = Colours::of(&[], 4);
        assert!(!colours.emphasized());
        let plain = super::palette_colours();
        assert_eq!(colours.get(0x16, &[], 2), plain[0x16]);
    }

    #[test]
    fn each_pixel_takes_its_own_emphasis() {
        let emphasis = [0, 7, 1];
        let colours = Colours::of(&emphasis, 3);
        let plain = super::palette_colours();
        assert_eq!(colours.get(0x30, &emphasis, 0), plain[0x30]);
        let dark = super::pack(zax_engine3d::palette::nes_color_emphasized(0x30, 7));
        assert_eq!(colours.get(0x30, &emphasis, 1), dark);
        assert_ne!(dark, plain[0x30]);
        // A plane not of the picture's size is no plane.
        assert!(!Colours::of(&emphasis, 4).emphasized());
    }
}
