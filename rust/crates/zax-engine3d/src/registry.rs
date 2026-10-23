// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The registry of patterns: the permanent set and the temporary set active during the session
//! (SPEC-04 §2, REQ-E3D-060 to 065).

use std::collections::HashMap;

use zax_profiles::profile::GameProfile;

use crate::error::EngineError;
use crate::image::ImageRegistry;
use crate::instance::{InstanceRef, PatternInstance};
use crate::pattern::{is_subset, EnginePattern, ImageSetKey};

/// The handle of a registry pattern.
///
/// A pattern taken out of the registry stays readable through its handle — a shape of a frame
/// still bound to it can be drawn and scripted — until the registry is cleared or the learned
/// state is dropped (REQ-E3D-008); a handle of a pattern dropped that way reads as absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PatternId {
    slot: u32,
    generation: u32,
}

/// The registry of patterns (SPEC-04 §2, REQ-E3D-060 to 065).
///
/// Lookup by key is a hash lookup and the candidates of an image are a list held by the image
/// registry, so both are constant time amortised as REQ-E3D-360 requires. Both sets keep
/// registration order; the candidate order of an image is kept by the image registry, where
/// AMD-021 R2-008 gives a newcomer the place of the last pattern removed from that image.
///
/// The patterns are stored in registration order: the slot of a pattern is its registration
/// index.
#[derive(Debug, Clone, Default)]
pub struct PatternRegistry {
    store: Vec<EnginePattern>,
    generations: Vec<u32>,
    permanent: Vec<PatternId>,
    temporary: Vec<PatternId>,
    permanent_by_key: HashMap<ImageSetKey, PatternId>,
    temporary_by_key: HashMap<ImageSetKey, PatternId>,
    profile_pattern_count: i32,
}

impl PatternRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The permanent patterns, in registration order.
    #[must_use]
    pub fn permanent(&self) -> &[PatternId] {
        &self.permanent
    }

    /// The temporary patterns, in registration order.
    #[must_use]
    pub fn temporary(&self) -> &[PatternId] {
        &self.temporary
    }

    /// Every pattern of the registry: the permanent ones then the temporary ones, each set in
    /// registration order.
    pub fn all(&self) -> impl Iterator<Item = PatternId> + '_ {
        self.permanent.iter().chain(&self.temporary).copied()
    }

    /// How many patterns came from the profile. Patterns registered later were learned during
    /// play (REQ-E3D-161) and are the ones a savestate carries (DEC-R2-T5).
    #[must_use]
    pub const fn profile_pattern_count(&self) -> i32 {
        self.profile_pattern_count
    }

    /// The store slot of a handle, or `None` when the pattern was dropped.
    fn slot_of(&self, id: PatternId) -> Option<usize> {
        let slot = id.slot as usize;
        (slot < self.store.len() && self.generations[slot] == id.generation).then_some(slot)
    }

    /// The pattern of a handle, or `None` when it was dropped.
    #[must_use]
    pub fn get(&self, id: PatternId) -> Option<&EnginePattern> {
        self.slot_of(id).map(|slot| &self.store[slot])
    }

    /// The pattern of a handle, for writing, or `None` when it was dropped.
    pub fn get_mut(&mut self, id: PatternId) -> Option<&mut EnginePattern> {
        self.slot_of(id).map(|slot| &mut self.store[slot])
    }

    /// The pattern instance at `instance`, or `None`.
    #[must_use]
    pub fn instance(&self, instance: InstanceRef) -> Option<&PatternInstance> {
        self.get(instance.pattern)
            .and_then(|pattern| pattern.instances().get(instance.index))
    }

    /// The pattern instance at `instance`, for writing, or `None`.
    pub fn instance_mut(&mut self, instance: InstanceRef) -> Option<&mut PatternInstance> {
        self.get_mut(instance.pattern)
            .and_then(|pattern| pattern.instances_mut().get_mut(instance.index))
    }

    /// True when the pattern is currently in the permanent or the temporary set.
    #[must_use]
    pub fn is_registered(&self, id: PatternId) -> bool {
        self.permanent.contains(&id) || self.temporary.contains(&id)
    }

    /// Registers the patterns of a profile: every permanent one in file order, then every
    /// temporary one in file order (REQ-E3D-060).
    ///
    /// # Errors
    ///
    /// [`EngineError::PatternSize`] when a pattern declares an invalid grid; the patterns
    /// before it stay registered.
    pub fn load_profile(
        &mut self,
        profile: &GameProfile,
        images: &mut ImageRegistry,
    ) -> Result<(), EngineError> {
        for pattern in &profile.permanent_patterns {
            self.register(EnginePattern::from_profile(pattern, images, true)?, images);
        }

        for pattern in &profile.temporary_patterns {
            self.register(EnginePattern::from_profile(pattern, images, false)?, images);
        }

        self.profile_pattern_count = self.next_registration_index();
        Ok(())
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // C# int index.
    fn next_registration_index(&self) -> i32 {
        self.store.len() as i32
    }

    /// Registers a pattern *without* the replacement rules of REQ-E3D-065, which is what
    /// loading a profile does (REQ-E3D-060).
    ///
    /// Q-011. REQ-E3D-060 requires every pattern of the file to be registered — its acceptance
    /// criterion counts 101 profiles out of 101 — while REQ-E3D-065 would drop any temporary
    /// pattern whose images contain those of a later one. The two cannot both hold: "Elevator
    /// Action" holds three such temporary patterns out of its 300. The file is therefore taken
    /// as it is and the replacement rules are kept for the patterns learned while playing,
    /// which is also what the deserialisation path of the earlier, public `3DNes_Engine` does.
    pub fn register(
        &mut self,
        mut pattern: EnginePattern,
        images: &mut ImageRegistry,
    ) -> PatternId {
        let slot = self.store.len();
        if self.generations.len() <= slot {
            self.generations.push(0);
        }

        #[allow(clippy::cast_possible_truncation)] // Slots follow C# int registration indices.
        let id = PatternId {
            slot: slot as u32,
            generation: self.generations[slot],
        };
        pattern.set_registration_index(self.next_registration_index());
        for image in pattern.images() {
            images.attach(image, id, pattern.permanent());
        }

        if pattern.permanent() {
            self.permanent.push(id);
            self.permanent_by_key.insert(pattern.key().clone(), id);
        } else {
            self.temporary.push(id);
            self.temporary_by_key.insert(pattern.key().clone(), id);
        }

        self.store.push(pattern);
        id
    }

    /// Adds a pattern, applying the replacement rules of REQ-E3D-065: a permanent pattern
    /// replaces the permanent one of the same key and swallows every temporary pattern whose
    /// images it contains; a temporary pattern replaces every temporary pattern that contains
    /// its images.
    pub fn add(&mut self, pattern: EnginePattern, images: &mut ImageRegistry) -> PatternId {
        if pattern.permanent() {
            if let Some(&existing) = self.permanent_by_key.get(pattern.key()) {
                self.remove(existing, images);
            }

            self.remove_temporaries(images, |candidate| {
                candidate.key().is_subset_of(pattern.key())
            });
        } else {
            self.remove_temporaries(images, |candidate| {
                pattern.key().is_subset_of(candidate.key())
            });
        }

        self.register(pattern, images)
    }

    /// Takes a pattern out of the registry and out of its images' candidate lists.
    pub fn remove(&mut self, id: PatternId, images: &mut ImageRegistry) {
        let Some(slot) = self.slot_of(id) else {
            return;
        };

        let pattern = &self.store[slot];
        let (list, by_key) = if pattern.permanent() {
            (&mut self.permanent, &mut self.permanent_by_key)
        } else {
            (&mut self.temporary, &mut self.temporary_by_key)
        };
        let Some(place) = list.iter().position(|&p| p == id) else {
            return;
        };
        list.remove(place);

        if by_key.get(pattern.key()) == Some(&id) {
            by_key.remove(pattern.key());
        }

        for image in pattern.images() {
            images.detach(image, id, pattern.permanent());
        }
    }

    /// Removes every permanent pattern that carries `tag`.
    ///
    /// Used by the script function `Pattern.RemoveWithTag` (SPEC-05); the candidate order of
    /// the survivors is unchanged, but the place each removal leaves free is remembered for
    /// the next pattern registered on that image (R2-008).
    pub fn remove_permanent_with_tag(&mut self, tag: &str, images: &mut ImageRegistry) -> usize {
        let doomed: Vec<PatternId> = self
            .permanent
            .iter()
            .copied()
            .filter(|&id| {
                self.get(id).is_some_and(|pattern| {
                    pattern
                        .instances()
                        .iter()
                        .any(|instance| instance.contains_tag(tag))
                })
            })
            .collect();

        for &id in &doomed {
            self.remove(id, images);
        }

        doomed.len()
    }

    /// The permanent pattern of this key (the sorted image identities), or `None`.
    #[must_use]
    pub fn find_permanent(&self, key: &[u32]) -> Option<PatternId> {
        self.permanent_by_key.get(key).copied()
    }

    /// The temporary pattern of this key (the sorted image identities), or `None`.
    #[must_use]
    pub fn find_temporary(&self, key: &[u32]) -> Option<PatternId> {
        self.temporary_by_key.get(key).copied()
    }

    /// The first temporary pattern, in registration order, whose images contain `key`
    /// (REQ-E3D-160 point 2).
    #[must_use]
    pub fn find_temporary_containing(&self, key: &[u32]) -> Option<PatternId> {
        self.temporary.iter().copied().find(|&id| {
            self.get(id)
                .is_some_and(|pattern| is_subset(key, pattern.key().ids()))
        })
    }

    /// The pattern instances carrying `tag`, walking the permanent patterns in registration
    /// order then their instances by index (SPEC-05 REQ-LUA-085).
    pub fn instances_with_tag<'a>(
        &'a self,
        tag: &'a str,
    ) -> impl Iterator<Item = InstanceRef> + 'a {
        self.permanent.iter().flat_map(move |&id| {
            self.get(id)
                .into_iter()
                .flat_map(|pattern| pattern.instances().iter().enumerate())
                .filter(move |(_, instance)| instance.contains_tag(tag))
                .map(move |(index, _)| InstanceRef { pattern: id, index })
        })
    }

    /// The first pattern instance that carries `tag` (SPEC-05 REQ-LUA-085).
    #[must_use]
    pub fn find_instance_with_tag(&self, tag: &str) -> Option<InstanceRef> {
        self.instances_with_tag(tag).next()
    }

    /// Every pattern instance that carries `tag`, in the order of
    /// [`PatternRegistry::find_instance_with_tag`] (SPEC-05 REQ-LUA-085).
    pub fn collect_instances_with_tag(&self, tag: &str, destination: &mut Vec<InstanceRef>) {
        destination.clear();
        destination.extend(self.instances_with_tag(tag));
    }

    /// Empties the registry. Done when the game changes (REQ-E3D-008).
    pub fn clear(&mut self, images: &mut ImageRegistry) {
        for id in self.permanent.iter().chain(&self.temporary) {
            if let Some(pattern) = self.get(*id) {
                for image in pattern.images() {
                    images.detach(image, *id, pattern.permanent());
                }
            }
        }

        self.permanent.clear();
        self.temporary.clear();
        self.permanent_by_key.clear();
        self.temporary_by_key.clear();
        self.truncate(0);
        self.profile_pattern_count = 0;
    }

    /// Drops everything learned during play: the patterns registered after the profile was
    /// loaded and every temporary pattern. Used before a savestate's learned state is put back
    /// (REQ-E3D-008).
    pub fn clear_learned(&mut self, images: &mut ImageRegistry) {
        for i in (0..self.permanent.len()).rev() {
            let id = self.permanent[i];
            if self
                .get(id)
                .is_some_and(|pattern| pattern.registration_index() >= self.profile_pattern_count)
            {
                self.remove(id, images);
            }
        }

        for i in (0..self.temporary.len()).rev() {
            self.remove(self.temporary[i], images);
        }

        #[allow(clippy::cast_sign_loss)] // A count, never negative.
        self.truncate(self.profile_pattern_count as usize);
    }

    /// Drops the stored patterns from `slot` on; their handles read as absent from now on and
    /// the slots are reused by the next registrations.
    fn truncate(&mut self, slot: usize) {
        let end = self.store.len();
        for generation in self.generations.iter_mut().take(end).skip(slot) {
            *generation = generation.wrapping_add(1);
        }
        self.store.truncate(slot);
    }

    fn remove_temporaries(
        &mut self,
        images: &mut ImageRegistry,
        doomed: impl Fn(&EnginePattern) -> bool,
    ) {
        for i in (0..self.temporary.len()).rev() {
            let id = self.temporary[i];
            if self.get(id).is_some_and(&doomed) {
                self.remove(id, images);
            }
        }
    }
}
