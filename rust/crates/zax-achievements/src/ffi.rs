//! The functions of `csrc/zax_ra.c`, the C layer over rcheevos' client.

use std::ffi::{c_char, c_int, c_void};

/// The client of the C layer, opaque.
#[repr(C)]
pub struct ZaxRa {
    _private: [u8; 0],
}

/// The event record of the C layer (`zax_ra_event_t`).
#[repr(C)]
pub struct RawEvent {
    pub kind: u32,
    pub result: i32,
    pub id: u32,
    pub number: u32,
    pub title: *const c_char,
    pub description: *const c_char,
    pub detail: *const c_char,
    pub badge_url: *const c_char,
}

/// The achievement record of the C layer (`zax_ra_entry_t`).
#[repr(C)]
pub struct RawEntry {
    pub bucket: u32,
    pub bucket_label: *const c_char,
    pub id: u32,
    pub points: u32,
    pub title: *const c_char,
    pub description: *const c_char,
    pub progress: *const c_char,
    pub percent: f32,
    pub unlocked: u32,
    pub state: u32,
    pub kind: u32,
    pub rarity: f32,
    pub badge_url: *const c_char,
    pub badge_locked_url: *const c_char,
}

pub type ReadFn = unsafe extern "C" fn(*mut c_void, u32, *mut u8, u32) -> u32;
pub type RequestFn =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char, *const c_char, *const c_char);
pub type EventFn = unsafe extern "C" fn(*mut c_void, *const RawEvent);
pub type LogFn = unsafe extern "C" fn(*mut c_void, *const c_char);
pub type VisitFn = unsafe extern "C" fn(*mut c_void, *const RawEntry);

/// Events of the C layer, after rcheevos' own.
pub const EVENT_LOGIN: u32 = 100;
pub const EVENT_GAME_LOADED: u32 = 101;

/// `RC_OK`.
pub const RC_OK: c_int = 0;

unsafe extern "C" {
    pub fn zax_ra_create(
        host: *mut c_void,
        read: ReadFn,
        request: RequestFn,
        event: EventFn,
        log: LogFn,
        log_level: c_int,
    ) -> *mut ZaxRa;
    pub fn zax_ra_destroy(ra: *mut ZaxRa);
    pub fn zax_ra_hash_nes(data: *const u8, size: usize, hash: *mut c_char) -> c_int;
    pub fn zax_ra_complete(pending: *mut c_void, body: *const c_char, length: usize, status: c_int);
    pub fn zax_ra_discard(pending: *mut c_void);
    pub fn zax_ra_user_agent_clause(ra: *mut ZaxRa, buffer: *mut c_char, size: usize) -> usize;
    pub fn zax_ra_set_host(ra: *mut ZaxRa, host: *const c_char);
    pub fn zax_ra_set_modes(ra: *mut ZaxRa, hardcore: c_int, encore: c_int, unofficial: c_int);
    pub fn zax_ra_login_password(ra: *mut ZaxRa, username: *const c_char, password: *const c_char);
    pub fn zax_ra_login_token(ra: *mut ZaxRa, username: *const c_char, token: *const c_char);
    pub fn zax_ra_logout(ra: *mut ZaxRa);
    pub fn zax_ra_user(
        ra: *mut ZaxRa,
        display_name: *mut *const c_char,
        username: *mut *const c_char,
        token: *mut *const c_char,
        score: *mut u32,
        score_softcore: *mut u32,
    ) -> c_int;
    pub fn zax_ra_load_game(ra: *mut ZaxRa, data: *const u8, size: usize, path: *const c_char);
    pub fn zax_ra_unload_game(ra: *mut ZaxRa);
    pub fn zax_ra_load_game_state(ra: *mut ZaxRa) -> c_int;
    pub fn zax_ra_game(
        ra: *mut ZaxRa,
        id: *mut u32,
        title: *mut *const c_char,
        hash: *mut *const c_char,
        badge_url: *mut *const c_char,
    ) -> c_int;
    pub fn zax_ra_summary(
        ra: *mut ZaxRa,
        core: *mut u32,
        unofficial: *mut u32,
        unlocked: *mut u32,
        unsupported: *mut u32,
        points_core: *mut u32,
        points_unlocked: *mut u32,
    );
    pub fn zax_ra_do_frame(ra: *mut ZaxRa);
    pub fn zax_ra_idle(ra: *mut ZaxRa);
    pub fn zax_ra_reset(ra: *mut ZaxRa);
    pub fn zax_ra_progress_size(ra: *mut ZaxRa) -> usize;
    pub fn zax_ra_serialize_progress(ra: *mut ZaxRa, buffer: *mut u8, size: usize) -> c_int;
    pub fn zax_ra_deserialize_progress(ra: *mut ZaxRa, data: *const u8, size: usize) -> c_int;
    pub fn zax_ra_rich_presence(ra: *mut ZaxRa, buffer: *mut c_char, size: usize) -> usize;
    pub fn zax_ra_list(ra: *mut ZaxRa, category: c_int, visit: VisitFn, context: *mut c_void);
}
