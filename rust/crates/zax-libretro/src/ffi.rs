//! The libretro API: the `retro_*` functions `RetroArch` calls, and the frontend's callbacks.
//!
//! This is the one module of the crate with `unsafe` (owner decision, 2026-11-12, as for the C
//! layers of `zax-achievements` and `zax-android`): a C interface hands raw pointers both ways.
//! It only moves data between them and [`Core`]; every decision is made in safe code.
//!
//! The structures and numbers are those of `libretro.h` (API version 1). The game and the
//! OpenGL renderer live in thread-local storage: the frontend calls a core from one thread, and
//! the Lua runtime and the GL objects are not to be shared between threads anyway.

use std::cell::RefCell;
use std::ffi::{c_char, c_uint, c_void, CStr, CString};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, PoisonError};

use zax_render::target::RenderTarget;
use zax_render::SceneRenderer;

use crate::core::{Core, Options, MAX_HEIGHT, MAX_WIDTH, SAMPLE_RATE};
use crate::options;
use crate::profiles;
use crate::three_d::CameraNews;

const API_VERSION: c_uint = 1;

const ENVIRONMENT_SET_MESSAGE: c_uint = 6;
const ENVIRONMENT_GET_SYSTEM_DIRECTORY: c_uint = 9;
const ENVIRONMENT_SET_PIXEL_FORMAT: c_uint = 10;
const ENVIRONMENT_SET_INPUT_DESCRIPTORS: c_uint = 11;
const ENVIRONMENT_SET_HW_RENDER: c_uint = 14;
const ENVIRONMENT_GET_VARIABLE: c_uint = 15;
const ENVIRONMENT_SET_VARIABLES: c_uint = 16;
const ENVIRONMENT_GET_VARIABLE_UPDATE: c_uint = 17;
const ENVIRONMENT_GET_LOG_INTERFACE: c_uint = 27;
const ENVIRONMENT_SET_SYSTEM_AV_INFO: c_uint = 32;
const ENVIRONMENT_SET_GEOMETRY: c_uint = 37;
const ENVIRONMENT_GET_CORE_OPTIONS_VERSION: c_uint = 52;
const ENVIRONMENT_SET_CORE_OPTIONS_V2: c_uint = 67;
/// `RETRO_NUM_CORE_OPTION_VALUES_MAX`: the values of an option, its closing null pair included.
const OPTION_VALUES_MAX: usize = 128;

const PIXEL_FORMAT_XRGB8888: c_uint = 1;
const HW_CONTEXT_OPENGL_CORE: c_uint = 3;
const DEVICE_JOYPAD: c_uint = 1;
/// `RETRO_DEVICE_LIGHTGUN`, the device type of the Zapper.
const DEVICE_LIGHTGUN: c_uint = 4;
const ENVIRONMENT_SET_CONTROLLER_INFO: c_uint = 35;
const LIGHTGUN_TRIGGER: c_uint = 2;
const LIGHTGUN_SCREEN_X: c_uint = 13;
const LIGHTGUN_SCREEN_Y: c_uint = 14;
const LIGHTGUN_IS_OFFSCREEN: c_uint = 15;
const LIGHTGUN_RELOAD: c_uint = 16;
/// What a Zapper port reads while its trigger is released, and held.
const ZAPPER_RELEASED: u8 = 0x10;
const ZAPPER_PRESSED: u8 = 0x00;
const DEVICE_ANALOG: c_uint = 5;
const ANALOG_RIGHT: c_uint = 1;
/// The `RetroPad`'s west and north buttons, and its shoulders.
const JOYPAD_Y: c_uint = 1;
const JOYPAD_X: c_uint = 9;
const JOYPAD_L: c_uint = 10;
const JOYPAD_R: c_uint = 11;
const JOYPAD_L2: c_uint = 12;
const MEMORY_SAVE_RAM: c_uint = 0;
const MEMORY_SYSTEM_RAM: c_uint = 2;
const REGION_NTSC: c_uint = 0;
const REGION_PAL: c_uint = 1;
const LOG_INFO: c_uint = 1;
const LOG_ERROR: c_uint = 3;
/// The widest and tallest the 3D is drawn at.
const MAX_3D: (c_uint, c_uint) = (3840, 2160);

/// The libretro joypad buttons in the NES pad's bit order: A, B, Select, Start, Up, Down,
/// Left, Right (`RETRO_DEVICE_ID_JOYPAD_*`).
const PAD: [(c_uint, &CStr); 8] = [
    (8, c"A"),
    (0, c"B"),
    (2, c"Select"),
    (3, c"Start"),
    (4, c"Up"),
    (5, c"Down"),
    (6, c"Left"),
    (7, c"Right"),
];

/// The other controls of the 3D, named in the frontend's input menu: the application's own
/// places (HOWTO-RUN, the gamepad table): west 2D / 3D, north the default view, LB / RB the
/// camera farther / nearer, L2 keeps and puts back its angle, the right stick turns it.
const CAMERA: [(c_uint, c_uint, c_uint, &CStr); 7] = [
    (DEVICE_ANALOG, ANALOG_RIGHT, 0, c"Camera: turn"),
    (DEVICE_ANALOG, ANALOG_RIGHT, 1, c"Camera: tilt"),
    (DEVICE_JOYPAD, 0, JOYPAD_L, c"Camera: farther"),
    (DEVICE_JOYPAD, 0, JOYPAD_R, c"Camera: nearer"),
    (DEVICE_JOYPAD, 0, JOYPAD_Y, c"2D / 3D (hold: 2D head on)"),
    (
        DEVICE_JOYPAD,
        0,
        JOYPAD_L2,
        c"Camera: put back (hold: keep)",
    ),
    (DEVICE_JOYPAD, 0, JOYPAD_X, c"Default view"),
];

type EnvironmentFn = unsafe extern "C" fn(cmd: c_uint, data: *mut c_void) -> bool;
type VideoRefreshFn =
    unsafe extern "C" fn(data: *const c_void, width: c_uint, height: c_uint, pitch: usize);
type AudioSampleFn = unsafe extern "C" fn(left: i16, right: i16);
type AudioSampleBatchFn = unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;
type InputPollFn = unsafe extern "C" fn();
type InputStateFn =
    unsafe extern "C" fn(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16;
type LogFn = unsafe extern "C" fn(level: c_uint, format: *const c_char, ...);
type HwContextFn = unsafe extern "C" fn();
type HwFramebufferFn = unsafe extern "C" fn() -> usize;
type HwProcAddressFn = unsafe extern "C" fn(symbol: *const c_char) -> *const c_void;

#[repr(C)]
pub struct SystemInfo {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GameGeometry {
    base_width: c_uint,
    base_height: c_uint,
    max_width: c_uint,
    max_height: c_uint,
    aspect_ratio: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SystemTiming {
    fps: f64,
    sample_rate: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SystemAvInfo {
    geometry: GameGeometry,
    timing: SystemTiming,
}

#[repr(C)]
pub struct GameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[repr(C)]
struct Variable {
    key: *const c_char,
    value: *const c_char,
}

/// `struct retro_core_option_value`.
#[repr(C)]
#[derive(Clone, Copy)]
struct OptionValue {
    value: *const c_char,
    label: *const c_char,
}

/// `struct retro_core_option_v2_category`.
#[repr(C)]
struct OptionCategory {
    key: *const c_char,
    desc: *const c_char,
    info: *const c_char,
}

/// `struct retro_core_option_v2_definition`.
#[repr(C)]
struct OptionDefinition {
    key: *const c_char,
    desc: *const c_char,
    desc_categorized: *const c_char,
    info: *const c_char,
    info_categorized: *const c_char,
    category_key: *const c_char,
    values: [OptionValue; OPTION_VALUES_MAX],
    default_value: *const c_char,
}

/// `struct retro_core_options_v2`.
#[repr(C)]
struct OptionsV2 {
    categories: *mut OptionCategory,
    definitions: *mut OptionDefinition,
}

/// `struct retro_controller_description`.
#[repr(C)]
struct ControllerDescription {
    desc: *const c_char,
    id: c_uint,
}

/// `struct retro_controller_info`.
#[repr(C)]
struct ControllerInfo {
    types: *const ControllerDescription,
    num_types: c_uint,
}

#[repr(C)]
struct InputDescriptor {
    port: c_uint,
    device: c_uint,
    index: c_uint,
    id: c_uint,
    description: *const c_char,
}

#[repr(C)]
struct LogCallback {
    log: Option<LogFn>,
}

/// `struct retro_hw_render_callback`.
#[repr(C)]
struct HwRenderCallback {
    context_type: c_uint,
    context_reset: Option<HwContextFn>,
    get_current_framebuffer: Option<HwFramebufferFn>,
    get_proc_address: Option<HwProcAddressFn>,
    depth: bool,
    stencil: bool,
    bottom_left_origin: bool,
    version_major: c_uint,
    version_minor: c_uint,
    cache_context: bool,
    context_destroy: Option<HwContextFn>,
    debug_context: bool,
}

/// The frontend's callbacks.
#[derive(Clone, Copy)]
struct Frontend {
    environment: Option<EnvironmentFn>,
    video_refresh: Option<VideoRefreshFn>,
    audio_sample_batch: Option<AudioSampleBatchFn>,
    input_poll: Option<InputPollFn>,
    input_state: Option<InputStateFn>,
    log: Option<LogFn>,
    framebuffer: Option<HwFramebufferFn>,
    proc_address: Option<HwProcAddressFn>,
}

static FRONTEND: Mutex<Frontend> = Mutex::new(Frontend {
    environment: None,
    video_refresh: None,
    audio_sample_batch: None,
    input_poll: None,
    input_state: None,
    log: None,
    framebuffer: None,
    proc_address: None,
});

/// The renderer of the 3D, on the frontend's OpenGL context, and the framebuffer it draws into.
struct Gl {
    renderer: SceneRenderer,
    context: Rc<glow::Context>,
    target: Option<OwnTarget>,
}

/// The core's own framebuffer: the 3D is drawn there with OpenGL's bottom-left origin, then
/// copied upside down into the frontend's, which is declared with a top-left origin as the 2D
/// frames are. So the 2D and the 3D reach the frontend the same way up: a shader preset needs
/// no flip of its own for this core under `glcore` (GitHub issue #2).
struct OwnTarget {
    framebuffer: glow::Framebuffer,
    colour: glow::Renderbuffer,
    depth: glow::Renderbuffer,
    size: (u32, u32),
}

impl OwnTarget {
    /// A framebuffer of `width` x `height`, colour, depth and stencil; none when OpenGL refuses.
    fn new(gl: &glow::Context, (width, height): (u32, u32)) -> Option<Self> {
        use glow::HasContext as _;
        let (w, h) = (
            i32::try_from(width).unwrap_or(1),
            i32::try_from(height).unwrap_or(1),
        );
        // SAFETY: OpenGL calls on the frontend's context, current during `retro_run`; every
        // object made here is deleted by `delete` before the context goes.
        unsafe {
            let framebuffer = gl.create_framebuffer().ok()?;
            let colour = gl.create_renderbuffer().ok()?;
            let depth = gl.create_renderbuffer().ok()?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(colour));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, w, h);
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH24_STENCIL8, w, h);
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(colour),
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_STENCIL_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth),
            );
            let complete =
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            let target = Self {
                framebuffer,
                colour,
                depth,
                size: (width, height),
            };
            if complete {
                Some(target)
            } else {
                target.delete(gl);
                None
            }
        }
    }

    fn delete(self, gl: &glow::Context) {
        use glow::HasContext as _;
        // SAFETY: objects this core made on the context still current.
        unsafe {
            gl.delete_framebuffer(self.framebuffer);
            gl.delete_renderbuffer(self.colour);
            gl.delete_renderbuffer(self.depth);
        }
    }
}

thread_local! {
    /// The game loaded. Boxed so that the memory handed out by [`retro_get_memory_data`] stays
    /// where it is until the game is unloaded.
    static GAME: RefCell<Option<Box<Core>>> = const { RefCell::new(None) };
    /// The 3D renderer, from the frontend's `context_reset` to its `context_destroy`.
    static GL: RefCell<Option<Gl>> = const { RefCell::new(None) };
}

/// The device the frontend plugged into each port, kept for a game loaded after.
static PORT_DEVICES: Mutex<[c_uint; 2]> = Mutex::new([DEVICE_JOYPAD; 2]);

/// The device types a port offers. The frontend keeps a pointer to them: static.
struct PortTypes([ControllerDescription; 2]);
// SAFETY: read-only data, pointers to static C strings.
unsafe impl Sync for PortTypes {}
static PORT_TYPES: PortTypes = PortTypes([
    ControllerDescription {
        desc: c"NES pad".as_ptr(),
        id: DEVICE_JOYPAD,
    },
    ControllerDescription {
        desc: c"Zapper".as_ptr(),
        id: DEVICE_LIGHTGUN,
    },
]);

/// Declares the two ports' devices: a pad, or the Zapper.
fn declare_controllers() {
    let port = || ControllerInfo {
        types: PORT_TYPES.0.as_ptr(),
        num_types: 2,
    };
    let mut ports = [
        port(),
        port(),
        ControllerInfo {
            types: std::ptr::null(),
            num_types: 0,
        },
    ];
    // The data is the array itself, ended by an empty entry; the frontend copies it (but not
    // the types it points to).
    if let Some(first) = ports.first_mut() {
        environment(ENVIRONMENT_SET_CONTROLLER_INFO, first);
    }
}

/// Plugs into `core` the devices the frontend chose.
fn plug_devices(core: &mut Core) {
    let devices = *PORT_DEVICES.lock().unwrap_or_else(PoisonError::into_inner);
    for (port, device) in devices.iter().enumerate() {
        core.set_zapper(port, *device == DEVICE_LIGHTGUN);
    }
}

/// Reads the light guns of the Zapper ports and aims them; their trigger goes in the input
/// word, as the console reads it.
fn aim_zappers(frontend: &Frontend, core: &mut Core, word: &mut u16) {
    for (port, plugged) in core.zappers().into_iter().enumerate() {
        if !plugged {
            continue;
        }
        let index = c_uint::try_from(port).unwrap_or(0);
        let gun = |id| pressed(frontend, index, DEVICE_LIGHTGUN, 0, id);
        let reload = gun(LIGHTGUN_RELOAD) != 0;
        let point = if reload || gun(LIGHTGUN_IS_OFFSCREEN) != 0 {
            None
        } else {
            let along = |value: i16| (f32::from(value) + 32767.0) / 65534.0;
            Some((along(gun(LIGHTGUN_SCREEN_X)), along(gun(LIGHTGUN_SCREEN_Y))))
        };
        core.aim_zapper(port, point);
        let byte = if reload || gun(LIGHTGUN_TRIGGER) != 0 {
            ZAPPER_PRESSED
        } else {
            ZAPPER_RELEASED
        };
        let shift = port * 8;
        *word = (*word & !(0xFF << shift)) | (u16::from(byte) << shift);
    }
}

fn frontend() -> Frontend {
    *FRONTEND.lock().unwrap_or_else(PoisonError::into_inner)
}

fn set_frontend(change: impl FnOnce(&mut Frontend)) {
    change(&mut FRONTEND.lock().unwrap_or_else(PoisonError::into_inner));
}

/// Runs `operation` on the game loaded, when there is one.
fn with_game<R>(operation: impl FnOnce(&mut Core) -> R) -> Option<R> {
    GAME.with(|game| game.borrow_mut().as_deref_mut().map(operation))
}

/// Asks the frontend `cmd`, with `data` as `libretro.h` says for it.
fn environment<T>(cmd: c_uint, data: &mut T) -> bool {
    let Some(callback) = frontend().environment else {
        return false;
    };
    // SAFETY: `data` is the structure `libretro.h` gives for `cmd`, alive for the call.
    unsafe { callback(cmd, std::ptr::from_mut(data).cast()) }
}

/// `struct retro_message`.
#[repr(C)]
struct Message {
    msg: *const c_char,
    frames: c_uint,
}

/// Shows `text` on the frontend's screen for two seconds.
fn show(text: &'static CStr) {
    let mut message = Message {
        msg: text.as_ptr(),
        frames: 120,
    };
    environment(ENVIRONMENT_SET_MESSAGE, &mut message);
}

/// Shows a script's notification `text` for `seconds` on the frontend's screen.
fn show_text(text: &str, seconds: f32) {
    let Ok(text) = CString::new(text.replace('\0', "")) else {
        return;
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // A few seconds.
    let frames = (seconds.clamp(0.5, 30.0) * 60.0) as c_uint;
    let mut message = Message {
        msg: text.as_ptr(),
        frames,
    };
    // The frontend copies the text.
    environment(ENVIRONMENT_SET_MESSAGE, &mut message);
}

fn log(level: c_uint, message: &str) {
    let Some(callback) = frontend().log else {
        return;
    };
    let Ok(text) = CString::new(format!("[Zaxnes] {message}\n")) else {
        return;
    };
    // SAFETY: a printf-style call with "%s" and one NUL-terminated string.
    unsafe { callback(level, c"%s".as_ptr(), text.as_ptr()) };
}

/// The scripts' log.
fn log_line(line: &str) {
    log(LOG_INFO, line);
}

/// The value of core option `key`, as the frontend has it.
fn variable(key: &str) -> Option<String> {
    let key = CString::new(key).ok()?;
    let mut variable = Variable {
        key: key.as_ptr(),
        value: std::ptr::null(),
    };
    if !environment(ENVIRONMENT_GET_VARIABLE, &mut variable) || variable.value.is_null() {
        return None;
    }
    // SAFETY: the frontend answered with a NUL-terminated string, valid until the next call.
    let value = unsafe { CStr::from_ptr(variable.value) };
    Some(value.to_string_lossy().into_owned())
}

/// The frontend's system folder.
fn system_directory() -> Option<PathBuf> {
    let mut folder: *const c_char = std::ptr::null();
    if !environment(ENVIRONMENT_GET_SYSTEM_DIRECTORY, &mut folder) || folder.is_null() {
        return None;
    }
    // SAFETY: a NUL-terminated path the frontend keeps.
    let folder = unsafe { CStr::from_ptr(folder) };
    Some(PathBuf::from(folder.to_string_lossy().into_owned()))
}

/// Declares the core options, the game's hack options with them: in their submenus when the
/// frontend knows the options of version 2, else as the flat list of the first API.
fn declare_options(hacks: &[zax_nes::builtin_patches::HackOption]) {
    let list = options::declarations(hacks);
    let mut version: c_uint = 0;
    if environment(ENVIRONMENT_GET_CORE_OPTIONS_VERSION, &mut version)
        && version >= 2
        && declare_options_v2(&list)
    {
        return;
    }
    declare_variables(&list);
}

/// A C string of `text`, empty when it holds a NUL.
fn c_text(text: &str) -> CString {
    CString::new(text).unwrap_or_default()
}

/// The strings of one option: key, name, name in its submenu, explanation, submenu, default;
/// then each value with its label.
type OptionTexts = ([CString; 6], Vec<(CString, Option<CString>)>);

/// `SET_CORE_OPTIONS_V2`: the submenus and the options with their explanations. The frontend
/// copies everything during the call, so the strings only live as long as it.
fn declare_options_v2(list: &[options::Declaration]) -> bool {
    let null = std::ptr::null::<c_char>();
    let category_texts: Vec<[CString; 3]> = options::CATEGORIES
        .iter()
        .map(|category| {
            [
                c_text(category.key),
                c_text(category.label),
                c_text(category.info),
            ]
        })
        .collect();
    let mut categories: Vec<OptionCategory> = category_texts
        .iter()
        .map(|[key, desc, info]| OptionCategory {
            key: key.as_ptr(),
            desc: desc.as_ptr(),
            info: info.as_ptr(),
        })
        .collect();
    categories.push(OptionCategory {
        key: null,
        desc: null,
        info: null,
    });
    // Every string of the options, kept until the call is over.
    let texts: Vec<OptionTexts> = list
        .iter()
        .map(|option| {
            (
                [
                    c_text(&option.key),
                    c_text(&option.label),
                    c_text(&option.label_in_category),
                    c_text(&option.info),
                    c_text(option.category),
                    c_text(option.default_value()),
                ],
                option
                    .values
                    .iter()
                    .take(OPTION_VALUES_MAX - 1)
                    .map(|(value, label)| (c_text(value), label.as_deref().map(c_text)))
                    .collect(),
            )
        })
        .collect();
    let empty = OptionValue {
        value: null,
        label: null,
    };
    let mut definitions: Vec<OptionDefinition> = texts
        .iter()
        .map(
            |([key, desc, in_category, info, category, default], values)| {
                let mut slots = [empty; OPTION_VALUES_MAX];
                for (slot, (value, label)) in slots.iter_mut().zip(values) {
                    *slot = OptionValue {
                        value: value.as_ptr(),
                        label: label.as_ref().map_or(null, |label| label.as_ptr()),
                    };
                }
                OptionDefinition {
                    key: key.as_ptr(),
                    desc: desc.as_ptr(),
                    desc_categorized: in_category.as_ptr(),
                    info: info.as_ptr(),
                    info_categorized: null,
                    category_key: if category.is_empty() {
                        null
                    } else {
                        category.as_ptr()
                    },
                    values: slots,
                    default_value: default.as_ptr(),
                }
            },
        )
        .collect();
    definitions.push(OptionDefinition {
        key: null,
        desc: null,
        desc_categorized: null,
        info: null,
        info_categorized: null,
        category_key: null,
        values: [empty; OPTION_VALUES_MAX],
        default_value: null,
    });
    let mut options = OptionsV2 {
        categories: categories.as_mut_ptr(),
        definitions: definitions.as_mut_ptr(),
    };
    environment(ENVIRONMENT_SET_CORE_OPTIONS_V2, &mut options)
}

/// `SET_VARIABLES`: the flat list of the first API, `"Name; default|other|..."`.
fn declare_variables(list: &[options::Declaration]) {
    let declared: Vec<(CString, CString)> = list
        .iter()
        .map(|option| (c_text(&option.key), c_text(&option.legacy())))
        .collect();
    let mut variables: Vec<Variable> = declared
        .iter()
        .map(|(key, text)| Variable {
            key: key.as_ptr(),
            value: text.as_ptr(),
        })
        .collect();
    variables.push(Variable {
        key: std::ptr::null(),
        value: std::ptr::null(),
    });
    // The data is the array itself, ended by a null entry (`const struct retro_variable *`).
    if let Some(first) = variables.first_mut() {
        environment(ENVIRONMENT_SET_VARIABLES, first);
    }
}

fn read_options(hacks: &[zax_nes::builtin_patches::HackOption]) -> Options {
    options::read(&variable, hacks)
}

fn hack_list(core: &Core) -> Vec<zax_nes::builtin_patches::HackOption> {
    core.hack_options()
        .into_iter()
        .map(|(option, _)| option)
        .collect()
}

fn av_info(core: Option<&Core>) -> SystemAvInfo {
    let (width, height, max, aspect, fps) = core.map_or(
        (256, 240, (MAX_WIDTH, MAX_HEIGHT), 4.0 / 3.0, 60.098_8),
        |core| {
            if core.is_three_d() {
                let (width, height) = core.three_d_size();
                #[allow(clippy::cast_precision_loss)] // Screen sizes.
                let aspect = width as f32 / height as f32;
                (width, height, MAX_3D, aspect, core.frames_per_second())
            } else {
                let (width, height) = core.picture_size();
                (
                    width,
                    height,
                    (MAX_WIDTH, MAX_HEIGHT),
                    core.aspect_ratio(),
                    core.frames_per_second(),
                )
            }
        },
    );
    SystemAvInfo {
        geometry: GameGeometry {
            base_width: width,
            base_height: height,
            max_width: max.0,
            max_height: max.1,
            aspect_ratio: aspect,
        },
        timing: SystemTiming {
            fps,
            sample_rate: f64::from(SAMPLE_RATE),
        },
    }
}

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    API_VERSION
}

#[no_mangle]
pub extern "C" fn retro_set_environment(callback: Option<EnvironmentFn>) {
    set_frontend(|frontend| frontend.environment = callback);
    let mut logging = LogCallback { log: None };
    let log = environment(ENVIRONMENT_GET_LOG_INTERFACE, &mut logging)
        .then_some(logging.log)
        .flatten();
    set_frontend(|frontend| frontend.log = log);
    declare_options(&[]);
    declare_controllers();
}

#[no_mangle]
pub extern "C" fn retro_set_video_refresh(callback: Option<VideoRefreshFn>) {
    set_frontend(|frontend| frontend.video_refresh = callback);
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample(_callback: Option<AudioSampleFn>) {}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(callback: Option<AudioSampleBatchFn>) {
    set_frontend(|frontend| frontend.audio_sample_batch = callback);
}

#[no_mangle]
pub extern "C" fn retro_set_input_poll(callback: Option<InputPollFn>) {
    set_frontend(|frontend| frontend.input_poll = callback);
}

#[no_mangle]
pub extern "C" fn retro_set_input_state(callback: Option<InputStateFn>) {
    set_frontend(|frontend| frontend.input_state = callback);
}

#[no_mangle]
pub extern "C" fn retro_init() {}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    GAME.with(|game| *game.borrow_mut() = None);
}

/// # Safety
///
/// `info` points to a `retro_system_info` the frontend owns.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut SystemInfo) {
    // SAFETY: the frontend passes a writable structure; the strings are static.
    if let Some(info) = unsafe { info.as_mut() } {
        *info = SystemInfo {
            library_name: c"Zaxnes".as_ptr(),
            library_version: concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast(),
            valid_extensions: c"nes".as_ptr(),
            need_fullpath: false,
            block_extract: false,
        };
    }
}

/// # Safety
///
/// `info` points to a `retro_system_av_info` the frontend owns.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut SystemAvInfo) {
    let av = GAME.with(|game| av_info(game.borrow().as_deref()));
    // SAFETY: the frontend passes a writable structure.
    if let Some(info) = unsafe { info.as_mut() } {
        *info = av;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(port: c_uint, device: c_uint) {
    let Ok(port) = usize::try_from(port) else {
        return;
    };
    if let Some(slot) = PORT_DEVICES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_mut(port)
    {
        *slot = device;
    }
    with_game(plug_devices);
}

#[no_mangle]
pub extern "C" fn retro_reset() {
    with_game(Core::reset);
}

/// Applies the options the frontend changed, and tells it what that changed.
fn follow_options(core: &mut Core) {
    let mut updated = false;
    if !environment(ENVIRONMENT_GET_VARIABLE_UPDATE, &mut updated) || !updated {
        return;
    }
    let changed = core.set_options(read_options(&hack_list(core)));
    if changed.hack_options {
        declare_options(&hack_list(core));
        let again = read_options(&hack_list(core));
        core.set_options(again);
    }
    if changed.timing {
        let mut av = av_info(Some(core));
        environment(ENVIRONMENT_SET_SYSTEM_AV_INFO, &mut av);
        core.take_new_size();
    }
}

/// Whether button `id` of pad `port` is down.
fn pressed(frontend: &Frontend, port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16 {
    let Some(state) = frontend.input_state else {
        return 0;
    };
    // SAFETY: the frontend's input callback, called as `libretro.h` says.
    unsafe { state(port, device, index, id) }
}

fn input_word(frontend: &Frontend) -> u16 {
    let mut word = 0_u16;
    for port in 0..2_u16 {
        for (bit, (id, _)) in PAD.iter().enumerate() {
            if pressed(frontend, c_uint::from(port), DEVICE_JOYPAD, 0, *id) != 0 {
                word |= 1 << (usize::from(port) * 8 + bit);
            }
        }
    }
    word
}

/// The camera controls of pad 1, applied to the 3D for one frame of `seconds`.
fn steer(frontend: &Frontend, core: &mut Core, seconds: f32) {
    let axis = |id| f32::from(pressed(frontend, 0, DEVICE_ANALOG, ANALOG_RIGHT, id)) / 32768.0;
    let dead = |value: f32| if value.abs() < 0.2 { 0.0 } else { value };
    let turn = (dead(axis(0)), dead(axis(1)));
    let button = |id| pressed(frontend, 0, DEVICE_JOYPAD, 0, id) != 0;
    let zoom = f32::from(i8::from(button(JOYPAD_L)) - i8::from(button(JOYPAD_R)));
    if let Some(three_d) = core.three_d_mut() {
        match three_d.buttons(seconds, button(JOYPAD_Y), button(JOYPAD_L2)) {
            Some(CameraNews::Kept) => show(c"Camera kept"),
            Some(CameraNews::Restored) => show(c"Camera put back"),
            None => {}
        }
        if button(JOYPAD_X) {
            three_d.reset_view();
        }
        three_d.advance(seconds, turn, zoom);
        for (text, shown) in three_d.take_notices() {
            show_text(&text, shown);
        }
    }
}

/// Draws the 3D into the frontend's framebuffer and hands it over; false when there is no
/// renderer yet (the frontend has not reset the context).
fn draw_three_d(frontend: &Frontend, core: &mut Core) -> bool {
    let (width, height) = core.three_d_size();
    let Some(framebuffer) = frontend.framebuffer else {
        return false;
    };
    // SAFETY: the frontend's callback, called while its context is current.
    let id = unsafe { framebuffer() };
    let frontend_framebuffer =
        NonZeroU32::new(u32::try_from(id).unwrap_or(0)).map(glow::NativeFramebuffer);
    let crosshairs = core.three_d_crosshairs();
    let Some(three_d) = core.three_d_mut() else {
        return false;
    };
    let drawn = GL.with(|gl| {
        let mut gl = gl.borrow_mut();
        let gl = gl.as_mut()?;
        if gl
            .target
            .as_ref()
            .is_none_or(|target| target.size != (width, height))
        {
            if let Some(old) = gl.target.take() {
                old.delete(&gl.context);
            }
            gl.target = OwnTarget::new(&gl.context, (width, height));
        }
        let own = gl.target.as_ref()?.framebuffer;
        let target = RenderTarget {
            framebuffer: Some(own.0),
            width: i32::try_from(width).unwrap_or(1),
            height: i32::try_from(height).unwrap_or(1),
        };
        let (scene, frame) = three_d.scene_and_frame();
        let result = gl.renderer.render(&target, scene, &frame);
        let (w, h) = (target.width, target.height);
        {
            use glow::HasContext as _;
            // SAFETY: OpenGL calls on the frontend's current context: the core's framebuffer
            // copied upside down into the frontend's, which stays bound for it.
            unsafe {
                gl.context
                    .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(own));
                gl.context
                    .bind_framebuffer(glow::DRAW_FRAMEBUFFER, frontend_framebuffer);
                gl.context.blit_framebuffer(
                    0,
                    0,
                    w,
                    h,
                    0,
                    h,
                    w,
                    0,
                    glow::COLOR_BUFFER_BIT,
                    glow::NEAREST,
                );
                gl.context
                    .bind_framebuffer(glow::FRAMEBUFFER, frontend_framebuffer);
            }
            draw_crosshairs(&gl.context, &crosshairs, (w, h));
        }
        Some(result)
    });
    match drawn {
        Some(Ok(())) => {}
        Some(Err(error)) => log(LOG_ERROR, &format!("3D: {error}")),
        None => return false,
    }
    if let Some(refresh) = frontend.video_refresh {
        // RETRO_HW_FRAME_BUFFER_VALID: the frame is in the frontend's framebuffer.
        // SAFETY: the frontend's video callback, with the marker `libretro.h` defines.
        unsafe { refresh(usize::MAX as *const c_void, width, height, 0) };
    }
    true
}

#[no_mangle]
pub extern "C" fn retro_run() {
    let frontend = frontend();
    with_game(|core| {
        follow_options(core);
        if let Some(poll) = frontend.input_poll {
            // SAFETY: the frontend's poll callback, called once a frame.
            unsafe { poll() };
        }
        let mut word = input_word(&frontend);
        aim_zappers(&frontend, core, &mut word);
        core.run(word);
        #[allow(clippy::cast_possible_truncation)] // A frame's length.
        let seconds = (1.0 / core.frames_per_second()) as f32;
        if core.is_three_d() {
            steer(&frontend, core, seconds);
            if !draw_three_d(&frontend, core) {
                if let Some(refresh) = frontend.video_refresh {
                    // SAFETY: a null frame asks the frontend to show the last one again.
                    unsafe { refresh(std::ptr::null(), 0, 0, 0) };
                }
            }
        } else {
            if core.take_new_size() {
                let mut geometry = av_info(Some(core)).geometry;
                environment(ENVIRONMENT_SET_GEOMETRY, &mut geometry);
            }
            if let Some(refresh) = frontend.video_refresh {
                let (pixels, width, height) = core.picture();
                // SAFETY: `pixels` holds `width * height` pixels of 4 bytes, alive for the call.
                unsafe {
                    refresh(
                        pixels.as_ptr().cast(),
                        width,
                        height,
                        usize::try_from(width).unwrap_or(0) * 4,
                    );
                };
            }
        }
        if let Some(batch) = frontend.audio_sample_batch {
            let mut sound = core.sound();
            while sound.len() >= 2 {
                // SAFETY: `sound` holds `len / 2` stereo frames, alive for the call.
                let taken = unsafe { batch(sound.as_ptr(), sound.len() / 2) };
                if taken == 0 {
                    break;
                }
                sound = sound.get(taken * 2..).unwrap_or(&[]);
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    with_game(|core| core.state_size()).unwrap_or(0)
}

/// # Safety
///
/// `data` points to `size` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn retro_serialize(data: *mut c_void, size: usize) -> bool {
    if data.is_null() {
        return false;
    }
    // SAFETY: the frontend hands `size` writable bytes.
    let out = unsafe { std::slice::from_raw_parts_mut(data.cast::<u8>(), size) };
    with_game(|core| core.save_state(out)).unwrap_or(false)
}

/// # Safety
///
/// `data` points to `size` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn retro_unserialize(data: *const c_void, size: usize) -> bool {
    if data.is_null() {
        return false;
    }
    // SAFETY: the frontend hands `size` readable bytes.
    let state = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    with_game(|core| core.load_state(state)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {}

#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: c_uint, _enabled: bool, _code: *const c_char) {}

/// Draws a crosshair at each point (0 to 1 across and down) of the frontend's framebuffer, bound
/// and declared top-left: a white cross with a black edge, its size following the height.
fn draw_crosshairs(gl: &glow::Context, points: &[(f32, f32)], (width, height): (i32, i32)) {
    use glow::HasContext as _;
    if points.is_empty() {
        return;
    }
    let unit = (height / 240).max(1);
    // SAFETY: OpenGL calls on the frontend's current context, its framebuffer bound; the
    // scissor test is put back as it was.
    unsafe {
        let scissor = gl.is_enabled(glow::SCISSOR_TEST);
        gl.enable(glow::SCISSOR_TEST);
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        for &(x, y) in points {
            let (cx, cy) = ((x * width as f32) as i32, (y * height as f32) as i32);
            // Arms from 2 to 6 units off the centre, 1 unit thick; the edge one unit around.
            let arms = [
                (cx + 2 * unit, cy, 4 * unit, unit),
                (cx - 6 * unit, cy, 4 * unit, unit),
                (cx, cy + 2 * unit, unit, 4 * unit),
                (cx, cy - 6 * unit, unit, 4 * unit),
            ];
            for (colour, grow) in [(0.0, unit), (1.0, 0)] {
                gl.clear_color(colour, colour, colour, 1.0);
                for (ax, ay, aw, ah) in arms {
                    gl.scissor(ax - grow, ay - grow, aw + 2 * grow, ah + 2 * grow);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }
            }
        }
        if !scissor {
            gl.disable(glow::SCISSOR_TEST);
        }
    }
}

/// The frontend made its OpenGL context (again): the renderer is made on it.
extern "C" fn context_reset() {
    let Some(proc_address) = frontend().proc_address else {
        log(LOG_ERROR, "3D: the frontend gave no get_proc_address");
        return;
    };
    // SAFETY: the frontend's loader, called with NUL-terminated names while its context is
    // current, as glow asks.
    let gl = Rc::new(unsafe {
        glow::Context::from_loader_function_cstr(|name| proc_address(name.as_ptr()))
    });
    match SceneRenderer::new(Rc::clone(&gl), None) {
        Ok(renderer) => {
            GL.with(|slot| {
                *slot.borrow_mut() = Some(Gl {
                    renderer,
                    context: gl,
                    target: None,
                });
            });
            log(LOG_INFO, "3D renderer ready");
        }
        Err(error) => log(LOG_ERROR, &format!("3D renderer refused: {error}")),
    }
}

/// The frontend is about to destroy its context: the renderer goes first.
extern "C" fn context_destroy() {
    GL.with(|slot| {
        if let Some(mut gl) = slot.borrow_mut().take() {
            if let Some(target) = gl.target.take() {
                target.delete(&gl.context);
            }
        }
    });
}

/// Asks the frontend for an OpenGL 3.3 core context, depth and stencil included.
fn request_opengl() -> bool {
    let mut callback = HwRenderCallback {
        context_type: HW_CONTEXT_OPENGL_CORE,
        context_reset: Some(context_reset),
        get_current_framebuffer: None,
        get_proc_address: None,
        depth: true,
        stencil: true,
        // The core flips its 3D itself (`OwnTarget`): top-left, as its 2D frames.
        bottom_left_origin: false,
        version_major: 3,
        version_minor: 3,
        cache_context: false,
        context_destroy: Some(context_destroy),
        debug_context: false,
    };
    if !environment(ENVIRONMENT_SET_HW_RENDER, &mut callback) {
        return false;
    }
    let (framebuffer, proc_address) = (callback.get_current_framebuffer, callback.get_proc_address);
    set_frontend(|frontend| {
        frontend.framebuffer = framebuffer;
        frontend.proc_address = proc_address;
    });
    true
}

/// Starts the 3D of `core` when the options ask for it and a profile suits the ROM.
fn start_three_d(core: &mut Core, name: &str, image: &[u8]) {
    // The import folder first (owner request, 2026-11-20), whatever the options: what was put
    // there is imported once, before a profile is looked for.
    if let Some(system) = system_directory() {
        for (ok, text) in profiles::import_waiting(&system) {
            log(
                if ok { LOG_INFO } else { LOG_ERROR },
                &format!("import: {text}"),
            );
        }
    }
    if !core.options().three_d {
        return;
    }
    let Some((title, document)) = profiles::find_titled(name, image, system_directory().as_deref())
    else {
        log(LOG_INFO, &format!("{name}: no 3D profile, 2D"));
        return;
    };
    let tiles = document.hash_state().map(|state| state.tiles.clone());
    if let Err(error) = core.start_three_d(&document.profile, tiles, log_line) {
        log(
            LOG_ERROR,
            &format!("{title}: the profile is refused ({error}), 2D"),
        );
        return;
    }
    if request_opengl() {
        log(LOG_INFO, &format!("3D with the profile {title}"));
    } else {
        core.stop_three_d();
        log(LOG_ERROR, "the frontend has no OpenGL 3.3 core context: 2D");
    }
}

/// # Safety
///
/// `info` points to a `retro_game_info` whose data is `size` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn retro_load_game(info: *const GameInfo) -> bool {
    // SAFETY: the frontend passes a readable structure.
    let Some(info) = (unsafe { info.as_ref() }) else {
        return false;
    };
    if info.data.is_null() {
        log(
            LOG_ERROR,
            "the ROM came without its data (need_fullpath is off)",
        );
        return false;
    }
    // SAFETY: `data` is `size` readable bytes for the call.
    let image = unsafe { std::slice::from_raw_parts(info.data.cast::<u8>(), info.size) };
    let name = if info.path.is_null() {
        "game.nes".to_owned()
    } else {
        // SAFETY: a NUL-terminated path.
        let path = unsafe { CStr::from_ptr(info.path) }.to_string_lossy();
        std::path::Path::new(path.as_ref()).file_name().map_or_else(
            || "game.nes".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    };
    let mut format = PIXEL_FORMAT_XRGB8888;
    if !environment(ENVIRONMENT_SET_PIXEL_FORMAT, &mut format) {
        log(LOG_ERROR, "the frontend refuses XRGB8888 pixels");
        return false;
    }
    describe_input();
    let mut core = match Core::load(image, &name, read_options(&[])) {
        Ok(core) => core,
        Err(message) => {
            log(LOG_ERROR, &format!("{name}: {message}"));
            return false;
        }
    };
    declare_controllers();
    plug_devices(&mut core);
    let hacks = hack_list(&core);
    declare_options(&hacks);
    core.set_options(read_options(&hacks));
    log(
        LOG_INFO,
        &format!(
            "{name} loaded{}",
            core.patch_name()
                .map(|patch| format!(", hack: {patch}"))
                .unwrap_or_default()
        ),
    );
    start_three_d(&mut core, &name, image);
    GAME.with(|game| *game.borrow_mut() = Some(Box::new(core)));
    true
}

/// Names the pad's buttons and the camera controls for the frontend's input menu.
fn describe_input() {
    let mut descriptors: Vec<InputDescriptor> = (0..2)
        .flat_map(|port| {
            PAD.iter().map(move |(id, name)| InputDescriptor {
                port,
                device: DEVICE_JOYPAD,
                index: 0,
                id: *id,
                description: name.as_ptr(),
            })
        })
        .collect();
    descriptors.extend(
        CAMERA
            .iter()
            .map(|(device, index, id, name)| InputDescriptor {
                port: 0,
                device: *device,
                index: *index,
                id: *id,
                description: name.as_ptr(),
            }),
    );
    descriptors.push(InputDescriptor {
        port: 0,
        device: 0,
        index: 0,
        id: 0,
        description: std::ptr::null(),
    });
    // The array itself, ended by a null description.
    if let Some(first) = descriptors.first_mut() {
        environment(ENVIRONMENT_SET_INPUT_DESCRIPTORS, first);
    }
}

#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _game_type: c_uint,
    _info: *const GameInfo,
    _num_info: usize,
) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    GAME.with(|game| *game.borrow_mut() = None);
}

#[no_mangle]
pub extern "C" fn retro_get_region() -> c_uint {
    if with_game(|core| core.is_pal()).unwrap_or(false) {
        REGION_PAL
    } else {
        REGION_NTSC
    }
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(id: c_uint) -> *mut c_void {
    with_game(|core| {
        let memory = match id {
            MEMORY_SAVE_RAM => core.save_ram(),
            MEMORY_SYSTEM_RAM => core.system_ram(),
            _ => return std::ptr::null_mut(),
        };
        if memory.is_empty() {
            return std::ptr::null_mut();
        }
        // The memory lives in the boxed game, which stays put until it is unloaded.
        memory.as_mut_ptr().cast()
    })
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(id: c_uint) -> usize {
    with_game(|core| match id {
        MEMORY_SAVE_RAM => core.save_ram().len(),
        MEMORY_SYSTEM_RAM => core.system_ram().len(),
        _ => 0,
    })
    .unwrap_or(0)
}
