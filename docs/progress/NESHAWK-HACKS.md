# NESHAWK-HACKS — hack packs and widescreen on the NesHawk engine

Branch `advanced-neshawk`, from `rewrite-ui` 91ffefce, worktree `G:\3DSen\wt-advanced-neshawk`
(its own target directory). Steps 1-2 were first written on a branch inside `wt-rewrite-ui`, which
another session was using: set aside in `G:\3DSen\neshawk-hacks-parked`, re-applied here.

## The request (owner, 2026-11-18)

The hacks and the widescreen active on the NesHawk engine as they are on Zaxnes (standard and
fast core), **without altering the timings in any way**, above all with the widescreen off, so
that `.bk2` movies keep passing as reliably as now.

## Why it is possible, and the rule that keeps the timings

Everything the widescreen and the packs do is already in `zax-nes` as engine-agnostic pieces that
`zax-neshawk` can call (it depends on `zax-nes` already): `wide::WideExtension` (the `$5000`
window, the execution zone, the overclock counter), `hack_pack` (the hooks), `builtin_patches`
(the BPS patches, the packs, the options), `wide_decode` (the margin decoders, which read a
`DecoderMemory`). What `zax-nes` has and NesHawk has not is the *wiring*: the bus hooks, the
overclock stall, the margin columns of the tile capture, the host API.

**The rule:** the extension is an `Option` on NesHawk's bus, **`None` unless a built-in patch was
applied or an aid is on**. With the widescreen off (the application then applies no built-in
patch: `builtin_patches_wanted` needs `VideoWidescreen != 0`) and no aid on, the machine is the
NesHawk port exactly as today: the same reads, writes and cycles, the same savestate bytes. The
only added instructions are a `None` test in the `$2000`-`$5FFF` branches of the bus (never on the
RAM/ROM fast path) and on instruction-stream reads; they change no emulated state.

With the widescreen on, the game runs the patched ROM and possibly the overclock ("No slowdown"):
like on Zaxnes, a movie recorded on the original game cannot sync with that, by nature. Inside a
frame the NesHawk cycle order is kept; the overclock adds CPU-only cycles at the start of line 240,
as Zaxnes does (PPU and APU still, the board clocked).

## Steps

- [x] 1. Bus layer in `zax-neshawk`, gated: `$5000` window (listen on writes, answer reads only
      once unlocked, `$500A` options), PRG window, execution zone on instruction-stream reads
      (`Mos6502xLink::read_code`, default = `read_memory`), zone data at `$3000`-`$3FFF`,
      sprite high bytes latched at OAM DMA, four screens.
- [x] 2. Overclock: CPU-only cycles at line 240 (board clocked, PPU/APU still), ended at the
      patch's `idle_loop`.
- [x] 3. `NesHawkEngine`: built-in patch applied to the image before the machine is built, the
      pack installed, aids applied to the board's PRG, options; the extension saved in the
      block only when present (a flag bit, so blocks without it are byte-identical to today's).
- [x] 4. Tile capture with margins (the columns beyond 256 px, the decoders, sprite x high
      bytes, tint), `FrameBatch` margin fields, `wide_picture_into`.
- [x] 5. Application: `Capabilities::NESHAWK` widescreen + hack packs, the host setters routed to
      NesHawk, the idle-loop savestates and components.
- [ ] 6. Proof: the `.bk2` corpus (Kirby whole movie, Faxanadu whole movie) frame-for-frame
      identical (work RAM + frame hash) to the build before the lot, widescreen off and on for a
      ROM without a pack; the movie-sync tests; then the widescreen packs played on NesHawk
      against Zaxnes.

## Progress

- Steps 1-2 (04762d5e): `crate::wide` (bus hooks, `read_code`, overclock via
  `run_cpu_cycle::<false>`), harness `ZAXNES_NESHAWK_WIDE=1` attaches a locked extension.
- Step 3 (cc5f465c): `crate::hacks`, engine API, `FLAG_WIDE` savestate block. `tests/hacks.rs`
  with `ZAXNES_ROMS=G:\3DSen\demo\roms`: nothing attached by default; a locked extension leaves
  600 frames of work RAM identical on every demo ROM and its states restore; the 13 patched demo
  ROMs (Adventure Island, Blaster Master, Castlevania x2, Excitebike, FF1, MM2, Metroid, NG3,
  SMB1, SMB3, Zelda II x2) run their patch, unlock the extension, states restore.
- Proof runs: `tools/neshawk-hacks/bk2-trace.sh`, outputs in `G:\3DSen\neshawk-hacks-runs`
  (baseline = rewrite-ui 70516d6b build, the core being identical up to 91ffefce).
- Step 4 (443e3f9c): `tiles.rs` (wide rows, sprite x high bytes, second page), `wide_decode.rs`
  (zax_nes's decoders run after the frame on the engine's copy of the records, each kept row with
  its 4 KiB pattern table and nametable map), `wide_picture.rs` (the wide 2D picture), margin
  sprites out of NesHawk's sprite evaluation, nametable write stamps. Two overclock bugs found by
  comparing with Zaxnes: the boards' `ClockPPU` (MMC1's write filter dropped every bank switch of
  the overclock after the first) and the PPU's delayed `$2001`/`$2006` installs (Metroid's CHR
  uploads went to a nametable) now keep running while the PPU is held.
  `dump_wide_pictures_of_both_engines` (1 500 frames of a scripted run, margin 64): 12 of the 13
  patched demo ROMs give **the same wide picture to the pixel** on NesHawk and Zaxnes. Ninja
  Gaiden III parts from Zaxnes near frame 700 **without any patch** (an emulation difference of
  the two engines, not of the lot).
- Step 5 (87d80ae3): `Capabilities::NESHAWK` widescreen + hack packs, `EngineHandle` routing,
  options greyed by capability (FCEUX unchanged). `.bk2` on NesHawk: `plain_for_bk2` keeps the
  machine plain (no patch, no aid, no margin, no extension) while the movie is set up and plays.
- Not done (Zaxnes has them): vertical margins and `hides_overscan`, smooth scrolling and its
  playfield end, steady sprites, sprites drawn over the centre by the wide picture (second page,
  across the left edge), four screens (`$5007` bit 0; no demo pack uses it), idle-loop savestates
  (NesHawk saves where it is), switching the widescreen mid-game (takes effect at the next load),
  aids on the boards that fall back to Zaxnes's (`boards::zaxnes`).
- Environment, 2026-11-18: C: full (20-40 MB free) so the page file cannot grow; nine harness
  processes at once died of failed allocations. `prove.sh` runs three at a time. Build with
  `TMP`/`TEMP` on G: and `--features zax-app/bundled` for the app tests, in release (the debug
  test binaries of zax-app took 11 GB and filled G:).
