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
    /// The bytes are not a version-3 `.fm2`, or the file is truncated.
    MovieFormat,
    /// The movie states a ROM checksum and it is not the loaded ROM's.
    MovieRom,
    /// The movie's header asks for something this build cannot play. The text names the thing,
    /// so that a host can say which one rather than "unsupported".
    MovieUnsupported(&'static str),
    /// A call that needs a movie, with none loaded.
    NoMovie,
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
            Self::MovieFormat => write!(
                formatter,
                "this is not a version 3 .fm2 movie (or the file is truncated)"
            ),
            Self::MovieRom => write!(
                formatter,
                "this movie was recorded on another ROM, so it would not replay"
            ),
            Self::MovieUnsupported(what) => write!(
                formatter,
                "this movie needs {what}, which the FCEUX engine cannot play here"
            ),
            Self::NoMovie => write!(formatter, "no movie is loaded"),
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

    /// Upper bound the core reports on the size of a savestate for the ROM loaded, which a host
    /// that has to announce a state size before serialising needs (REQ-ARCH-013).
    ///
    /// # Errors
    ///
    /// [`Error::NoRom`] when no ROM is loaded, [`Error::State`] if the core could not measure
    /// one.
    pub fn state_size_hint(&self) -> Result<usize, Error> {
        // SAFETY: a query on a live instance, which `&self` proves; negative on error.
        let hint = unsafe { sys::zax_state_size_hint() };
        if hint <= 0 {
            return Err(code_to_error(hint));
        }
        Ok(usize::try_from(hint).unwrap_or(0))
    }

    /// Bytes the cartridge's battery-backed memory occupies, 0 when it has none.
    ///
    /// These are the bytes FCEUX itself would write to a `.sav` file; the fork's C API hands
    /// them over without one (`zax/README.md`, *Battery-backed save memory*).
    ///
    /// # Errors
    ///
    /// [`Error::Unexpected`] for a code the C API is not documented to return.
    pub fn battery_ram_size(&self) -> Result<usize, Error> {
        // SAFETY: a pure query on a live instance; 0 for a cart without a battery, negative on
        // an error.
        let size = unsafe { sys::zax_battery_ram_size() };
        if size < 0 {
            return Err(code_to_error(size));
        }
        Ok(usize::try_from(size).unwrap_or(0))
    }

    /// Copies the battery-backed memory into `out`, replacing its contents and keeping its
    /// allocation. `out` is left empty for a cartridge without a battery.
    ///
    /// # Errors
    ///
    /// As [`Fceux::battery_ram_size`].
    pub fn battery_ram_into(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        let needed = self.battery_ram_size()?;
        out.clear();
        if needed == 0 {
            return Ok(());
        }

        out.resize(needed, 0);
        // SAFETY: `out` has just been resized to `needed`, which is the length handed over and
        // the length the C API was asked for; it writes at most that many bytes and refuses a
        // short buffer with a code rather than overrunning.
        let got = unsafe { sys::zax_get_battery_ram(out.as_mut_ptr(), needed) };
        if got < 0 {
            out.clear();
            return Err(code_to_error(got));
        }

        out.truncate(usize::try_from(got).unwrap_or(0).min(needed));
        Ok(())
    }

    /// Restores battery-backed memory saved by [`Fceux::battery_ram_into`]. `false` when the
    /// cartridge has no battery or `data` is not exactly the length its board registered — in
    /// which case nothing is written.
    ///
    /// # Errors
    ///
    /// [`Error::Unexpected`] for a code the C API is not documented to return.
    pub fn set_battery_ram(&mut self, data: &[u8]) -> Result<bool, Error> {
        let needed = self.battery_ram_size()?;
        if needed == 0 || data.len() != needed {
            return Ok(false);
        }

        // SAFETY: `data` is a live slice, so the pointer is valid for `len()` bytes, which is the
        // length handed over; the C API only reads through it, and demands exactly that length.
        let written = unsafe { sys::zax_set_battery_ram(data.as_ptr(), data.len()) };
        if written < 0 {
            return Err(code_to_error(written));
        }
        Ok(usize::try_from(written).unwrap_or(0) == needed)
    }
}

// ------------------------------------------------------------------------------------------------
// `.fm2` movie playback (lot FCEUX-48 step 3)
// ------------------------------------------------------------------------------------------------

/// Where playback is.
///
/// One thing to know before using [`MovieMode::Finished`] as "the movie is over": FCEUX decides
/// that at the **top of a frame**, not at the bottom of the last one. `FCEUMOV_AddInputState`
/// calls `FinishPlayback` when it finds `currFrameCounter` past the last record
/// (movie.cpp:1245-1250), so after running a movie's last frame the mode is still `Playing` and it
/// becomes `Finished` only once another frame is run. [`Fceux::movie_position`] is the signal that
/// does not lag: the clock reaching the length *is* the end of the movie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovieMode {
    /// No movie loaded.
    None,
    /// A movie is loaded and has frames left — or has just run its last one; see above.
    Playing,
    /// A movie is loaded and a frame was asked for past its last record. The machine keeps
    /// running, which is FCEUX's own behaviour, with the pads live again.
    Finished,
}

/// A `.fm2` header, as the core reads it without touching the machine.
///
/// This is what makes a refusal nameable: the host reads the header, says *which* thing the movie
/// needs that this build has not, and only then asks for playback.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // the `.fm2` header's own independent flags, key by key
pub struct MovieHeader {
    /// The `.fm2` format version. Only 3 can be played.
    pub version: i32,
    /// Input records, which is the movie's length in frames.
    pub frames: u32,
    /// The region the movie was recorded at (`palFlag`), which playback applies.
    pub region: Region,
    /// The RAM fill the movie was recorded with, seed included.
    pub ram_init: RamInit,
    /// The header asks for FCEUX's new PPU.
    pub new_ppu: bool,
    /// The movie is of a Famicom Disk System game.
    pub fds: bool,
    /// Four pads are logged per frame.
    pub fourscore: bool,
    /// The Famicom microphone is logged.
    pub microphone: bool,
    /// FCEUX's device numbers for the two pad ports and the expansion port (`ESI`, `ESI`,
    /// `ESIFC`). A pad port may be 1 (`SI_GAMEPAD`) or 0 (`SI_NONE`, an empty socket, which is
    /// what a one-player `.fm2` records for port 2); the expansion port must be 0 (`SIFC_NONE`).
    pub ports: [i32; 3],
    /// The movie carries a savestate to start from, rather than starting at power-on.
    pub from_savestate: bool,
    /// The movie carries battery RAM to start from.
    pub from_saveram: bool,
    /// The header's re-record count, for a host that shows it.
    pub rerecord_count: i32,
    /// The MD5 the movie states for the ROM it was recorded on; `None` when it states none.
    pub rom_md5: Option<[u8; 16]>,
    /// Whether the stated MD5 is the loaded ROM's: `None` when the movie states none, or when no
    /// ROM is loaded.
    pub rom_matches: Option<bool>,
}

impl MovieHeader {
    /// Why this movie cannot be played here, naming the one thing — `None` when it can.
    ///
    /// The order is the order a user would want to hear about them in: the format first, then
    /// what the movie is of, then the hardware it logs.
    #[must_use]
    pub fn refusal(&self) -> Option<Error> {
        if self.version != 3 {
            return Some(Error::MovieFormat);
        }
        if self.fds {
            return Some(Error::MovieUnsupported("a Famicom Disk System disk"));
        }
        if self.new_ppu {
            return Some(Error::MovieUnsupported("FCEUX's new PPU"));
        }
        if self.fourscore {
            return Some(Error::MovieUnsupported("a Four Score (four pads)"));
        }
        if self.microphone {
            return Some(Error::MovieUnsupported("the Famicom microphone"));
        }
        // `SI_GAMEPAD` is 1 and `SI_NONE` is 0 (`src/git.h`), and **both are fine**: a one-player
        // `.fm2` records `port1 0` for the empty socket, which is the common case rather than an
        // exotic one, and the core installs what the header asks for so an empty socket reads as
        // empty. Anything else is a device this build has no word for, and the movie's records
        // would be replayed against a standard pad.
        if !matches!(self.ports[0], 0 | 1) || !matches!(self.ports[1], 0 | 1) {
            return Some(Error::MovieUnsupported(
                "an input device other than a standard pad",
            ));
        }
        if self.ports[2] != 0 {
            return Some(Error::MovieUnsupported("an expansion-port device"));
        }
        if self.rom_matches == Some(false) {
            return Some(Error::MovieRom);
        }
        None
    }
}

impl Fceux {
    /// Reads the header of a `.fm2` **without touching the machine**: nothing is loaded, no
    /// power-on happens, and a movie already playing is undisturbed.
    ///
    /// # Errors
    ///
    /// [`Error::MovieFormat`] when the bytes are not a version-3 `.fm2`.
    pub fn inspect_movie(&self, movie: &[u8]) -> Result<MovieHeader, Error> {
        if movie.is_empty() {
            return Err(Error::MovieFormat);
        }

        let mut info = sys::ZaxMovieInfo::default();
        // SAFETY: `movie` is a live slice, so the pointer is valid for `len()` bytes, which is
        // the length handed over and the only thing the C API reads through it; `info` is a live
        // `repr(C)` mirror of `zax_movie_info`, which the call fills and does not keep.
        let code = unsafe { sys::zax_movie_inspect(movie.as_ptr(), movie.len(), &raw mut info) };
        if code != sys::ZAX_OK {
            return Err(code_to_error(code));
        }

        Ok(MovieHeader {
            version: info.version,
            frames: u32::try_from(info.frames).unwrap_or(0),
            region: Region::from_code(info.region).unwrap_or(Region::Ntsc),
            ram_init: match info.ram_init_option {
                sys::ZAX_RAM_INIT_FF => RamInit::Ff,
                sys::ZAX_RAM_INIT_00 => RamInit::Zero,
                sys::ZAX_RAM_INIT_RANDOM => RamInit::Random(info.ram_init_seed),
                _ => RamInit::Default,
            },
            new_ppu: info.new_ppu != 0,
            fds: info.fds != 0,
            fourscore: info.fourscore != 0,
            microphone: info.microphone != 0,
            ports: [info.port0, info.port1, info.port2],
            from_savestate: info.from_savestate != 0,
            from_saveram: info.from_saveram != 0,
            rerecord_count: info.rerecord_count,
            rom_md5: (info.rom_md5_known != 0).then_some(info.rom_md5),
            rom_matches: match info.rom_md5_matches {
                1 => Some(true),
                0 => Some(false),
                _ => None,
            },
        })
    }

    /// Loads a `.fm2` and starts playing it from its own beginning, through FCEUX's own movie
    /// mode: the machine is re-powered (or restored from the movie's savestate) with the region
    /// and the RAM fill the header asks for, and the next [`Fceux::run_frame`] plays frame 0.
    ///
    /// The header is read and judged **before** anything is committed, so a movie this build
    /// cannot play is refused with the reason named and the machine left as it was. That is also
    /// why the error is the specific one rather than "unsupported": the header says which.
    ///
    /// # Errors
    ///
    /// [`Error::MovieFormat`], [`Error::MovieRom`], [`Error::MovieUnsupported`] as
    /// [`MovieHeader::refusal`] gives them, or [`Error::NoRom`] with no cartridge loaded.
    pub fn play_movie(&mut self, movie: &[u8]) -> Result<MovieHeader, Error> {
        let header = self.inspect_movie(movie)?;
        if let Some(refusal) = header.refusal() {
            return Err(refusal);
        }

        // SAFETY: `movie` is a live slice, so the pointer is valid for `len()` bytes, which is
        // the length handed over; the C API only reads through it, and copies what it keeps
        // (FCEUX parses the bytes into its own `MovieData`) before returning.
        let code = unsafe { sys::zax_movie_play(movie.as_ptr(), movie.len()) };
        if code != sys::ZAX_OK {
            return Err(code_to_error(code));
        }
        Ok(header)
    }

    /// Stops playback and unloads the movie. The machine keeps running from where it is, which is
    /// FCEUX's own behaviour; a caller that wants the game from the start calls [`Fceux::power`].
    ///
    /// Not an error with no movie loaded, so that stopping twice is harmless.
    ///
    /// # Errors
    ///
    /// Only an unexpected code from the core.
    pub fn stop_movie(&mut self) -> Result<(), Error> {
        // SAFETY: documented as safe whatever movie state the core is in, on a live instance.
        let code = unsafe { sys::zax_movie_stop() };
        if code != sys::ZAX_OK {
            return Err(code_to_error(code));
        }
        Ok(())
    }

    /// Where playback is.
    #[must_use]
    pub fn movie_mode(&self) -> MovieMode {
        // SAFETY: a pure query on a live instance.
        match unsafe { sys::zax_movie_mode() } {
            sys::ZAX_MOVIE_PLAYING => MovieMode::Playing,
            sys::ZAX_MOVIE_FINISHED => MovieMode::Finished,
            _ => MovieMode::None,
        }
    }

    /// The movie's length in frames, and the frame the next [`Fceux::run_frame`] will play —
    /// FCEUX's own `currFrameCounter`, which is the movie's clock and not
    /// [`Fceux::frame_count`]'s. `None` with no movie loaded.
    ///
    /// Two facts about that clock, both measured rather than assumed, and both of which a host
    /// has to know (`fceux_movie_tests.rs` pins them):
    ///
    /// * the clock reaching the length **is** the end of the movie, and it gets there one frame
    ///   before [`Fceux::movie_mode`] says [`MovieMode::Finished`] — see that type;
    /// * the clock **goes on counting past the end** while the movie stays loaded
    ///   (`currFrameCounter++` at movie.cpp:1329 is unconditional). So the test for "over" is
    ///   `frame >= length`, and a host computing a remaining count has to saturate.
    #[must_use]
    pub fn movie_position(&self) -> Option<(u32, u32)> {
        // SAFETY: two pure queries on a live instance; both answer negatively with no movie.
        let (length, frame) = unsafe { (sys::zax_movie_length(), sys::zax_movie_frame()) };
        if length < 0 || frame < 0 {
            return None;
        }
        Some((
            u32::try_from(length).unwrap_or(0),
            u32::try_from(frame).unwrap_or(0),
        ))
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
        sys::ZAX_ERR_MOVIE_FORMAT => Error::MovieFormat,
        sys::ZAX_ERR_MOVIE_ROM => Error::MovieRom,
        // The C API has one code for every unsupported header; the caller of `play_movie` turns
        // it into the named reason by inspecting the header first, so this is the fallback for a
        // path that did not.
        sys::ZAX_ERR_MOVIE_UNSUPPORTED => Error::MovieUnsupported("something this build has not"),
        sys::ZAX_ERR_NO_MOVIE => Error::NoMovie,
        other => Error::Unexpected(other),
    }
}

// ------------------------------------------------------------------------------------------------
// Tile records (lot FCEUX-1 step 6)
// ------------------------------------------------------------------------------------------------

/// One 8x8 tile of a frame, in the layout Zaxnes's own `TileRecord` uses: the two CHR bit planes
/// with the flips **not** applied, where the tile sits, its sub-palette and its flags.
///
/// This is deliberately a type of its own rather than `zax_nes::TileRecord`: `zax-fceux` must not
/// depend on the core it is an alternative to. [`TileCapture::records`] is what a caller converts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Four flags, because the record of SPEC-02 §4.1 has four and this type mirrors it field for
// field; packing them into a bitfield here would only move the unpacking to every caller.
#[allow(clippy::struct_excessive_bools)]
pub struct Tile {
    /// Low bit plane, the eight CHR bytes packed little-endian by row.
    pub plane0: u64,
    /// High bit plane, likewise.
    pub plane1: u64,
    /// Screen x of the top-left pixel.
    pub x: i16,
    /// Screen y of the top-left pixel.
    pub y: i16,
    /// Background: row * 33 + column. Sprite: the OAM number, +64 for an 8x16's lower half.
    pub index: i32,
    /// Horizontal flip (OAM attribute bit 6); never set on a background tile.
    pub mirror_h: bool,
    /// Vertical flip (bit 7); never set on a background tile.
    pub mirror_v: bool,
    /// "Behind the background" (bit 5); never set on a background tile.
    pub hidden: bool,
    /// Sub-palette, 0 to 3.
    pub palette: u8,
    /// True for a background tile, false for a sprite.
    pub background: bool,
}

/// One frame's tile records and its palette.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TileCapture {
    /// The records in capture order: chronological by line, and for one band the sprites before
    /// the background row.
    pub records: Vec<Tile>,
    /// The 32 bytes of `$3F00-$3F1F`, each a NES colour index 0 to 63.
    pub palette: [u8; 32],
}

/// Where the sink leaves the frame it was handed, for [`Fceux::take_tiles`] to collect.
///
/// A `static` for the same reason [`LOG`] is one: the C API's sink is a plain function pointer
/// with no Rust closure behind it.
static TILES: Mutex<Option<TileCapture>> = Mutex::new(None);

/// The sink the C API calls once per frame.
///
/// # Safety
///
/// `records` must point to `count` valid [`sys::ZaxTileRecord`]s and `palette` to 32 bytes, both
/// valid for the duration of the call — which is what `zax/zaxtiles.cpp` guarantees (it passes
/// its own arrays and does not free them until the next frame).
unsafe extern "C" fn tile_sink(
    _user: *mut c_void,
    records: *const sys::ZaxTileRecord,
    count: usize,
    palette: *const u8,
) {
    if records.is_null() || palette.is_null() {
        return;
    }

    // SAFETY: the C API's contract for this callback (zaxfceux.h) is exactly the two slices built
    // here — `count` records and 32 palette bytes, owned by the library and valid until it
    // returns. Nothing is kept past the call: both are copied into owned Rust values below.
    let (raw, palette) = unsafe {
        (
            std::slice::from_raw_parts(records, count),
            std::slice::from_raw_parts(palette, 32),
        )
    };

    let mut capture = TileCapture {
        records: Vec::with_capacity(raw.len()),
        palette: [0; 32],
    };
    capture.palette.copy_from_slice(palette);
    capture.records.extend(raw.iter().map(|record| Tile {
        plane0: record.plane0,
        plane1: record.plane1,
        x: record.x,
        y: record.y,
        index: record.index,
        mirror_h: (record.flags & sys::ZAX_TILE_MIRROR_H) != 0,
        mirror_v: (record.flags & sys::ZAX_TILE_MIRROR_V) != 0,
        hidden: (record.flags & sys::ZAX_TILE_HIDDEN) != 0,
        palette: record.palette,
        background: (record.flags & sys::ZAX_TILE_BACKGROUND) != 0,
    }));

    // A panic must not cross back into C, so a poisoned lock simply drops the frame.
    if let Ok(mut slot) = TILES.lock() {
        *slot = Some(capture);
    }
}

impl Fceux {
    /// Turns the tile capture on or off. **Off by default**: with no sink installed the two
    /// per-scanline hooks in the fork return at once, so a caller that only wants the picture
    /// pays nothing.
    pub fn set_tile_capture(&mut self, enabled: bool) {
        if let Ok(mut slot) = TILES.lock() {
            *slot = None;
        }

        // SAFETY: installing or removing the sink is safe at any time per the C API, and
        // `tile_sink` only reads the two buffers it is handed (see its own safety note).
        unsafe {
            if enabled {
                sys::zax_set_tile_sink(Some(tile_sink), std::ptr::null_mut());
            } else {
                sys::zax_set_tile_sink(None, std::ptr::null_mut());
            }
        }
    }

    /// Whether the capture is on.
    #[must_use]
    pub fn tile_capture_enabled(&self) -> bool {
        // SAFETY: a pure query, safe at any time.
        unsafe { sys::zax_tiles_enabled() == 1 }
    }

    /// The records of the frame just run, or `None` when the capture is off or no frame has run
    /// since the last call.
    #[must_use]
    pub fn take_tiles(&self) -> Option<TileCapture> {
        TILES.lock().ok().and_then(|mut slot| slot.take())
    }
}

/// Judges a `.fm2` against the ROM `image` **without disturbing anything** (lot FCEUX-48 step 7).
///
/// A throw-away engine is created, the ROM loaded into it, the movie's header read and judged
/// against that ROM, and the engine dropped before this returns. Nothing the caller holds is
/// touched — which is the point: a host must be able to refuse a movie *before* it switches a
/// player's game to another engine, and switching engines re-powers the machine.
///
/// It answers two questions a header alone cannot: whether this engine can **load the ROM** at
/// all (an `Err(Error::LoadFailed)`, which is about the game and not about the movie), and
/// whether the movie's stated checksum is that ROM's.
///
/// # Errors
///
/// [`Error::AlreadyLive`] when an engine of this kind is already alive — the caller holds one and
/// should ask *it* instead, there being nothing to lose by then. [`Error::LoadFailed`] when the
/// ROM will not load on this engine. Otherwise what [`MovieHeader::refusal`] gives.
pub fn judge_movie(image: &[u8], name: &str, movie: &[u8]) -> Result<MovieHeader, Error> {
    let mut engine = Fceux::new()?;
    engine.load_rom(image, name)?;
    let header = engine.inspect_movie(movie)?;
    match header.refusal() {
        Some(refusal) => Err(refusal),
        None => Ok(header),
    }
}
