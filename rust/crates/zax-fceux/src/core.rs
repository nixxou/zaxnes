//! The safe interface over the fork's C API.
//!
//! Every `unsafe` block here is an FFI call and nothing more; each carries a `// SAFETY:` note
//! saying which of the C API's preconditions it is relying on. The preconditions themselves come
//! from `zax/zaxfceux.h`: an instance must be live, buffers come with their lengths, and no two
//! threads may be inside the library at once — which [`Fceux`] enforces by being `!Sync` and by
//! being the only way to reach any of it.

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;

use crate::sys;

/// Width of a frame in pixels.
pub const FRAME_WIDTH: usize = sys::ZAX_SCREEN_WIDTH;
/// Height of a frame in pixels.
pub const FRAME_HEIGHT: usize = sys::ZAX_SCREEN_HEIGHT;
/// Bytes in one frame buffer: one byte a pixel, no padding.
pub const FRAME_LEN: usize = sys::FRAME_LEN;

/// What the engine can refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An instance is already alive. There is one FCEUX per process (see the module docs).
    AlreadyLive,
    /// The core refused to initialise.
    InitFailed,
    /// No ROM is loaded.
    NoRom,
    /// The image is not a ROM FCEUX can load; the lines the core printed are attached.
    LoadFailed(Vec<String>),
    /// An argument was out of range.
    BadArgument(&'static str),
    /// The destination is too small; the number of bytes needed is attached.
    BufferTooSmall(usize),
    /// A savestate could not be written, or could not be read back.
    State,
    /// A code the C API is not documented to return.
    Unexpected(i32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyLive => write!(
                formatter,
                "an FCEUX instance is already alive (FCEUX keeps its machine in globals, so there \
                 is one per process)"
            ),
            Self::InitFailed => write!(formatter, "the FCEUX core refused to initialise"),
            Self::NoRom => write!(formatter, "no ROM is loaded"),
            Self::LoadFailed(log) if log.is_empty() => {
                write!(formatter, "FCEUX refused the ROM image")
            }
            Self::LoadFailed(log) => {
                write!(formatter, "FCEUX refused the ROM image: {}", log.join("; "))
            }
            Self::BadArgument(what) => write!(formatter, "{what}"),
            Self::BufferTooSmall(needed) => {
                write!(
                    formatter,
                    "the buffer is too small; {needed} bytes are needed"
                )
            }
            Self::State => write!(formatter, "the savestate could not be written or read"),
            Self::Unexpected(code) => write!(formatter, "the FCEUX core returned {code}"),
        }
    }
}

impl std::error::Error for Error {}

/// The emulated region. The values are FCEUX's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Ntsc,
    Pal,
    Dendy,
}

impl Region {
    const fn code(self) -> i32 {
        match self {
            Self::Ntsc => sys::ZAX_REGION_NTSC,
            Self::Pal => sys::ZAX_REGION_PAL,
            Self::Dendy => sys::ZAX_REGION_DENDY,
        }
    }

    const fn from_code(code: i32) -> Option<Self> {
        match code {
            sys::ZAX_REGION_NTSC => Some(Self::Ntsc),
            sys::ZAX_REGION_PAL => Some(Self::Pal),
            sys::ZAX_REGION_DENDY => Some(Self::Dendy),
            _ => None,
        }
    }
}

/// What power-on writes into the 2 KiB of work RAM — the two fields a `.fm2` header records,
/// because a movie only replays if the RAM it started from is reproduced.
///
/// `Default`, `Ff` and `Zero` are deterministic. [`RamInit::Random`] is **not** reproducible from
/// outside: FCEUX reseeds it from `rand()` on every power-on unless a movie is playing, which is
/// its own behaviour and exactly why a movie header carries the seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RamInit {
    /// FCEUX's own `00 00 00 00 FF FF FF FF` pattern.
    #[default]
    Default,
    /// All `$FF`.
    Ff,
    /// All `$00`.
    Zero,
    /// Pseudo-random, from the given seed — see the warning above.
    Random(i32),
}

impl RamInit {
    const fn parts(self) -> (i32, i32) {
        match self {
            Self::Default => (sys::ZAX_RAM_INIT_DEFAULT, 0),
            Self::Ff => (sys::ZAX_RAM_INIT_FF, 0),
            Self::Zero => (sys::ZAX_RAM_INIT_00, 0),
            Self::Random(seed) => (sys::ZAX_RAM_INIT_RANDOM, seed),
        }
    }
}

/// The lines the core has printed since they were last taken.
///
/// A `static` because the C API's log sink is a plain function pointer with no Rust closure behind
/// it: pushing to a mutex from the callback is the whole mechanism, and it costs nothing when no
/// instance exists.
static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The sink the C API calls, once per line.
///
/// # Safety
///
/// `text` must be a NUL-terminated string valid for the duration of the call, which is what
/// `zax/zaxstubs.cpp` guarantees (it passes a `std::string`'s `c_str()` from its line buffer).
unsafe extern "C" fn log_sink(_user: *mut c_void, text: *const c_char) {
    if text.is_null() {
        return;
    }

    // SAFETY: the C API's contract for this callback (zaxfceux.h) is that `text` is a
    // NUL-terminated string owned by the library and valid until the call returns, so reading it
    // here and copying it is sound. A panic must not cross back into C, so the lock is handled
    // without unwrapping: a poisoned mutex simply drops the line.
    let line = unsafe { CStr::from_ptr(text) }
        .to_string_lossy()
        .into_owned();
    if let Ok(mut log) = LOG.lock() {
        // A runaway core must not grow this without bound; the loading chatter of one ROM is a
        // dozen lines, so this ceiling is only ever hit by a fault.
        if log.len() < 4096 {
            log.push(line);
        }
    }
}

fn take_log() -> Vec<String> {
    LOG.lock()
        .map(|mut log| std::mem::take(&mut *log))
        .unwrap_or_default()
}

/// The one live FCEUX instance.
///
/// Dropping it closes the game and frees the core's buffers, after which a new one can be made.
/// It is `Send` but **not `Sync`**: it may move between threads, but the library must never be
/// entered from two at once.
#[derive(Debug)]
pub struct Fceux {
    /// Makes the handle `!Sync` without making it `!Send`, which is exactly the rule the C API
    /// states: one thread at a time, but not necessarily always the same one.
    _not_sync: PhantomData<std::cell::Cell<()>>,
}

impl Fceux {
    /// Creates the instance and initialises the core.
    ///
    /// # Errors
    ///
    /// [`Error::AlreadyLive`] if one is already alive — there is one FCEUX per process.
    /// [`Error::InitFailed`] if the core refused to initialise.
    pub fn new() -> Result<Self, Error> {
        // SAFETY: both calls are safe at any time per the C API. The sink is installed before the
        // instance exists so that a failure in `zax_create` is still reported, and `log_sink`
        // only ever reads the string it is handed.
        unsafe {
            sys::zax_set_log_sink(Some(log_sink), std::ptr::null_mut());
        }
        let _ = take_log();

        // SAFETY: `zax_create` takes no argument and is documented to be callable at any time; it
        // reports an already-live instance with a code rather than by misbehaving.
        let code = unsafe { sys::zax_create() };
        match code {
            sys::ZAX_OK => Ok(Self {
                _not_sync: PhantomData,
            }),
            sys::ZAX_ERR_ALREADY_LIVE => Err(Error::AlreadyLive),
            _ => Err(Error::InitFailed),
        }
    }

    /// The library's version string: the FCEUX version forked and the build's configuration.
    #[must_use]
    pub fn version() -> String {
        // SAFETY: `zax_version` returns a pointer to a NUL-terminated string in static storage
        // that is never freed and never null (zaxfceux.h), so it is valid to read here.
        let text = unsafe { CStr::from_ptr(sys::zax_version()) };
        text.to_string_lossy().into_owned()
    }

    /// The lines the core has printed since this was last called.
    #[must_use]
    pub fn take_log(&self) -> Vec<String> {
        // SAFETY: flushing hands over whatever line is buffered but unterminated; it only calls
        // the sink, which is sound (see `log_sink`).
        unsafe {
            sys::zax_flush_log();
        }
        take_log()
    }

    /// Loads a ROM from memory. `name` is what FCEUX reports and what it uses, with the image's
    /// MD5, against its own cartridge database. Nothing touches the filesystem.
    ///
    /// On success the machine is powered on.
    ///
    /// # Errors
    ///
    /// [`Error::BadArgument`] for an empty image or a name containing a NUL byte;
    /// [`Error::LoadFailed`] with the core's own explanation otherwise.
    pub fn load_rom(&mut self, image: &[u8], name: &str) -> Result<(), Error> {
        if image.is_empty() {
            return Err(Error::BadArgument("the ROM image is empty"));
        }
        let Ok(name) = CString::new(name) else {
            return Err(Error::BadArgument("the ROM name contains a NUL byte"));
        };

        let _ = self.take_log();
        // SAFETY: `image` is a live slice, so `as_ptr()` is valid for `len()` bytes, and the C API
        // copies those bytes into its own stream before returning (EMUFILE_MEMORY's constructor).
        // `name` is NUL-terminated and outlives the call.
        let code = unsafe { sys::zax_load_rom(image.as_ptr(), image.len(), name.as_ptr()) };
        match code {
            sys::ZAX_OK => Ok(()),
            sys::ZAX_ERR_BAD_ARGUMENT => Err(Error::BadArgument("FCEUX refused the arguments")),
            _ => Err(Error::LoadFailed(self.take_log())),
        }
    }

    /// Closes the loaded game, if any.
    pub fn close_rom(&mut self) {
        // SAFETY: documented as idempotent and safe with no ROM loaded.
        unsafe {
            sys::zax_close_rom();
        }
    }

    /// Whether a game is loaded.
    #[must_use]
    pub fn has_rom(&self) -> bool {
        // SAFETY: a pure query, safe at any time.
        unsafe { sys::zax_has_rom() == 1 }
    }

    /// Power-cycles the machine.
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded.
    pub fn power(&mut self) -> Result<(), Error> {
        // SAFETY: no argument; refuses with a code when no ROM is loaded.
        check(unsafe { sys::zax_power() })
    }

    /// Taps the reset line.
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded.
    pub fn reset(&mut self) -> Result<(), Error> {
        // SAFETY: as `power`.
        check(unsafe { sys::zax_reset() })
    }

    /// The region in force.
    ///
    /// # Errors
    ///
    /// [`Error::Unexpected`] if the core names a region this crate does not know.
    pub fn region(&self) -> Result<Region, Error> {
        // SAFETY: a pure query on a live instance, which `&self` proves.
        let code = unsafe { sys::zax_get_region() };
        Region::from_code(code).ok_or(Error::Unexpected(code))
    }

    /// Sets the region. Loading a ROM applies the ROM's own, so set this afterwards to override
    /// it.
    ///
    /// # Errors
    ///
    /// [`Error::BadArgument`] if the core refuses the value.
    pub fn set_region(&mut self, region: Region) -> Result<(), Error> {
        // SAFETY: `Region::code` can only produce one of the three values the C API accepts.
        check(unsafe { sys::zax_set_region(region.code()) })
    }

    /// Sets what the next power-on writes into the work RAM. Set it before loading.
    ///
    /// # Errors
    ///
    /// [`Error::BadArgument`] if the core refuses the value.
    pub fn set_ram_init(&mut self, init: RamInit) -> Result<(), Error> {
        let (option, seed) = init.parts();
        // SAFETY: `RamInit::parts` can only produce one of the four options the C API accepts.
        check(unsafe { sys::zax_set_ram_init(option, seed) })
    }

    /// The buttons of the two pads for the next frame: A, B, Select, Start, Up, Down, Left, Right
    /// from bit 0 to bit 7, the order both engines shift them out in.
    ///
    /// # Errors
    ///
    /// [`Error::Unexpected`] if the core refuses, which it cannot do for these arguments.
    pub fn set_input(&mut self, pad1: u8, pad2: u8) -> Result<(), Error> {
        // SAFETY: two by-value bytes; nothing can be out of range.
        check(unsafe { sys::zax_set_input(pad1, pad2) })
    }

    /// The input word Zaxnes's own core takes, split into the two pad bytes FCEUX wants: the low
    /// byte is port 1 and the high byte port 2, which is `zax-nes`'s layout as well.
    ///
    /// # Errors
    ///
    /// As [`Fceux::set_input`].
    pub fn set_input_word(&mut self, word: u16) -> Result<(), Error> {
        let [pad1, pad2] = word.to_le_bytes();
        self.set_input(pad1, pad2)
    }

    /// Runs exactly one frame.
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded.
    pub fn run_frame(&mut self) -> Result<(), Error> {
        // SAFETY: no argument; refuses with a code when no ROM is loaded.
        check(unsafe { sys::zax_run_frame() })
    }

    /// How many frames have run since power-on.
    #[must_use]
    pub fn frame_count(&self) -> i64 {
        // SAFETY: a pure query on a live instance.
        unsafe { sys::zax_frame_count() }.max(0)
    }

    /// Copies the frame's picture into `out`, one NES colour index (0 to 63) a pixel, row-major.
    /// `out` is resized to [`FRAME_LEN`].
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] before the first frame.
    pub fn frame_indices_into(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        self.copy_frame(out, false)
    }

    /// Copies the frame's per-pixel emphasis bits into `out` (bits 0-2 of `$2001`, 0 for none).
    /// `out` is resized to [`FRAME_LEN`].
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] before the first frame.
    pub fn frame_emphasis_into(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        self.copy_frame(out, true)
    }

    // `&self` does not appear in the body and is still the right signature: it is the proof that
    // an instance is live, which is the C API's precondition for reading a frame, and it is what
    // keeps the two public wrappers above `&self` methods rather than free functions.
    #[allow(clippy::unused_self)]
    fn copy_frame(&self, out: &mut Vec<u8>, emphasis: bool) -> Result<(), Error> {
        out.clear();
        out.resize(FRAME_LEN, 0);
        // SAFETY: `out` has just been resized to FRAME_LEN, so the pointer is valid for writing
        // exactly the length handed over, which is the only thing the C API writes through it. It
        // refuses a short buffer with a code rather than overrunning, and this one is never short.
        let code = unsafe {
            if emphasis {
                sys::zax_get_frame_emphasis(out.as_mut_ptr(), FRAME_LEN)
            } else {
                sys::zax_get_frame_indices(out.as_mut_ptr(), FRAME_LEN)
            }
        };
        check(code)
    }

    /// Sets the audio sample rate in Hz: 44100, 48000, 96000, or 0 for silence.
    ///
    /// # Errors
    ///
    /// [`Error::BadArgument`] for any other value — FCEUX supports only those.
    pub fn set_sample_rate(&mut self, rate: i32) -> Result<(), Error> {
        // SAFETY: a by-value integer; the C API validates it and refuses with a code.
        check(unsafe { sys::zax_set_sample_rate(rate) })
    }

    /// The sample rate in force.
    #[must_use]
    pub fn sample_rate(&self) -> i32 {
        // SAFETY: a pure query on a live instance.
        unsafe { sys::zax_get_sample_rate() }.max(0)
    }

    /// Appends the last frame's samples to `out` and returns how many were added. Mono.
    ///
    /// The core's buffer does not carry over from one frame to the next, so this reads a frame's
    /// worth once; calling it twice for one frame returns the same samples again.
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded.
    pub fn read_audio_into(&self, out: &mut Vec<i16>) -> Result<usize, Error> {
        // A frame is about 800 samples at 48 kHz and 1600 at 96 kHz; this is far above both, so
        // nothing is ever dropped.
        const CHUNK: usize = 8192;

        let start = out.len();
        out.resize(start + CHUNK, 0);
        // SAFETY: `out` has just been grown so that `CHUNK` samples fit from `start`, and the C
        // API writes at most the length it is given. It returns the count it wrote, or a negative
        // code, and never writes past `out_len`.
        let got = unsafe { sys::zax_get_audio(out.as_mut_ptr().add(start), CHUNK) };
        if got < 0 {
            out.truncate(start);
            return Err(code_to_error(got));
        }

        let got = usize::try_from(got).unwrap_or(0).min(CHUNK);
        out.truncate(start + got);
        Ok(got)
    }

    /// Writes a savestate into `out`, replacing its contents. The allocation is reused, which is
    /// what a rollback history wants.
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded, [`Error::State`] if the core could not write one.
    pub fn save_state_into(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        // Ask once, and if the guess was short, ask again with the length the core reported. The
        // second call always fits: the size depends on the board, not on what the game is doing.
        let hint = {
            // SAFETY: a pure query on a live instance; negative on error.
            let hint = unsafe { sys::zax_state_size_hint() };
            if hint <= 0 {
                return Err(code_to_error(hint));
            }
            usize::try_from(hint).unwrap_or(0)
        };

        // First attempt, at whichever is larger: the hint, or whatever the caller's buffer already
        // holds (a rollback history hands back the same allocation every time, so this is usually
        // already big enough and nothing is reallocated).
        match Self::write_state(out, hint.max(out.len())) {
            Ok(()) => Ok(()),
            // The core reports what it would have needed, and that length always fits: a state's
            // size depends on the board, not on what the game is doing.
            Err(Error::BufferTooSmall(needed)) if needed > 0 => Self::write_state(out, needed),
            Err(error) => Err(error),
        }
    }

    /// One attempt at a savestate into `out` grown to `capacity`. On [`Error::BufferTooSmall`] the
    /// error carries the length the core asked for.
    fn write_state(out: &mut Vec<u8>, capacity: usize) -> Result<(), Error> {
        out.clear();
        out.resize(capacity, 0);

        let mut written: usize = 0;
        // SAFETY: `out` holds `capacity` writable bytes and that same length is handed over; the
        // C API writes at most that many and otherwise refuses with a code, setting `written` to
        // what it would have needed. `written` is a live local for the whole call.
        let code = unsafe { sys::zax_save_state(out.as_mut_ptr(), capacity, &raw mut written) };

        if code == sys::ZAX_OK {
            out.truncate(written.min(capacity));
            return Ok(());
        }

        out.clear();
        if code == sys::ZAX_ERR_BUFFER_TOO_SMALL {
            return Err(Error::BufferTooSmall(written));
        }
        Err(code_to_error(code))
    }

    /// Loads a savestate written by [`Fceux::save_state_into`] for the same ROM.
    ///
    /// # Errors
    ///
    /// [`Error::State`] if the core refuses it, in which case the machine may be left part-way
    /// through and should be powered on again.
    pub fn load_state(&mut self, data: &[u8]) -> Result<(), Error> {
        if data.is_empty() {
            return Err(Error::BadArgument("the savestate is empty"));
        }
        // SAFETY: `data` is a live slice, so the pointer is valid for `len()` bytes, which is the
        // length handed over; the C API only reads through it.
        check(unsafe { sys::zax_load_state(data.as_ptr(), data.len()) })
    }

    /// Reads one byte of the CPU address space **without side effects**: no bank is switched and
    /// no register is clocked, which is what a Lua watch or an achievement condition needs.
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded.
    pub fn read_memory(&self, address: u16) -> Result<u8, Error> {
        // SAFETY: a by-value address; the C API answers with the byte or a negative code.
        let value = unsafe { sys::zax_read_memory(address) };
        if value < 0 {
            return Err(code_to_error(value));
        }
        Ok(u8::try_from(value & 0xFF).unwrap_or(0))
    }

    /// Writes one byte of the CPU address space as the CPU would, side effects included.
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded.
    pub fn write_memory(&mut self, address: u16, value: u8) -> Result<(), Error> {
        // SAFETY: two by-value arguments; nothing can be out of range.
        check(unsafe { sys::zax_write_memory(address, value) })
    }
}

impl Drop for Fceux {
    fn drop(&mut self) {
        // SAFETY: documented as idempotent and safe whatever state the core is in; this is the
        // only place it is called, and the handle cannot be used afterwards.
        unsafe {
            sys::zax_destroy();
            sys::zax_set_log_sink(None, std::ptr::null_mut());
        }
        let _ = take_log();
    }
}

fn check(code: i32) -> Result<(), Error> {
    if code == sys::ZAX_OK {
        Ok(())
    } else {
        Err(code_to_error(code))
    }
}

fn code_to_error(code: i32) -> Error {
    match code {
        sys::ZAX_ERR_ALREADY_LIVE => Error::AlreadyLive,
        sys::ZAX_ERR_NO_INSTANCE => Error::InitFailed,
        sys::ZAX_ERR_NO_ROM => Error::NoRom,
        sys::ZAX_ERR_BAD_ARGUMENT => Error::BadArgument("the core refused an argument"),
        sys::ZAX_ERR_LOAD_FAILED => Error::LoadFailed(take_log()),
        sys::ZAX_ERR_BUFFER_TOO_SMALL => Error::BufferTooSmall(0),
        sys::ZAX_ERR_STATE_FAILED => Error::State,
        other => Error::Unexpected(other),
    }
}
