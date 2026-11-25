//! Hashing the ROMs of the list in the background (lot GAMEHASH): each file the hash cache has
//! not seen as it is now is read once, on a thread of its own, so that the list can find the game
//! in the game database (its data CRC) without the player resting on it or starting it. The
//! results go into [`super::rom_crc_cache::RomCrcCache`] on the thread that owns it.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

use super::rom_crc_cache::{RomHashes, Stamp};

/// One file hashed: its stamp when it was read, and its hashes (none when it could not be read).
pub(crate) type Hashed = (PathBuf, Option<(Stamp, RomHashes)>);

/// The files being hashed and the results not taken yet.
#[derive(Debug)]
pub struct RomHasher {
    outbox: Sender<Hashed>,
    inbox: Receiver<Hashed>,
    pending: HashSet<PathBuf>,
}

impl Default for RomHasher {
    fn default() -> Self {
        let (outbox, inbox) = channel();
        Self {
            outbox,
            inbox,
            pending: HashSet::new(),
        }
    }
}

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
        let work = fresh.clone();
        let spawned = std::thread::Builder::new()
            .name("zaxnes-rom-hash".to_owned())
            .spawn(move || {
                for path in work {
                    let hashed = Stamp::of(&path).and_then(|stamp| {
                        let rom = crate::emulation::rom_file::read(&path).ok()?;
                        Some((stamp, RomHashes::of(&rom.image)))
                    });
                    if outbox.send((path, hashed)).is_err() {
                        return;
                    }
                }
            });
        if spawned.is_ok() {
            self.pending.extend(fresh);
        }
    }

    /// The files hashed since the last call.
    pub(crate) fn poll(&mut self) -> Vec<Hashed> {
        let done: Vec<Hashed> = self.inbox.try_iter().collect();
        for (path, _) in &done {
            self.pending.remove(path);
        }
        done
    }

    /// Whether files are still being hashed.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        !self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut done = Vec::new();
        for _ in 0..500 {
            done.extend(hasher.poll());
            if !hasher.is_busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
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
}
