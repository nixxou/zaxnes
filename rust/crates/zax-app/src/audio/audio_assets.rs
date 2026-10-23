//! The interface sounds and the menu music read from the `audio` folder of the assets
//! (SPEC-11 §3.1).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use zax_platform::wave;
use zax_platform::WaveAudio;

use super::ui_sound::{UiAudioLibrary, UiSound};

/// The menu music tracks, in the order of the track indices.
pub const TRACK_FILE_NAMES: [&str; 3] = [
    "happynes.wav",
    "Komiku_-_02_-_Home.wav",
    "dire-space-emergency-chiptune.wav",
];

const SOUND_FILE_NAMES: [(UiSound, &str); 4] = [
    (UiSound::TabChange, "sfx_menu_move3.wav"),
    (UiSound::PageChange, "sfx_menu_move2.wav"),
    (UiSound::Move, "sfx_menu_move1.wav"),
    (UiSound::Confirm, "sfx_coin_double1.wav"),
];

/// The sounds and tracks found in the assets.
#[derive(Debug)]
pub struct AudioAssets {
    sounds: HashMap<UiSound, WaveAudio>,
    directory: PathBuf,
    available_tracks: usize,
}

impl AudioAssets {
    /// Opens the `audio` folder of `asset_root`, reading the interface sounds. A sound that
    /// cannot be read is reported to `log`. Returns `None` when there is no folder or nothing
    /// usable in it, so that the caller falls back to the synthesised sounds.
    pub fn try_open(asset_root: Option<&Path>, mut log: impl FnMut(&str)) -> Option<Self> {
        let directory = asset_root?.join("audio");
        if !directory.is_dir() {
            return None;
        }
        let mut assets = Self {
            sounds: HashMap::new(),
            directory,
            available_tracks: 0,
        };
        for (sound, file) in SOUND_FILE_NAMES {
            let path = assets.directory.join(file);
            if !path.is_file() {
                continue;
            }
            match wave::load(&path) {
                Ok(clip) => {
                    assets.sounds.insert(sound, clip);
                }
                Err(error) => log(&format!("{file}: {error}")),
            }
        }
        assets.available_tracks = TRACK_FILE_NAMES
            .iter()
            .filter(|track| assets.directory.join(track).is_file())
            .count();
        (!assets.sounds.is_empty() || assets.available_tracks > 0).then_some(assets)
    }

    /// The `audio` folder.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// How many of the track files exist.
    #[must_use]
    pub const fn available_tracks(&self) -> usize {
        self.available_tracks
    }
}

impl UiAudioLibrary for AudioAssets {
    fn track_count(&self) -> usize {
        if self.available_tracks > 0 {
            TRACK_FILE_NAMES.len()
        } else {
            0
        }
    }

    fn sound(&self, sound: UiSound) -> Option<&WaveAudio> {
        self.sounds.get(&sound)
    }

    fn load_track(&self, index: usize) -> Option<WaveAudio> {
        let path = self.directory.join(TRACK_FILE_NAMES.get(index)?);
        // An unreadable track is skipped by the mixer, which tries another one.
        wave::load(&path).ok()
    }
}
