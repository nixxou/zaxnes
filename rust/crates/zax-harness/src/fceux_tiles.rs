//! `fceux-tiles`: the measuring stick of lot FCEUX-6 — where FCEUX's **tile records** differ from
//! `zax-nes`'s, field by field, and whether the difference is the capture's or the emulation's.
//!
//! Lot FCEUX-1 could only say *how many* records matched on one ROM at one frame. That is not
//! enough to finish the capture: a record that differs differs in a particular field, on a
//! particular row, for a particular reason, and the only way to fix the capture is to be told
//! which. So this runs both engines side by side in one process and reports, per ROM and per
//! frame:
//!
//! * the record count on each side;
//! * the records identical on **all ten fields** — in order (which is what
//!   [`zax_core_api::FrameBatch`] promises the renderer) and as a multiset (which is what the mesh
//!   builder would accept);
//! * for every differing record the set of fields that differ, with the record's row and column;
//! * the **class** of the frame:
//!   * `identical` — nothing differs;
//!   * `capture` — FCEUX's 256×240 picture for that frame is *identical* to Zaxnes's, so the two
//!     machines agree on what is on screen and only the capture disagrees. **These are ours to
//!     fix.**
//!   * `emulation` — the pictures already differ, so the two cores are not showing the same frame.
//!     A record difference there is FCEUX's emulation against Zaxnes's and out of this lot's
//!     scope (lot FCEUX-1 step 5 measured that disagreement: of 113 public ROMs that report a
//!     verdict, Zaxnes passes 95 and FCEUX 58).
//!
//! The class is the single most important column. Without it every table mixes two unrelated
//! things and no amount of work on the capture can make the numbers reach 100 %.
//!
//! One FCEUX per process is a hard rule (its machine is in file-scope globals), and one public ROM
//! takes the engine down (`other/oam3.nes`, lot FCEUX-1 §7), so a directory sweep gives each ROM
//! its own child process exactly as `fceux-run` does.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use zax_core_api::TileRecord;
use zax_fceux::{Fceux, RamInit, Tile};
use zax_nes::rom::InesHeader;
use zax_nes::NesCore;

use crate::golden_core::{collect_roms, hex, relative_name};
use crate::HarnessError;

pub const USAGE: &str = "zax-harness fceux-tiles <roms dir|rom.nes> <out.jsonl> [options]
    Runs both engines over every ROM and compares their tile records field by field, frame by
    frame. Needs the `fceux-core` feature and a built fork.
      --frames a,b,c    the frames to compare (default 60,300,1200,3000)
      --profiles-only   only the ROMs this tool knows a 3dn profile for
      --in-process      do not isolate each ROM in a child process
      --detail          print the differing fields and rows of every frame that differs
      --records <n>     print the first n differing record pairs of each frame, field by field
      --ram zero|default|ff   what FCEUX's power-on writes into the uninitialised memory
                        (default: zero, which is what zax-nes does -- see the module docs)";

/// The frames the baseline table of lot FCEUX-6 step 1 uses on the originals.
const DEFAULT_FRAMES: [u32; 4] = [60, 300, 1200, 3000];

/// How many distinct rows of differing records a row of the report names before it gives up and
/// just counts them. The point is to recognise a pattern (a status bar, a parallax split), not to
/// list 240 lines.
const MAX_ROWS_LISTED: usize = 12;

// ------------------------------------------------------------------------------------------------
// The ROM → 3dn profile map
// ------------------------------------------------------------------------------------------------

/// Which 3dn profile of `assets-neutral/profiles.json` (edition `full`) belongs to an original of
/// `G:/3DSen/test-originals`, matched on the dump's name.
///
/// The application has no automatic mapping — a profile is a per-game choice the user makes
/// (`zax-app`'s `choose_profile_for_rom`) — so this table is the comparison's own, and it is
/// listed in `docs/progress/FCEUX-6.md` so that a reader can check it. Each entry is a prefix of
/// the dump's file name, which is enough: the 98 dumps are named
/// `<title> [<region>][<flags>].nes`.
///
/// Three entries are not simply the game's own name, and each is a translated title of a game the
/// catalogue holds under its western one:
///
/// * `Akumajou Dracula` is *Castlevania*;
/// * `Rockman <n>` is *Mega Man <n>*, and `Rockman` alone is *Mega Man 1*;
/// * `Chou-WS-M` is *Chou Wakusei Senki: `MetaFight`*, the Japanese *Blaster Master* (and its header
///   agrees: MMC1, 8 PRG banks, 16 CHR banks, exactly the western dumps');
/// * `Takahashi Meijin no Bouken-j` is *Takahashi Meijin no Bouken-jima*, the Japanese *Adventure
///   Island*, and `Kiddy Sun in Fantasia` is an Asian hack of it (CNROM, 2 PRG, 4 CHR, the same
///   shape as the two Adventure Island dumps).
///
/// The originals with **no** profile in the catalogue are *Mega Man 4*, *Mega Man 5*, *Mega Man 6*
/// (and the `Rockman 4/5/6` dumps of the same games) and *Metal Force*; they are compared as
/// field-identity only.
const PROFILES: [(&str, &str); 19] = [
    ("Adventure Island", "Adventure Island"),
    ("Takahashi Meijin no Bouken-j", "Adventure Island"),
    ("Kiddy Sun in Fantasia", "Adventure Island"),
    ("Akumajou Dracula", "Castlevania"),
    ("Castlevania", "Castlevania"),
    ("Excitebike", "Excitebike"),
    ("Final Fantasy", "Final Fantasy"),
    ("Blaster Master", "Blaster Master"),
    ("Chou-WS-M", "Blaster Master"),
    ("Mega Man 3", "Mega Man 3"),
    ("Rockman 3", "Mega Man 3"),
    ("Mega Man 2", "Mega Man 2"),
    ("Rockman 2", "Mega Man 2"),
    ("Metroid", "Metroid"),
    ("Ninja Gaiden III", "Ninja Gaiden III"),
    ("Ninja Ryuuken Den III", "Ninja Gaiden III"),
    ("Super Mario Bros. 3", "Super Mario Bros. 3"),
    ("Super Mario Bros.", "Super Mario Bros."),
    ("Zelda II", "Zelda II: The Adventure of Link"),
];

/// The profile of the dump named `rom` (a path relative to the ROM root), or `None`.
///
/// The longest matching prefix wins, so that *Super Mario Bros. 3* is not taken for *Super Mario
/// Bros.* and *Mega Man 3* is not taken for *Mega Man*. `Mega Man` and `Rockman` alone are
/// *Mega Man 1*, and are matched last for the same reason.
#[must_use]
pub fn profile_for(rom: &str) -> Option<&'static str> {
    let base = Path::new(rom).file_name().map_or_else(
        || rom.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let best = PROFILES
        .iter()
        .filter(|(prefix, _)| base.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len());
    if let Some((_, title)) = best {
        return Some(title);
    }
    // `Mega Man [` / `Rockman [` — the first game, whose dumps carry no number at all.
    (base.starts_with("Mega Man [") || base.starts_with("Rockman [")).then_some("Mega Man 1")
}

// ------------------------------------------------------------------------------------------------
// Options
// ------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Options {
    pub roms: String,
    pub out: String,
    pub frames: Vec<u32>,
    pub profiles_only: bool,
    pub in_process: bool,
    pub detail: bool,
    /// How many differing record pairs of each frame to print in full. 0 prints none.
    pub records: usize,
    /// What FCEUX's power-on writes into the work RAM **and the CHR-RAM**.
    pub ram: RamInit,
}

impl Options {
    /// Parses `<roms> <out.jsonl> [options]`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] on a missing argument or an unparsable frame list.
    pub fn parse(args: &[String]) -> Result<Self, HarnessError> {
        let usage = || HarnessError::Usage(USAGE.to_owned());
        let [roms, out, rest @ ..] = args else {
            return Err(usage());
        };
        let mut options = Self {
            roms: roms.clone(),
            out: out.clone(),
            frames: DEFAULT_FRAMES.to_vec(),
            profiles_only: false,
            in_process: false,
            detail: false,
            records: 0,
            ram: RamInit::Zero,
        };
        let mut rest = rest.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--profiles-only" => options.profiles_only = true,
                "--in-process" => options.in_process = true,
                "--detail" => options.detail = true,
                "--ram" => {
                    let which = rest.next().ok_or_else(usage)?;
                    options.ram = match which.as_str() {
                        "zero" => RamInit::Zero,
                        "default" => RamInit::Default,
                        "ff" => RamInit::Ff,
                        _ => return Err(usage()),
                    };
                }
                "--records" => {
                    let count = rest.next().ok_or_else(usage)?;
                    options.records = count.parse().map_err(|_| {
                        HarnessError::Usage(format!(
                            "{count:?} is not a count
{USAGE}"
                        ))
                    })?;
                }
                "--frames" => {
                    let list = rest.next().ok_or_else(usage)?;
                    options.frames = list
                        .split(',')
                        .map(|part| part.trim().parse::<u32>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| {
                            HarnessError::Usage(format!("{list:?} is not a frame list\n{USAGE}"))
                        })?;
                    options.frames.sort_unstable();
                    options.frames.dedup();
                }
                _ => return Err(usage()),
            }
        }
        if options.frames.is_empty() {
            return Err(usage());
        }
        Ok(options)
    }
}

// ------------------------------------------------------------------------------------------------
// The comparison of one frame
// ------------------------------------------------------------------------------------------------

/// The ten fields of a record, flattened so that two records can be compared and the differing
/// fields named. The order is `TileRecord`'s own.
const FIELDS: [&str; 10] = [
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
type Key = (u64, u64, i16, i16, i32, bool, bool, bool, u8, bool);

const fn key_of_record(record: &TileRecord) -> Key {
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

const fn key_of_tile(tile: &Tile) -> Key {
    (
        tile.plane0,
        tile.plane1,
        tile.x,
        tile.y,
        tile.index,
        tile.mirror_h,
        tile.mirror_v,
        tile.hidden,
        tile.palette,
        tile.background,
    )
}

/// Which of the ten fields differ between two records, as a bitmask over [`FIELDS`].
fn differing_fields(left: Key, right: Key) -> u16 {
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

fn field_names(mask: u16) -> String {
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
/// strictest. When the lengths differ, the records are grouped by `(background, index)` and zipped
/// inside each group, with the leftovers counted as unmatched — a deterministic alignment that
/// still names the differing fields instead of reporting a bare count.
#[must_use]
pub fn compare_frame(zaxnes: &[TileRecord], fceux: &[Tile], pixels_identical: bool) -> FrameDiff {
    compare_frame_with(zaxnes, fceux, pixels_identical, 0)
}

/// [`compare_frame`], keeping the first `pairs_wanted` differing record pairs in full.
#[must_use]
pub fn compare_frame_with(
    zaxnes: &[TileRecord],
    fceux: &[Tile],
    pixels_identical: bool,
    pairs_wanted: usize,
) -> FrameDiff {
    compare_frame_full(zaxnes, fceux, pixels_identical, false, pairs_wanted)
}

/// [`compare_frame_with`], told as well whether the frame's picture is a single colour.
#[must_use]
pub fn compare_frame_full(
    zaxnes: &[TileRecord],
    fceux: &[Tile],
    pixels_identical: bool,
    pixels_uniform: bool,
    pairs_wanted: usize,
) -> FrameDiff {
    let left: Vec<Key> = zaxnes.iter().map(key_of_record).collect();
    let right: Vec<Key> = fceux.iter().map(key_of_tile).collect();

    let mut diff = FrameDiff {
        zaxnes: left.len(),
        fceux: right.len(),
        pixels_identical,
        pixels_uniform,
        ..FrameDiff::default()
    };

    diff.same_in_order = left.iter().zip(&right).filter(|(a, b)| a == b).count();
    diff.same_as_multiset = multiset_intersection(&left, &right);

    let mut rows = Vec::new();
    let mut note = |mask: u16, pair: (Key, Key), rows: &mut Vec<i16>| {
        *diff.by_fields.entry((pair.0 .9, mask)).or_insert(0) += 1;
        rows.push(pair.0 .3);
        if diff.pairs.len() < pairs_wanted {
            diff.pairs.push((describe(pair.0), describe(pair.1)));
        }
    };

    if left.len() == right.len() {
        for (a, b) in left.iter().zip(&right) {
            let mask = differing_fields(*a, *b);
            if mask != 0 {
                note(mask, (*a, *b), &mut rows);
            }
        }
    } else {
        // Group by (background, index); inside a group the order is each engine's own.
        let mut groups: HashMap<(bool, i32), (Vec<Key>, Vec<Key>)> = HashMap::new();
        for key in &left {
            groups.entry((key.9, key.4)).or_default().0.push(*key);
        }
        for key in &right {
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
    diff.zaxnes_rows = background_rows(&left);
    diff.fceux_rows = background_rows(&right);
    diff.oam_rotation = oam_rotation(&left, &right);
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
fn oam_rotation(left: &[Key], right: &[Key]) -> Option<i32> {
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
fn describe(key: Key) -> String {
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
fn background_rows(records: &[Key]) -> Vec<(i16, usize)> {
    let mut counts: HashMap<i16, usize> = HashMap::new();
    for key in records.iter().filter(|key| key.9) {
        *counts.entry(key.3).or_insert(0) += 1;
    }
    let mut rows: Vec<(i16, usize)> = counts.into_iter().collect();
    rows.sort_unstable();
    rows
}

/// How many records appear in both lists with every field equal, counted with multiplicity.
fn multiset_intersection(left: &[Key], right: &[Key]) -> usize {
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

// ------------------------------------------------------------------------------------------------
// Running one ROM
// ------------------------------------------------------------------------------------------------

/// Everything one ROM's comparison found.
#[derive(Debug, Clone, Default)]
pub struct RomDiff {
    pub frames: Vec<(u32, FrameDiff)>,
    /// The first frame of the whole run whose pictures differ, if any — what step 6 lists for the
    /// emulation-class ROMs.
    pub first_pixel_diff: Option<u32>,
    pub stopped_at: Option<String>,
}

/// Runs both engines over `image` and compares at each frame of `frames`.
///
/// The two engines are driven exactly as `golden-core` and `fceux-run` drive them — the file stem
/// as Zaxnes's name and the basename as FCEUX's, FCEUX's deterministic default RAM fill, a reset
/// after loading, no button pressed — so a difference here is the engines' and not the harness's.
/// The entries where the two engines' 32-byte palettes differ, as `(index, zaxnes, fceux)`.
///
/// Both sides are plain NES colour indices 0 to 63 by the time they get here: `zax-nes`'s
/// `Ppu2C02::snapshot_palette` returns the shown palette with the mirrors of `$3F10`, `$3F14`,
/// `$3F18` and `$3F1C` filled in, and the fork's capture does the same and masks to `0x3F`
/// (`zax/zaxtiles.cpp`), so neither carries a priority bit and the two are comparable.
fn palette_differences(zaxnes: &[u8], fceux: &[u8]) -> Vec<(usize, u8, u8)> {
    zaxnes
        .iter()
        .zip(fceux.iter())
        .enumerate()
        .filter(|(_, (ours, theirs))| ours != theirs)
        .map(|(index, (ours, theirs))| (index, *ours, *theirs))
        .collect()
}

fn compare_rom(
    engine: &mut Fceux,
    rom: &str,
    image: &[u8],
    frames: &[u32],
    pairs_wanted: usize,
    ram: RamInit,
) -> Result<RomDiff, String> {
    let stem = Path::new(rom)
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let base = Path::new(rom).file_name().map_or_else(
        || rom.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );

    let mut zaxnes = NesCore::new();
    if !zaxnes.load_rom(image, &stem).success {
        return Err("Zaxnes refused the image".to_owned());
    }
    zaxnes.reset();

    engine.close_rom();
    engine.set_ram_init(ram).map_err(|e| e.to_string())?;
    engine.load_rom(image, &base).map_err(|e| e.to_string())?;
    engine.set_tile_capture(true);
    engine.reset().map_err(|e| e.to_string())?;

    let last = frames.iter().copied().max().unwrap_or(1);
    let mut ours = Vec::new();
    let mut theirs = Vec::new();
    let mut result = RomDiff::default();

    for frame in 1..=last {
        zaxnes.run_frame(0);
        if let Err(error) = engine.set_input(0, 0).and_then(|()| engine.run_frame()) {
            result.stopped_at = Some(format!("frame {frame}: {error}"));
            break;
        }

        zaxnes.pixel_frame_indices_into(&mut ours);
        if engine.frame_indices_into(&mut theirs).is_err() {
            result.stopped_at = Some(format!("frame {frame}: FCEUX gave no picture"));
            break;
        }
        let pixels_identical = ours == theirs;
        if !pixels_identical && result.first_pixel_diff.is_none() {
            result.first_pixel_diff = Some(frame);
        }

        if !frames.contains(&frame) {
            continue;
        }

        let capture = engine.take_tiles().unwrap_or_default();
        let mut diff = compare_frame_full(
            zaxnes.tile_records(),
            &capture.records,
            pixels_identical,
            ours.first()
                .is_some_and(|first| ours.iter().all(|p| p == first)),
            pairs_wanted,
        );
        // The palette travels with the records in a frame batch (REQ-CORE-045) and the 3D
        // renderer reads both, so a comparison that stops at the records is incomplete -- lot
        // FCEUX-6 step 8. Taken here because it is the only place that holds both engines.
        diff.palette_differences =
            palette_differences(&zaxnes.frame_batch().palette, &capture.palette);
        result.frames.push((frame, diff));
    }

    Ok(result)
}

/// The JSON row of one ROM.
fn row_for(engine: &mut Fceux, rom: &str, image: &[u8], options: &Options) -> Value {
    let mut row = Map::new();
    row.insert("rom".into(), json!(rom));
    row.insert("rom_sha256".into(), json!(hex(&Sha256::digest(image))));
    row.insert(
        "mapper".into(),
        InesHeader::try_parse(image).map_or(Value::Null, |header| json!(header.mapper)),
    );
    row.insert("profile".into(), json!(profile_for(rom)));

    match compare_rom(
        engine,
        rom,
        image,
        &options.frames,
        options.records,
        options.ram,
    ) {
        Err(error) => {
            row.insert("load_ok".into(), json!(false));
            row.insert("load_error".into(), json!(error));
        }
        Ok(result) => {
            row.insert("load_ok".into(), json!(true));
            row.insert("first_pixel_diff".into(), json!(result.first_pixel_diff));
            if let Some(stopped) = result.stopped_at {
                row.insert("stopped_at".into(), json!(stopped));
            }
            let mut frames = Map::new();
            for (frame, diff) in &result.frames {
                frames.insert(
                    frame.to_string(),
                    json!({
                        "zaxnes": diff.zaxnes,
                        "fceux": diff.fceux,
                        "same_in_order": diff.same_in_order,
                        "same_as_multiset": diff.same_as_multiset,
                        "pixels_identical": diff.pixels_identical,
                        "pixels_uniform": diff.pixels_uniform,
                        "class": diff.class(),
                        "fields": diff.fields_summary(),
                        "rows": diff.rows,
                        "unmatched_zaxnes": diff.unmatched_zaxnes,
                        "unmatched_fceux": diff.unmatched_fceux,
                        "zaxnes_rows": rows_text(&diff.zaxnes_rows),
                        "fceux_rows": rows_text(&diff.fceux_rows),
                        "oam_rotation": diff.oam_rotation,
                        "palette": diff.palette_summary(),
                        "cause": diff.cause(*frame, result.first_pixel_diff),
                        "pairs": diff.pairs.iter().map(|(ours, theirs)| json!([ours, theirs])).collect::<Vec<_>>(),
                    }),
                );
            }
            row.insert("frames".into(), Value::Object(frames));
        }
    }
    Value::Object(row)
}

// ------------------------------------------------------------------------------------------------
// The command
// ------------------------------------------------------------------------------------------------

/// The command line, parsed, run and printed. Its own function for the same reason
/// `fceux_run::command` is one: `main`'s `run` is at clippy's line budget.
///
/// # Errors
///
/// Whatever [`Options::parse`] or [`run`] returns.
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let options = Options::parse(args)?;
    print!("{}", run(&options)?);
    Ok(())
}

/// Runs the comparison and writes the rows, then prints the table.
///
/// # Errors
///
/// [`HarnessError`] when the ROMs cannot be walked, a file cannot be read or the output written,
/// or the FCEUX engine cannot be created.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let root = Path::new(&options.roms);
    let rows = if root.is_dir() && !options.in_process {
        rows_isolated(options, root)?
    } else {
        rows_in_process(options, root)?
    };

    let out = Path::new(&options.out);
    let mut writer = BufWriter::new(fs::File::create(out)?);
    for row in &rows {
        serde_json::to_writer(&mut writer, row).map_err(std::io::Error::from)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;

    Ok(report(&rows, options))
}

/// One child process per ROM, so that a ROM FCEUX cannot survive costs one row instead of the
/// whole sweep (lot FCEUX-1 §7: `other/oam3.nes` corrupts the heap and then *hangs* rather than
/// dying, so the child needs a deadline as well as isolation).
fn rows_isolated(options: &Options, roms: &Path) -> Result<Vec<Value>, HarnessError> {
    let mut files = Vec::new();
    collect_roms(roms, &mut files)?;
    files.sort_by_key(|file| relative_name(roms, file));

    let executable = std::env::current_exe()?;
    let scratch = Path::new(&options.out).with_extension("one.jsonl");
    let frames: Vec<String> = options.frames.iter().map(ToString::to_string).collect();
    let budget = Duration::from_secs(
        120 + u64::from(options.frames.iter().copied().max().unwrap_or(60)) / 5,
    );

    let mut out_rows = Vec::new();
    for file in &files {
        let name = relative_name(roms, file);
        if options.profiles_only && profile_for(&name).is_none() {
            continue;
        }

        let status = Command::new(&executable)
            .arg("fceux-tiles")
            .arg(file)
            .arg(&scratch)
            .arg("--frames")
            .arg(frames.join(","))
            .arg("--in-process")
            .arg("--ram")
            .arg(ram_name(options.ram))
            .stdout(std::process::Stdio::null())
            .spawn()
            .and_then(|child| crate::fceux_run::wait_or_kill(child, budget));

        let row = match (status, fs::read_to_string(&scratch)) {
            (Ok(status), Ok(text)) if status.success() => text
                .lines()
                .find(|line| !line.trim().is_empty())
                .and_then(|line| serde_json::from_str::<Value>(line).ok())
                .map(|mut row| {
                    if let Value::Object(fields) = &mut row {
                        fields.insert("rom".into(), json!(name));
                        fields.insert("profile".into(), json!(profile_for(&name)));
                    }
                    row
                }),
            _ => None,
        };

        out_rows.push(row.unwrap_or_else(|| {
            json!({
                "rom": name,
                "profile": profile_for(&name),
                "load_ok": false,
                "engine_crashed": true,
                "load_error": "the FCEUX engine took its process down or stopped answering",
            })
        }));
    }

    let _ = fs::remove_file(&scratch);
    Ok(out_rows)
}

fn rows_in_process(options: &Options, roms: &Path) -> Result<Vec<Value>, HarnessError> {
    let (root, mut files) = if roms.is_dir() {
        let mut found = Vec::new();
        collect_roms(roms, &mut found)?;
        (roms.to_path_buf(), found)
    } else {
        let root = roms.parent().unwrap_or(Path::new(".")).to_path_buf();
        (root, vec![roms.to_path_buf()])
    };
    files.sort_by_key(|file| relative_name(&root, file));

    let mut engine = Fceux::new().map_err(|error| HarnessError::Usage(error.to_string()))?;
    let mut out_rows = Vec::new();
    for file in &files {
        let name = relative_name(&root, file);
        if options.profiles_only && profile_for(&name).is_none() {
            continue;
        }
        let image = fs::read(file)?;
        out_rows.push(row_for(&mut engine, &name, &image, options));
    }
    Ok(out_rows)
}

/// The printed table: one line per ROM and frame that is not identical, then the counts per class.
fn report(rows: &[Value], options: &Options) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<52} {:>6} {:>6} {:>7} {:>7} {:>6} {:<10}",
        "ROM", "frame", "zaxnes", "fceux", "same", "px=", "class"
    );

    let mut classes: HashMap<&str, usize> = HashMap::new();
    let mut causes: HashMap<String, usize> = HashMap::new();
    let mut roms_all_identical = 0;
    let mut roms_total = 0;
    let mut crashed = 0;
    let mut refused = 0;
    let mut emulation_first: Vec<(String, Value)> = Vec::new();

    for row in rows {
        let name = row.get("rom").and_then(Value::as_str).unwrap_or("?");
        if row.get("engine_crashed").is_some() {
            crashed += 1;
            let _ = writeln!(
                out,
                "{name:<52} {:>6} the FCEUX engine did not come back",
                "-"
            );
            continue;
        }
        if row.get("load_ok") != Some(&Value::Bool(true)) {
            refused += 1;
            let _ = writeln!(
                out,
                "{name:<52} {:>6} refused: {}",
                "-",
                row.get("load_error").and_then(Value::as_str).unwrap_or("?")
            );
            continue;
        }
        roms_total += 1;
        let all_identical = rom_lines(&mut out, row, options, &mut classes, &mut causes);
        if all_identical {
            roms_all_identical += 1;
        } else if let Some(first) = row.get("first_pixel_diff").filter(|v| !v.is_null()) {
            emulation_first.push((name.to_owned(), first.clone()));
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{roms_all_identical} of {roms_total} ROMs identical at every compared frame\
         {}{}",
        if crashed == 0 {
            String::new()
        } else {
            format!("; {crashed} took the engine down")
        },
        if refused == 0 {
            String::new()
        } else {
            format!("; {refused} refused")
        }
    );
    let mut names: Vec<&&str> = classes.keys().collect();
    names.sort_unstable();
    for class in names {
        let _ = writeln!(out, "  {class}: {}", classes[*class]);
    }
    if !causes.is_empty() {
        // Why each capture-class frame is capture-class without the capture being at fault
        // (`FrameDiff::cause`). The last line is the one that matters: a lot that claims the
        // capture is finished has to show `unexplained: 0`.
        let _ = writeln!(out, "\nCapture-class frames by cause:");
        let mut tally: Vec<(&String, &usize)> = causes.iter().collect();
        tally.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (cause, count) in tally {
            let _ = writeln!(out, "  {cause}: {count}");
        }
        let unexplained: usize = causes
            .iter()
            .filter(|(cause, _)| cause.starts_with("unexplained"))
            .map(|(_, count)| *count)
            .sum();
        let _ = writeln!(out, "  => unexplained: {unexplained}");
    }
    if !emulation_first.is_empty() {
        let _ = writeln!(
            out,
            "\nFirst frame whose pictures differ, for the ROMs that are not identical:"
        );
        for (name, frame) in &emulation_first {
            let _ = writeln!(out, "  {name}: {frame}");
        }
    }
    out
}

/// Writes one line per compared frame of `row` that is not identical, counting the classes as it
/// goes; returns whether every compared frame was identical.
fn rom_lines(
    out: &mut String,
    row: &Value,
    options: &Options,
    classes: &mut HashMap<&'static str, usize>,
    causes: &mut HashMap<String, usize>,
) -> bool {
    let name = row.get("rom").and_then(Value::as_str).unwrap_or("?");
    let empty = Map::new();
    let frames = row
        .get("frames")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let mut all_identical = true;
    for frame in &options.frames {
        let Some(diff) = frames.get(&frame.to_string()) else {
            continue;
        };
        let class = diff.get("class").and_then(Value::as_str).unwrap_or("?");
        *classes.entry(class_name(class)).or_insert(0) += 1;
        if class == "identical" {
            continue;
        }
        all_identical = false;
        let pixels = if diff.get("pixels_identical") == Some(&Value::Bool(true)) {
            "yes"
        } else {
            "no"
        };
        let mut detail = String::new();
        // The cause supersedes the two ad-hoc markers this used to print (`[blank screen]` and
        // `[OAM rotated by k]`): it names the same discriminators and the frame the two pictures
        // first parted company, and it is `unexplained` when none of them accounts for the frame
        // — which is the only value that is a defect (lot FCEUX-6 step 6).
        let cause = diff.get("cause").and_then(Value::as_str).unwrap_or("");
        if !cause.is_empty() {
            let _ = write!(detail, " [{cause}]");
            *causes.entry(cause.to_owned()).or_insert(0) += 1;
        }
        // The differing palette entries, which are not records and so are not in `fields`.
        let palette = diff.get("palette").and_then(Value::as_str).unwrap_or("");
        if !palette.is_empty() {
            let _ = write!(detail, " palette {palette}");
        }
        if options.detail {
            let _ = write!(detail, " @y {}", list(diff.get("rows")));
            let _ = write!(
                detail,
                "\n      zaxnes rows: {}\n      fceux  rows: {}",
                text(diff, "zaxnes_rows"),
                text(diff, "fceux_rows"),
            );
        }
        for pair in diff
            .get("pairs")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_array)
        {
            let _ = write!(
                detail,
                "\n      zaxnes {}\n      fceux  {}",
                pair.first().and_then(Value::as_str).unwrap_or(""),
                pair.get(1).and_then(Value::as_str).unwrap_or(""),
            );
        }
        let _ = writeln!(
            out,
            "{name:<52} {frame:>6} {:>6} {:>6} {:>7} {pixels:>6} {class:<10} {}{detail}",
            number(diff, "zaxnes"),
            number(diff, "fceux"),
            number(diff, "same_in_order"),
            diff.get("fields").and_then(Value::as_str).unwrap_or(""),
        );
    }
    all_identical
}

/// The `--ram` spelling of a fill, for the child process's command line.
const fn ram_name(ram: RamInit) -> &'static str {
    match ram {
        RamInit::Zero => "zero",
        RamInit::Ff => "ff",
        _ => "default",
    }
}

/// A string field of a frame's JSON, or the empty string.
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

/// `(y, count)` pairs as `193x32 202x33`, which is short enough to read in a table row.
fn rows_text(rows: &[(i16, usize)]) -> String {
    rows.iter()
        .map(|(y, count)| format!("{y}x{count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A JSON array as `a,b,c`.
fn list(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map_or_else(String::new, |items| {
            items
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        })
}

/// `class` padded so the summary sorts in the order the classes matter in.
fn class_name(class: &str) -> &'static str {
    match class {
        "identical" => "1 identical",
        "capture" => "2 capture (ours to fix)",
        _ => "3 emulation (FCEUX vs Zaxnes)",
    }
}

fn number(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_map_takes_the_longest_prefix() {
        assert_eq!(
            profile_for("Super Mario Bros. 3 [USA][!!].nes"),
            Some("Super Mario Bros. 3")
        );
        assert_eq!(
            profile_for("Super Mario Bros. [World][!!].nes"),
            Some("Super Mario Bros.")
        );
        assert_eq!(profile_for("Mega Man 3 [USA][!!].nes"), Some("Mega Man 3"));
        assert_eq!(profile_for("Mega Man [USA][!!].nes"), Some("Mega Man 1"));
        assert_eq!(
            profile_for("Rockman [Japan][En][!!].nes"),
            Some("Mega Man 1")
        );
        assert_eq!(
            profile_for("Rockman 3-DWnS [Japan][!!].nes"),
            Some("Mega Man 3")
        );
        assert_eq!(
            profile_for("Akumajou Dracula [Japan][!!].nes"),
            Some("Castlevania")
        );
        assert_eq!(
            profile_for("Chou-WS-M [Japan][!!].nes"),
            Some("Blaster Master")
        );
        // No profile in the catalogue for these.
        assert_eq!(profile_for("Mega Man 4 [USA][!!].nes"), None);
        assert_eq!(profile_for("Metal Force [Korea][Unl][!].nes"), None);
    }

    #[test]
    fn a_frame_with_the_same_records_is_identical() {
        let record = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let tile = Tile {
            plane0: 1,
            plane1: 2,
            x: 3,
            y: 4,
            index: 5,
            mirror_h: false,
            mirror_v: false,
            hidden: false,
            palette: 1,
            background: true,
        };
        let diff = compare_frame(&[record], &[tile], true);
        assert_eq!(diff.class(), "identical");
        assert_eq!(diff.same_in_order, 1);
        assert_eq!(diff.same_as_multiset, 1);
        assert!(diff.by_fields.is_empty());
    }

    #[test]
    fn a_differing_field_is_named_with_its_row() {
        let record = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let tile = Tile {
            plane0: 1,
            plane1: 99,
            x: 3,
            y: 4,
            index: 5,
            mirror_h: false,
            mirror_v: false,
            hidden: false,
            palette: 2,
            background: true,
        };
        let diff = compare_frame(&[record], &[tile], true);
        assert_eq!(diff.class(), "capture");
        assert_eq!(diff.same_in_order, 0);
        assert_eq!(diff.fields_summary(), "bg.plane1+palette:1");
        assert_eq!(diff.rows, vec![4]);
    }

    #[test]
    fn differing_pictures_make_the_frame_an_emulation_difference() {
        let record = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let diff = compare_frame(&[record], &[], false);
        assert_eq!(diff.class(), "emulation");
        assert_eq!(diff.unmatched_zaxnes, 1);
    }

    /// Lot FCEUX-6 step 6: the four verdicts [`FrameDiff::cause`] can reach, and the one that
    /// must stay at zero. A background record that differs on a plane is the capture's and has to
    /// come out `unexplained`, which is what keeps the discriminators from excusing a real bug.
    #[test]
    fn the_cause_names_why_a_capture_class_frame_is_not_the_captures_fault() {
        let background = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let sprite = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, false);
        let tile = |plane1: u64, index: i32, background: bool| Tile {
            plane0: 1,
            plane1,
            x: 3,
            y: 4,
            index,
            mirror_h: false,
            mirror_v: false,
            hidden: false,
            palette: 1,
            background,
        };

        // A background plane that differs on an informative picture: ours, and named so.
        let diff = compare_frame(&[background], &[tile(99, 5, true)], true);
        assert_eq!(diff.cause(300, None), "unexplained");
        // ... and the frame the two pictures first parted company is carried through.
        assert_eq!(diff.cause(300, Some(8)), "unexplained, diverged at 8");
        // A later divergence than this frame is not a divergence yet.
        assert_eq!(diff.cause(300, Some(900)), "unexplained");

        // A sprite in another OAM slot: the machine's state, not the capture's. `oam_rotation` is
        // cleared by hand because a frame holding a *single* sprite is trivially a rotation of
        // itself — the real frames this branch exists for are the ones where no one `k` moves
        // every sprite (`Rockman 4-AY`'s 52 records, `Super Mario Bros.`'s 15).
        let mut diff = compare_frame(&[sprite], &[tile(2, 9, false)], true);
        assert_eq!(diff.fields_summary(), "spr.index:1");
        assert_eq!(diff.oam_rotation, Some(60));
        diff.oam_rotation = None;
        assert_eq!(diff.cause(300, Some(8)), "sprite-oam-slot, diverged at 8");

        // A uniform picture says almost nothing, so neither does a record difference on it.
        let mut diff = compare_frame(&[background], &[tile(99, 5, true)], true);
        diff.pixels_uniform = true;
        assert_eq!(diff.cause(60, Some(4)), "blank-screen, diverged at 4");

        // A whole-array rotation outranks every other reading.
        let mut diff = compare_frame(&[sprite], &[tile(2, 9, false)], true);
        diff.oam_rotation = Some(32);
        assert_eq!(diff.cause(300, Some(7)), "oam-rotated-by-32, diverged at 7");

        // Nothing is said about a frame that is not capture-class.
        let diff = compare_frame(&[background], &[tile(99, 5, true)], false);
        assert_eq!(diff.class(), "emulation");
        assert_eq!(diff.cause(300, Some(8)), "");
    }

    /// Lot FCEUX-6 step 8: the palette travels with the records and the 3D renderer reads both,
    /// so identical records are **not** enough to call a frame identical. *Final Fantasy* at
    /// frame 300 is the case that found it, and these are its numbers.
    #[test]
    fn a_frame_whose_records_match_but_whose_palette_does_not_is_not_identical() {
        let record = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let tile = Tile {
            plane0: 1,
            plane1: 2,
            x: 3,
            y: 4,
            index: 5,
            mirror_h: false,
            mirror_v: false,
            hidden: false,
            palette: 1,
            background: true,
        };

        // Every record equal and the palettes equal: identical, and nothing to explain.
        let mut diff = compare_frame(&[record], &[tile], true);
        assert_eq!(diff.class(), "identical");
        assert_eq!(diff.palette_summary(), "");
        assert_eq!(diff.cause(300, Some(3)), "");

        // The same records, one palette entry apart — Final Fantasy's `$3F0B`, 32 against 0.
        let mut ours = [0u8; 32];
        let mut theirs = [0u8; 32];
        ours[0x0B] = 32;
        theirs[0x0B] = 0;
        diff.palette_differences = palette_differences(&ours, &theirs);
        assert_eq!(diff.palette_summary(), "$3F0B:32/0");
        assert_eq!(diff.class(), "capture");
        assert_eq!(diff.cause(300, Some(3)), "palette-only, diverged at 3");
        // And it is never counted as an unexplained capture bug.
        assert!(!diff.cause(300, Some(3)).starts_with("unexplained"));
    }

    #[test]
    fn identical_palettes_report_no_difference_at_all() {
        let palette: [u8; 32] = std::array::from_fn(|index| u8::try_from(index).unwrap_or(0));
        assert_eq!(palette_differences(&palette, &palette), vec![]);
    }
}
