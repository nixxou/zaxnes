//! The tiles of a "3dh" profile and their resolution (AMD-062 REQ-PROF-078, REQ-PROF-080 to
//! REQ-PROF-082): which manifest strings are still waiting for their sixteen bytes, how a ROM or
//! a running game gives them, and how the patterns that hold them are rebuilt.
//!
//! Resolution is monotone: a string that has its bytes never loses them for the life of the
//! profile. The set of resolved strings is session knowledge, like the cache — no savestate and
//! no rollback snapshot carries it (REQ-PROF-080).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::enums::TileFlip;
use crate::hash::envelope::HashProfileManifest;
use crate::hash::raw_tile::{self, RawTile};
use crate::hash::tile_id::{self, TileId};
use crate::pattern::ProfilePattern;
use crate::profile::GameProfile;
use crate::tile::{PatternGrid, ProfileTile};

/// The four mirrorings of REQ-PROF-079, in flip-bit order.
const FLIPS: [TileFlip; 4] = [
    TileFlip::NONE,
    TileFlip::HORIZONTAL,
    TileFlip::VERTICAL,
    TileFlip::from_bits(TileFlip::HORIZONTAL.bits() | TileFlip::VERTICAL.bits()),
];

/// Which manifest strings of a "3dh" profile have their sixteen bytes, and which are still
/// waiting (REQ-PROF-081).
#[derive(Debug, Clone, Default)]
pub struct TileResolver {
    /// The strings still waiting.
    waiting: HashSet<Arc<str>>,
    /// How many waiting strings have each identifier prefix, the fast filter of REQ-PROF-077. A
    /// legacy digest has no prefix and is not counted here.
    waiting_prefixes: HashMap<u32, usize>,
    /// How many waiting strings are legacy digests, which have to be hashed in full
    /// (REQ-PROF-076).
    waiting_digests: usize,
    /// The strings that have their bytes.
    resolved: HashMap<Arc<str>, RawTile>,
}

impl TileResolver {
    /// A resolver over `manifest`, every string waiting.
    #[must_use]
    pub fn of(manifest: &HashProfileManifest) -> Self {
        let mut resolver = Self::default();
        for key in manifest.keys() {
            let key: Arc<str> = Arc::from(key.as_str());
            if let Some(id) = TileId::parse_text(&key) {
                *resolver.waiting_prefixes.entry(id.prefix()).or_insert(0) += 1;
            } else {
                resolver.waiting_digests += 1;
            }
            resolver.waiting.insert(key);
        }

        resolver
    }

    /// Number of strings still waiting.
    #[must_use]
    pub fn waiting_count(&self) -> usize {
        self.waiting.len()
    }

    /// Number of strings that have their bytes.
    #[must_use]
    pub fn resolved_count(&self) -> usize {
        self.resolved.len()
    }

    /// True when nothing is waiting any more, which is when every path of REQ-PROF-080 may stop
    /// costing anything.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.waiting.is_empty()
    }

    /// The strings still waiting, in ordinal order.
    #[must_use]
    pub fn waiting_keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.waiting.iter().map(|key| &**key).collect();
        keys.sort_unstable();
        keys
    }

    /// The sixteen bytes of a resolved string, or none while it waits.
    #[must_use]
    pub fn bytes_of(&self, key: &str) -> Option<&RawTile> {
        self.resolved.get(key)
    }

    /// Every resolved string with its bytes, in ordinal order, which is what the cache writes
    /// (REQ-PROF-082).
    #[must_use]
    pub fn entries(&self) -> Vec<(&str, &RawTile)> {
        let mut entries: Vec<(&str, &RawTile)> = self
            .resolved
            .iter()
            .map(|(key, raw)| (&**key, raw))
            .collect();
        entries.sort_unstable_by_key(|(key, _)| *key);
        entries
    }

    /// Confronts the four mirrorings of one sixteen-byte window with the strings still waiting
    /// (REQ-PROF-078 point 2): a string whose identifier — or whose legacy digest — is one of the
    /// four receives the mirrored bytes. Returns how many strings this resolved.
    ///
    /// The mixing prefix is computed from the planes alone, so a window whose prefix no waiting
    /// string carries costs four multiplications and no digest at all.
    pub fn offer(&mut self, window: &RawTile) -> usize {
        if self.waiting.is_empty() {
            return 0;
        }

        let (low, high) = raw_tile::to_planes(window);
        let mut count = 0;
        for flip in FLIPS {
            let (low, high) = raw_tile::flipped_planes(low, high, flip);
            let prefix = tile_id::compute_prefix(low, high);
            let known_prefix = self.waiting_prefixes.contains_key(&prefix);
            if !known_prefix && self.waiting_digests == 0 {
                continue;
            }

            let mirrored = raw_tile::from_planes(low, high);
            if known_prefix {
                let id = TileId::compute_with_prefix(&mirrored, prefix);
                count += usize::from(self.take(&id.to_text(), &mirrored));
            }
            if self.waiting_digests > 0 {
                let digest = raw_tile::legacy_digest(&mirrored);
                count += usize::from(self.take(&digest, &mirrored));
            }
        }

        count
    }

    /// Scans the PRG zone then the CHR zone of an iNES file at every byte position, and stops as
    /// soon as nothing is waiting (REQ-PROF-078). A file that is not iNES resolves nothing.
    /// Returns how many strings this resolved.
    pub fn resolve_from_rom(&mut self, file: &[u8]) -> usize {
        let Some(zones) = crate::hash::rom_zones::RomZones::of(file) else {
            return 0;
        };

        let mut count = 0;
        for zone in zones.in_scan_order() {
            let Some(bytes) = file.get(zone) else {
                continue;
            };
            for window in bytes.windows(raw_tile::BYTE_COUNT) {
                if self.waiting.is_empty() {
                    return count;
                }
                let mut raw = [0_u8; raw_tile::BYTE_COUNT];
                raw.copy_from_slice(window);
                count += self.offer(&raw);
            }
        }

        count
    }

    /// Applies the entries of a cache (REQ-PROF-082): a pair whose string is not waiting, or
    /// whose string does not recompute from its bytes, is ignored. Returns how many strings this
    /// resolved.
    pub fn apply_cache<'a>(
        &mut self,
        entries: impl IntoIterator<Item = (&'a str, RawTile)>,
    ) -> usize {
        let mut count = 0;
        for (key, raw) in entries {
            if recomputes(key, &raw) {
                count += usize::from(self.take(key, &raw));
            }
        }

        count
    }

    /// Gives `key` its bytes when it is waiting. False when it was not.
    fn take(&mut self, key: &str, raw: &RawTile) -> bool {
        let Some(shared) = self.waiting.get(key).map(Arc::clone) else {
            return false;
        };

        self.waiting.remove(key);
        if let Some(id) = TileId::parse_text(key) {
            if let Some(count) = self.waiting_prefixes.get_mut(&id.prefix()) {
                *count -= 1;
                if *count == 0 {
                    self.waiting_prefixes.remove(&id.prefix());
                }
            }
        } else {
            self.waiting_digests -= 1;
        }

        self.resolved.insert(shared, *raw);
        true
    }
}

/// True when `key` is the identifier or the legacy digest `raw` gives, which is what makes a
/// cache entry verifiable (REQ-PROF-082).
#[must_use]
pub fn recomputes(key: &str, raw: &RawTile) -> bool {
    if tile_id::is_identifier_text(key) {
        return TileId::compute(raw).to_text() == key;
    }
    if tile_id::is_legacy_digest(key) {
        return raw_tile::legacy_digest(raw) == key;
    }

    false
}

/// Rebuilds the patterns of `profile` whose waiting tiles `resolver` can now fill, and returns
/// how many tile objects that turned into ordinary tiles.
///
/// One new [`Arc`] per distinct old one, substituted everywhere that old one appears: the
/// identity graph stays isomorphic, which is what keeps the rewrite of REQ-PROF-084 lossless, and
/// the cost falls only on the patterns a resolution actually changes.
pub fn apply(profile: &mut GameProfile, resolver: &TileResolver) -> usize {
    let mut tiles: HashMap<*const ProfileTile, Arc<ProfileTile>> = HashMap::new();
    for tile in profile.distinct_tile_objects() {
        let Some(raw) = tile
            .hash_key()
            .filter(|_| tile.is_waiting())
            .and_then(|key| resolver.bytes_of(key))
        else {
            continue;
        };
        let (low, high) = raw_tile::to_planes(raw);
        tiles.insert(Arc::as_ptr(tile), Arc::new(tile.with_planes(low, high)));
    }

    // The memorised slots of the settings hold their patterns by value, outside the shared graph.
    if let Some(settings) = profile.settings.as_mut() {
        for slot in settings
            .saved_patterns
            .iter_mut()
            .chain(&mut settings.saved_anim_patterns)
        {
            if let Some(pattern) = slot.pattern.as_mut() {
                substitute_in_place(pattern, resolver);
            }
        }
    }

    if tiles.is_empty() {
        return 0;
    }

    let mut patterns: HashMap<*const ProfilePattern, Arc<ProfilePattern>> = HashMap::new();
    for pattern in profile.all_patterns() {
        let Some(grid) = pattern.grid.as_ref() else {
            continue;
        };
        if !grid
            .cells()
            .iter()
            .flatten()
            .any(|tile| tiles.contains_key(&Arc::as_ptr(tile)))
        {
            continue;
        }

        let cells = grid
            .cells()
            .iter()
            .map(|cell| {
                cell.as_ref().map(|tile| {
                    tiles
                        .get(&Arc::as_ptr(tile))
                        .map_or_else(|| Arc::clone(tile), Arc::clone)
                })
            })
            .collect();
        let mut rebuilt = ProfilePattern::clone(pattern);
        rebuilt.grid = Some(PatternGrid::new(grid.width(), grid.height(), cells));
        patterns.insert(Arc::as_ptr(pattern), Arc::new(rebuilt));
    }

    for list in [
        &mut profile.permanent_patterns,
        &mut profile.temporary_patterns,
    ] {
        for pattern in list.iter_mut() {
            if let Some(rebuilt) = patterns.get(&Arc::as_ptr(pattern)) {
                *pattern = Arc::clone(rebuilt);
            }
        }
    }

    tiles.len()
}

/// Substitutes the resolved tiles of a pattern held by value.
fn substitute_in_place(pattern: &mut ProfilePattern, resolver: &TileResolver) {
    let Some(grid) = pattern.grid.as_ref() else {
        return;
    };

    let mut rebuilt: HashMap<*const ProfileTile, Arc<ProfileTile>> = HashMap::new();
    for tile in grid.cells().iter().flatten() {
        let Some(raw) = tile
            .hash_key()
            .filter(|_| tile.is_waiting())
            .and_then(|key| resolver.bytes_of(key))
        else {
            continue;
        };
        let (low, high) = raw_tile::to_planes(raw);
        rebuilt
            .entry(Arc::as_ptr(tile))
            .or_insert_with(|| Arc::new(tile.with_planes(low, high)));
    }

    if rebuilt.is_empty() {
        return;
    }

    let cells = grid
        .cells()
        .iter()
        .map(|cell| {
            cell.as_ref().map(|tile| {
                rebuilt
                    .get(&Arc::as_ptr(tile))
                    .map_or_else(|| Arc::clone(tile), Arc::clone)
            })
        })
        .collect();
    pattern.grid = Some(PatternGrid::new(grid.width(), grid.height(), cells));
}
