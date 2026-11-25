//! Which profile (`.3dn`) and which hack pack (`.zxpk`) of an archive go with one of its games
//! (owner request, 2026-11-25).
//!
//! - A file whose name, without its extension, is a game's of the archive goes with that game
//!   only, and before any other.
//! - A file whose name carries a region or timing tag — `usa`, `europe`, `japan`, `world`, `ntsc`,
//!   `pal`, as whole words (`castlevania-europe-ntsc-widescreen`, `Game (Europe)`; never the `pal`
//!   of `paladin`) — goes only with a game every tag of which it has. A game the game database
//!   does not know has no region: only its timing, read in its header, can agree.
//! - Any other file goes with every game of the archive, after those two.
//!
//! Between files of the same rank, the first in the archive's order.

use super::{base_name, stem_of};

/// The region tags, as the built-in packs (`blastermaster-europe-widescreen`) and No-Intro
/// (`(Europe)`) write them.
const REGIONS: [&str; 4] = ["usa", "europe", "japan", "world"];

/// The timing tags.
const TIMINGS: [&str; 2] = ["ntsc", "pal"];

/// What a game is, as far as the tags go: its No-Intro regions (lowercase) and its timing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RomTraits {
    /// The regions the game database gives its image; empty when it does not know it.
    pub regions: Vec<String>,
    /// `ntsc` or `pal`; none when neither the header nor the regions say.
    pub timing: Option<&'static str>,
}

impl RomTraits {
    /// The traits of the image `image`: its regions from the game database (by its data CRC),
    /// its timing from its header (NES 2.0 timing, else the iNES PAL flag when set), else from
    /// its regions (Europe PAL, USA and Japan NTSC).
    #[must_use]
    pub fn of(image: &[u8]) -> Self {
        let data_crc = zax_nes::builtin_patches::data_crc(image);
        let regions: Vec<String> = crate::gamedb::rom(data_crc)
            .map(|rom| {
                rom.region
                    .split_whitespace()
                    .map(str::to_lowercase)
                    .collect()
            })
            .unwrap_or_default();
        let timing = header_timing(image).or_else(|| {
            let has = |region: &str| regions.iter().any(|known| known == region);
            if has("europe") && !has("usa") && !has("japan") {
                Some("pal")
            } else if has("usa") || has("japan") {
                Some("ntsc")
            } else {
                None
            }
        });
        Self { regions, timing }
    }

    /// Whether the game has the tag `tag`. World stands for every region.
    fn has(&self, tag: &str) -> bool {
        if TIMINGS.contains(&tag) {
            return self.timing == Some(tag);
        }
        self.regions
            .iter()
            .any(|region| region == tag || region == "world")
    }
}

/// The timing the header of `image` says: NES 2.0's own field, or iNES's PAL flag when it is
/// set (an iNES header with the flag clear says nothing, most PAL dumps leave it clear).
fn header_timing(image: &[u8]) -> Option<&'static str> {
    if !image.starts_with(b"NES\x1a") || image.len() < 16 {
        return None;
    }
    if image[7] & 0x0C == 0x08 {
        return match image[12] & 0x03 {
            0 => Some("ntsc"),
            1 => Some("pal"),
            _ => None,
        };
    }
    (image[9] & 0x01 == 1).then_some("pal")
}

/// The words of `stem`, lowercase: what lies between spaces, `-`, `_`, `.`, brackets and
/// parentheses.
fn words(stem: &str) -> Vec<String> {
    stem.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The tags of the file `name`.
fn tags(name: &str) -> Vec<String> {
    words(stem_of(base_name(name)))
        .into_iter()
        .filter(|word| REGIONS.contains(&word.as_str()) || TIMINGS.contains(&word.as_str()))
        .collect()
}

/// The name of an entry without its folders nor its extension, lowercase.
fn key(name: &str) -> String {
    stem_of(base_name(name)).to_lowercase()
}

/// The file of `candidates` (entries of one kind, in the archive's order) that goes with the
/// game `game`, the archive holding the games `games`; none when none does.
#[must_use]
pub fn choose<'a>(
    candidates: &[&'a str],
    game: &str,
    games: &[&str],
    traits: &RomTraits,
) -> Option<&'a str> {
    let own = key(game);
    let game_keys: Vec<String> = games.iter().map(|name| key(name)).collect();
    let rank = |name: &str| -> Option<u8> {
        let name_key = key(name);
        if name_key == own {
            return Some(0);
        }
        // Another game's own file.
        if game_keys.contains(&name_key) {
            return None;
        }
        let tags = tags(name);
        if tags.is_empty() {
            return Some(2);
        }
        tags.iter().all(|tag| traits.has(tag)).then_some(1)
    };
    candidates
        .iter()
        .filter_map(|name| rank(name).map(|rank| (rank, *name)))
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn europe_pal() -> RomTraits {
        RomTraits {
            regions: vec!["europe".to_owned()],
            timing: Some("pal"),
        }
    }

    #[test]
    fn a_file_named_as_the_game_goes_first() {
        let games = ["Game (USA).nes", "Game (Europe).nes"];
        let candidates = ["generic.zxpk", "Game (Europe).zxpk", "Game (USA).zxpk"];
        assert_eq!(
            Some("Game (Europe).zxpk"),
            choose(&candidates, games[1], &games, &europe_pal())
        );
    }

    #[test]
    fn another_game_s_own_file_never_goes() {
        let games = ["Game (USA).nes", "Game (Europe).nes"];
        let candidates = ["Game (USA).3dn"];
        assert_eq!(None, choose(&candidates, games[1], &games, &europe_pal()));
    }

    #[test]
    fn a_tagged_file_goes_with_its_region_only_and_before_a_generic_one() {
        let games = ["Castlevania.nes"];
        let candidates = [
            "generic.zxpk",
            "castlevania-usa-widescreen.zxpk",
            "castlevania-europe-widescreen.zxpk",
        ];
        assert_eq!(
            Some("castlevania-europe-widescreen.zxpk"),
            choose(&candidates, games[0], &games, &europe_pal())
        );
        let usa = RomTraits {
            regions: vec!["usa".to_owned()],
            timing: Some("ntsc"),
        };
        assert_eq!(
            Some("castlevania-usa-widescreen.zxpk"),
            choose(&candidates, games[0], &games, &usa)
        );
    }

    #[test]
    fn every_tag_must_agree() {
        let games = ["Blaster Master.nes"];
        let candidates = ["bm-europe-ntsc.zxpk", "bm-europe.zxpk"];
        assert_eq!(
            Some("bm-europe.zxpk"),
            choose(&candidates, games[0], &games, &europe_pal())
        );
    }

    #[test]
    fn tags_are_whole_words() {
        let games = ["Paladin Quest.nes"];
        let candidates = ["Paladin Quest hack.zxpk"];
        // `paladin` is no `pal`: a generic file, which goes with an NTSC game too.
        let ntsc = RomTraits {
            regions: Vec::new(),
            timing: Some("ntsc"),
        };
        assert_eq!(
            Some("Paladin Quest hack.zxpk"),
            choose(&candidates, games[0], &games, &ntsc)
        );
    }

    #[test]
    fn an_unknown_game_agrees_with_its_timing_only() {
        let games = ["Homebrew.nes"];
        let candidates = ["hb-europe.3dn", "hb-pal.3dn"];
        let unknown = RomTraits {
            regions: Vec::new(),
            timing: Some("pal"),
        };
        assert_eq!(
            Some("hb-pal.3dn"),
            choose(&candidates, games[0], &games, &unknown)
        );
    }

    #[test]
    fn the_header_says_the_timing() {
        let mut image = vec![0u8; 16];
        image[..4].copy_from_slice(b"NES\x1a");
        image[7] = 0x08;
        image[12] = 1;
        assert_eq!(Some("pal"), header_timing(&image));
        image[12] = 0;
        assert_eq!(Some("ntsc"), header_timing(&image));
        image[7] = 0;
        assert_eq!(None, header_timing(&image));
    }
}
