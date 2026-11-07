//! No background row at the split line of Super Mario Bros. 3 (AMD-021 R2-005). Needs the
//! user's own dump: set `ZAXNES_ROMS` to the folder that holds it.

mod common;

use std::collections::BTreeSet;

use common::machine_with;

const SMB3: &str = "Super Mario Bros. 3 (USA).nes";
const ROWS_WITHOUT_SPLIT: usize = 30;
const SPLIT_LINE: i32 = 193;

/// The smallest distance allowed between two emitted background rows, which **depends on which
/// `$2007` rule the build ships** (Q-137, Q-139).
///
/// With `ppu-2007-render-read` on, the banner's first row is emitted on line 194 — the line that
/// follows the split, which is R2-005 criterion 2's own "à la ligne suivante" — and the pair two
/// lines apart is then lines 192 and 194, neither of them the split line. That is the bound lot
/// MF-4e relaxed from 3 to 2, on 10 of 5 000 frames, and Q-137 asks R0 for it by name.
///
/// With the feature **off**, which is `default` since lot MF-4e-gate, the geometry is `11407930`'s
/// again: the row arrives on line 195, two lines after the split, and **nothing in the run comes
/// within 3**. So the relaxation costs nothing in the build the owner runs, and the bound of a P1
/// requirement's restatement is only ever relaxed in a build asked for by name.
const SMALLEST_GAP: i32 = if cfg!(feature = "ppu-2007-render-read") {
    2
} else {
    3
};

#[test]
#[ignore = "needs a user ROM: set ZAXNES_ROMS to the folder holding Super Mario Bros. 3 (USA).nes"]
fn amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line() {
    let Ok(folder) = std::env::var("ZAXNES_ROMS") else {
        eprintln!("skipped: ZAXNES_ROMS is not set");
        return;
    };
    let Ok(image) = std::fs::read(std::path::Path::new(&folder).join(SMB3)) else {
        eprintln!("skipped: {SMB3} not found in {folder}");
        return;
    };

    let mut core = machine_with(&image, SMB3);
    let mut rows_per_frame = [0usize; ROWS_WITHOUT_SPLIT + 8];
    let mut worst_rows = 0;
    let mut worst_frame = -1;
    let mut at_split_line = 0;
    let mut smallest_gap = i32::MAX;
    for frame in 0..5_000 {
        core.run_frame(0);
        let ys: BTreeSet<i32> = core
            .frame_batch()
            .records
            .iter()
            .filter(|record| record.background)
            .map(|record| i32::from(record.y))
            .collect();
        // REQ-CORE-038: the row captured on line L carries y = L - 1.
        if ys.contains(&(SPLIT_LINE - 1)) {
            at_split_line += 1;
        }

        let mut previous = i32::MIN / 2;
        for &y in &ys {
            smallest_gap = smallest_gap.min(y - previous);
            previous = y;
        }

        rows_per_frame[ys.len().min(rows_per_frame.len() - 1)] += 1;
        if ys.len() > worst_rows {
            worst_rows = ys.len();
            worst_frame = frame;
        }
    }

    // The criterion itself: the split line never emits a row.
    assert_eq!(0, at_split_line);

    // **The banner side, and the bound that depends on the build** (Q-137, Q-139; see
    // `SMALLEST_GAP`). This assertion is not the criterion — `at_split_line` above is — it is a
    // restatement of it in terms of the geometry the rows end up with, and that geometry is
    // exactly what lot MF-4e's rule changed and lot MF-4e-gate put behind a feature.
    //
    // R2-005 criterion 2's second half reads "la première rangée du bandeau est émise **à la ligne
    // suivante**". Without the rule the banner's first row is emitted on line **195**, two lines
    // after the split: a `$2007` access during rendering moves `v` by the PPUCTRL step, so the
    // borrowed Y increment the hardware performs is missing and the row phase arrives late. With
    // the rule on, that row is emitted on line **194** — the line that follows the split, which is
    // what the sentence asks for.
    //
    // The pair that is then two lines apart is lines **192 and 194**, and **neither of them is the
    // split line** (193); the criterion is untouched. It appears on **10 of the 5 000 frames**, and
    // they are the same ten the `ROWS_WITHOUT_SPLIT + 1` bound below already carves out as the grid
    // phase of Q-059 — their row sets are `{-1, 7, …, 183, 191, 193, 201, …, 233}`, 31 rows.
    // Nothing else in the run comes within 7.
    //
    // The row count is the same either way: 960 captured tiles over 60 frames of golden-core,
    // against the 928 of the variant that let the increment reach neither row field and so **lost**
    // this row rather than moving it. The two bounds below are untouched in both builds.
    assert!(
        smallest_gap >= SMALLEST_GAP,
        "two rows only {smallest_gap} lines apart, under {SMALLEST_GAP}"
    );
    // No frame carries more than one row above the thirty of a split-free image, and the
    // frames that do are only the ten of the grid phase (Q-059).
    assert!(
        worst_rows <= ROWS_WITHOUT_SPLIT + 1,
        "frame {worst_frame} emitted {worst_rows} rows"
    );
    assert!(
        rows_per_frame[ROWS_WITHOUT_SPLIT + 1] <= 10,
        "{} frames exceeded {ROWS_WITHOUT_SPLIT} rows",
        rows_per_frame[ROWS_WITHOUT_SPLIT + 1]
    );
}
