# NESHAWK-HACKS — hack packs and widescreen on the NesHawk engine

Branch `advanced-neshawk`, from `rewrite-ui` 91ffefce, worktree `G:DSen\wt-advanced-neshawk`
(its own target directory). Steps 1-2 were first written on a branch inside `wt-rewrite-ui`, which
another session was using: set aside in `G:DSen
eshawk-hacks-parked`, re-applied here.

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
- [ ] 4. Tile capture with margins (the columns beyond 256 px, the decoders, sprite x high
      bytes, tint), `FrameBatch` margin fields, `wide_picture_into`.
- [ ] 5. Application: `Capabilities::NESHAWK` widescreen + hack packs, the host setters routed to
      NesHawk, the idle-loop savestates and components.
- [ ] 6. Proof: the `.bk2` corpus (Kirby whole movie, Faxanadu whole movie) frame-for-frame
      identical (work RAM + frame hash) to the build before the lot, widescreen off and on for a
      ROM without a pack; the movie-sync tests; then the widescreen packs played on NesHawk
      against Zaxnes.

## Progress

- Steps 1-2 (04762d5e): `crate::wide` (bus hooks, `read_code`, overclock via
  `run_cpu_cycle::<false>`), harness `ZAXNES_NESHAWK_WIDE=1` attaches a locked extension.
- Step 3 (cc5f465c): `crate::hacks`, engine API, `FLAG_WIDE` savestate block. `tests/hacks.rs`
  with `ZAXNES_ROMS=G:DSen\demo
oms`: nothing attached by default; a locked extension leaves
  600 frames of work RAM identical on every demo ROM and its states restore; the 13 patched demo
  ROMs (Adventure Island, Blaster Master, Castlevania x2, Excitebike, FF1, MM2, Metroid, NG3,
  SMB1, SMB3, Zelda II x2) run their patch, unlock the extension, states restore.
- Proof runs: `tools/neshawk-hacks/bk2-trace.sh`, outputs in `G:DSen
eshawk-hacks-runs`
  (baseline = rewrite-ui 70516d6b build, the core being identical up to 91ffefce).
- Step 4 next: `zax_nes::ppu::tiles` (1 800 lines on `Ppu2C02`) is the reference; the plan is a
  NesHawk-side copy adapted to this PPU, leaving `zax-nes` untouched.
