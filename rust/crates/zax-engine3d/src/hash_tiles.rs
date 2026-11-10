//! The tiles a "3dh" profile is still waiting for while the game runs, and their resolution
//! (AMD-062 REQ-PROF-080 to 082).
//!
//! The table is session knowledge, like the cache of REQ-PROF-082: a tile resolved here never
//! goes back to waiting — not on a rollback, a run-ahead, a state load or a reset — and it
//! belongs neither to the savestates nor to the rollback snapshots. It costs nothing once no cell
//! of the profile waits for anything.

use std::collections::HashMap;
use std::sync::Arc;

use zax_nes::TileRecord;
use zax_profiles::hash::raw_tile::{self, BYTE_COUNT};
use zax_profiles::hash::tiles::{self, TileResolver};
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;

use crate::registry::PatternId;

/// The patterns of a loaded "3dh" profile whose cells are still waiting for their sixteen bytes,
/// and what resolves them (AMD-062 REQ-PROF-080).
///
/// The profile is read-only to the engine, so the table owns its own list of patterns in
/// registration order: a resolution rebuilds the ones it changes, as a new [`Arc`], and the
/// registry pattern of the same index is rebuilt from it.
#[derive(Debug)]
pub struct HashProfileTiles {
    /// Which manifest strings have their sixteen bytes (REQ-PROF-078 to 082).
    resolver: TileResolver,
    /// The patterns of the profile, permanent then temporary, each in file order and as the
    /// resolutions so far have left it.
    patterns: Vec<Arc<ProfilePattern>>,
    /// The registry handle of each pattern, in the same order.
    ids: Vec<PatternId>,
    /// The distinct manifest strings at least one cell of `patterns` named while waiting, in
    /// pattern order then cell order: a deterministic order, never a hash order.
    keys: Vec<Box<str>>,
    /// For each string of `keys`, the patterns that hold a cell naming it, ascending.
    holders: Vec<Vec<u32>>,
    /// Which strings of `keys` are still waiting; the gate of REQ-PROF-080's cost rule.
    waiting: Vec<u32>,
    /// The patterns the last resolution rebuilt, with their handles.
    changed: Vec<(PatternId, Arc<ProfilePattern>)>,
    /// Every pattern a resolution has rebuilt since the profile was loaded, ascending. A state
    /// load re-interns their images, which the registry had put in the learned range.
    rebuilt: Vec<u32>,
    /// How many manifest strings the running game resolved, for the log of REQ-PROF-080.
    resolved_while_playing: usize,
    /// The CHR-RAM as the last scan saw it. REQ-PROF-080 allows skipping a block that has not
    /// changed: the waiting set only ever shrinks, so a block already confronted with the
    /// manifest resolves nothing new, and the resolved set stays the full scan's.
    chr_ram_seen: Vec<u8>,
}

impl HashProfileTiles {
    /// The table of a profile whose tiles `resolver` has already resolved what it could — the
    /// cache of REQ-PROF-082 and then the ROM of REQ-PROF-078, both the host's business, happen
    /// before the profile is handed to the engine.
    #[must_use]
    pub fn new(resolver: TileResolver, profile: &GameProfile) -> Self {
        let mut patterns =
            Vec::with_capacity(profile.permanent_patterns.len() + profile.temporary_patterns.len());
        let mut keys: Vec<Box<str>> = Vec::new();
        let mut holders: Vec<Vec<u32>> = Vec::new();
        let mut known: HashMap<Box<str>, u32> = HashMap::new();

        for pattern in profile
            .permanent_patterns
            .iter()
            .chain(&profile.temporary_patterns)
        {
            let index = u32::try_from(patterns.len()).unwrap_or(u32::MAX);
            for key in tiles::waiting_keys_of(pattern) {
                let slot = if let Some(&slot) = known.get(key) {
                    slot
                } else {
                    let slot = u32::try_from(keys.len()).unwrap_or(u32::MAX);
                    let owned: Box<str> = Box::from(key);
                    keys.push(owned.clone());
                    holders.push(Vec::new());
                    known.insert(owned, slot);
                    slot
                };
                let holders = &mut holders[slot as usize];
                if holders.last() != Some(&index) {
                    holders.push(index);
                }
            }
            patterns.push(Arc::clone(pattern));
        }

        let waiting = (0..u32::try_from(keys.len()).unwrap_or(u32::MAX)).collect();
        Self {
            resolver,
            patterns,
            ids: Vec::new(),
            keys,
            holders,
            waiting,
            changed: Vec::new(),
            rebuilt: Vec::new(),
            resolved_while_playing: 0,
            chr_ram_seen: Vec::new(),
        }
    }

    /// Binds the registry handles the profile's patterns got, in the same order.
    pub(crate) fn bind(&mut self, ids: Vec<PatternId>) {
        self.ids = ids;
    }

    /// Which manifest strings have their sixteen bytes, which is what the cache of REQ-PROF-082
    /// is written from at the end of a session.
    #[must_use]
    pub const fn resolver(&self) -> &TileResolver {
        &self.resolver
    }

    /// True while at least one cell of the profile waits for its sixteen bytes (REQ-PROF-081).
    /// Once false, the resolution of REQ-PROF-080 costs nothing at all.
    #[must_use]
    pub fn is_waiting(&self) -> bool {
        !self.waiting.is_empty()
    }

    /// How many distinct manifest strings at least one cell is still waiting for.
    #[must_use]
    pub fn waiting_count(&self) -> usize {
        self.waiting.len()
    }

    /// How many manifest strings the running game has resolved (REQ-PROF-080, logged count).
    #[must_use]
    pub const fn resolved_while_playing(&self) -> usize {
        self.resolved_while_playing
    }

    /// The tiles of a frame, planes in clear and not flipped, confronted with the strings still
    /// waiting, each with its four mirrorings (REQ-PROF-080, REQ-PROF-079). Returns how many
    /// strings this resolved.
    ///
    /// A record whose planes repeat the previous one's is skipped: offering the same window twice
    /// resolves nothing, so the resolved set is the full walk's.
    pub(crate) fn offer_frame_tiles(&mut self, records: &[TileRecord]) -> usize {
        if self.waiting.is_empty() {
            return 0;
        }

        let mut count = 0;
        let mut previous: Option<(u64, u64)> = None;
        for record in records {
            let planes = (record.plane0, record.plane1);
            if previous == Some(planes) {
                continue;
            }

            previous = Some(planes);
            count += self
                .resolver
                .offer(&raw_tile::from_planes(planes.0, planes.1));
        }

        self.resolved_while_playing += count;
        count
    }

    /// Every aligned sixteen-byte block of the cartridge's CHR-RAM (SPEC-02 REQ-CORE-123),
    /// confronted with the strings still waiting, each with its four mirrorings (REQ-PROF-080).
    /// Returns how many strings this resolved.
    pub(crate) fn offer_chr_ram(&mut self, chr_ram: &[u8]) -> usize {
        if self.waiting.is_empty() || chr_ram.len() < BYTE_COUNT {
            return 0;
        }

        let mut count = 0;
        if self.chr_ram_seen.len() == chr_ram.len() {
            for (block, chunk) in chr_ram.chunks_exact(BYTE_COUNT).enumerate() {
                let start = block * BYTE_COUNT;
                if self.chr_ram_seen[start..start + BYTE_COUNT] == *chunk {
                    continue;
                }

                self.chr_ram_seen[start..start + BYTE_COUNT].copy_from_slice(chunk);
                count += self.offer_block(chunk);
            }
        } else {
            // First scan of this memory: nothing has been confronted yet.
            for chunk in chr_ram.chunks_exact(BYTE_COUNT) {
                count += self.offer_block(chunk);
            }
            self.chr_ram_seen.clear();
            self.chr_ram_seen.extend_from_slice(chr_ram);
        }

        self.resolved_while_playing += count;
        count
    }

    fn offer_block(&mut self, block: &[u8]) -> usize {
        let mut raw = [0_u8; BYTE_COUNT];
        raw.copy_from_slice(block);
        self.resolver.offer(&raw)
    }

    /// Rebuilds the patterns whose waiting tiles have just been resolved and returns them with
    /// their registry handles: the cell that was waiting now holds a tile, so the pattern, its
    /// variants and their textures have to be taken again before this frame is recognised
    /// (REQ-PROF-080).
    pub(crate) fn rebuild_changed(&mut self) -> &[(PatternId, Arc<ProfilePattern>)] {
        self.changed.clear();
        let mut waiting = std::mem::take(&mut self.waiting);
        let mut touched: Vec<u32> = Vec::new();
        waiting.retain(|&slot| {
            if self.resolver.bytes_of(&self.keys[slot as usize]).is_none() {
                return true;
            }

            touched.extend_from_slice(&self.holders[slot as usize]);
            false
        });
        self.waiting = waiting;
        if touched.is_empty() {
            return &self.changed;
        }

        touched.sort_unstable();
        touched.dedup();
        for index in touched {
            let pattern = &self.patterns[index as usize];
            let Some(grid) = pattern
                .grid
                .as_ref()
                .and_then(|grid| tiles::resolved_grid(grid, &self.resolver))
            else {
                continue;
            };

            let mut rebuilt = ProfilePattern::clone(pattern);
            rebuilt.grid = Some(grid);
            let rebuilt = Arc::new(rebuilt);
            self.patterns[index as usize] = Arc::clone(&rebuilt);
            if let Err(place) = self.rebuilt.binary_search(&index) {
                self.rebuilt.insert(place, index);
            }
            if let Some(&id) = self.ids.get(index as usize) {
                self.changed.push((id, rebuilt));
            }
        }

        &self.changed
    }

    /// Every pattern a resolution has rebuilt since the profile was loaded, with its handle: what
    /// a state load has to take again, since the images those patterns gained were interned after
    /// the profile's and a restore re-interns only what the state recorded (REQ-E3D-008).
    pub(crate) fn resolved_patterns(&self) -> Vec<(PatternId, Arc<ProfilePattern>)> {
        self.rebuilt
            .iter()
            .filter_map(|&index| {
                let id = *self.ids.get(index as usize)?;
                Some((id, Arc::clone(self.patterns.get(index as usize)?)))
            })
            .collect()
    }
}
