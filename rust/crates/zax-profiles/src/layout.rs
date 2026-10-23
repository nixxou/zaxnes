//! Which fields a decoded node carried in the file (Q-010).
//!
//! REQ-PROF-051 asks for "scalars always written" and, in the same breath, for a byte-identical
//! rewrite of files that leave scalars out. Recording the field set of every decoded node
//! satisfies both: a file is rewritten as it was, a new profile is written whole.

/// A set of field numbers 1 to 63 of one message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldMask(u64);

impl FieldMask {
    /// The empty set.
    pub const EMPTY: Self = Self(0);

    /// The set of `fields`.
    #[must_use]
    pub const fn of(fields: &[u32]) -> Self {
        let mut mask = 0;
        let mut index = 0;
        while index < fields.len() {
            mask |= Self::bit(fields[index]);
            index += 1;
        }
        Self(mask)
    }

    /// The set of the fields `first` to `last`, both included.
    #[must_use]
    pub const fn range(first: u32, last: u32) -> Self {
        let mut mask = 0;
        let mut field = first;
        while field <= last {
            mask |= Self::bit(field);
            field += 1;
        }
        Self(mask)
    }

    /// True when `field` is in the set.
    #[must_use]
    pub const fn contains(self, field: u32) -> bool {
        self.0 & Self::bit(field) != 0
    }

    /// Adds `field` to the set.
    pub fn insert(&mut self, field: u32) {
        self.0 |= Self::bit(field);
    }

    /// Removes `field` from the set.
    pub fn remove(&mut self, field: u32) {
        self.0 &= !Self::bit(field);
    }

    /// The field numbers in the set, ascending.
    #[must_use]
    pub fn fields(self) -> Vec<u32> {
        (1..64).filter(|field| self.contains(*field)).collect()
    }

    const fn bit(field: u32) -> u64 {
        if field < 64 {
            1 << field
        } else {
            0
        }
    }
}

/// Collects the fields seen while a message is decoded and keeps them only when they differ
/// from the schema, which is what [`ProfileDocument`](crate::ProfileDocument) needs to rewrite
/// the file as it was.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PresenceRecorder(FieldMask);

impl PresenceRecorder {
    pub(crate) fn saw(&mut self, field: u32) {
        self.0.insert(field);
    }

    pub(crate) fn unknown(&mut self, field: u32) {
        self.0.remove(field);
    }

    /// The recorded set, or none when it equals `schema` (nothing to remember).
    pub(crate) fn finish(self, schema: FieldMask) -> Option<FieldMask> {
        (self.0 != schema).then_some(self.0)
    }
}

/// How the encoder decides which scalar fields to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldPresence {
    /// A profile built in memory: every scalar is written (REQ-PROF-051).
    All,
    /// A profile that came from a file: each node writes the fields it was read with.
    AsRecorded,
}

/// Whether `node` (with its recorded field set) writes `field` under `presence`.
pub(crate) fn is_present(presence: FieldPresence, recorded: Option<FieldMask>, field: u32) -> bool {
    match (presence, recorded) {
        (FieldPresence::AsRecorded, Some(mask)) => mask.contains(field),
        _ => true,
    }
}
