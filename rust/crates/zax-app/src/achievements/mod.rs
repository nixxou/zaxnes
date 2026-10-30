//! RetroAchievements in the application (Rust-only, owner request), in softcore only: save
//! states, rollback, fast forward and the other conveniences stay allowed, and the unlocks count
//! as softcore ones.
//!
//! The host drives it: the game's ROM when a game starts, each emulated frame once it has
//! ended, a reset, the achievements' progress in and out of save states. What the player sees
//! is read from here by the interfaces: the account, the pop-ups, the challenge and progress
//! indicators, the list of the game's achievements and their icons.

mod badges;
mod library;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use zax_achievements::{
    Achievement, Client, Event, Item, Memory, NoMemory, Summary, Transport, UreqTransport,
};
use zax_nes::NesCore;
use zax_platform::ui_image::PictureUpdate;
use zax_platform::WaveAudio;

pub use badges::{decode_png, BadgeCache, FIRST_BADGE_ID};
pub use library::{AchievementLibrary, KnownAchievement, KnownGame};

use crate::audio::chip_voices;
use crate::ui::{keys, strings};

/// The environment variable that names another server than retroachievements.org, for tests.
pub const HOST_VARIABLE: &str = "ZAXNES_RA_HOST";

/// The folder of the user data the icons and the games studied are kept in.
pub const CACHE_FOLDER: &str = "achievements";

/// How long an unlock stays on screen, in real seconds.
pub const UNLOCK_SECONDS: f64 = 5.0;
/// How long the game's summary stays on screen when it loads.
pub const GAME_SECONDS: f64 = 6.0;
/// How long the progress of an achievement stays on screen after it changed.
pub const PROGRESS_SECONDS: f64 = 2.5;

/// The CPU bus of the console as rcheevos addresses a NES: $0000 to $FFFF, read without side
/// effects.
pub struct CoreMemory<'a>(pub &'a NesCore);

impl Memory for CoreMemory<'_> {
    fn read(&self, address: u32) -> Option<u8> {
        u16::try_from(address)
            .ok()
            .map(|address| self.0.read_memory(address))
    }
}

/// The RetroAchievements account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Account {
    /// Nobody is signed in.
    SignedOut,
    /// A sign-in is under way.
    SigningIn,
    /// Signed in.
    SignedIn {
        /// The name shown.
        name: String,
        /// The softcore points.
        points: u32,
    },
    /// The last sign-in failed, for this reason.
    Failed(String),
}

/// What a pop-up announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    /// An achievement is unlocked.
    Unlocked,
    /// The game's achievements are loaded.
    Game,
    /// Every achievement of the game is unlocked.
    Completed,
}

/// A pop-up of the top of the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Popup {
    /// What it announces.
    pub kind: PopupKind,
    /// The first line.
    pub title: String,
    /// The second line.
    pub text: String,
    /// The address of the icon.
    pub badge_url: String,
    /// Real seconds left on screen.
    pub remaining: f64,
}

/// A message for the notification area of the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The text.
    pub text: String,
    /// Whether it reports an error.
    pub error: bool,
}

/// The game's achievements, as loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStatus {
    /// Its title on the site.
    pub title: String,
    /// The address of its icon.
    pub badge_url: String,
    /// The player's progress.
    pub summary: Summary,
}

/// What the host keeps in the settings after a sign-in, or clears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credentials {
    /// The account name and the token to sign in next time.
    Keep { username: String, token: String },
    /// The token no longer works, or the player signed out.
    Forget,
}

/// RetroAchievements in the application.
#[allow(clippy::struct_excessive_bools)] // independent flags
pub struct AchievementSystem {
    transport: Arc<dyn Transport>,
    product: String,
    client: Option<Client>,
    account: Account,
    encore: bool,
    unofficial: bool,
    /// The ROM of the game being played, as an iNES file, kept for a sign-in that comes later.
    image: Option<Vec<u8>>,
    /// Whether the game of `image` was handed to the client.
    game_requested: bool,
    game: Option<GameStatus>,
    /// The progress of a save state loaded before the game's achievements were (empty: start
    /// them over).
    pending_progress: Option<Vec<u8>>,
    /// The frame counter of the core at the last frame evaluated.
    last_frame: Option<i64>,
    popups: VecDeque<Popup>,
    challenges: Vec<Item>,
    progress: Option<(Item, f64)>,
    notices: Vec<Notice>,
    credentials: Option<Credentials>,
    sound: bool,
    log: Vec<String>,
    badges: RefCell<BadgeCache>,
    /// Whether the last sign-in was with a saved token.
    token_login: bool,
    /// What the list found of each ROM.
    library: AchievementLibrary,
    /// The file of the game being played, whose entry of the library follows its achievements.
    rom_path: Option<PathBuf>,
    /// Whether the game's achievements changed since the library last had them.
    library_stale: bool,
}

impl AchievementSystem {
    /// The system, off, reaching the server over HTTPS as `product` ("Zaxnes/0.3.0").
    #[must_use]
    pub fn new(product: &str) -> Self {
        Self::with_transport(Arc::new(UreqTransport::new()), product)
    }

    /// The system, off, reaching the server through `transport` (a fake one in the tests).
    #[must_use]
    pub fn with_transport(transport: Arc<dyn Transport>, product: &str) -> Self {
        Self {
            badges: RefCell::new(BadgeCache::new(Arc::clone(&transport), product)),
            transport,
            product: product.to_owned(),
            client: None,
            account: Account::SignedOut,
            encore: false,
            unofficial: false,
            image: None,
            game_requested: false,
            game: None,
            pending_progress: None,
            last_frame: None,
            popups: VecDeque::new(),
            challenges: Vec::new(),
            progress: None,
            notices: Vec::new(),
            credentials: None,
            sound: false,
            log: Vec::new(),
            token_login: false,
            library: AchievementLibrary::new(),
            rom_path: None,
            library_stale: false,
        }
    }

    /// Keeps the icons in `directory` (the user data's `achievements` folder), when there is
    /// one: they are downloaded once.
    #[must_use]
    pub fn with_cache(mut self, directory: Option<PathBuf>) -> Self {
        self.library.set_directory(directory.as_deref());
        self.badges.get_mut().set_directory(directory);
        self
    }

    // ------------------------------------------------------------------ configuration

    /// Turns the system on or off. On, it signs in with the saved token when there is one, and
    /// loads the game being played, if any.
    pub fn set_enabled(&mut self, enabled: bool, username: &str, token: &str) {
        if enabled == self.client.is_some() {
            return;
        }
        if !enabled {
            self.client = None;
            self.account = Account::SignedOut;
            self.forget_game();
            return;
        }
        let Some(mut client) = Client::new(Arc::clone(&self.transport), &self.product) else {
            self.log
                .push("rcheevos could not create its client.".to_owned());
            return;
        };
        client.set_options(self.encore, self.unofficial);
        // A private server for testing, instead of retroachievements.org.
        if let Some(host) = std::env::var(HOST_VARIABLE)
            .ok()
            .filter(|host| !host.trim().is_empty())
        {
            client.set_host(host.trim());
        }
        self.client = Some(client);
        self.game_requested = false;
        if !username.is_empty() && !token.is_empty() {
            self.token_login = true;
            self.account = Account::SigningIn;
            if let Some(client) = &mut self.client {
                client.login_with_token(username, token);
            }
        }
    }

    /// Whether the system is on.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.client.is_some()
    }

    /// Encore mode and the unofficial achievements, for the next game loaded.
    pub fn set_options(&mut self, encore: bool, unofficial: bool) {
        self.encore = encore;
        self.unofficial = unofficial;
        if let Some(client) = &mut self.client {
            client.set_options(encore, unofficial);
        }
    }

    /// Signs in with the password, which is not kept: the token of the answer is
    /// ([`AchievementSystem::take_credentials`]).
    pub fn sign_in(&mut self, username: &str, password: &str) {
        let Some(client) = &mut self.client else {
            return;
        };
        if self.account == Account::SigningIn {
            return;
        }
        client.logout();
        self.game_requested = false;
        self.game = None;
        self.token_login = false;
        self.account = Account::SigningIn;
        client.login_with_password(username.trim(), password);
    }

    /// Signs out: the game's achievements go, the saved token too.
    pub fn sign_out(&mut self) {
        if let Some(client) = &mut self.client {
            client.logout();
        }
        self.account = Account::SignedOut;
        self.game = None;
        self.game_requested = false;
        self.clear_indicators();
        self.credentials = Some(Credentials::Forget);
    }

    // ------------------------------------------------------------------ the game

    /// A game starts (a launch, a reset, a save state): its achievements load once signed in.
    /// The same game again keeps them and only starts them over. `image` is the ROM as an
    /// iNES file.
    pub fn start_game(&mut self, core: &NesCore, image: Vec<u8>) {
        if self.image.as_deref() == Some(image.as_slice()) && self.game_requested {
            self.reset(core);
            return;
        }
        self.forget_game();
        self.image = Some(image);
        self.last_frame = Some(core.frame_count());
        self.request_game(core);
    }

    /// The game stops.
    pub fn stop_game(&mut self) {
        self.forget_game();
    }

    /// The file of the game being played: what its achievements become is kept for the list.
    pub fn set_rom_path(&mut self, path: Option<PathBuf>) {
        self.rom_path = path;
    }

    // ------------------------------------------------------------------ the list's games

    /// What RetroAchievements has for the ROM in `path`, as the player signed in has it:
    /// `Some(None)` for a ROM it does not know, `None` when not studied yet (or nobody is
    /// signed in).
    #[must_use]
    pub fn known_game(&self, path: &Path) -> Option<Option<&KnownGame>> {
        let user = self.client.as_ref()?.user()?;
        self.library.game(path, &user.username)
    }

    /// Studies the ROM in `path` (the list rests on it), unless it is known or being studied:
    /// its game and the player's achievements in it come in the background.
    pub fn study_rom(&mut self, path: &Path) {
        if self.known_game(path).is_some() || self.library.is_studying(path) {
            return;
        }
        let Some(client) = &self.client else {
            return;
        };
        let Some(user) = client.user() else {
            return;
        };
        let credentials = library::Credentials {
            server: server(),
            username: user.username,
            token: user.token,
            user_agent: client.user_agent(),
        };
        self.library
            .study(path, credentials, Arc::clone(&self.transport));
    }

    /// The game being played and its official achievements, once they are loaded.
    #[must_use]
    pub fn current_game(&self) -> Option<KnownGame> {
        self.game.as_ref()?;
        let client = self.client.as_ref()?;
        let game = client.game()?;
        let achievements = client
            .achievements()
            .into_iter()
            .filter(|achievement| {
                !matches!(
                    achievement.bucket,
                    zax_achievements::Bucket::Unofficial | zax_achievements::Bucket::Unsupported
                )
            })
            .map(|achievement| KnownAchievement {
                id: achievement.id,
                title: achievement.title,
                description: achievement.description,
                points: achievement.points,
                badge_url: achievement.badge_url,
                badge_locked_url: achievement.badge_locked_url,
                unlocked: achievement.unlocked,
            })
            .collect();
        Some(KnownGame {
            id: game.id,
            title: game.title,
            icon_url: game.badge_url,
            achievements,
        })
    }

    /// The game being played as the library keeps it, once its achievements changed.
    fn update_library(&mut self) {
        if !self.library_stale {
            return;
        }
        let Some(known) = self.current_game() else {
            return;
        };
        let (Some(user), Some(path)) = (
            self.client.as_ref().and_then(Client::user),
            self.rom_path.clone(),
        ) else {
            return;
        };
        self.library_stale = false;
        self.library.remember(&path, &user.username, known);
    }

    /// Whether the ROM in `path` is being studied.
    #[must_use]
    pub fn is_studying(&self, path: &Path) -> bool {
        self.library.is_studying(path)
    }

    /// The console was reset: every achievement starts over.
    pub fn reset(&mut self, core: &NesCore) {
        self.last_frame = Some(core.frame_count());
        if let Some(client) = &mut self.client {
            client.reset(&CoreMemory(core));
        }
        self.clear_indicators();
    }

    /// The progress of the achievements under way, for a save state.
    #[must_use]
    pub fn progress(&self) -> Option<Vec<u8>> {
        self.game.as_ref()?;
        self.client.as_ref()?.progress()
    }

    /// A save state was loaded: `progress` is what it holds, `None` for a state without it
    /// (every achievement then starts over).
    pub fn restore_progress(&mut self, core: &NesCore, progress: Option<&[u8]>) {
        self.last_frame = Some(core.frame_count());
        self.clear_indicators();
        let Some(client) = &mut self.client else {
            return;
        };
        if self.game.is_some() {
            if !client.restore_progress(&CoreMemory(core), progress) {
                client.restore_progress(&CoreMemory(core), None);
            }
        } else if self.game_requested {
            self.pending_progress = Some(progress.map(<[u8]>::to_vec).unwrap_or_default());
        }
    }

    /// Evaluates the frame the core just ended, once: the frame counter tells a new frame
    /// from one already seen, and from a jump back (a rollback, a state loaded), which is only
    /// noted.
    pub fn after_frame(&mut self, core: &NesCore) {
        let frame = core.frame_count();
        let previous = self.last_frame.replace(frame);
        if previous.is_some_and(|previous| frame <= previous) {
            return;
        }
        if self.game.is_none() {
            return;
        }
        if let Some(client) = &mut self.client {
            client.do_frame(&CoreMemory(core));
        }
    }

    /// The work of a displayed frame: the server's answers, the periodic work when no frame
    /// ran, the events, the timers of the pop-ups.
    pub fn tick(&mut self, core: Option<&NesCore>, real_seconds: f64, frames_ran: bool) {
        self.badges.get_mut().poll();
        self.library.poll();
        self.log.extend(self.library.take_log());
        let Some(client) = &mut self.client else {
            return;
        };
        if let Some(core) = core {
            let memory = CoreMemory(core);
            client.pump(&memory);
            if !frames_ran {
                client.idle(&memory);
            }
        } else {
            client.pump(&NoMemory);
            if !frames_ran {
                client.idle(&NoMemory);
            }
        }
        self.log.extend(client.take_log());
        let events = client.take_events();
        for event in events {
            self.on_event(core, event);
        }
        self.update_library();
        self.advance(real_seconds);
    }

    fn request_game(&mut self, core: &NesCore) {
        if self.game_requested || !matches!(self.account, Account::SignedIn { .. }) {
            return;
        }
        let (Some(client), Some(image)) = (&mut self.client, &self.image) else {
            return;
        };
        client.set_options(self.encore, self.unofficial);
        client.load_game(&CoreMemory(core), image);
        self.game_requested = true;
    }

    fn forget_game(&mut self) {
        if let Some(client) = &mut self.client {
            client.unload_game();
        }
        self.image = None;
        self.game = None;
        self.game_requested = false;
        self.pending_progress = None;
        self.last_frame = None;
        self.popups.retain(|popup| popup.kind != PopupKind::Game);
        self.clear_indicators();
        self.badges.get_mut().clear();
    }

    fn clear_indicators(&mut self) {
        self.challenges.clear();
        self.progress = None;
    }

    fn refresh_summary(&mut self) {
        if let (Some(game), Some(client)) = (&mut self.game, &self.client) {
            game.summary = client.summary();
        }
    }

    // ------------------------------------------------------------------ events

    fn on_event(&mut self, core: Option<&NesCore>, event: Event) {
        match event {
            Event::Login(result) => self.on_login(core, result),
            Event::GameLoaded(result) => self.on_game_loaded(core, result),
            Event::AchievementUnlocked(item) => {
                self.refresh_summary();
                self.library_stale = true;
                self.popups.push_back(Popup {
                    kind: PopupKind::Unlocked,
                    title: strings::get(keys::ACH_UNLOCKED),
                    text: unlock_text(&item),
                    badge_url: item.badge_url,
                    remaining: UNLOCK_SECONDS,
                });
                self.sound = true;
                if let Account::SignedIn { points, .. } = &mut self.account {
                    *points += item.number;
                }
            }
            Event::GameCompleted => {
                let title = self
                    .game
                    .as_ref()
                    .map(|game| game.title.clone())
                    .unwrap_or_default();
                self.popups.push_back(Popup {
                    kind: PopupKind::Completed,
                    title: strings::get(keys::ACH_COMPLETED),
                    text: strings::format(&strings::get(keys::ACH_COMPLETED_TEXT), &[&title]),
                    badge_url: self
                        .game
                        .as_ref()
                        .map(|game| game.badge_url.clone())
                        .unwrap_or_default(),
                    remaining: GAME_SECONDS,
                });
            }
            Event::ChallengeShown(item) => {
                self.challenges.retain(|shown| shown.id != item.id);
                self.challenges.push(item);
            }
            Event::ChallengeHidden(item) => self.challenges.retain(|shown| shown.id != item.id),
            Event::ProgressShown(item) => self.progress = Some((item, PROGRESS_SECONDS)),
            Event::ProgressHidden => self.progress = None,
            Event::ServerError(item) => self.notices.push(Notice {
                text: strings::format(&strings::get(keys::ACH_SERVER_ERROR), &[&item.detail]),
                error: true,
            }),
            Event::Disconnected => self.notices.push(Notice {
                text: strings::get(keys::ACH_DISCONNECTED),
                error: true,
            }),
            Event::Reconnected => self.notices.push(Notice {
                text: strings::get(keys::ACH_RECONNECTED),
                error: false,
            }),
            // Leaderboards only run in hardcore, and the reset is asked only when hardcore
            // turns on: neither happens in softcore.
            Event::LeaderboardStarted(_)
            | Event::LeaderboardFailed(_)
            | Event::LeaderboardSubmitted(_)
            | Event::TrackerShown(_)
            | Event::TrackerHidden(_)
            | Event::Scoreboard(_)
            | Event::SubsetCompleted(_)
            | Event::ResetRequested => {}
        }
    }

    fn on_login(&mut self, core: Option<&NesCore>, result: Result<(), String>) {
        match result {
            Ok(()) => {
                let Some(user) = self.client.as_ref().and_then(Client::user) else {
                    return;
                };
                self.account = Account::SignedIn {
                    name: user.display_name.clone(),
                    points: user.score_softcore,
                };
                self.credentials = Some(Credentials::Keep {
                    username: user.username,
                    token: user.token,
                });
                self.notices.push(Notice {
                    text: strings::format(
                        &strings::get(keys::ACH_WELCOME),
                        &[&user.display_name, &user.score_softcore.to_string()],
                    ),
                    error: false,
                });
                if let Some(core) = core {
                    self.request_game(core);
                }
            }
            Err(message) => {
                self.notices.push(Notice {
                    text: strings::format(&strings::get(keys::ACH_SIGN_IN_FAILED), &[&message]),
                    error: true,
                });
                self.account = Account::Failed(message);
                if self.token_login {
                    // A saved token that no longer works: the player signs in again.
                    self.credentials = Some(Credentials::Forget);
                }
            }
        }
    }

    fn on_game_loaded(&mut self, core: Option<&NesCore>, result: Result<(), String>) {
        let Some(client) = &mut self.client else {
            return;
        };
        let game = client.game().filter(|game| game.id != 0);
        let (Ok(()), Some(game)) = (result.clone(), game) else {
            let text = match result {
                Err(message) if !message.is_empty() && !message.contains("Unknown game") => {
                    strings::format(&strings::get(keys::ACH_GAME_FAILED), &[&message])
                }
                _ => strings::get(keys::ACH_GAME_UNKNOWN),
            };
            self.notices.push(Notice { text, error: false });
            return;
        };
        let summary = client.summary();
        self.library_stale = true;
        if let (Some(progress), Some(core)) = (self.pending_progress.take(), core) {
            let progress = (!progress.is_empty()).then_some(progress.as_slice());
            if !client.restore_progress(&CoreMemory(core), progress) {
                client.restore_progress(&CoreMemory(core), None);
            }
        }
        let text = if summary.core == 0 {
            strings::get(keys::ACH_GAME_NO_ACHIEVEMENTS)
        } else {
            strings::format(
                &strings::get(keys::ACH_GAME_SUMMARY),
                &[
                    &summary.unlocked.to_string(),
                    &summary.core.to_string(),
                    &summary.points_unlocked.to_string(),
                    &summary.points_core.to_string(),
                ],
            )
        };
        self.popups.push_back(Popup {
            kind: PopupKind::Game,
            title: game.title.clone(),
            text,
            badge_url: game.badge_url.clone(),
            remaining: GAME_SECONDS,
        });
        self.game = Some(GameStatus {
            title: game.title,
            badge_url: game.badge_url,
            summary,
        });
    }

    fn advance(&mut self, real_seconds: f64) {
        if let Some(popup) = self.popups.front_mut() {
            popup.remaining -= real_seconds;
            if popup.remaining <= 0.0 {
                self.popups.pop_front();
            }
        }
        if let Some((_, remaining)) = &mut self.progress {
            *remaining -= real_seconds;
            if *remaining <= 0.0 {
                self.progress = None;
            }
        }
    }

    // ------------------------------------------------------------------ what is shown

    /// The account.
    #[must_use]
    pub const fn account(&self) -> &Account {
        &self.account
    }

    /// The game's achievements, once loaded.
    #[must_use]
    pub const fn game(&self) -> Option<&GameStatus> {
        self.game.as_ref()
    }

    /// The pop-up on screen: they show one after the other.
    #[must_use]
    pub fn popup(&self) -> Option<&Popup> {
        self.popups.front()
    }

    /// The challenges under way.
    #[must_use]
    pub fn challenges(&self) -> &[Item] {
        &self.challenges
    }

    /// The achievement whose progress changed last, while it shows.
    #[must_use]
    pub fn progress_indicator(&self) -> Option<&Item> {
        self.progress.as_ref().map(|(item, _)| item)
    }

    /// The game's achievements, grouped by progress.
    #[must_use]
    pub fn achievements(&self) -> Vec<Achievement> {
        if self.game.is_none() {
            return Vec::new();
        }
        self.client
            .as_ref()
            .map(Client::achievements)
            .unwrap_or_default()
    }

    /// What the player is doing, as the game's rich presence says.
    #[must_use]
    pub fn rich_presence(&self) -> String {
        self.client
            .as_ref()
            .filter(|_| self.game.is_some())
            .map(Client::rich_presence)
            .unwrap_or_default()
    }

    /// The texture of the icon at `url`, once fetched; asks for it the first time.
    pub fn badge(&self, url: &str) -> Option<usize> {
        self.badges.borrow_mut().texture(url)
    }

    /// The icon textures that changed.
    pub fn take_picture_updates(&mut self) -> Vec<PictureUpdate> {
        self.badges.get_mut().take_updates()
    }

    /// The messages for the notification area.
    pub fn take_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }

    /// What to keep in the settings, after a sign-in or a sign-out.
    pub fn take_credentials(&mut self) -> Option<Credentials> {
        self.credentials.take()
    }

    /// Whether an unlock asks for its sound.
    pub fn take_sound(&mut self) -> bool {
        std::mem::take(&mut self.sound)
    }

    /// rcheevos' log lines.
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }
}

impl std::fmt::Debug for AchievementSystem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AchievementSystem")
            .field("enabled", &self.client.is_some())
            .field("account", &self.account)
            .field("game", &self.game)
            .finish_non_exhaustive()
    }
}

/// The server the requests go to: retroachievements.org, or the one `ZAXNES_RA_HOST` names (a
/// name without a scheme reached over plain HTTP, as rcheevos does).
fn server() -> String {
    match std::env::var(HOST_VARIABLE) {
        Ok(host) if !host.trim().is_empty() => {
            let host = host.trim();
            if host.contains("://") {
                host.to_owned()
            } else {
                format!("http://{host}")
            }
        }
        _ => "https://retroachievements.org".to_owned(),
    }
}

/// The second line of an unlock: the title and the points.
fn unlock_text(item: &Item) -> String {
    strings::format(
        &strings::get(keys::ACH_UNLOCKED_TEXT),
        &[&item.title, &item.number.to_string()],
    )
}

/// The ROM of `core` as rcheevos hashes a NES game: an iNES header (left out of the hash) and
/// the PRG and CHR data. The player's ROM when a built-in patch changed it in memory.
#[must_use]
pub fn hash_image(core: &NesCore) -> Option<Vec<u8>> {
    let mut image = vec![0x4E, 0x45, 0x53, 0x1A, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    if let Some(original) = core.original_image() {
        let rom = zax_nes::rom::RomImage::try_load(original, "").ok()?;
        image.extend_from_slice(rom.prg_rom());
        image.extend_from_slice(rom.chr_rom());
    } else {
        let rom = core.cartridge()?.rom();
        image.extend_from_slice(rom.prg_rom());
        image.extend_from_slice(rom.chr_rom());
    }
    Some(image)
}

/// The jingle of an unlock: a rising arpeggio on a pulse voice, as the console would play it.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // a positive duration times the rate; frames are far below 2^52
pub fn unlock_jingle(sample_rate: u32) -> WaveAudio {
    const NOTES: [(f64, f64); 4] = [(72.0, 0.07), (76.0, 0.07), (79.0, 0.07), (84.0, 0.32)];
    let rate = f64::from(sample_rate.max(8000));
    let mut samples = Vec::new();
    let mut phase = 0.0_f64;
    for (note, seconds) in NOTES {
        let frames = (seconds * rate).round() as usize;
        let step = chip_voices::frequency(note) / rate;
        for frame in 0..frames {
            let time = frame as f64 / rate;
            let level = 0.45 * chip_voices::envelope(time, seconds, 0.002, seconds * 0.6);
            samples.push(level * chip_voices::pulse(phase, 0.25));
            phase += step;
        }
    }
    WaveAudio {
        samples,
        channels: 1,
        sample_rate,
    }
}
