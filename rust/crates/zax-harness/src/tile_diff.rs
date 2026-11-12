//! The measuring stick of lot FCEUX-6, with no engine in it: how two lists of tile records
//! differ, field by field.
//!
//! Lifted out of `fceux_tiles` by lot NESHAWK-REWRITE so that `neshawk-tiles` can use the **same**
//! comparison -- the ten-field mask, the multiset count, the OAM-rotation check, the row
//! histograms, the readable record pairs. None of it knows which engine produced a record: it all
//! works on [`Key`], a record as a tuple of its ten fields. `fceux_tiles` is behind the
//! `fceux-core` feature and this is not, which is the other half of the reason.
//!
//! The arguments for every field of [`FrameDiff`] are lot FCEUX-6's and are kept verbatim at
//! their sites; nothing here was redesigned in the move.

use std::collections::HashMap;

use zax_core_api::TileRecord;

/// How many differing rows a summary lists before it says "and N more" (lot FCEUX-6).
const MAX_ROWS_LISTED: usize = 12;

/// The ten fields of a record, flattened so that two records can be compared and the differing
/// fields named. The order is `TileRecord`'s own.
pub const FIELDS: [&str; 10] = [
    "plane0",
    "plane1",
    "x",
    "y",
    "index",
    "mirror_h",
    "mirror_v",
    "hidden",
    "palette",
    "background",
];

/// A record as a tuple of its ten fields, so that both engines' types compare without either
/// crate depending on the other.
pub type Key = (u64, u64, i16, i16, i32, bool, bool, bool, u8, bool);

#[must_use]
pub const fn key_of_record(record: &TileRecord) -> Key {
    (
        record.plane0,
        record.plane1,
        record.x,
        record.y,
        record.index,
        record.mirror_h,
        record.mirror_v,
        record.hidden,
        record.palette,
        record.background,
    )
}

/// Which of the ten fields differ between two records, as a bitmask over [`FIELDS`].
#[must_use]
pub fn differing_fields(left: Key, right: Key) -> u16 {
    let mut mask = 0u16;
    if left.0 != right.0 {
        mask |= 1 << 0;
    }
    if left.1 != right.1 {
        mask |= 1 << 1;
    }
    if left.2 != right.2 {
        mask |= 1 << 2;
    }
    if left.3 != right.3 {
        mask |= 1 << 3;
    }
    if left.4 != right.4 {
        mask |= 1 << 4;
    }
    if left.5 != right.5 {
        mask |= 1 << 5;
    }
    if left.6 != right.6 {
        mask |= 1 << 6;
    }
    if left.7 != right.7 {
        mask |= 1 << 7;
    }
    if left.8 != right.8 {
        mask |= 1 << 8;
    }
    if left.9 != right.9 {
        mask |= 1 << 9;
    }
    mask
}

#[must_use]
pub fn field_names(mask: u16) -> String {
    let names: Vec<&str> = FIELDS
        .iter()
        .enumerate()
        .filter(|(bit, _)| (mask & (1 << bit)) != 0)
        .map(|(_, name)| *name)
        .collect();
    names.join("+")
}

/// What one frame's comparison found.
#[derive(Debug, Clone, Default)]
pub struct FrameDiff {
    pub zaxnes: usize,
    pub fceux: usize,
    /// Records equal at the same position in both lists — the strictest reading, and the one the
    /// renderer sees (`FrameBatch::records` is ordered).
    pub same_in_order: usize,
    /// Records present on both sides with every field equal, order ignored.
    pub same_as_multiset: usize,
    /// Whether the two engines' 256×240 index pictures are identical for this frame.
    pub pixels_identical: bool,
    /// Whether Zaxnes's picture for this frame is a **single colour**. When it is, two identical
    /// pictures prove much less than usual: a game that blanks the screen with `$2001` and a
    /// game that draws a screenful of blank tiles look exactly the same, so a record difference
    /// on such a frame is not evidence about the capture either way. The tool says so rather
    /// than letting the `capture` class claim it.
    pub pixels_uniform: bool,
    /// Differing records per kind of record (`true` = background) and set of differing fields.
    /// The kind matters: an `index` that differs on a **background** record is the row and
    /// column numbering and would be the capture's fault, while an `index` that differs on a
    /// **sprite** is the OAM entry the game put it in, which is the machine's state and not the
    /// capture's (see [`FrameDiff::oam_rotation`]).
    pub by_fields: HashMap<(bool, u16), usize>,
    /// Distinct `y` of the differing records, sorted.
    pub rows: Vec<i16>,
    /// Records with no partner at all on the other side.
    pub unmatched_zaxnes: usize,
    pub unmatched_fceux: usize,
    /// The background rows each engine emitted, as `y`:count — the first thing to look at when a
    /// row is emitted on the wrong line, because it says which engine emitted what and where.
    pub zaxnes_rows: Vec<(i16, usize)>,
    pub fceux_rows: Vec<(i16, usize)>,
    /// The differing record pairs, in capture order, each side as one readable line. Only filled
    /// when `--records` asks for them: it is a diagnostic, not part of the table.
    pub pairs: Vec<(String, String)>,
    /// When every sprite record of the frame matches after turning FCEUX's OAM number `n` into
    /// `(n + k) mod 64`, the `k` that does it. A non-zero `k` means the two engines' **OAM
    /// arrays are rotations of each other** — the `$4014` transfer started at a different
    /// `$2003` on the two machines — which is an emulation difference and not the capture's:
    /// the capture reads OAM entry `n` on both sides, as REQ-CORE-043 requires.
    pub oam_rotation: Option<i32>,
    /// Whether the **32-byte palette** the two engines hand over with the records differs, and
    /// the entries that do.
    ///
    /// Lot FCEUX-6 step 8's finding. A frame batch is the records *and* the palette
    /// (REQ-CORE-045), and the 3D renderer reads both — it colours a mesh from the palette entry
    /// the record's sub-palette selects — so comparing the records alone can call a frame
    /// identical when the 3D picture is not. *Final Fantasy* at frame 300 is the case that found
    /// it: all 960 records equal on all ten fields, `$3F0B` **32 on Zaxnes and 0 on FCEUX**, the
    /// two 2D pictures identical because no pixel on screen uses that entry, and 0.69 % of the
    /// 3D picture's pixels different.
    ///
    /// Each entry is `(index, zaxnes, fceux)`. Empty when the palettes agree, which is also the
    /// default, so a `FrameDiff` built by a caller that has no palette to offer claims nothing.
    pub palette_differences: Vec<(usize, u8, u8)>,
}

impl FrameDiff {
    /// `identical`, `capture` (ours to fix) or `emulation` (FCEUX against Zaxnes).
    ///
    /// The palette counts: a frame whose records all match but whose palette does not is not
    /// `identical`, because the 3D picture the lot exists to make the same is not the same. See
    /// [`FrameDiff::palette_differences`].
    #[must_use]
    pub fn class(&self) -> &'static str {
        let records_identical = self.zaxnes == self.fceux && self.same_in_order == self.zaxnes;
        if records_identical && self.palette_differences.is_empty() {
            "identical"
        } else if self.pixels_identical {
            "capture"
        } else {
            "emulation"
        }
    }

    /// The differing palette entries as `$3F0B:32/0`, or empty when they agree.
    #[must_use]
    pub fn palette_summary(&self) -> String {
        self.palette_differences
            .iter()
            .map(|(index, ours, theirs)| format!("$3F{index:02X}:{ours}/{theirs}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Why this frame is **capture**-class without the capture being at fault, or
    /// `"unexplained"` when nothing here accounts for it — the one value that is a defect.
    ///
    /// Lot FCEUX-6 step 6 exists to drive the `unexplained` count to zero, and this is where the
    /// reasoning lives rather than in a note nobody can re-run. The `capture` class is decided by
    /// one question — "do the two engines' pictures agree for this frame?" — and that question has
    /// three known blind spots, each of which this names:
    ///
    /// * the two OAM arrays are **rotations** of each other, so every sprite record is right but
    ///   numbered differently (REQ-CORE-043 numbers a record after its OAM entry, so the capture
    ///   cannot cause it);
    /// * every differing record is a **sprite that differs on its `index`**, without one `k`
    ///   rotating the whole array: the game put that sprite in another OAM slot, which is again
    ///   the machine's state;
    /// * Zaxnes's picture is a **single colour**, so "the pictures agree" carries almost no
    ///   information — a blanked screen and a screenful of blank tiles look identical.
    ///
    /// `first_pixel_diff` sharpens all three: when the pictures already differed at an earlier
    /// frame the two machines have provably diverged, and that they agree again at this one is a
    /// coincidence rather than evidence. Empty for a frame that is not capture-class.
    #[must_use]
    pub fn cause(&self, frame: u32, first_pixel_diff: Option<u32>) -> String {
        if self.class() != "capture" {
            return String::new();
        }

        let diverged = first_pixel_diff.filter(|first| *first < frame);
        let since = diverged.map_or_else(String::new, |first| format!(", diverged at {first}"));

        // The palette first, and only when **no record differs**: the two engines hold a
        // different colour in palette RAM, which is machine state the capture reads and does not
        // make. It is named rather than lumped into `unexplained` because the 2D pixel test
        // cannot see an entry no on-screen pixel uses, while the 3D path colours meshes from all
        // 32 of them (step 8).
        if self.by_fields.is_empty() && !self.palette_differences.is_empty() {
            return format!("palette-only{since}");
        }
        if let Some(k) = self.oam_rotation {
            return format!("oam-rotated-by-{k}{since}");
        }
        if self.sprite_index_only() {
            return format!("sprite-oam-slot{since}");
        }
        if self.pixels_uniform {
            return format!("blank-screen{since}");
        }
        format!("unexplained{since}")
    }

    /// Whether **every** differing record is a sprite whose differing fields include `index`.
    ///
    /// The `index` is what makes it conclusive: once the two engines hold the same sprite in
    /// different OAM entries, the positional pairing is comparing two different sprites, so the
    /// position and the planes differing as well is the consequence and not a second defect.
    fn sprite_index_only(&self) -> bool {
        const INDEX: u16 = 1 << 4;
        !self.by_fields.is_empty()
            && self
                .by_fields
                .keys()
                .all(|(background, mask)| !*background && (mask & INDEX) != 0)
    }

    /// The differing-field sets, most frequent first, as `plane0+plane1:106`.
    #[must_use]
    pub fn fields_summary(&self) -> String {
        let mut pairs: Vec<((bool, u16), usize)> =
            self.by_fields.iter().map(|(m, c)| (*m, *c)).collect();
        // Count first, then the key, so the order never depends on the hash map's.
        pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        pairs
            .iter()
            .map(|((background, mask), count)| {
                format!(
                    "{}{}:{count}",
                    if *background { "bg." } else { "spr." },
                    field_names(*mask)
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The rows the differing records sit on, or a count when there are too many to name.
    #[must_use]
    pub fn rows_summary(&self) -> String {
        if self.rows.len() > MAX_ROWS_LISTED {
            return format!("{} rows", self.rows.len());
        }
        self.rows
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Compares one frame's records.
///
/// Pairing: **positional** while both lists are the same length, which is the normal case and the
/// Compares two lists of [`TileRecord`], which is what `neshawk-tiles` has on both sides.
///
/// Pairing: **positional** while both lists are the same length, which is the normal case and the
/// strictest. When the lengths differ, the records are grouped by `(background, index)` and zipped
/// inside each group, with the leftovers counted as unmatched — a deterministic alignment that
/// still names the differing fields instead of reporting a bare count.
#[must_use]
pub fn compare_records(
    left: &[TileRecord],
    right: &[TileRecord],
    pixels_identical: bool,
    pixels_uniform: bool,
    pairs_wanted: usize,
) -> FrameDiff {
    let left: Vec<Key> = left.iter().map(key_of_record).collect();
    let right: Vec<Key> = right.iter().map(key_of_record).collect();
    compare_keys(
        &left,
        &right,
        pixels_identical,
        pixels_uniform,
        pairs_wanted,
    )
}

/// The comparison itself, which knows nothing about either engine: every field of [`FrameDiff`]
/// is derived from the two key lists. Both entry points above are two `map`s and this call.
#[must_use]
pub fn compare_keys(
    left: &[Key],
    right: &[Key],
    pixels_identical: bool,
    pixels_uniform: bool,
    pairs_wanted: usize,
) -> FrameDiff {
    let mut diff = FrameDiff {
        zaxnes: left.len(),
        fceux: right.len(),
        pixels_identical,
        pixels_uniform,
        ..FrameDiff::default()
    };

    diff.same_in_order = left.iter().zip(right).filter(|(a, b)| a == b).count();
    diff.same_as_multiset = multiset_intersection(left, right);

    let mut rows = Vec::new();
    let mut note = |mask: u16, pair: (Key, Key), rows: &mut Vec<i16>| {
        *diff.by_fields.entry((pair.0 .9, mask)).or_insert(0) += 1;
        rows.push(pair.0 .3);
        if diff.pairs.len() < pairs_wanted {
            diff.pairs.push((describe(pair.0), describe(pair.1)));
        }
    };

    if left.len() == right.len() {
        for (a, b) in left.iter().zip(right) {
            let mask = differing_fields(*a, *b);
            if mask != 0 {
                note(mask, (*a, *b), &mut rows);
            }
        }
    } else {
        // Group by (background, index); inside a group the order is each engine's own.
        let mut groups: HashMap<(bool, i32), (Vec<Key>, Vec<Key>)> = HashMap::new();
        for key in left {
            groups.entry((key.9, key.4)).or_default().0.push(*key);
        }
        for key in right {
            groups.entry((key.9, key.4)).or_default().1.push(*key);
        }
        for (ours, theirs) in groups.values() {
            for (a, b) in ours.iter().zip(theirs) {
                let mask = differing_fields(*a, *b);
                if mask != 0 {
                    note(mask, (*a, *b), &mut rows);
                }
            }
            if ours.len() > theirs.len() {
                diff.unmatched_zaxnes += ours.len() - theirs.len();
                for key in &ours[theirs.len()..] {
                    rows.push(key.3);
                }
            } else {
                diff.unmatched_fceux += theirs.len() - ours.len();
                for key in &theirs[ours.len()..] {
                    rows.push(key.3);
                }
            }
        }
    }

    rows.sort_unstable();
    rows.dedup();
    diff.rows = rows;
    diff.zaxnes_rows = background_rows(left);
    diff.fceux_rows = background_rows(right);
    diff.oam_rotation = oam_rotation(left, right);
    diff
}

/// The `k` in 1..64 for which every one of FCEUX's sprite records becomes one of Zaxnes's once its
/// OAM number `n` is read as `(n + k) mod 64`, when the sprites differ at all and such a `k`
/// exists.
///
/// This exists because of what the measurement kept finding: pairs of sprite records carrying the
/// *same* planes, position, palette and flags on both engines but a different `index`, the
/// difference always a constant. That is not something a capture can get wrong — both engines walk
/// OAM entries 0 to 63 and number the record after the entry (REQ-CORE-043) — it means the two
/// OAM arrays hold the same sprites at different offsets, which happens when the `$4014` transfer
/// starts at a different `$2003`. Naming it keeps those frames out of the capture's account.
#[must_use]
pub fn oam_rotation(left: &[Key], right: &[Key]) -> Option<i32> {
    let sprites =
        |records: &[Key]| -> Vec<Key> { records.iter().copied().filter(|key| !key.9).collect() };
    let (ours, theirs) = (sprites(left), sprites(right));
    if ours.is_empty() || ours.len() != theirs.len() {
        return None;
    }
    // Nothing to explain when the sprites already agree.
    let mine: std::collections::HashSet<Key> = ours.iter().copied().collect();
    if theirs.iter().all(|key| mine.contains(key)) {
        return None;
    }
    (1..64).find(|&k| {
        theirs.iter().all(|key| {
            let mut moved = *key;
            // An 8 x 16 sprite's lower half is numbered 64 + n and moves with its upper half;
            // the widescreen extension's second page (128 up) is not the console's OAM.
            moved.4 = match key.4 {
                n @ 0..64 => (n + k) % 64,
                n @ 64..128 => 64 + ((n - 64 + k) % 64),
                n => n,
            };
            mine.contains(&moved)
        })
    })
}

/// One record as a line short enough to put two of them side by side.
#[must_use]
pub fn describe(key: Key) -> String {
    format!(
        "{} y={:>4} x={:>4} i={:>5} pal={} flags={}{}{} p0={:016x} p1={:016x}",
        if key.9 { "bg " } else { "spr" },
        key.3,
        key.2,
        key.4,
        key.8,
        u8::from(key.5),
        u8::from(key.6),
        u8::from(key.7),
        key.0,
        key.1,
    )
}

/// The background rows of a record list, as `(y, how many records)` sorted by `y`.
#[must_use]
pub fn background_rows(records: &[Key]) -> Vec<(i16, usize)> {
    let mut counts: HashMap<i16, usize> = HashMap::new();
    for key in records.iter().filter(|key| key.9) {
        *counts.entry(key.3).or_insert(0) += 1;
    }
    let mut rows: Vec<(i16, usize)> = counts.into_iter().collect();
    rows.sort_unstable();
    rows
}

/// How many records appear in both lists with every field equal, counted with multiplicity.
#[must_use]
pub fn multiset_intersection(left: &[Key], right: &[Key]) -> usize {
    let mut counts: HashMap<Key, i32> = HashMap::new();
    for key in left {
        *counts.entry(*key).or_insert(0) += 1;
    }
    let mut same = 0;
    for key in right {
        if let Some(slot) = counts.get_mut(key) {
            if *slot > 0 {
                *slot -= 1;
                same += 1;
            }
        }
    }
    same
}
