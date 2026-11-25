//! Hashing the ROMs of the list in the background (lot GAMEHASH): each file the hash cache has
//! not seen as it is now is read once, on a thread of its own, so that the list can find the game
//! in the game database (its data CRC) without the player resting on it or starting it. The
//! results go into [`super::rom_crc_cache::RomCrcCache`] on the thread that owns it.
//!
//! An archive is listed first: its games and the CRC-32 its directory keeps for each. A game the
//! game database knows by that CRC needs no decompression (its data CRC and MD5 are the
//! database's); the others are read in one pass of the archive. An archive of one game is hashed
//! under its own path, as a file of the disk is; the games of an archive of several, under
//! `<archive>/<entry>` (lot LAUNCH-OUTSIDE).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use super::hack_compat_cache::Certificate;
use super::rom_crc_cache::{RomHashes, Stamp};
use crate::emulation::rom_file;

/// One file hashed: its stamp when it was read, and its hashes (none when it could not be read).
pub(crate) type Hashed = (PathBuf, Option<(Stamp, RomHashes)>);

/// One archive listed: its stamp when it was read, and its games (none when it could not be
/// read).
pub(crate) type Listed = (PathBuf, Option<(Stamp, Vec<String>)>);

/// One ROM studied: its stamp, hashes and certificate (none when it could not be read).
pub(crate) type Studied = (PathBuf, Option<(Stamp, RomHashes, Certificate)>);

/// What the thread sends.
#[derive(Debug)]
enum Message {
    Rom(Hashed),
    Archive(Listed),
    Studied(Studied),
    /// Everything of the queued path `0` was sent.
    Done(PathBuf),
}

/// What came back since the last poll.
#[derive(Debug, Default)]
pub(crate) struct Polled {
    /// The ROMs hashed.
    pub roms: Vec<Hashed>,
    /// The archives listed.
    pub archives: Vec<Listed>,
    /// The ROMs studied.
    pub studied: Vec<Studied>,
}

impl Polled {
    /// Whether nothing came back.
    pub fn is_empty(&self) -> bool {
        self.roms.is_empty() && self.archives.is_empty() && self.studied.is_empty()
    }
}

/// The files being hashed and the results not taken yet.
#[derive(Debug)]
pub struct RomHasher {
    outbox: Sender<Message>,
    inbox: Receiver<Message>,
    pending: HashSet<PathBuf>,
    /// Set while a game plays: the thread waits before its next file (owner request: the
    /// hashing must not take a core from the game).
    paused: Arc<AtomicBool>,
}

impl Default for RomHasher {
    fn default() -> Self {
        let (outbox, inbox) = channel();
        Self {
            outbox,
            inbox,
            pending: HashSet::new(),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// How long the paused thread sleeps before it looks again.
const PAUSE_POLL: std::time::Duration = std::time::Duration::from_millis(200);

impl RomHasher {
    /// Hashes `paths` on a thread, the ones not being hashed already. Nothing is done when no
    /// thread can be started (the files are then hashed when studied or started, as before).
    pub fn queue(&mut self, paths: Vec<PathBuf>) {
        let fresh: Vec<PathBuf> = paths
            .into_iter()
            .filter(|path| !self.pending.contains(path))
            .collect();
        if fresh.is_empty() {
            return;
        }
        let outbox = self.outbox.clone();
        let paused = Arc::clone(&self.paused);
        let work = fresh.clone();
        let spawned = std::thread::Builder::new()
            .name("zaxnes-rom-hash".to_owned())
            .spawn(move || {
                for path in work {
                    while paused.load(Ordering::Relaxed) {
                        std::thread::sleep(PAUSE_POLL);
                    }
                    let sent = if rom_file::is_archive(&path) {
                        hash_archive(&path, &outbox)
                    } else {
                        let hashed = Stamp::of(&path).and_then(|stamp| {
                            let rom = rom_file::read(&path).ok()?;
                            Some((stamp, RomHashes::of(&rom.image)))
                        });
                        outbox.send(Message::Rom((path.clone(), hashed))).is_ok()
                    };
                    if !sent || outbox.send(Message::Done(path)).is_err() {
                        return;
                    }
                }
            });
        if spawned.is_ok() {
            self.pending.extend(fresh);
        }
    }

    /// What was hashed and listed since the last call.
    pub(crate) fn poll(&mut self) -> Polled {
        let mut polled = Polled::default();
        for message in self.inbox.try_iter() {
            match message {
                Message::Rom(hashed) => polled.roms.push(hashed),
                Message::Archive(listed) => polled.archives.push(listed),
                Message::Studied(studied) => polled.studied.push(studied),
                Message::Done(path) => {
                    self.pending.remove(&path);
                }
            }
        }
        polled
    }

    /// Studies the ROM in `path` on a thread of its own, at once (the player rests on it): its
    /// hashes and what widescreen hack fits it. Nothing when it is being studied already.
    pub fn study(&mut self, path: PathBuf) {
        let key = study_key(&path);
        if self.pending.contains(&key) {
            return;
        }
        let outbox = self.outbox.clone();
        let done = key.clone();
        let spawned = std::thread::Builder::new()
            .name("zaxnes-rom-study".to_owned())
            .spawn(move || {
                let studied = Stamp::of(&path).and_then(|stamp| {
                    let rom = rom_file::read(&path).ok()?;
                    let hashes = RomHashes::of(&rom.image);
                    let certificate =
                        crate::emulation::host::certificate_of(hashes.data_crc, &rom.image);
                    Some((stamp, hashes, certificate))
                });
                if outbox.send(Message::Studied((path, studied))).is_ok() {
                    let _ = outbox.send(Message::Done(done));
                }
            });
        if spawned.is_ok() {
            self.pending.insert(key);
        }
    }

    /// Holds the hashing between two files while `paused` (a game plays), lets it go on after.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    /// Whether files are still being hashed.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        !self.pending.is_empty()
    }
}

/// The key a study of `path` is pending under, apart from a hashing of the same file.
fn study_key(path: &Path) -> PathBuf {
    let mut key = path.as_os_str().to_owned();
    key.push("#study");
    PathBuf::from(key)
}

/// Lists the archive `path` and hashes its games; false once the receiver is gone.
fn hash_archive(path: &Path, outbox: &Sender<Message>) -> bool {
    let Some(stamp) = Stamp::of(path) else {
        return outbox
            .send(Message::Archive((path.to_path_buf(), None)))
            .is_ok();
    };
    let Ok(games) = rom_file::games_with_crc(path) else {
        return outbox
            .send(Message::Archive((path.to_path_buf(), None)))
            .is_ok();
    };
    let single = games.len() == 1;
    let target = |entry: &str| {
        if single {
            path.to_path_buf()
        } else {
            rom_file::member_path(path, entry)
        }
    };
    let mut to_read = Vec::new();
    for (entry, crc) in &games {
        match crc.and_then(known_by_the_database) {
            Some(hashes) => {
                if outbox
                    .send(Message::Rom((target(entry), Some((stamp, hashes)))))
                    .is_err()
                {
                    return false;
                }
            }
            None => to_read.push(entry.as_str()),
        }
    }
    if !to_read.is_empty() {
        let read = rom_file::read_games(path, &to_read).unwrap_or_default();
        for entry in to_read {
            let hashed = read
                .iter()
                .find(|(name, _)| name == entry)
                .map(|(_, rom)| (stamp, RomHashes::of(&rom.image)));
            if outbox.send(Message::Rom((target(entry), hashed))).is_err() {
                return false;
            }
        }
    }
    let names = games.into_iter().map(|(entry, _)| entry).collect();
    outbox
        .send(Message::Archive((path.to_path_buf(), Some((stamp, names)))))
        .is_ok()
}

/// The hashes of the file of CRC-32 `file_crc32` when the game database knows it as one image
/// with its MD5: nothing to decompress.
fn known_by_the_database(file_crc32: u32) -> Option<RomHashes> {
    let mut images = crate::gamedb::roms_in_file(file_crc32);
    let rom = images.next()?;
    if images.any(|other| other.data_crc != rom.data_crc) || rom.data_md5.len() != 32 {
        return None;
    }
    let mut data_md5 = [0u8; 16];
    for (n, byte) in data_md5.iter_mut().enumerate() {
        *byte = u8::from_str_radix(rom.data_md5.get(n * 2..n * 2 + 2)?, 16).ok()?;
    }
    Some(RomHashes {
        data_crc: rom.data_crc,
        data_md5,
        file_crc32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(hasher: &mut RomHasher) -> Polled {
        let mut all = Polled::default();
        for _ in 0..500 {
            let polled = hasher.poll();
            all.roms.extend(polled.roms);
            all.archives.extend(polled.archives);
            if !hasher.is_busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        all
    }

    #[test]
    fn every_queued_file_comes_back_once() {
        let folder = std::env::temp_dir().join(format!("zaxnes-rom-hasher-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let rom = folder.join("game.nes");
        std::fs::write(&rom, b"NES\x1a0123456789abDATA").unwrap();
        let missing = folder.join("missing.nes");
        let mut hasher = RomHasher::default();
        hasher.queue(vec![rom.clone(), missing.clone()]);
        hasher.queue(vec![rom.clone()]);
        let done = drain(&mut hasher).roms;
        assert_eq!(2, done.len());
        let found = done.iter().find(|(path, _)| *path == rom).unwrap();
        assert_eq!(
            zax_nes::builtin_patches::crc32(b"DATA"),
            found.1.unwrap().1.data_crc
        );
        assert!(done
            .iter()
            .any(|(path, hashed)| *path == missing && hashed.is_none()));
        drop(std::fs::remove_dir_all(&folder));
    }

    #[test]
    fn a_paused_hasher_waits_until_it_goes_on() {
        let folder =
            std::env::temp_dir().join(format!("zaxnes-rom-hasher-pause-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let rom = folder.join("game.nes");
        std::fs::write(&rom, b"NES0123456789abDATA").unwrap();
        let mut hasher = RomHasher::default();
        hasher.set_paused(true);
        hasher.queue(vec![rom.clone()]);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(hasher.poll().is_empty());
        assert!(hasher.is_busy());
        hasher.set_paused(false);
        assert_eq!(1, drain(&mut hasher).roms.len());
        drop(std::fs::remove_dir_all(&folder));
    }

    #[test]
    fn an_archive_of_several_games_is_listed_and_each_game_hashed() {
        use std::io::Write as _;
        let folder =
            std::env::temp_dir().join(format!("zaxnes-rom-hasher-zip-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let archive = folder.join("pack.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        for (name, data) in [("a.nes", b"AAAA"), ("b.nes", b"BBBB")] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"NES\x1a0123456789ab").unwrap();
            zip.write_all(data).unwrap();
        }
        zip.start_file("readme.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.finish().unwrap();
        let mut hasher = RomHasher::default();
        hasher.queue(vec![archive.clone()]);
        let done = drain(&mut hasher);
        let games = done.archives[0].1.as_ref().unwrap().1.clone();
        assert_eq!(vec!["a.nes".to_owned(), "b.nes".to_owned()], games);
        let member = rom_file::member_path(&archive, "b.nes");
        let found = done.roms.iter().find(|(path, _)| *path == member).unwrap();
        assert_eq!(
            zax_nes::builtin_patches::crc32(b"BBBB"),
            found.1.unwrap().1.data_crc
        );
        drop(std::fs::remove_dir_all(&folder));
    }
}
