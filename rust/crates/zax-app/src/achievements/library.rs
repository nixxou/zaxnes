//! What RetroAchievements has for each ROM of the list (Rust-only, owner request): the game, its
//! achievements and those the player has, found without playing it. The list rests on a game a
//! moment: its ROM is hashed and the server asked, on a thread of its own, with requests that
//! only read (no session: the player is not seen playing the games the list goes over). Kept in
//! `achievements/games.json` of the user data with the size and the modification time of each
//! file, and the account they are of; a game played brings its entry up to date.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zax_achievements::{HttpRequest, Transport};

use crate::persistence::rom_crc_cache::Stamp;

/// The file of the cache, in the achievements folder.
pub const FILE_NAME: &str = "games.json";

/// `Flags` of an official achievement in the server's answers.
const OFFICIAL_FLAGS: u64 = 3;

/// One achievement of a game, as the list knows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownAchievement {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub points: u32,
    /// The address of its icon.
    pub badge_url: String,
    /// The address of its grey icon.
    pub badge_locked_url: String,
    /// Whether the player has it (softcore or hardcore).
    pub unlocked: bool,
}

/// A game of RetroAchievements and the player's achievements in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownGame {
    /// Its id on the site (0 when it came from a game played, which does not give it).
    pub id: u32,
    pub title: String,
    /// The address of its icon.
    pub icon_url: String,
    /// Its official achievements.
    pub achievements: Vec<KnownAchievement>,
}

impl KnownGame {
    /// How many achievements the player has.
    #[must_use]
    pub fn unlocked(&self) -> usize {
        self.achievements
            .iter()
            .filter(|achievement| achievement.unlocked)
            .count()
    }

    /// The share of the achievements the player has, in percent; `None` for a game without any.
    #[must_use]
    pub fn percent(&self) -> Option<u32> {
        let total = self.achievements.len();
        (total > 0).then(|| {
            u32::try_from(self.unlocked() * 100 / total)
                .unwrap_or(100)
                .min(100)
        })
    }
}

/// One ROM as the JSON holds it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    size: u64,
    seconds: u64,
    nanos: u32,
    /// The account the unlocks are of.
    user: String,
    /// The game; `None` for a ROM RetroAchievements does not know.
    game: Option<KnownGame>,
}

impl Entry {
    fn is_current(&self, stamp: &Stamp, user: &str) -> bool {
        self.size == stamp.size
            && self.seconds == stamp.seconds
            && self.nanos == stamp.nanos
            && self.user == user
    }
}

/// What a study needs to ask the server.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// The server, as `https://retroachievements.org`.
    pub server: String,
    pub username: String,
    pub token: String,
    pub user_agent: String,
}

/// The answer of a study: the game, or `None` for a ROM the server does not know.
type Answer = Result<Option<KnownGame>, String>;

/// The games found, and the studies under way.
pub struct AchievementLibrary {
    file: Option<PathBuf>,
    entries: HashMap<String, Entry>,
    studying: HashSet<String>,
    inbox: Receiver<(String, String, Answer)>,
    outbox: Sender<(String, String, Answer)>,
    /// What went wrong, for the log.
    log: Vec<String>,
}

impl AchievementLibrary {
    /// An empty library, kept nowhere.
    #[must_use]
    pub fn new() -> Self {
        let (outbox, inbox) = channel();
        Self {
            file: None,
            entries: HashMap::new(),
            studying: HashSet::new(),
            inbox,
            outbox,
            log: Vec::new(),
        }
    }

    /// Keeps the library in `directory`, reading what it holds.
    pub fn set_directory(&mut self, directory: Option<&Path>) {
        self.file = directory.map(|directory| directory.join(FILE_NAME));
        self.entries = self
            .file
            .as_ref()
            .and_then(|file| std::fs::read(file).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
    }

    /// What the study of the ROM in `path` found for `user`, while the file is as it was then:
    /// `Some(None)` for a ROM RetroAchievements does not know, `None` when it was not studied.
    #[must_use]
    pub fn game(&self, path: &Path, user: &str) -> Option<Option<&KnownGame>> {
        let entry = self.entries.get(&key(path))?;
        let stamp = Stamp::of(path)?;
        entry
            .is_current(&stamp, user)
            .then_some(entry.game.as_ref())
    }

    /// Whether the ROM in `path` is being studied.
    #[must_use]
    pub fn is_studying(&self, path: &Path) -> bool {
        self.studying.contains(&key(path))
    }

    /// Studies the ROM in `path` for the account of `credentials`, on a thread of its own; the
    /// answer is taken by [`AchievementLibrary::poll`].
    pub fn study(&mut self, path: &Path, credentials: Credentials, transport: Arc<dyn Transport>) {
        let name = key(path);
        if !self.studying.insert(name.clone()) {
            return;
        }
        let outbox = self.outbox.clone();
        let path = path.to_path_buf();
        let user = credentials.username.clone();
        let spawned = std::thread::Builder::new()
            .name("zaxnes-ra-study".to_owned())
            .spawn({
                let (name, user) = (name.clone(), user.clone());
                move || {
                    let answer = study(&path, &credentials, transport.as_ref());
                    let _ = outbox.send((name, user, answer));
                }
            });
        if spawned.is_err() {
            self.studying.remove(&name);
        }
    }

    /// Takes the studies that ended; whether one changed what the list shows.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok((name, user, answer)) = self.inbox.try_recv() {
            self.studying.remove(&name);
            match answer {
                Ok(game) => {
                    if let Some(stamp) = Stamp::of(Path::new(&name)) {
                        self.entries.insert(name, entry(&stamp, user, game));
                        changed = true;
                    }
                }
                // Not kept: the next rest on the game asks again.
                Err(error) => self.log.push(format!("'{name}': {error}")),
            }
        }
        if changed {
            self.save();
        }
        changed
    }

    /// The game being played, as its achievements are now: the entry of its ROM follows.
    pub fn remember(&mut self, path: &Path, user: &str, mut game: KnownGame) {
        let Some(stamp) = Stamp::of(path) else {
            return;
        };
        let name = key(path);
        if let Some(Some(known)) = self.entries.get(&name).map(|entry| &entry.game) {
            // A game played does not give its id; the study's stays.
            if game.id == 0 {
                game.id = known.id;
            }
            if known == &game {
                return;
            }
        }
        self.entries
            .insert(name, entry(&stamp, user.to_owned(), Some(game)));
        self.save();
    }

    /// What went wrong since the last call, for the log.
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }

    fn save(&mut self) {
        let Some(file) = &self.file else {
            return;
        };
        let Ok(json) = serde_json::to_vec_pretty(&self.entries) else {
            return;
        };
        if let Some(directory) = file.parent() {
            let _ = std::fs::create_dir_all(directory);
        }
        if let Err(error) = crate::user_data_paths::write_atomic(file, &json) {
            self.log
                .push(format!("the games could not be kept: {error}"));
        }
    }
}

impl Default for AchievementLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AchievementLibrary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AchievementLibrary")
            .field("entries", &self.entries.len())
            .field("studying", &self.studying.len())
            .finish_non_exhaustive()
    }
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn entry(stamp: &Stamp, user: String, game: Option<KnownGame>) -> Entry {
    Entry {
        size: stamp.size,
        seconds: stamp.seconds,
        nanos: stamp.nanos,
        user,
        game,
    }
}

/// The ROM in `path` as rcheevos hashes a NES game: an iNES header (left out of the hash), the
/// PRG and the CHR data, as the game played is hashed.
fn hash_of(path: &Path) -> Result<String, String> {
    let file = crate::emulation::rom_file::read(path)?;
    let rom =
        zax_nes::rom::RomImage::try_load(&file.image, "").map_err(|error| error.to_string())?;
    let mut image = vec![0x4E, 0x45, 0x53, 0x1A, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    image.extend_from_slice(rom.prg_rom());
    image.extend_from_slice(rom.chr_rom());
    zax_achievements::nes_hash(&image).ok_or_else(|| "no hash".to_owned())
}

/// Asks the server for the game of the ROM in `path` and the player's achievements in it.
fn study(path: &Path, credentials: &Credentials, transport: &dyn Transport) -> Answer {
    let hash = hash_of(path)?;
    let sets = ask(
        credentials,
        transport,
        &format!("r=achievementsets&m={hash}"),
    )?;
    let Some(sets) = sets else {
        return Ok(None);
    };
    let mut game = read_game(&sets).ok_or("an unreadable game")?;
    if game.id == 0 {
        return Ok(None);
    }
    let unlocks = ask(
        credentials,
        transport,
        &format!("r=unlocks&g={}&h=0", game.id),
    )?
    .ok_or("no unlocks")?;
    let unlocked: HashSet<u64> = unlocks
        .get("UserUnlocks")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default();
    for achievement in &mut game.achievements {
        achievement.unlocked = unlocked.contains(&u64::from(achievement.id));
    }
    Ok(Some(game))
}

/// One request to the server: its answer, `None` for a game it does not know.
fn ask(
    credentials: &Credentials,
    transport: &dyn Transport,
    form: &str,
) -> Result<Option<Value>, String> {
    let request = HttpRequest {
        url: format!("{}/dorequest.php", credentials.server.trim_end_matches('/')),
        post_data: Some(format!(
            "{form}&u={}&t={}",
            encode(&credentials.username),
            encode(&credentials.token)
        )),
        content_type: Some("application/x-www-form-urlencoded".to_owned()),
        user_agent: credentials.user_agent.clone(),
    };
    let answer = transport.send(&request);
    let json: Option<Value> = serde_json::from_slice(&answer.body).ok();
    let not_found = answer.status == 404
        || json
            .as_ref()
            .and_then(|json| json.get("Code"))
            .and_then(Value::as_str)
            == Some("not_found");
    if not_found {
        return Ok(None);
    }
    if answer.status != 200 {
        return Err(format!("status {}", answer.status));
    }
    let json = json.ok_or("an unreadable answer")?;
    if json.get("Success").and_then(Value::as_bool) == Some(false) {
        let error = json
            .get("Error")
            .and_then(Value::as_str)
            .unwrap_or("refused");
        return Err(error.to_owned());
    }
    Ok(Some(json))
}

/// The game of an `achievementsets` answer, its official achievements locked.
fn read_game(json: &Value) -> Option<KnownGame> {
    let text = |value: &Value, name: &str| {
        value
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let number = |value: &Value, name: &str| {
        value
            .get(name)
            .and_then(Value::as_u64)
            .and_then(|number| u32::try_from(number).ok())
            .unwrap_or_default()
    };
    let achievements = json
        .get("Sets")?
        .as_array()?
        .iter()
        .filter_map(|set| set.get("Achievements").and_then(Value::as_array))
        .flatten()
        .filter(|achievement| {
            achievement.get("Flags").and_then(Value::as_u64) == Some(OFFICIAL_FLAGS)
        })
        .map(|achievement| KnownAchievement {
            id: number(achievement, "ID"),
            title: text(achievement, "Title"),
            description: text(achievement, "Description"),
            points: number(achievement, "Points"),
            badge_url: text(achievement, "BadgeURL"),
            badge_locked_url: text(achievement, "BadgeLockedURL"),
            unlocked: false,
        })
        .collect();
    Some(KnownGame {
        id: number(json, "GameId"),
        title: text(json, "Title"),
        icon_url: text(json, "ImageIconUrl"),
        achievements,
    })
}

/// `text` as a form value: letters, digits and `-_.~` as they are, the rest `%XX`.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_achievement_set_reads_its_official_achievements() {
        let json: Value = serde_json::from_str(
            r#"{"Success":true,"GameId":7,"Title":"Counter","ImageIconUrl":"https://media/Images/1.png",
               "Sets":[{"Type":"core","Achievements":[
                 {"ID":1,"Title":"One","Description":"d","Points":5,"Flags":3,
                  "BadgeURL":"https://media/Badge/1.png","BadgeLockedURL":"https://media/Badge/1_lock.png"},
                 {"ID":2,"Title":"Two","Description":"d","Points":5,"Flags":5,
                  "BadgeURL":"","BadgeLockedURL":""}]}]}"#,
        )
        .unwrap();
        let game = read_game(&json).unwrap();
        assert_eq!((7, "Counter"), (game.id, game.title.as_str()));
        assert_eq!(1, game.achievements.len());
        assert_eq!(
            "https://media/Badge/1_lock.png",
            game.achievements[0].badge_locked_url
        );
    }

    #[test]
    fn the_percent_counts_the_achievements() {
        let achievement = |unlocked| KnownAchievement {
            id: 1,
            title: String::new(),
            description: String::new(),
            points: 1,
            badge_url: String::new(),
            badge_locked_url: String::new(),
            unlocked,
        };
        let game = KnownGame {
            id: 1,
            title: String::new(),
            icon_url: String::new(),
            achievements: vec![achievement(true), achievement(false), achievement(false)],
        };
        assert_eq!(Some(33), game.percent());
    }

    #[test]
    fn a_form_value_is_encoded() {
        assert_eq!("a%20b%26c", encode("a b&c"));
    }
}
