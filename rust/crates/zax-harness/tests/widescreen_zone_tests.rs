//! The widescreen extension's execution zone, run for real (lots ZR-1 and ZR-2).
//!
//! `acfa6e20` made a code fetch from the zone perform a read of the PPU register its address
//! decodes to, which the hardware does not: the byte comes from the hack pack's code buffer. The
//! games it reached are the MMC3 ones that play DPCM samples while their pack's code runs at a
//! status-bar split, and the picture it broke is the row of ground above Super Mario Bros. 3's
//! status bar, from frame 1 500 on.
//!
//! ZR-1 removed the fetch's own read and ZR-2 the DMA halt's repeats of it, which were 3 166 of
//! 1 500 frames on their own and kept the row broken.
//!
//! Needs the user's own dump: set `ZAXNES_ROMS` to the folder that holds it.

// A test: a failed capture or a missing profile should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use zax_engine3d::settings::RenderAlgorithm;
use zax_harness::render::input_script::InputScript;
use zax_harness::render::profile;
use zax_harness::render::rom::{self, RomRequest};
use zax_nes::ppu::SpriteLimit;

const SMB3: &str = "Super Mario Bros. 3 (USA).nes";

/// R0's script: start through the title screens, then run and fly through world 1-1 so that the
/// status-bar split and the DPCM samples are both live at frame 1 500.
const PRESS: &str = "100-105:start,250-255:start,700-715:right,800-815:up,900-905:a,\
1050-1090:right+b,1091-1110:right+a+b,1111-1151:right+b,1152-1171:right+a+b,1172-1212:right+b,\
1213-1232:right+a+b,1233-1273:right+b,1274-1293:right+a+b,1294-1334:right+b,1335-1354:right+a+b,\
1355-1395:right+b,1396-1415:right+a+b,1416-1456:right+b,1457-1476:right+a+b,1477-1517:right+b";

/// FNV-1a, 64 bits. A golden over raw colour indices wants no encoder and no crate between the
/// picture and the number.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The colour-index picture of Super Mario Bros. 3 after `frames` frames, with the advanced
/// widescreen patches and a 128-pixel margin — R0's `render --rom … --advanced --widescreen 128`
/// without the part that needs a GPU.
fn smb3_picture(folder: &str, frames: u32) -> Option<(Vec<u8>, usize)> {
    let path = Path::new(folder).join(SMB3);
    if !path.is_file() {
        eprintln!("skipped: {SMB3} not found in {folder}");
        return None;
    }
    let script = InputScript::parse(PRESS).unwrap();
    let profile = profile::load(None, None).unwrap();
    let scene = rom::capture(&RomRequest {
        rom: &path,
        frames,
        profile: &profile,
        mesh: RenderAlgorithm::Greedy,
        input: &script,
        // The hack pack reads $7CFF to know which of its modes to run.
        pokes: &[(1000, 0x7CFF, 0x01)],
        battery: None,
        dump_frame: None,
        dump_every: 0,
        widescreen: 128,
        widescreen_y: 0,
        builtin_patches: true,
        advanced: true,
        png_nes: None,
        load_state: None,
        movie: None,
        hack_options: None,
        sprite_limit: SpriteLimit::Kept,
        run_ahead: 0,
        save_state: None,
        dump_mem: None,
        trace_ram: None,
        delay_mesh: 0,
    })
    .unwrap();
    Some((scene.nes_picture, scene.nes_picture_height))
}

/// The picture at frame 1 500, pinned — and it is **`c4577878`'s own number** (lot ZR-2).
///
/// `c4577878`, the last build before `acfa6e20`, is the reference these two lots restore. ZR-1
/// pinned a different number here and had to explain why: it had taken away the CPU's own read of
/// the zone address but kept the DMA halt's and the dummy cycles' repeats of it, and MF-4d step 6
/// (`a758f923`, "a fetching PPU moves `v` by a coarse X and a Y at once") then made those repeats
/// move the picture. ZR-2 took the repeats away too — no fetch from the zone reaches the PPU by
/// any road — and with them gone MF-4d step 6 has nothing here to apply to: this picture is the
/// reference's again, byte for byte.
///
/// Measured, not assumed. The number below was read back from a clean build of `c4577878` itself,
/// with this very test file dropped into its tree (its `RomRequest` and `SyntheticScene` are
/// field for field the same), and the frame-1500 `--png-nes` of the two builds is byte-identical
/// as well — two independent composers of the same frame agreeing. Lot ZR-2 step 4 has the table.
///
/// To re-derive it: the `render` command of the lot's brief, then this test's `fnv1a` of the
/// composed colour indices — or simply read the number this test prints when it fails.
const SMB3_FRAME_1500: u64 = 1_834_880_705_716_891_653;

#[test]
#[ignore = "needs a user ROM: set ZAXNES_ROMS to the folder holding Super Mario Bros. 3 (USA).nes"]
fn zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack() {
    let Ok(folder) = std::env::var("ZAXNES_ROMS") else {
        eprintln!("skipped: ZAXNES_ROMS is not set");
        return;
    };
    let Some((picture, height)) = smb3_picture(&folder, 1500) else {
        return;
    };

    assert_eq!(240, height, "the picture is 240 rows");
    assert_eq!(
        512 * 240,
        picture.len(),
        "128 pixels of margin on each side of the 256"
    );
    assert_eq!(
        SMB3_FRAME_1500,
        fnv1a(&picture),
        "the frame-1500 picture moved; see this test's note before changing the golden"
    );
}
