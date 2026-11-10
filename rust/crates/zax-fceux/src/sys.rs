//! The raw declarations of the fork's C API (`zax/zaxfceux.h`), and nothing else.
//!
//! This is the **only** place in the whole workspace where `extern "C"` appears for FCEUX. Every
//! item here mirrors one declaration of that header; if the two ever drift, the link fails rather
//! than the behaviour changing, because every symbol is resolved at link time against the static
//! library the fork built.
//!
//! The constants are copied rather than generated: `bindgen` would be a new dependency, a C
//! toolchain requirement for every build of this crate, and a moving target between its versions,
//! for a header of thirty declarations that changes when we change it. `tests::codes_match_header`
//! reads the fork's header and checks every constant against it, so a drift is a test failure.

use std::os::raw::{c_char, c_int, c_void};

pub const ZAX_OK: i32 = 0;
pub const ZAX_ERR_ALREADY_LIVE: i32 = -1;
pub const ZAX_ERR_NO_INSTANCE: i32 = -2;
pub const ZAX_ERR_NO_ROM: i32 = -3;
pub const ZAX_ERR_BAD_ARGUMENT: i32 = -4;
pub const ZAX_ERR_LOAD_FAILED: i32 = -5;
pub const ZAX_ERR_BUFFER_TOO_SMALL: i32 = -6;
pub const ZAX_ERR_STATE_FAILED: i32 = -7;
pub const ZAX_ERR_MOVIE_FORMAT: i32 = -8;
pub const ZAX_ERR_MOVIE_ROM: i32 = -9;
pub const ZAX_ERR_MOVIE_UNSUPPORTED: i32 = -10;
pub const ZAX_ERR_NO_MOVIE: i32 = -11;

pub const ZAX_MOVIE_NONE: i32 = 0;
pub const ZAX_MOVIE_PLAYING: i32 = 1;
pub const ZAX_MOVIE_FINISHED: i32 = 2;

pub const ZAX_REGION_NTSC: i32 = 0;
pub const ZAX_REGION_PAL: i32 = 1;
pub const ZAX_REGION_DENDY: i32 = 2;

pub const ZAX_RAM_INIT_DEFAULT: i32 = 0;
pub const ZAX_RAM_INIT_FF: i32 = 1;
pub const ZAX_RAM_INIT_00: i32 = 2;
pub const ZAX_RAM_INIT_RANDOM: i32 = 3;

pub const ZAX_SCREEN_WIDTH: usize = 256;
pub const ZAX_SCREEN_HEIGHT: usize = 240;

/// The frame buffers' length: one byte a pixel, no padding.
pub const FRAME_LEN: usize = ZAX_SCREEN_WIDTH * ZAX_SCREEN_HEIGHT;

pub type ZaxLogFn = Option<unsafe extern "C" fn(user: *mut c_void, text: *const c_char)>;

pub const ZAX_TILE_MIRROR_H: u8 = 0x01;
pub const ZAX_TILE_MIRROR_V: u8 = 0x02;
pub const ZAX_TILE_HIDDEN: u8 = 0x04;
pub const ZAX_TILE_BACKGROUND: u8 = 0x08;

/// `zax_tile_record` of `zax/zaxfceux.h`, field for field. `repr(C)` is what makes that true.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ZaxTileRecord {
    pub plane0: u64,
    pub plane1: u64,
    pub x: i16,
    pub y: i16,
    pub index: i32,
    pub flags: u8,
    pub palette: u8,
}

pub type ZaxTilesFn = Option<
    unsafe extern "C" fn(
        user: *mut c_void,
        records: *const ZaxTileRecord,
        count: usize,
        palette: *const u8,
    ),
>;

/// `zax_movie_info` of `zax/zaxfceux.h`, field for field. `repr(C)` is what makes that true.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ZaxMovieInfo {
    pub version: i32,
    pub frames: i32,
    pub region: i32,
    pub ram_init_option: i32,
    pub ram_init_seed: i32,
    pub new_ppu: i32,
    pub fds: i32,
    pub fourscore: i32,
    pub microphone: i32,
    pub port0: i32,
    pub port1: i32,
    pub port2: i32,
    pub from_savestate: i32,
    pub from_saveram: i32,
    pub rerecord_count: i32,
    pub rom_md5: [u8; 16],
    pub rom_md5_known: i32,
    pub rom_md5_matches: i32,
}

extern "C" {
    pub fn zax_set_log_sink(sink: ZaxLogFn, user: *mut c_void);
    pub fn zax_flush_log();
    pub fn zax_version() -> *const c_char;

    pub fn zax_create() -> i32;
    pub fn zax_destroy();
    pub fn zax_is_live() -> i32;

    pub fn zax_load_rom(image: *const u8, image_len: usize, name: *const c_char) -> i32;
    pub fn zax_close_rom();
    pub fn zax_has_rom() -> i32;
    pub fn zax_power() -> i32;
    pub fn zax_reset() -> i32;
    pub fn zax_get_region() -> i32;
    pub fn zax_set_region(region: i32) -> i32;
    pub fn zax_set_ram_init(option: i32, seed: i32) -> i32;

    pub fn zax_set_input(pad1: u8, pad2: u8) -> i32;
    pub fn zax_run_frame() -> i32;
    pub fn zax_frame_count() -> i64;
    pub fn zax_get_frame_indices(out: *mut u8, out_len: usize) -> i32;
    pub fn zax_get_frame_emphasis(out: *mut u8, out_len: usize) -> i32;
    pub fn zax_set_sample_rate(rate: i32) -> i32;
    pub fn zax_get_sample_rate() -> i32;
    pub fn zax_get_audio(out: *mut i16, out_len: usize) -> i32;

    pub fn zax_state_size_hint() -> i32;
    pub fn zax_save_state(out: *mut u8, out_len: usize, written: *mut usize) -> i32;
    pub fn zax_load_state(data: *const u8, data_len: usize) -> i32;

    pub fn zax_set_tile_sink(sink: ZaxTilesFn, user: *mut c_void);
    pub fn zax_tiles_enabled() -> i32;

    pub fn zax_read_memory(address: u16) -> i32;
    pub fn zax_write_memory(address: u16, value: u8) -> c_int;

    pub fn zax_battery_ram_size() -> i32;
    pub fn zax_get_battery_ram(out: *mut u8, out_len: usize) -> i32;
    pub fn zax_set_battery_ram(data: *const u8, data_len: usize) -> i32;

    pub fn zax_movie_inspect(data: *const u8, data_len: usize, out: *mut ZaxMovieInfo) -> i32;
    pub fn zax_movie_play(data: *const u8, data_len: usize) -> i32;
    pub fn zax_movie_stop() -> i32;
    pub fn zax_movie_mode() -> i32;
    pub fn zax_movie_length() -> i64;
    pub fn zax_movie_frame() -> i64;
}
