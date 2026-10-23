//! No background row at the split line of Super Mario Bros. 3 (AMD-021 R2-005). Needs the
//! user's own dump: set `ZAXNES_ROMS` to the folder that holds it.

mod common;

use std::collections::BTreeSet;

use common::machine_with;

const SMB3: &str = "Super Mario Bros. 3 (USA).nes";
const ROWS_WITHOUT_SPLIT: usize = 30;
const SPLIT_LINE: i32 = 193;

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
        core.run_frame();
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
    // The same fact seen from the banner side: the first banner row (line 195) is no longer
    // two lines after a row of the split line.
    assert!(
        smallest_gap >= 3,
        "two rows only {smallest_gap} lines apart"
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
