//! RetroAchievements for the NES, over rcheevos' client (vendored in `rcheevos/`, MIT).
//!
//! The client lives on one thread, the one that owns the emulated machine. It reads the machine's
//! memory only during the calls that take a [`Memory`] (a frame, a load, a reset), sends its
//! requests on threads of their own through a [`Transport`], and hands their answers back to
//! rcheevos in [`Client::pump`]. What happens is reported as [`Event`]s.
//!
//! The mode is always softcore: save states, rewind and the other conveniences stay allowed,
//! and the unlocks are recorded as softcore ones.

mod ffi;
mod http;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr::{self, NonNull};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

pub use http::{HttpRequest, HttpResponse, Transport, UreqTransport, RETRYABLE_ERROR};

/// The emulated machine's memory as rcheevos addresses it: for a NES, the CPU bus from $0000 to
/// $FFFF.
pub trait Memory {
    /// The byte at `address`, read without side effects; `None` outside the memory.
    fn read(&self, address: u32) -> Option<u8>;
}

/// No memory: the calls made between frames, when no game is lent.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoMemory;

impl Memory for NoMemory {
    fn read(&self, _address: u32) -> Option<u8> {
        None
    }
}

/// The player signed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// The name shown.
    pub display_name: String,
    /// The account name.
    pub username: String,
    /// The token that signs in again without the password.
    pub token: String,
    /// The hardcore points.
    pub score: u32,
    /// The softcore points.
    pub score_softcore: u32,
}

/// The game RetroAchievements identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    /// Its id on the site, 0 for a game the site does not know.
    pub id: u32,
    /// Its title.
    pub title: String,
    /// The hash of the ROM.
    pub hash: String,
    /// The address of its icon.
    pub badge_url: String,
}

/// The player's progress in the game.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// The official achievements.
    pub core: u32,
    /// The unofficial ones, when they are loaded.
    pub unofficial: u32,
    /// Those the player has.
    pub unlocked: u32,
    /// Those this version of rcheevos cannot run.
    pub unsupported: u32,
    /// The points of the official achievements.
    pub points_core: u32,
    /// The points the player has.
    pub points_unlocked: u32,
}

/// The group an achievement is listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    /// Not yet unlocked.
    Locked,
    /// Unlocked.
    Unlocked,
    /// Cannot run with this version of rcheevos.
    Unsupported,
    /// Unofficial.
    Unofficial,
    /// Unlocked during this session.
    RecentlyUnlocked,
    /// A challenge in progress.
    ActiveChallenge,
    /// Close to being unlocked.
    AlmostThere,
    /// Unlocked, the server not told yet.
    Unsynced,
    /// Anything else.
    Other,
}

impl Bucket {
    const fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Locked,
            2 => Self::Unlocked,
            3 => Self::Unsupported,
            4 => Self::Unofficial,
            5 => Self::RecentlyUnlocked,
            6 => Self::ActiveChallenge,
            7 => Self::AlmostThere,
            8 => Self::Unsynced,
            _ => Self::Other,
        }
    }
}

/// One achievement of the game's list.
#[derive(Debug, Clone, PartialEq)]
pub struct Achievement {
    /// The group it is listed in.
    pub bucket: Bucket,
    /// The title of the group, as rcheevos writes it (English).
    pub bucket_label: String,
    /// Its id.
    pub id: u32,
    /// Its points.
    pub points: u32,
    /// Its title.
    pub title: String,
    /// What to do.
    pub description: String,
    /// The progress measured, as "12/50", or empty.
    pub progress: String,
    /// The progress measured, from 0 to 100.
    pub percent: f32,
    /// Whether the player has it (softcore or hardcore).
    pub unlocked: bool,
    /// The share of players who have it, in percent.
    pub rarity: f32,
    /// The address of its icon.
    pub badge_url: String,
    /// The address of its grey icon.
    pub badge_locked_url: String,
}

/// An achievement or a leaderboard, as an event carries it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Item {
    /// Its id.
    pub id: u32,
    /// Its points, or the rank of a scoreboard.
    pub number: u32,
    /// Its title.
    pub title: String,
    /// Its description.
    pub description: String,
    /// The progress, the tracker's value, the score submitted or the server's message.
    pub detail: String,
    /// The address of its icon.
    pub badge_url: String,
}

/// What the client reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The sign-in ended: `Ok` with the player in [`Client::user`], or the server's message.
    Login(Result<(), String>),
    /// The game's achievements are loaded, or the reason they are not.
    GameLoaded(Result<(), String>),
    /// An achievement is unlocked.
    AchievementUnlocked(Item),
    /// A challenge achievement is under way: its icon stays on screen.
    ChallengeShown(Item),
    /// The challenge ended.
    ChallengeHidden(Item),
    /// The progress of an achievement changed.
    ProgressShown(Item),
    /// The progress indicator goes away.
    ProgressHidden,
    /// A leaderboard attempt started.
    LeaderboardStarted(Item),
    /// A leaderboard attempt failed.
    LeaderboardFailed(Item),
    /// A leaderboard attempt was submitted.
    LeaderboardSubmitted(Item),
    /// A leaderboard tracker appears or changes.
    TrackerShown(Item),
    /// A leaderboard tracker goes away.
    TrackerHidden(Item),
    /// The rank of a leaderboard submission.
    Scoreboard(Item),
    /// Every achievement of the game is unlocked.
    GameCompleted,
    /// Every achievement of a subset is unlocked.
    SubsetCompleted(Item),
    /// The server refused a request for good.
    ServerError(Item),
    /// An unlock could not reach the server; it is kept and sent again.
    Disconnected,
    /// The kept unlocks reached the server.
    Reconnected,
    /// rcheevos asks for a reset of the machine (only when hardcore turns on, never here).
    ResetRequested,
}

/// The memory lent for one call: a thin pointer and the function that reads through it.
#[derive(Clone, Copy)]
struct MemoryRef {
    data: *const (),
    read: unsafe fn(*const (), u32) -> Option<u8>,
}

/// Reads through a `MemoryRef` of a `M`.
///
/// # Safety
///
/// `data` points to a live `M`.
unsafe fn read_through<M: Memory>(data: *const (), address: u32) -> Option<u8> {
    // SAFETY: the caller guarantees `data` points to a live `M`.
    unsafe { &*data.cast::<M>() }.read(address)
}

/// What the C side calls back into: interior mutability only, since it is reached through a
/// shared pointer while the client runs.
#[derive(Default)]
struct Host {
    memory: Cell<Option<MemoryRef>>,
    requests: RefCell<Vec<(*mut c_void, HttpRequest)>>,
    events: RefCell<Vec<Event>>,
    log: RefCell<Vec<String>>,
    user_agent: RefCell<String>,
}

/// The RetroAchievements client.
pub struct Client {
    raw: NonNull<ffi::ZaxRa>,
    host: NonNull<Host>,
    transport: Arc<dyn Transport>,
    answers: Receiver<(u64, HttpResponse)>,
    outbox: Sender<(u64, HttpResponse)>,
    pending: HashMap<u64, *mut c_void>,
    next_request: u64,
}

impl Client {
    /// A client that sends its requests through `transport`, as `product` (the `User-Agent`'s
    /// first part, as "Zaxnes/0.3.0"). `None` when rcheevos could not allocate it.
    #[must_use]
    pub fn new(transport: Arc<dyn Transport>, product: &str) -> Option<Self> {
        let host = NonNull::from(Box::leak(Box::<Host>::default()));
        // SAFETY: the callbacks match the C layer's types; `host` lives until `Drop`, after the
        // client is destroyed.
        let raw = unsafe {
            ffi::zax_ra_create(
                host.as_ptr().cast(),
                read_memory,
                send_request,
                receive_event,
                receive_log,
                // RC_CLIENT_LOG_LEVEL_INFO
                3,
            )
        };
        let Some(raw) = NonNull::new(raw) else {
            // SAFETY: `host` came from `Box::leak` above and nothing else holds it.
            drop(unsafe { Box::from_raw(host.as_ptr()) });
            return None;
        };
        let (outbox, answers) = channel();
        let client = Self {
            raw,
            host,
            transport,
            answers,
            outbox,
            pending: HashMap::new(),
            next_request: 0,
        };
        let mut clause = [0 as c_char; 64];
        // SAFETY: the buffer is writable for its whole length.
        let length =
            unsafe { ffi::zax_ra_user_agent_clause(raw.as_ptr(), clause.as_mut_ptr(), 64) };
        let clause = if length > 0 {
            text(clause.as_ptr())
        } else {
            String::new()
        };
        format!("{product} {clause}")
            .trim()
            .clone_into(&mut client.host().user_agent.borrow_mut());
        // Softcore, always.
        // SAFETY: `raw` is live.
        unsafe { ffi::zax_ra_set_modes(raw.as_ptr(), 0, 0, 0) };
        Some(client)
    }

    const fn host(&self) -> &Host {
        // SAFETY: `host` lives as long as the client, and is only ever reached through shared
        // references.
        unsafe { self.host.as_ref() }
    }

    /// Talks to another server than retroachievements.org (`http://localhost:8080`, a private
    /// test server); a name without a scheme is reached over plain HTTP.
    pub fn set_host(&mut self, host: &str) {
        if let Ok(host) = CString::new(host) {
            // SAFETY: `raw` is live; rcheevos copies the name.
            unsafe { ffi::zax_ra_set_host(self.raw.as_ptr(), host.as_ptr()) };
        }
    }

    /// Whether the next game loads with encore mode (the unlocked achievements fire again) and
    /// with the unofficial achievements. Read when a game loads.
    pub fn set_options(&mut self, encore: bool, unofficial: bool) {
        // SAFETY: `raw` is live.
        unsafe {
            ffi::zax_ra_set_modes(
                self.raw.as_ptr(),
                0,
                c_int::from(encore),
                c_int::from(unofficial),
            );
        };
    }

    /// Signs in with the password; the answer comes as [`Event::Login`]. The password is not
    /// kept: the token of [`Client::user`] signs in next time.
    pub fn login_with_password(&mut self, username: &str, password: &str) {
        let (Ok(username), Ok(password)) = (CString::new(username), CString::new(password)) else {
            self.host()
                .events
                .borrow_mut()
                .push(Event::Login(Err("Invalid characters.".to_owned())));
            return;
        };
        // SAFETY: `raw` is live; the strings outlive the call, which copies them.
        unsafe {
            ffi::zax_ra_login_password(self.raw.as_ptr(), username.as_ptr(), password.as_ptr());
        };
        self.dispatch();
    }

    /// Signs in with a token saved from an earlier sign-in; the answer comes as
    /// [`Event::Login`].
    pub fn login_with_token(&mut self, username: &str, token: &str) {
        let (Ok(username), Ok(token)) = (CString::new(username), CString::new(token)) else {
            self.host()
                .events
                .borrow_mut()
                .push(Event::Login(Err("Invalid characters.".to_owned())));
            return;
        };
        // SAFETY: `raw` is live; the strings outlive the call, which copies them.
        unsafe { ffi::zax_ra_login_token(self.raw.as_ptr(), username.as_ptr(), token.as_ptr()) };
        self.dispatch();
    }

    /// Signs out; the game loaded, if any, is unloaded.
    pub fn logout(&mut self) {
        // SAFETY: `raw` is live.
        unsafe { ffi::zax_ra_logout(self.raw.as_ptr()) };
    }

    /// The player signed in.
    #[must_use]
    pub fn user(&self) -> Option<User> {
        let mut display_name = ptr::null();
        let mut username = ptr::null();
        let mut token = ptr::null();
        let mut score = 0;
        let mut score_softcore = 0;
        // SAFETY: `raw` is live and every out-pointer is writable.
        let found = unsafe {
            ffi::zax_ra_user(
                self.raw.as_ptr(),
                &raw mut display_name,
                &raw mut username,
                &raw mut token,
                &raw mut score,
                &raw mut score_softcore,
            )
        };
        (found != 0).then(|| User {
            display_name: text(display_name),
            username: text(username),
            token: text(token),
            score,
            score_softcore,
        })
    }

    /// Identifies the game by its ROM (`image`: an iNES file) and loads its achievements; the
    /// answer comes as [`Event::GameLoaded`]. Waits for a sign-in in progress.
    pub fn load_game<M: Memory>(&mut self, memory: &M, image: &[u8]) {
        let raw = self.raw.as_ptr();
        self.with_memory(memory, |_| {
            // SAFETY: `raw` is live; rcheevos hashes the image during the call.
            unsafe { ffi::zax_ra_load_game(raw, image.as_ptr(), image.len(), c"".as_ptr()) };
        });
        self.dispatch();
    }

    /// Unloads the game.
    pub fn unload_game(&mut self) {
        // SAFETY: `raw` is live.
        unsafe { ffi::zax_ra_unload_game(self.raw.as_ptr()) };
    }

    /// Whether a game load is under way.
    #[must_use]
    pub fn is_loading_game(&self) -> bool {
        // SAFETY: `raw` is live.
        let state = unsafe { ffi::zax_ra_load_game_state(self.raw.as_ptr()) };
        // AWAIT_LOGIN, IDENTIFYING_GAME, FETCHING_GAME_DATA, STARTING_SESSION
        (1..=4).contains(&state)
    }

    /// The game loaded (a game the site does not know has the id 0).
    #[must_use]
    pub fn game(&self) -> Option<Game> {
        let mut id = 0;
        let mut title = ptr::null();
        let mut hash = ptr::null();
        let mut badge_url = ptr::null();
        // SAFETY: `raw` is live and every out-pointer is writable.
        let found = unsafe {
            ffi::zax_ra_game(
                self.raw.as_ptr(),
                &raw mut id,
                &raw mut title,
                &raw mut hash,
                &raw mut badge_url,
            )
        };
        (found != 0).then(|| Game {
            id,
            title: text(title),
            hash: text(hash),
            badge_url: text(badge_url),
        })
    }

    /// The player's progress in the game loaded.
    #[must_use]
    pub fn summary(&self) -> Summary {
        let mut summary = Summary::default();
        // SAFETY: `raw` is live and every out-pointer is writable.
        unsafe {
            ffi::zax_ra_summary(
                self.raw.as_ptr(),
                &raw mut summary.core,
                &raw mut summary.unofficial,
                &raw mut summary.unlocked,
                &raw mut summary.unsupported,
                &raw mut summary.points_core,
                &raw mut summary.points_unlocked,
            );
        };
        summary
    }

    /// Evaluates the achievements on the frame that just ended.
    pub fn do_frame<M: Memory>(&mut self, memory: &M) {
        let raw = self.raw.as_ptr();
        // SAFETY: `raw` is live.
        self.with_memory(memory, |_| unsafe { ffi::zax_ra_do_frame(raw) });
        self.dispatch();
    }

    /// The periodic work while no frame runs (pause, menus): the pings and the retries.
    pub fn idle<M: Memory>(&mut self, memory: &M) {
        let raw = self.raw.as_ptr();
        // SAFETY: `raw` is live.
        self.with_memory(memory, |_| unsafe { ffi::zax_ra_idle(raw) });
        self.dispatch();
    }

    /// The machine was reset: every achievement starts over.
    pub fn reset<M: Memory>(&mut self, memory: &M) {
        let raw = self.raw.as_ptr();
        // SAFETY: `raw` is live.
        self.with_memory(memory, |_| unsafe { ffi::zax_ra_reset(raw) });
    }

    /// The state of the achievements under way (hit counts, measured values), for a save
    /// state; `None` without a game.
    #[must_use]
    pub fn progress(&self) -> Option<Vec<u8>> {
        // SAFETY: `raw` is live.
        let size = unsafe { ffi::zax_ra_progress_size(self.raw.as_ptr()) };
        if size == 0 {
            return None;
        }
        let mut buffer = vec![0; size];
        // SAFETY: the buffer is writable for `size` bytes.
        let result = unsafe {
            ffi::zax_ra_serialize_progress(self.raw.as_ptr(), buffer.as_mut_ptr(), buffer.len())
        };
        (result == ffi::RC_OK).then_some(buffer)
    }

    /// Puts back the state of [`Client::progress`] after a save state loads; `None` (a state
    /// without it) starts every achievement over. Returns whether rcheevos took it.
    pub fn restore_progress<M: Memory>(&mut self, memory: &M, progress: Option<&[u8]>) -> bool {
        let raw = self.raw.as_ptr();
        let (data, size) = progress.map_or((ptr::null(), 0), |data| (data.as_ptr(), data.len()));
        self.with_memory(memory, |_| {
            // SAFETY: `raw` is live; `data` is readable for `size` bytes, or null.
            unsafe { ffi::zax_ra_deserialize_progress(raw, data, size) == ffi::RC_OK }
        })
    }

    /// The rich presence text of the game ("Level 3, 2 lives"), or empty.
    #[must_use]
    pub fn rich_presence(&self) -> String {
        let mut buffer = [0 as c_char; 256];
        // SAFETY: the buffer is writable for its whole length.
        let length = unsafe {
            ffi::zax_ra_rich_presence(self.raw.as_ptr(), buffer.as_mut_ptr(), buffer.len())
        };
        if length == 0 {
            String::new()
        } else {
            text(buffer.as_ptr())
        }
    }

    /// The achievements of the game loaded, grouped by progress, the unofficial ones too when
    /// they are loaded.
    #[must_use]
    pub fn achievements(&self) -> Vec<Achievement> {
        let mut list: Vec<Achievement> = Vec::new();
        // SAFETY: `raw` is live; `list` outlives the call, which is the only one to use it.
        unsafe {
            ffi::zax_ra_list(
                self.raw.as_ptr(),
                // RC_CLIENT_ACHIEVEMENT_CATEGORY_CORE_AND_UNOFFICIAL
                3,
                visit_entry,
                ptr::from_mut(&mut list).cast(),
            );
        };
        list
    }

    /// Hands the server's answers that arrived to rcheevos, and sends what that asks for.
    pub fn pump<M: Memory>(&mut self, memory: &M) {
        let answers: Vec<_> = self.answers.try_iter().collect();
        if answers.is_empty() {
            return;
        }
        self.with_memory(memory, |client| {
            for (id, answer) in answers {
                let Some(pending) = client.pending.remove(&id) else {
                    continue;
                };
                // SAFETY: `pending` came from the C layer's request and is completed once; the
                // body is readable for its length during the call.
                unsafe {
                    ffi::zax_ra_complete(
                        pending,
                        answer.body.as_ptr().cast(),
                        answer.body.len(),
                        answer.status,
                    );
                };
            }
        });
        self.dispatch();
    }

    /// Whether requests are waiting for their answer.
    #[must_use]
    pub fn has_pending_requests(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Takes the events raised since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut *self.host().events.borrow_mut())
    }

    /// Takes rcheevos' log lines since the last call.
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.host().log.borrow_mut())
    }

    /// Lends `memory` to the calls `work` makes into rcheevos.
    fn with_memory<M: Memory, R>(&mut self, memory: &M, work: impl FnOnce(&mut Self) -> R) -> R {
        struct Lend<'a>(&'a Cell<Option<MemoryRef>>);
        impl Drop for Lend<'_> {
            fn drop(&mut self) {
                self.0.set(None);
            }
        }
        // SAFETY: the pointer is only read while `memory` is borrowed: the lend ends with the
        // guard, before this function returns.
        let host = unsafe { self.host.as_ref() };
        host.memory.set(Some(MemoryRef {
            data: ptr::from_ref(memory).cast(),
            read: read_through::<M>,
        }));
        let _lend = Lend(&host.memory);
        work(self)
    }

    /// Sends the requests rcheevos made, each on a thread of its own.
    fn dispatch(&mut self) {
        let requests = std::mem::take(&mut *self.host().requests.borrow_mut());
        for (pending, request) in requests {
            let id = self.next_request;
            self.next_request += 1;
            self.pending.insert(id, pending);
            let transport = Arc::clone(&self.transport);
            let outbox = self.outbox.clone();
            let spawned = std::thread::Builder::new()
                .name("zaxnes-achievements".to_owned())
                .spawn(move || {
                    let answer = transport.send(&request);
                    // The client may be gone; the answer is then dropped.
                    let _ = outbox.send((id, answer));
                });
            if spawned.is_err() {
                let _ = self.outbox.send((id, HttpResponse::unreachable()));
            }
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        for (_, pending) in self.pending.drain() {
            // SAFETY: each pending request is freed once, without its callback: the client
            // that would receive it is going.
            unsafe { ffi::zax_ra_discard(pending) };
        }
        // SAFETY: `raw` is live and destroyed once.
        unsafe { ffi::zax_ra_destroy(self.raw.as_ptr()) };
        for (pending, _) in self.host().requests.borrow_mut().drain(..) {
            // SAFETY: as above, for the requests made during the destruction.
            unsafe { ffi::zax_ra_discard(pending) };
        }
        // SAFETY: `host` came from `Box::leak` in `new`, and the C side is gone.
        drop(unsafe { Box::from_raw(self.host.as_ptr()) });
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Client")
            .field("pending", &self.pending.len())
            .finish_non_exhaustive()
    }
}

/// A C string as an owned one; empty for null.
fn text(raw: *const c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    // SAFETY: rcheevos hands out NUL-terminated strings that live during the call.
    unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned()
}

/// The host behind the C layer's pointer.
///
/// # Safety
///
/// `host` is the pointer given to `zax_ra_create`, alive.
const unsafe fn host_of<'a>(host: *mut c_void) -> &'a Host {
    // SAFETY: guaranteed by the caller.
    unsafe { &*host.cast::<Host>() }
}

unsafe extern "C" fn read_memory(
    host: *mut c_void,
    address: u32,
    buffer: *mut u8,
    count: u32,
) -> u32 {
    // SAFETY: the C layer passes back the host pointer it was given.
    let host = unsafe { host_of(host) };
    let Some(memory) = host.memory.get() else {
        return 0;
    };
    for index in 0..count {
        // SAFETY: the memory is lent for the duration of the call (`with_memory`).
        let Some(byte) = (unsafe { (memory.read)(memory.data, address.wrapping_add(index)) })
        else {
            return index;
        };
        // SAFETY: rcheevos' buffer holds `count` bytes.
        unsafe { buffer.add(index as usize).write(byte) };
    }
    count
}

unsafe extern "C" fn send_request(
    host: *mut c_void,
    pending: *mut c_void,
    url: *const c_char,
    post_data: *const c_char,
    content_type: *const c_char,
) {
    // SAFETY: the C layer passes back the host pointer it was given.
    let host = unsafe { host_of(host) };
    let request = HttpRequest {
        url: text(url),
        post_data: (!post_data.is_null()).then(|| text(post_data)),
        content_type: (!content_type.is_null()).then(|| text(content_type)),
        user_agent: host.user_agent.borrow().clone(),
    };
    host.requests.borrow_mut().push((pending, request));
}

unsafe extern "C" fn receive_event(host: *mut c_void, raw: *const ffi::RawEvent) {
    // SAFETY: the C layer passes back the host pointer it was given, and a live record.
    let (host, raw) = unsafe { (host_of(host), &*raw) };
    let item = Item {
        id: raw.id,
        number: raw.number,
        title: text(raw.title),
        description: text(raw.description),
        detail: text(raw.detail),
        badge_url: text(raw.badge_url),
    };
    let outcome = |message: String| {
        if raw.result == ffi::RC_OK {
            Ok(())
        } else if message.is_empty() {
            Err(format!("Error {}.", raw.result))
        } else {
            Err(message)
        }
    };
    let event = match raw.kind {
        ffi::EVENT_LOGIN => Event::Login(outcome(item.detail)),
        ffi::EVENT_GAME_LOADED => Event::GameLoaded(outcome(item.detail)),
        1 => Event::AchievementUnlocked(item),
        2 => Event::LeaderboardStarted(item),
        3 => Event::LeaderboardFailed(item),
        4 => Event::LeaderboardSubmitted(item),
        5 => Event::ChallengeShown(item),
        6 => Event::ChallengeHidden(item),
        7 | 9 => Event::ProgressShown(item),
        8 => Event::ProgressHidden,
        10 | 12 => Event::TrackerShown(item),
        11 => Event::TrackerHidden(item),
        13 => Event::Scoreboard(item),
        14 => Event::ResetRequested,
        15 => Event::GameCompleted,
        16 => Event::ServerError(item),
        17 => Event::Disconnected,
        18 => Event::Reconnected,
        19 => Event::SubsetCompleted(item),
        _ => return,
    };
    host.events.borrow_mut().push(event);
}

unsafe extern "C" fn receive_log(host: *mut c_void, message: *const c_char) {
    // SAFETY: the C layer passes back the host pointer it was given.
    let host = unsafe { host_of(host) };
    host.log.borrow_mut().push(text(message));
}

unsafe extern "C" fn visit_entry(context: *mut c_void, raw: *const ffi::RawEntry) {
    // SAFETY: `context` is the list `Client::achievements` passed, and the record is live.
    let (list, raw) = unsafe { (&mut *context.cast::<Vec<Achievement>>(), &*raw) };
    list.push(Achievement {
        bucket: Bucket::from_raw(raw.bucket),
        bucket_label: text(raw.bucket_label),
        id: raw.id,
        points: raw.points,
        title: text(raw.title),
        description: text(raw.description),
        progress: text(raw.progress),
        percent: raw.percent,
        unlocked: raw.unlocked != 0,
        rarity: raw.rarity,
        badge_url: text(raw.badge_url),
        badge_locked_url: text(raw.badge_locked_url),
    });
}
