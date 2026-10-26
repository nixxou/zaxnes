# Adventure Island widescreen — handoff (work in progress)

Branch: `feat/widescreen-adventure-island` (from `dev`), temporary, same method as the Zelda II /
Mega Man 2 / Super Mario Bros. widescreen work. Read first: `rust/docs/WIDESCREEN-EXTENSION.md`,
`rust/docs/hacks/smb1/WIDESCREEN-HACK.md` (closest model: boot window + added PRG bank, advanced mode),
then `rust/docs/hacks/adventure_island/WIDESCREEN-HACK.md` (this hack, complete).

## ROM
*Adventure Island (USA)*, CNROM (mapper 3), 32 KiB PRG + 32 KiB CHR, data CRC-32 `F8A713BE`.
The owner's dump (NES 2.0 header) and the disassembly's rebuild have the same data. Disassembly:
https://github.com/cyneprepou4uk/NES-Games-Disassembly/tree/main/Adventure%20Island (rebuild:
`handoff/tools` has no prep script; strip the listing prefix with the regex of `preparations.lua`,
then `ca65 copy_bank_FF.asm` / `ld65 -C ld65.cfg`; header.bin + prg + CHR_ROM.chr = SHA-1 22EE75B8...).
No ROM is committed: get it from the owner.

## Done (committed)
1. `093485e` Margin decoder `rust/crates/zax-nes/src/wide_decode/adventure_island.rs` (unpatched ROM,
   `for_rom`, and the patch, `for_patch`): round tables pointed to by RAM `$1A-$27`, 56-tile columns,
   slopes (`$31` per column, `$BE5D/$BE63`), vertical scrolling (`$32`), overlays the game draws over
   columns (cloud platforms, `$A629`) taken from the nametables for columns already drawn. Checked on
   all 32 rounds (see WIDESCREEN-EXTENSION.md section 4).
2. `4698551` Core fix `ppu/tiles.rs`: second-page sprites were dropped in bands without console sprites
   (affected SMB1 too).
3. `37d9080` Advanced patch `rust/crates/zax-nes/patches/adventure-island-usa-widescreen.bps` +
   `builtin_patches.rs` entry (advanced, boot_window (0x82,0), idle_loop B759-B75C), options
   level_select (Select on title), invincible, no_slowdown. Sprites with 16-bit x in the margins,
   left-edge "ghost" objects (behaviour runs, no collision/sound/spawn). `for_game_name` ignores
   sequels (Adventure Island II, SMB 3). Docs: README, HOWTO-RUN, WIDESCREEN-EXTENSION, hack doc.
4. Last commit (this one): decoder vertical margin (`decode_extra_row`; columns' first/last metatile
   repeated past top/bottom), harness `--png-nes` as tall as the margins, this handoff.
   NOT yet run after the last edit: `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D
   warnings` and `cargo test -p zax-nes` (fmt done; the two clippy errors were fixed just before).
   Run `rust/check.sh`. Known pre-existing failure unrelated: `zax-harness` `bench_core_tests`
   (`rust/test-roms/nestest.nes` absent). Tests need `libsdl2-dev`.

## Hack sources (not elsewhere in the repo)
`handoff/src/wide.s`, `handoff/tools/build.py`, `make_bps.py`, `make_doc.py` (also reproduced in the
hack doc's appendices). Rebuild: `python -I tools/build.py "Adventure Island (USA).nes"
build/aiwide.bps` (needs cc65) → copy to `rust/crates/zax-nes/patches/`; target CRC must be
`4CDF2FE8` for the committed patch (update the hack doc if it changes; `make_doc.py` regenerates
appendices).

## Test scripts (scratch, in `handoff/tools`, paths are absolute to /tmp/claude-0, adapt)
`tour.py area round frames name [margin] [speed] [harness args]`: starts a round by pokes ($37/$38),
forces scrolling by poking player x ($0584) and death ($72): fast decoder survey.
`play.py level frames name [options] [margin]`: patched ROM, level select + right+B + jumps.
`parity.py frames [args]`: same inputs on original vs patched, RAM diff (margin 0 → only stack bytes
differ; margin 96 → same events). `fm2press.py`: FCEUX .fm2 → `--press`; the disassembly's
walkthrough movie desyncs in Zaxnes (enemy timing), not usable. 3D render needs EGL (absent here):
use `--png-nes`, `--dump-mem` (RAM; extension RAM at file offset 0x2800).

## Remaining ideas / limits (also in the hack doc section 7)
- Objects are created at x=320; past a 64 px margin they pop in there (would need early dormant
  spawning; risky for slot order).
- Overlays (clouds) appear in the right margin only when the game draws the column (could read the
  object stream ahead, `$8C5A`/`sub_8D74`).
- Margins past the round's end are empty (boss room).
- Not tested: bonus stages (cloud/pot rooms), the ending, save states made the other way, the app UI
  (hacks screen should show "Advanced mode" generically).
- The owner asked for level select + invincibility to be used with Zaxnes movies (`.zmv`) for
  anomaly hunting: done as options; a movie test campaign is still to do.
- When done: merge into `dev` like `feat/widescreen` (merge commit), owner decides.
