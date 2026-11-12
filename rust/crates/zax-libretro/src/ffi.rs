//! The libretro API: the `retro_*` functions `RetroArch` calls, and the frontend's callbacks.
//!
//! This is the one module of the crate with `unsafe` (owner decision, 2026-11-12, as for the C
//! layers of `zax-achievements` and `zax-android`): a C interface hands raw pointers both ways.
//! It only moves data between them and [`Core`]; every decision is made in safe code.
//!
//! The structures and numbers are those of `libretro.h` (API version 1).

use std::ffi::{c_char, c_uint, c_void, CStr, CString};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::core::{Core, Options, MAX_HEIGHT, MAX_WIDTH, SAMPLE_RATE};
use crate::options;

const API_VERSION: c_uint = 1;

const ENVIRONMENT_SET_PIXEL_FORMAT: c_uint = 10;
const ENVIRONMENT_SET_INPUT_DESCRIPTORS: c_uint = 11;
const ENVIRONMENT_GET_VARIABLE: c_uint = 15;
const ENVIRONMENT_SET_VARIABLES: c_uint = 16;
const ENVIRONMENT_GET_VARIABLE_UPDATE: c_uint = 17;
const ENVIRONMENT_GET_LOG_INTERFACE: c_uint = 27;
const ENVIRONMENT_SET_SYSTEM_AV_INFO: c_uint = 32;
const ENVIRONMENT_SET_GEOMETRY: c_uint = 37;

const PIXEL_FORMAT_XRGB8888: c_uint = 1;
const DEVICE_JOYPAD: c_uint = 1;
const MEMORY_SAVE_RAM: c_uint = 0;
const MEMORY_SYSTEM_RAM: c_uint = 2;
const REGION_NTSC: c_uint = 0;
const REGION_PAL: c_uint = 1;
const LOG_INFO: c_uint = 1;
const LOG_ERROR: c_uint = 3;

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

type EnvironmentFn = unsafe extern "C" fn(cmd: c_uint, data: *mut c_void) -> bool;
type VideoRefreshFn =
    unsafe extern "C" fn(data: *const c_void, width: c_uint, height: c_uint, pitch: usize);
type AudioSampleFn = unsafe extern "C" fn(left: i16, right: i16);
type AudioSampleBatchFn = unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;
type InputPollFn = unsafe extern "C" fn();
type InputStateFn =
    unsafe extern "C" fn(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16;
type LogFn = unsafe extern "C" fn(level: c_uint, format: *const c_char, ...);

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

/// The frontend's callbacks.
#[derive(Clone, Copy)]
struct Frontend {
    environment: Option<EnvironmentFn>,
    video_refresh: Option<VideoRefreshFn>,
    audio_sample_batch: Option<AudioSampleBatchFn>,
    input_poll: Option<InputPollFn>,
    input_state: Option<InputStateFn>,
    log: Option<LogFn>,
}

static FRONTEND: Mutex<Frontend> = Mutex::new(Frontend {
    environment: None,
    video_refresh: None,
    audio_sample_batch: None,
    input_poll: None,
    input_state: None,
    log: None,
});

/// The game loaded. Boxed so that the memory handed out by [`retro_get_memory_data`] stays where
/// it is until the game is unloaded.
static GAME: Mutex<Option<Box<Core>>> = Mutex::new(None);

fn frontend() -> Frontend {
    *FRONTEND.lock().unwrap_or_else(PoisonError::into_inner)
}

fn game() -> MutexGuard<'static, Option<Box<Core>>> {
    GAME.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Asks the frontend `cmd`, with `data` as `libretro.h` says for it.
fn environment<T>(cmd: c_uint, data: &mut T) -> bool {
    let Some(callback) = frontend().environment else {
        return false;
    };
    // SAFETY: `data` is the structure `libretro.h` gives for `cmd`, alive for the call.
    unsafe { callback(cmd, std::ptr::from_mut(data).cast()) }
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

/// Declares the core options, the game's hack options with them.
fn declare_options(hacks: &[zax_nes::builtin_patches::HackOption]) {
    let declared: Vec<(CString, CString)> = options::declarations(hacks)
        .into_iter()
        .filter_map(|(key, text)| Some((CString::new(key).ok()?, CString::new(text).ok()?)))
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
    let mut first = variables.as_mut_ptr();
    environment(ENVIRONMENT_SET_VARIABLES, &mut first);
}

fn read_options(hacks: &[zax_nes::builtin_patches::HackOption]) -> Options {
    options::read(&variable, hacks)
}

fn hack_list(core: &Core) -> Vec<zax_nes::builtin_patches::HackOption> {
    core.hack_options().into_iter().map(|(option, _)| option).collect()
}

fn av_info(core: Option<&Core>) -> SystemAvInfo {
    let (width, height, aspect, fps) = core.map_or((256, 240, 4.0 / 3.0, 60.098_8), |core| {
        let (width, height) = core.picture_size();
        (width, height, core.aspect_ratio(), core.frames_per_second())
    });
    SystemAvInfo {
        geometry: GameGeometry {
            base_width: width,
            base_height: height,
            max_width: MAX_WIDTH,
            max_height: MAX_HEIGHT,
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
    FRONTEND
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .environment = callback;
    let mut logging = LogCallback { log: None };
    let log = environment(ENVIRONMENT_GET_LOG_INTERFACE, &mut logging)
        .then_some(logging.log)
        .flatten();
    FRONTEND.lock().unwrap_or_else(PoisonError::into_inner).log = log;
    declare_options(&[]);
}

#[no_mangle]
pub extern "C" fn retro_set_video_refresh(callback: Option<VideoRefreshFn>) {
    FRONTEND
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .video_refresh = callback;
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample(_callback: Option<AudioSampleFn>) {}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(callback: Option<AudioSampleBatchFn>) {
    FRONTEND
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .audio_sample_batch = callback;
}

#[no_mangle]
pub extern "C" fn retro_set_input_poll(callback: Option<InputPollFn>) {
    FRONTEND
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .input_poll = callback;
}

#[no_mangle]
pub extern "C" fn retro_set_input_state(callback: Option<InputStateFn>) {
    FRONTEND
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .input_state = callback;
}

#[no_mangle]
pub extern "C" fn retro_init() {}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    *game() = None;
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
    let av = av_info(game().as_deref());
    // SAFETY: the frontend passes a writable structure.
    if let Some(info) = unsafe { info.as_mut() } {
        *info = av;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {}

#[no_mangle]
pub extern "C" fn retro_reset() {
    if let Some(core) = game().as_mut() {
        core.reset();
    }
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

fn input_word(frontend: &Frontend) -> u16 {
    let Some(state) = frontend.input_state else {
        return 0;
    };
    let mut word = 0_u16;
    for port in 0..2_u16 {
        for (bit, (id, _)) in PAD.iter().enumerate() {
            // SAFETY: the frontend's input callback, called as `libretro.h` says.
            let pressed = unsafe { state(c_uint::from(port), DEVICE_JOYPAD, 0, *id) } != 0;
            if pressed {
                word |= 1 << (usize::from(port) * 8 + bit);
            }
        }
    }
    word
}

#[no_mangle]
pub extern "C" fn retro_run() {
    let frontend = frontend();
    let mut guard = game();
    let Some(core) = guard.as_mut() else {
        return;
    };
    follow_options(core);
    if let Some(poll) = frontend.input_poll {
        // SAFETY: the frontend's poll callback, called once a frame.
        unsafe { poll() };
    }
    core.run(input_word(&frontend));
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
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    game().as_ref().map_or(0, |core| core.state_size())
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
    game().as_ref().is_some_and(|core| core.save_state(out))
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
    game().as_mut().is_some_and(|core| core.load_state(state))
}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {}

#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: c_uint, _enabled: bool, _code: *const c_char) {}

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
        log(LOG_ERROR, "the ROM came without its data (need_fullpath is off)");
        return false;
    }
    // SAFETY: `data` is `size` readable bytes for the call.
    let image = unsafe { std::slice::from_raw_parts(info.data.cast::<u8>(), info.size) };
    let name = if info.path.is_null() {
        "game.nes".to_owned()
    } else {
        // SAFETY: a NUL-terminated path.
        let path = unsafe { CStr::from_ptr(info.path) }.to_string_lossy();
        std::path::Path::new(path.as_ref())
            .file_name()
            .map_or_else(|| "game.nes".to_owned(), |name| name.to_string_lossy().into_owned())
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
    *game() = Some(Box::new(core));
    true
}

/// Names the pad's buttons for the frontend's input menu.
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
    descriptors.push(InputDescriptor {
        port: 0,
        device: 0,
        index: 0,
        id: 0,
        description: std::ptr::null(),
    });
    let mut first = descriptors.as_mut_ptr();
    environment(ENVIRONMENT_SET_INPUT_DESCRIPTORS, &mut first);
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
    *game() = None;
}

#[no_mangle]
pub extern "C" fn retro_get_region() -> c_uint {
    if game().as_ref().is_some_and(|core| core.is_pal()) {
        REGION_PAL
    } else {
        REGION_NTSC
    }
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(id: c_uint) -> *mut c_void {
    let mut guard = game();
    let Some(core) = guard.as_mut() else {
        return std::ptr::null_mut();
    };
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
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(id: c_uint) -> usize {
    let mut guard = game();
    let Some(core) = guard.as_mut() else {
        return 0;
    };
    match id {
        MEMORY_SAVE_RAM => core.save_ram().len(),
        MEMORY_SYSTEM_RAM => core.system_ram().len(),
        _ => 0,
    }
}
